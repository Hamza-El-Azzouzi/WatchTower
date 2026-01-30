#![allow(dead_code)] // Some methods are for future use

use anyhow::Result;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

/// API key for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: i64,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked: bool,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_by: String,
    pub max_agents: Option<i32>,
    pub used_by_agents: Vec<String>,
}

/// Request to create a new API key
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub description: Option<String>,
    pub expires_in_days: Option<i64>,
    pub max_agents: Option<i32>,
}

/// Response when creating an API key
#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub key: String,
    pub name: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub max_agents: Option<i32>,
}

/// Admin user for dashboard access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUser {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub email: Option<String>,
    pub full_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

/// Login request for admin authentication
#[derive(Debug, Deserialize)]
pub struct AdminLoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response with JWT token
#[derive(Debug, Serialize)]
pub struct AdminLoginResponse {
    pub token: String,
    pub username: String,
    pub full_name: Option<String>,
}

impl ApiKey {
    /// Generate a new secure API key
    pub fn generate() -> String {
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        use base64::{engine::general_purpose, Engine as _};
        format!(
            "msk_{}",
            general_purpose::URL_SAFE_NO_PAD.encode(&random_bytes)
        )
    }

    /// Check if the API key is valid (not expired and not revoked)
    pub fn is_valid(&self) -> bool {
        if self.revoked {
            return false;
        }

        if let Some(expires_at) = self.expires_at {
            if Utc::now() > expires_at {
                return false;
            }
        }

        true
    }
}

/// Authentication service for managing API keys
pub struct AuthService {
    pool: PgPool,
}

