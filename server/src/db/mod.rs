#![allow(dead_code)] // Some methods are for future use/background jobs

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;
use tracing::info;

pub struct Database {
    pool: PgPool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DatabaseStats {
    pub agents: u64,
    pub metrics: u64,
    pub metrics_1min: u64,
    pub metrics_1hour: u64,
    pub logs: u64,
    pub alert_rules: u64,
    pub alerts: u64,
    pub api_keys: u64,
    pub admin_users: u64,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("Connecting to PostgreSQL database");

        // Create connection pool with production settings
        let pool = PgPoolOptions::new()
            .max_connections(50)
            .min_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .connect(database_url)
            .await
            .context("Failed to connect to PostgreSQL")?;

        let db = Self { pool };

        // Run migrations automatically
        db.run_migrations().await?;

        info!("Database connection established successfully");
        Ok(db)
    }

    async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations");

        // Use SQLx's built-in migration runner
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .context("Failed to run database migrations")?;

        info!("Database migrations completed successfully");
        Ok(())
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // Check if database is healthy
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .context("Database health check failed")?;
        Ok(())
    }

    // Get comprehensive database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        let row = sqlx::query(
            "SELECT 
                (SELECT COUNT(*) FROM agents) as agents,
                (SELECT COUNT(*) FROM metrics) as metrics,
                (SELECT COUNT(*) FROM metrics_1min) as metrics_1min,
                (SELECT COUNT(*) FROM metrics_1hour) as metrics_1hour,
                (SELECT COUNT(*) FROM logs) as logs,
                (SELECT COUNT(*) FROM alert_rules) as rules,
                (SELECT COUNT(*) FROM alerts) as alerts,
                (SELECT COUNT(*) FROM api_keys WHERE NOT revoked) as api_keys,
                (SELECT COUNT(*) FROM admin_users WHERE is_active) as admin_users",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(DatabaseStats {
            agents: row.get::<i64, _>("agents") as u64,
            metrics: row.get::<i64, _>("metrics") as u64,
            metrics_1min: row.get::<i64, _>("metrics_1min") as u64,
            metrics_1hour: row.get::<i64, _>("metrics_1hour") as u64,
            logs: row.get::<i64, _>("logs") as u64,
            alert_rules: row.get::<i64, _>("rules") as u64,
            alerts: row.get::<i64, _>("alerts") as u64,
            api_keys: row.get::<i64, _>("api_keys") as u64,
            admin_users: row.get::<i64, _>("admin_users") as u64,
        })
    }

    // Data Retention: Clean up old raw metrics (keep last 24 hours)
    pub async fn cleanup_old_raw_metrics(&self, retention_hours: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::hours(retention_hours);

        let result = sqlx::query("DELETE FROM metrics WHERE timestamp < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    // Data Retention: Clean up old 1-minute aggregates (keep last 7 days)
    pub async fn cleanup_old_minute_aggregates(&self, retention_days: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(retention_days);

        let result = sqlx::query("DELETE FROM metrics_1min WHERE timestamp < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    // Data Retention: Clean up old 1-hour aggregates (keep last 30 days)
    pub async fn cleanup_old_hour_aggregates(&self, retention_days: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(retention_days);

        let result = sqlx::query("DELETE FROM metrics_1hour WHERE timestamp < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    // Data Retention: Clean up old logs
    pub async fn cleanup_old_logs(&self, retention_hours: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::hours(retention_hours);

        let result = sqlx::query("DELETE FROM logs WHERE timestamp < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    // Clean up resolved alerts older than specified days
    pub async fn cleanup_old_alerts(&self, retention_days: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(retention_days);

        let result = sqlx::query(
            "DELETE FROM alerts 
             WHERE state = 'resolved' 
             AND resolved_at < $1",
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    // Aggregation: Create 1-minute aggregates from raw metrics
    pub async fn aggregate_metrics_to_1min(&self, hours_back: i64) -> Result<u64> {
        let start_time = Utc::now() - chrono::Duration::hours(hours_back);

        let result = sqlx::query(
            "INSERT INTO metrics_1min (agent_id, metric_name, min_value, max_value, avg_value, sum_value, count, timestamp)
             SELECT 
                 agent_id,
                 metric_name,
                 MIN(value) as min_value,
                 MAX(value) as max_value,
                 AVG(value) as avg_value,
                 SUM(value) as sum_value,
                 COUNT(*) as count,
                 DATE_TRUNC('minute', timestamp) as timestamp
             FROM metrics
             WHERE timestamp >= $1
             GROUP BY agent_id, metric_name, DATE_TRUNC('minute', timestamp)
             ON CONFLICT (agent_id, metric_name, timestamp) DO NOTHING"
        )
        .bind(start_time)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    // Aggregation: Create 1-hour aggregates from 1-minute aggregates
    pub async fn aggregate_metrics_to_1hour(&self, days_back: i64) -> Result<u64> {
        let start_time = Utc::now() - chrono::Duration::days(days_back);

        let result = sqlx::query(
            "INSERT INTO metrics_1hour (agent_id, metric_name, min_value, max_value, avg_value, sum_value, count, timestamp)
             SELECT 
                 agent_id,
                 metric_name,
                 MIN(min_value) as min_value,
                 MAX(max_value) as max_value,
                 AVG(avg_value) as avg_value,
                 SUM(sum_value) as sum_value,
                 SUM(count) as count,
                 DATE_TRUNC('hour', timestamp) as timestamp
             FROM metrics_1min
             WHERE timestamp >= $1
             GROUP BY agent_id, metric_name, DATE_TRUNC('hour', timestamp)
             ON CONFLICT (agent_id, metric_name, timestamp) DO NOTHING"
        )
        .bind(start_time)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    // Get the count of agents using a specific API key
    pub async fn get_agent_count_for_api_key(&self, api_key_id: i32) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM agents WHERE api_key_id = $1")
            .bind(api_key_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get::<i64, _>("count"))
    }

    // Check if an API key has reached its agent limit
    pub async fn check_api_key_agent_limit(
        &self,
        api_key: &str,
    ) -> Result<(bool, Option<i32>, Option<i32>)> {
        let row = sqlx::query(
            "SELECT 
                k.id,
                k.max_agents,
                (SELECT COUNT(*) FROM agents WHERE api_key_id = k.id) as current_count
             FROM api_keys k
             WHERE k.key = $1 AND NOT k.revoked AND (k.expires_at IS NULL OR k.expires_at > NOW())",
        )
        .bind(api_key)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let api_key_id: i32 = row.get("id");
            let max_agents: Option<i32> = row.get("max_agents");
            let current_count: i64 = row.get("current_count");

            // If max_agents is NULL, it's unlimited
            if let Some(max) = max_agents {
                let can_register = current_count < max as i64;
                Ok((can_register, Some(api_key_id), Some(max)))
            } else {
                // Unlimited
                Ok((true, Some(api_key_id), None))
            }
        } else {
            // API key not found or revoked
            Ok((false, None, None))
        }
    }

    // Update API key last_used_at timestamp
    pub async fn update_api_key_last_used(&self, api_key: &str) -> Result<()> {
        sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE key = $1")
            .bind(api_key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // Update API key properties (except the key itself)
    pub async fn update_api_key_properties(
        &self,
        key_id: i32,
        name: Option<&str>,
        description: Option<&str>,
        max_agents: Option<Option<i32>>,
        expires_at: Option<Option<DateTime<Utc>>>,
    ) -> Result<()> {
        let mut query_parts = vec![];
        let mut bind_count = 1;

        if name.is_some() {
            query_parts.push(format!("name = ${}", bind_count));
            bind_count += 1;
        }
        if description.is_some() {
            query_parts.push(format!("description = ${}", bind_count));
            bind_count += 1;
        }
        if max_agents.is_some() {
            query_parts.push(format!("max_agents = ${}", bind_count));
            bind_count += 1;
        }
        if expires_at.is_some() {
            query_parts.push(format!("expires_at = ${}", bind_count));
            bind_count += 1;
        }

        if query_parts.is_empty() {
            return Ok(());
        }

        let query_str = format!(
            "UPDATE api_keys SET {} WHERE id = ${}",
            query_parts.join(", "),
            bind_count
        );

        let mut query = sqlx::query(&query_str);

        if let Some(n) = name {
            query = query.bind(n);
        }
        if let Some(d) = description {
            query = query.bind(d);
        }
        if let Some(m) = max_agents {
            query = query.bind(m);
        }
        if let Some(e) = expires_at {
            query = query.bind(e);
        }
        query = query.bind(key_id);

        query.execute(&self.pool).await?;

        Ok(())
    }

    /// Get all agent IDs registered with a specific API key
    pub async fn get_agents_by_api_key(&self, api_key_id: i64) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT agent_id FROM agents WHERE api_key_id = $1")
            .bind(api_key_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|row| row.get("agent_id")).collect())
    }

    /// Register or update agent in database
    pub async fn register_agent(
        &self,
        agent_id: &str,
        api_key_id: Option<i64>,
        hostname: Option<&str>,
        os: Option<&str>,
        arch: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO agents (agent_id, api_key_id, hostname, os, arch, last_seen)
            VALUES ($1, $2, $3, $4, $5, NOW())
            ON CONFLICT (agent_id)
            DO UPDATE SET
                api_key_id = COALESCE(EXCLUDED.api_key_id, agents.api_key_id),
                hostname = COALESCE(EXCLUDED.hostname, agents.hostname),
                os = COALESCE(EXCLUDED.os, agents.os),
                arch = COALESCE(EXCLUDED.arch, agents.arch),
                last_seen = NOW()
            "#,
        )
        .bind(agent_id)
        .bind(api_key_id)
        .bind(hostname)
        .bind(os)
        .bind(arch)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Insert metrics into database
    pub async fn insert_metrics(
        &self,
        agent_id: &str,
        metrics: &[(String, f64)],
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        // Batch insert for performance
        for (metric_name, value) in metrics {
            sqlx::query(
                "INSERT INTO metrics (agent_id, metric_name, value, timestamp) VALUES ($1, $2, $3, $4)",
            )
            .bind(agent_id)
            .bind(metric_name)
            .bind(value)
            .bind(timestamp)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Insert log entry into database
    pub async fn insert_log(
        &self,
        agent_id: &str,
        level: &str,
        source: &str,
        message: &str,
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO logs (agent_id, level, source, message, timestamp) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(agent_id)
        .bind(level)
        .bind(source)
        .bind(message)
        .bind(timestamp)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
