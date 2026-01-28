use anyhow::Result;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::Path;
use tracing::info;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        // Create database file if it doesn't exist
        if database_url.starts_with("sqlite:") {
            let path = database_url.strip_prefix("sqlite:").unwrap();
            if !Path::new(path).exists() {
                info!("Creating new database at {}", path);
                std::fs::File::create(path)?;
            }
        }

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        let db = Self { pool };

        // Run migrations
        db.run_migrations().await?;

        Ok(db)
    }

    async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations");

        // Run migrations in order
        let migrations = [
            include_str!("../../migrations/001_initial_schema.sql"),
            include_str!("../../migrations/002_api_keys.sql"),
            include_str!("../../migrations/003_api_key_agent_limit.sql"),
        ];

        for (idx, migration_sql) in migrations.iter().enumerate() {
            info!("Running migration {}", idx + 1);
            sqlx::query(migration_sql).execute(&self.pool).await?;
        }

        info!("Database migrations completed");
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // Check if database is healthy
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await?;
        Ok(())
    }

    // Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        let row = sqlx::query(
            "SELECT 
                (SELECT COUNT(*) FROM agents) as agents,
                (SELECT COUNT(*) FROM metrics) as metrics,
                (SELECT COUNT(*) FROM alert_rules) as rules,
                (SELECT COUNT(*) FROM alerts) as alerts",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(DatabaseStats {
            agents: row.get::<i64, _>("agents") as u64,
            metrics: row.get::<i64, _>("metrics") as u64,
            alert_rules: row.get::<i64, _>("rules") as u64,
            alerts: row.get::<i64, _>("alerts") as u64,
        })
    }

    // Clean up old metrics (data retention)
    pub async fn cleanup_old_metrics(&self, retention_hours: i64) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM metrics 
             WHERE timestamp < datetime('now', '-' || ? || ' hours')",
        )
        .bind(retention_hours)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    // Clean up resolved alerts older than specified days
    pub async fn cleanup_old_alerts(&self, retention_days: i64) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM alerts 
             WHERE state = 'resolved' 
             AND resolved_at < datetime('now', '-' || ? || ' days')",
        )
        .bind(retention_days)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[derive(Debug, serde::Serialize)]
pub struct DatabaseStats {
    pub agents: u64,
    pub metrics: u64,
    pub alert_rules: u64,
    pub alerts: u64,
}