impl AuthService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Validate an API key and return the key ID if valid
    pub async fn validate_api_key(&self, key: &str) -> Result<Option<i64>> {
        let result = sqlx::query(
            r#"
            SELECT id, revoked, expires_at
            FROM api_keys
            WHERE key = $1
            "#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            let revoked: bool = row.get("revoked");
            if revoked {
                return Ok(None);
            }

            let expires_at: Option<DateTime<Utc>> = row.get("expires_at");
            if let Some(exp_time) = expires_at {
                if Utc::now() > exp_time {
                    return Ok(None);
                }
            }

            // Update last_used_at
            let key_id: i64 = row.get("id");
            sqlx::query("UPDATE api_keys SET last_used_at = $1 WHERE id = $2")
                .bind(Utc::now())
                .bind(key_id)
                .execute(&self.pool)
                .await?;

            Ok(Some(key_id))
        } else {
            Ok(None)
        }
    }

    /// Create new API key
    pub async fn create_api_key(
        &self,
        request: CreateApiKeyRequest,
    ) -> Result<CreateApiKeyResponse> {
        let key = ApiKey::generate();
        let expires_at = request
            .expires_in_days
            .map(|days| Utc::now() + Duration::days(days));

        sqlx::query(
            r#"
            INSERT INTO api_keys (key, name, description, expires_at, created_by, max_agents, created_at)
            VALUES ($1, $2, $3, $4, 'admin', $5, $6)
            "#,
        )
        .bind(&key)
        .bind(&request.name)
        .bind(&request.description)
        .bind(expires_at)
        .bind(request.max_agents)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        Ok(CreateApiKeyResponse {
            key,
            name: request.name,
            expires_at,
            max_agents: request.max_agents,
        })
    }

    /// List all API keys
    pub async fn list_api_keys(&self) -> Result<Vec<ApiKey>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, key, name, description, created_at, expires_at,
                last_used_at, revoked, revoked_at, 'admin' as created_by, max_agents
            FROM api_keys
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut keys = Vec::new();
        for row in rows {
            let key_id: i64 = row.get("id");

            // Get agents using this key
            let agent_rows = sqlx::query("SELECT id FROM agents WHERE api_key_id = $1")
                .bind(key_id)
                .fetch_all(&self.pool)
                .await?;

            let used_by_agents: Vec<String> = agent_rows.into_iter().map(|r| r.get("id")).collect();

            keys.push(ApiKey {
                id: key_id,
                key: row.get("key"),
                name: row.get("name"),
                description: row.get("description"),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                last_used_at: row.get("last_used_at"),
                revoked: row.get("revoked"),
                revoked_at: row.get("revoked_at"),
                created_by: row.get("created_by"),
                max_agents: row.get("max_agents"),
                used_by_agents,
            });
        }

        Ok(keys)
    }

    /// Revoke an API key
    pub async fn revoke_api_key(&self, key_id: i64) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE api_keys
            SET revoked = true, revoked_at = $1
            WHERE id = $2
            "#,
        )
        .bind(Utc::now())
        .bind(key_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Validate API key with agent limit enforcement
    pub async fn validate_api_key_with_agent(
        &self,
        key: &str,
        agent_id: Option<&str>,
    ) -> Result<Option<i64>> {
        // First validate the key itself
        match self.validate_api_key(key).await? {
            Some(key_id) => {
                // If we have an agent_id, check limits
                if let Some(aid) = agent_id {
                    // Get max_agents for this key
                    let max_result = sqlx::query("SELECT max_agents FROM api_keys WHERE id = $1")
                        .bind(key_id)
                        .fetch_optional(&self.pool)
                        .await?;

                    if let Some(row) = max_result {
                        let max_agents: Option<i32> = row.get("max_agents");

                        if let Some(max) = max_agents {
                            // Count current agents using this key (excluding the current agent id)
                            let count_result = sqlx::query(
                                "SELECT COUNT(*) as count FROM agents WHERE api_key_id = $1 AND id != $2"
                            )
                            .bind(key_id)
                            .bind(aid)
                            .fetch_one(&self.pool)
                            .await?;

                            let current_count: i64 = count_result.get("count");

                            if current_count >= max as i64 {
                                return Ok(None); // Limit reached
                            }
                        }
                    }
                }

                Ok(Some(key_id))
            }
            None => Ok(None),
        }
    }

    /// Authenticate admin user
    pub async fn authenticate_admin(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<AdminUser>> {
        let result = sqlx::query(
            r#"
            SELECT id, username, password_hash, email, full_name, created_at, last_login_at, is_active
            FROM admin_users
            WHERE username = $1 AND is_active = true
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            let password_hash: String = row.get("password_hash");

            if verify(password, &password_hash).unwrap_or(false) {
                // Update last login
                sqlx::query("UPDATE admin_users SET last_login_at = $1 WHERE username = $2")
                    .bind(Utc::now())
                    .bind(username)
                    .execute(&self.pool)
                    .await?;

                return Ok(Some(AdminUser {
                    id: row.get("id"),
                    username: row.get("username"),
                    password_hash,
                    email: row.get("email"),
                    full_name: row.get("full_name"),
                    created_at: row.get("created_at"),
                    last_login_at: Some(Utc::now()),
                    is_active: row.get("is_active"),
                }));
            }
        }

        Ok(None)
    }

    /// Generate admin JWT token
    pub fn generate_admin_token() -> String {
        use crate::auth::admin::generate_jwt_token;
        generate_jwt_token("admin", "admin")
    }

    /// Change admin password
    pub async fn change_admin_password(
        &self,
        username: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<()> {
        // First verify the old password
        match self.authenticate_admin(username, old_password).await? {
            Some(_) => {
                // Hash and update the new password
                let password_hash = hash(new_password, DEFAULT_COST)?;

                sqlx::query(
                    r#"
                    UPDATE admin_users
                    SET password_hash = $1
                    WHERE username = $2
                    "#,
                )
                .bind(&password_hash)
                .bind(username)
                .execute(&self.pool)
                .await?;

                Ok(())
            }
            None => anyhow::bail!("Invalid username or password"),
        }
    }

    /// Create a new admin user
    pub async fn create_admin(
        &self,
        username: &str,
        password: &str,
        email: Option<String>,
        full_name: Option<String>,
    ) -> Result<AdminUser> {
        let password_hash = hash(password, DEFAULT_COST)?;

        sqlx::query(
            r#"
            INSERT INTO admin_users (username, password_hash, email, full_name, created_at, is_active)
            VALUES ($1, $2, $3, $4, $5, true)
            "#,
        )
        .bind(username)
        .bind(&password_hash)
        .bind(&email)
        .bind(&full_name)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        Ok(AdminUser {
            id: 0, // Will be assigned by DB
            username: username.to_string(),
            password_hash,
            email,
            full_name,
            created_at: Utc::now(),
            last_login_at: None,
            is_active: true,
        })
    }

    /// List all admin users
    pub async fn list_admins(&self) -> Result<Vec<AdminUser>> {
        let rows = sqlx::query(
            r#"
            SELECT id, username, password_hash, email, full_name, created_at, last_login_at, is_active
            FROM admin_users
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?
            .into_iter()
            .map(|row| AdminUser {
                id: row.get("id"),
                username: row.get("username"),
                password_hash: row.get("password_hash"),
                email: row.get("email"),
                full_name: row.get("full_name"),
                created_at: row.get("created_at"),
                last_login_at: row.get("last_login_at"),
                is_active: row.get("is_active"),
            })
            .collect();

        Ok(rows)
    }

    /// Deactivate an admin user
    pub async fn deactivate_admin(&self, username: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE admin_users
            SET is_active = false
            WHERE username = $1
            "#,
        )
        .bind(username)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            anyhow::bail!("User not found");
        }

        Ok(())
    }

    /// Activate an admin user
    pub async fn activate_admin(&self, username: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE admin_users
            SET is_active = true
            WHERE username = $1
            "#,
        )
        .bind(username)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            anyhow::bail!("User not found");
        }

        Ok(())
    }
}
