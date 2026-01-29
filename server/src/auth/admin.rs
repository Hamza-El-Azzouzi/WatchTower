#![allow(dead_code)] // Functions will be used when admin routes are fully integrated

use anyhow::{Context, Result};
use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;

const JWT_SECRET: &str = "your-secret-key-change-this-in-production"; // TODO: Move to config
const TOKEN_EXPIRY_HOURS: i64 = 8; // 8 hours

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUser {
    pub id: i32,
    pub username: String,
    pub full_name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: i32, // User ID
    username: String,
    exp: usize, // Expiry timestamp
    iat: usize, // Issued at
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: AdminUser,
    pub expires_at: String,
}

// Admin login handler
pub async fn admin_login(
    State(pool): State<Arc<PgPool>>,
    Json(login): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    // Fetch user from database
    let user_row = sqlx::query(
        "SELECT id, username, password_hash, full_name, email 
         FROM admin_users 
         WHERE username = $1 AND is_active = true",
    )
    .bind(&login.username)
    .fetch_optional(pool.as_ref())
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user_row = user_row.ok_or((
        StatusCode::UNAUTHORIZED,
        "Invalid username or password".to_string(),
    ))?;

    let user_id: i32 = user_row.get("id");
    let username: String = user_row.get("username");
    let password_hash: String = user_row.get("password_hash");
    let full_name: Option<String> = user_row.get("full_name");
    let email: Option<String> = user_row.get("email");

    // Verify password
    let valid = bcrypt::verify(&login.password, &password_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid username or password".to_string(),
        ));
    }

    // Update last login
    let _ = sqlx::query("UPDATE admin_users SET last_login = NOW() WHERE id = $1")
        .bind(user_id)
        .execute(pool.as_ref())
        .await;

    // Generate JWT token
    let now = Utc::now();
    let expires_at = now + Duration::hours(TOKEN_EXPIRY_HOURS);

    let claims = Claims {
        sub: user_id,
        username: username.clone(),
        iat: now.timestamp() as usize,
        exp: expires_at.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_ref()),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(LoginResponse {
        token,
        user: AdminUser {
            id: user_id,
            username,
            full_name,
            email,
        },
        expires_at: expires_at.to_rfc3339(),
    }))
}

/// Generate a JWT token for an admin user (public helper)
pub fn generate_jwt_token(username: &str, _password: &str) -> String {
    let now = Utc::now();
    let expires_at = now + Duration::hours(TOKEN_EXPIRY_HOURS);

    let claims = Claims {
        sub: 1, // Default ID for simple token generation
        username: username.to_string(),
        iat: now.timestamp() as usize,
        exp: expires_at.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_ref()),
    )
    .unwrap_or_default()
}

// Middleware to verify admin JWT token
pub async fn admin_auth_middleware(
    State(pool): State<Arc<PgPool>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract token from Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Decode and validate token
    let token_data = decode::<Claims>(
        auth_header,
        &DecodingKey::from_secret(JWT_SECRET.as_ref()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Verify user still exists and is active
    let user_row = sqlx::query(
        "SELECT id, username, full_name, email 
         FROM admin_users 
         WHERE id = $1 AND is_active = true",
    )
    .bind(token_data.claims.sub)
    .fetch_optional(pool.as_ref())
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::UNAUTHORIZED)?;

    let admin_user = AdminUser {
        id: user_row.get("id"),
        username: user_row.get("username"),
        full_name: user_row.get("full_name"),
        email: user_row.get("email"),
    };

    // Add admin user to request extensions
    request.extensions_mut().insert(admin_user);

    Ok(next.run(request).await)
}

// Create a new admin user (for setup/migrations)
pub async fn create_admin_user(
    pool: &PgPool,
    username: &str,
    password: &str,
    full_name: Option<&str>,
    email: Option<&str>,
) -> Result<i32> {
    let password_hash =
        bcrypt::hash(password, bcrypt::DEFAULT_COST).context("Failed to hash password")?;

    let row = sqlx::query(
        "INSERT INTO admin_users (username, password_hash, full_name, email)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (username) DO NOTHING
         RETURNING id",
    )
    .bind(username)
    .bind(password_hash)
    .bind(full_name)
    .bind(email)
    .fetch_one(pool)
    .await
    .context("Failed to create admin user")?;

    Ok(row.get("id"))
}

// Update admin password
pub async fn update_admin_password(
    pool: &PgPool,
    username: &str,
    old_password: &str,
    new_password: &str,
) -> Result<(), String> {
    // Verify old password
    let user_row = sqlx::query("SELECT password_hash FROM admin_users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("User not found")?;

    let current_hash: String = user_row.get("password_hash");
    let valid = bcrypt::verify(old_password, &current_hash).map_err(|e| e.to_string())?;

    if !valid {
        return Err("Invalid old password".to_string());
    }

    // Hash new password
    let new_hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST).map_err(|e| e.to_string())?;

    // Update password
    sqlx::query("UPDATE admin_users SET password_hash = $1 WHERE username = $2")
        .bind(new_hash)
        .bind(username)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
