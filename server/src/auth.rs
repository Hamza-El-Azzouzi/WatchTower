use anyhow::Result;
use bcrypt::verify;
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

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

/// Admin user for dashboard access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUser {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing)]
    #[allow(dead_code)]
    pub password_hash: String,
    pub email: Option<String>,
    pub full_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

/// Admin login request
#[derive(Debug, Deserialize)]
pub struct AdminLoginRequest {
    pub username: String,
    pub password: String,
}

/// Admin login response
#[derive(Debug, Serialize)]
pub struct AdminLoginResponse {
    pub username: String,
    pub full_name: Option<String>,
    pub token: String, // Session token
}

/// Authentication service for managing API keys
pub struct AuthService {
    pool: SqlitePool,
}

impl AuthService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
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
            WHERE username = ? AND is_active = 1
            "#
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            let password_hash: String = row.get("password_hash");

            // Verify password
            if verify(password, &password_hash).unwrap_or(false) {
                // Update last login
                sqlx::query("UPDATE admin_users SET last_login_at = ? WHERE username = ?")
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
                    last_login_at: row.get("last_login_at"),
                    is_active: row.get("is_active"),
                }));
            }
        }

        Ok(None)
    }

    /// Generate admin session token
    pub fn generate_admin_token() -> String {
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        use base64::{engine::general_purpose, Engine as _};
        format!(
            "admin_{}",
            general_purpose::URL_SAFE_NO_PAD.encode(&random_bytes)
        )
    }

    /// Create a new API key
    pub async fn create_api_key(
        &self,
        request: CreateApiKeyRequest,
        created_by: &str,
    ) -> Result<CreateApiKeyResponse> {
        let key = ApiKey::generate();
        let expires_at = request
            .expires_in_days
            .map(|days| Utc::now() + Duration::days(days));

        sqlx::query(
            "INSERT INTO api_keys (key, name, description, expires_at, created_by, max_agents)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&key)
        .bind(&request.name)
        .bind(&request.description)
        .bind(expires_at)
        .bind(created_by)
        .bind(request.max_agents)
        .execute(&self.pool)
        .await?;

        Ok(CreateApiKeyResponse {
            key,
            name: request.name,
            expires_at,
        })
    }

    /// Validate an API key with agent limit enforcement
    pub async fn validate_api_key_with_agent(
        &self,
        key: &str,
        agent_id: Option<&str>,
    ) -> Result<Option<ApiKey>> {
        let api_key = self.validate_api_key(key).await?;

        if let Some(ref key_data) = api_key {
            // Check agent limit if agent_id is provided
            if let (Some(agent), Some(max)) = (agent_id, key_data.max_agents) {
                if !key_data.used_by_agents.contains(&agent.to_string()) {
                    // New agent trying to use the key
                    if key_data.used_by_agents.len() >= max as usize {
                        return Ok(None); // Agent limit reached
                    }
                    // Add agent to the list
                    let mut agents = key_data.used_by_agents.clone();
                    agents.push(agent.to_string());
                    let agents_json = serde_json::to_string(&agents)?;

                    sqlx::query("UPDATE api_keys SET used_by_agents = ? WHERE key = ?")
                        .bind(agents_json)
                        .bind(key)
                        .execute(&self.pool)
                        .await?;
                }
            }
        }

        Ok(api_key)
    }

    /// Validate an API key and update last_used_at
    pub async fn validate_api_key(&self, key: &str) -> Result<Option<ApiKey>> {
        let result = sqlx::query(
            r#"
            SELECT id, key, name, description, created_at, expires_at, last_used_at, revoked, revoked_at, created_by, max_agents, used_by_agents
            FROM api_keys
            WHERE key = ? AND revoked = 0
            "#
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        let api_key = if let Some(row) = result {
            let used_by_agents: Vec<String> = row
                .get::<Option<String>, _>("used_by_agents")
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            Some(ApiKey {
                id: row.get("id"),
                key: row.get("key"),
                name: row.get("name"),
                description: row.get("description"),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                last_used_at: row.get("last_used_at"),
                revoked: row.get::<i32, _>("revoked") != 0,
                revoked_at: row.get("revoked_at"),
                created_by: row.get("created_by"),
                max_agents: row.get("max_agents"),
                used_by_agents,
            })
        } else {
            None
        };

        if let Some(ref api_key) = api_key {
            if api_key.is_valid() {
                // Update last_used_at
                sqlx::query("UPDATE api_keys SET last_used_at = ? WHERE key = ?")
                    .bind(Utc::now())
                    .bind(key)
                    .execute(&self.pool)
                    .await?;

                return Ok(Some(api_key.clone()));
            }
        }

        Ok(None)
    }

    /// List all API keys (without exposing the actual key value)
    pub async fn list_api_keys(&self) -> Result<Vec<ApiKey>> {
        let rows = sqlx::query(
            r#"
            SELECT id, key, name, description, created_at, expires_at, last_used_at, revoked, revoked_at, created_by, max_agents, used_by_agents
            FROM api_keys
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let keys = rows
            .iter()
            .map(|row| {
                let used_by_agents: Vec<String> = row
                    .get::<Option<String>, _>("used_by_agents")
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                ApiKey {
                    id: row.get("id"),
                    key: row.get("key"),
                    name: row.get("name"),
                    description: row.get("description"),
                    created_at: row.get("created_at"),
                    expires_at: row.get("expires_at"),
                    last_used_at: row.get("last_used_at"),
                    revoked: row.get::<i32, _>("revoked") != 0,
                    revoked_at: row.get("revoked_at"),
                    created_by: row.get("created_by"),
                    max_agents: row.get("max_agents"),
                    used_by_agents,
                }
            })
            .collect();

        Ok(keys)
    }

    /// Revoke an API key
    pub async fn revoke_api_key(&self, key_id: i64) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE api_keys SET revoked = 1, revoked_at = ? WHERE id = ? AND revoked = 0",
        )
        .bind(Utc::now())
        .bind(key_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get an API key by ID
    #[allow(dead_code)]
    pub async fn get_api_key(&self, key_id: i64) -> Result<Option<ApiKey>> {
        let result = sqlx::query(
            r#"
            SELECT id, key, name, description, created_at, expires_at, last_used_at, revoked, revoked_at, created_by, max_agents, used_by_agents
            FROM api_keys
            WHERE id = ?
            "#
        )
        .bind(key_id)
        .fetch_optional(&self.pool)
        .await?;

        let key = if let Some(row) = result {
            let used_by_agents: Vec<String> = row
                .get::<Option<String>, _>("used_by_agents")
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            Some(ApiKey {
                id: row.get("id"),
                key: row.get("key"),
                name: row.get("name"),
                description: row.get("description"),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                last_used_at: row.get("last_used_at"),
                revoked: row.get::<i32, _>("revoked") != 0,
                revoked_at: row.get("revoked_at"),
                created_by: row.get("created_by"),
                max_agents: row.get("max_agents"),
                used_by_agents,
            })
        } else {
            None
        };

        Ok(key)
    }

    /// Change admin password
    pub async fn change_admin_password(
        &self,
        username: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<bool> {
        // First authenticate with old password
        let admin = self.authenticate_admin(username, old_password).await?;

        if admin.is_none() {
            return Ok(false);
        }

        // Hash the new password
        let new_hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)?;

        // Update the password
        sqlx::query("UPDATE admin_users SET password_hash = ? WHERE username = ?")
            .bind(&new_hash)
            .bind(username)
            .execute(&self.pool)
            .await?;

        Ok(true)
    }

    /// Create a new admin user
    pub async fn create_admin(
        &self,
        username: &str,
        password: &str,
        email: Option<&str>,
        full_name: Option<&str>,
    ) -> Result<i64> {
        let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;

        let result = sqlx::query(
            "INSERT INTO admin_users (username, password_hash, email, full_name) VALUES (?, ?, ?, ?)"
        )
        .bind(username)
        .bind(&password_hash)
        .bind(email)
        .bind(full_name)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// List all admin users
    pub async fn list_admins(&self) -> Result<Vec<AdminUser>> {
        let rows = sqlx::query(
            r#"
            SELECT id, username, password_hash, email, full_name, created_at, last_login_at, is_active
            FROM admin_users
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let admins = rows
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

        Ok(admins)
    }

    /// Deactivate an admin user
    pub async fn deactivate_admin(&self, username: &str) -> Result<()> {
        sqlx::query("UPDATE admin_users SET is_active = 0 WHERE username = ?")
            .bind(username)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Activate an admin user
    pub async fn activate_admin(&self, username: &str) -> Result<()> {
        sqlx::query("UPDATE admin_users SET is_active = 1 WHERE username = ?")
            .bind(username)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
