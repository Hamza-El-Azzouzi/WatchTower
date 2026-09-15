#![allow(dead_code)] // Some methods are for future use/background jobs

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{Postgres, QueryBuilder, Row};
use tracing::info;

use crate::alerts::{
    Alert, AlertCondition, AlertRule, AlertSeverity, AlertSilence, AlertState,
    CreateAlertSilenceRequest, CreateNotificationChannelRequest, IncidentEvent,
    NotificationChannel, NotificationChannelType, NotificationDelivery, PendingNotification,
};

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
        let rows = sqlx::query("SELECT id FROM agents WHERE api_key_id = $1")
            .bind(api_key_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|row| row.get("id")).collect())
    }

    /// Check ownership without loading metrics or exposing another tenant's agent.
    pub async fn agent_belongs_to_api_key(&self, agent_id: &str, api_key_id: i64) -> Result<bool> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM agents WHERE id = $1 AND api_key_id = $2)",
        )
        .bind(agent_id)
        .bind(api_key_id)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    /// Register or update agent in database
    pub async fn register_agent(
        &self,
        agent_id: &str,
        api_key_id: Option<i64>,
        hostname: Option<&str>,
        os: Option<&str>,
        _arch: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO agents (id, name, agent_type, api_key_id, hostname, os, last_seen)
            VALUES ($1, $1, 'server', $2, $3, $4, NOW())
            ON CONFLICT (id)
            DO UPDATE SET
                api_key_id = COALESCE(agents.api_key_id, EXCLUDED.api_key_id),
                hostname = COALESCE(EXCLUDED.hostname, agents.hostname),
                os = COALESCE(EXCLUDED.os, agents.os),
                last_seen = NOW()
            "#,
        )
        .bind(agent_id)
        .bind(api_key_id)
        .bind(hostname)
        .bind(os)
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
        if metrics.is_empty() {
            return Ok(());
        }

        let mut query = QueryBuilder::<Postgres>::new(
            "INSERT INTO metrics (agent_id, metric_name, value, timestamp) ",
        );
        query.push_values(metrics, |mut row, (metric_name, value)| {
            row.push_bind(agent_id)
                .push_bind(metric_name)
                .push_bind(value)
                .push_bind(timestamp);
        });
        query.build().execute(&self.pool).await?;

        Ok(())
    }

    /// Query metrics from database with optional time range and limit
    pub async fn query_metrics(
        &self,
        agent_id: &str,
        metric_name: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Result<Vec<crate::storage::DataPoint>> {
        let query_str = match (from, to, limit) {
            (Some(_), Some(_), Some(lim)) => format!(
                "SELECT value, timestamp FROM metrics 
                 WHERE agent_id = $1 AND metric_name = $2 AND timestamp >= $3 AND timestamp <= $4 
                 ORDER BY timestamp DESC LIMIT {}",
                lim
            ),
            (Some(_), None, Some(lim)) => format!(
                "SELECT value, timestamp FROM metrics 
                 WHERE agent_id = $1 AND metric_name = $2 AND timestamp >= $3 
                 ORDER BY timestamp DESC LIMIT {}",
                lim
            ),
            (None, Some(_), Some(lim)) => format!(
                "SELECT value, timestamp FROM metrics 
                 WHERE agent_id = $1 AND metric_name = $2 AND timestamp <= $3 
                 ORDER BY timestamp DESC LIMIT {}",
                lim
            ),
            (None, None, Some(lim)) => format!(
                "SELECT value, timestamp FROM metrics 
                 WHERE agent_id = $1 AND metric_name = $2 
                 ORDER BY timestamp DESC LIMIT {}",
                lim
            ),
            (Some(_), Some(_), None) => "SELECT value, timestamp FROM metrics 
                 WHERE agent_id = $1 AND metric_name = $2 AND timestamp >= $3 AND timestamp <= $4 
                 ORDER BY timestamp DESC"
                .to_string(),
            (Some(_), None, None) => "SELECT value, timestamp FROM metrics 
                 WHERE agent_id = $1 AND metric_name = $2 AND timestamp >= $3 
                 ORDER BY timestamp DESC"
                .to_string(),
            (None, Some(_), None) => "SELECT value, timestamp FROM metrics 
                 WHERE agent_id = $1 AND metric_name = $2 AND timestamp <= $3 
                 ORDER BY timestamp DESC"
                .to_string(),
            (None, None, None) => "SELECT value, timestamp FROM metrics 
                 WHERE agent_id = $1 AND metric_name = $2 
                 ORDER BY timestamp DESC"
                .to_string(),
        };

        let mut query = sqlx::query(&query_str).bind(agent_id).bind(metric_name);

        if let Some(f) = from {
            query = query.bind(f);
        }
        if let Some(t) = to {
            query = query.bind(t);
        }

        let rows = query.fetch_all(&self.pool).await?;

        let mut data_points: Vec<crate::storage::DataPoint> = rows
            .into_iter()
            .map(|row| crate::storage::DataPoint {
                value: row.get("value"),
                timestamp: row.get("timestamp"),
            })
            .collect();

        // Reverse to get chronological order
        data_points.reverse();

        Ok(data_points)
    }

    /// Get latest metrics for an agent from database
    pub async fn get_latest_metrics(
        &self,
        agent_id: &str,
    ) -> Result<Vec<(String, f64, DateTime<Utc>)>> {
        let rows = sqlx::query(
            "SELECT DISTINCT ON (metric_name) metric_name, value, timestamp 
             FROM metrics 
             WHERE agent_id = $1 
             ORDER BY metric_name, timestamp DESC",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                (
                    row.get::<String, _>("metric_name"),
                    row.get::<f64, _>("value"),
                    row.get::<DateTime<Utc>, _>("timestamp"),
                )
            })
            .collect())
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

    /// Query durable logs. Optional tenant IDs keep API-key users isolated.
    #[allow(clippy::too_many_arguments)]
    pub async fn query_logs(
        &self,
        allowed_agent_ids: Option<&[String]>,
        agent_id: Option<&str>,
        level: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        keyword: Option<&str>,
        limit: usize,
    ) -> Result<Vec<crate::storage::LogEntry>> {
        let allowed = allowed_agent_ids.map(|ids| ids.to_vec());
        let rows = sqlx::query(
            "SELECT id, agent_id, timestamp, level, COALESCE(source, '') AS source, message
             FROM logs
             WHERE ($1::text[] IS NULL OR agent_id = ANY($1))
               AND ($2::text IS NULL OR agent_id = $2)
               AND ($3::text IS NULL OR level = $3)
               AND ($4::timestamptz IS NULL OR timestamp >= $4)
               AND ($5::timestamptz IS NULL OR timestamp <= $5)
               AND ($6::text IS NULL OR message ILIKE ('%' || $6 || '%'))
             ORDER BY timestamp DESC
             LIMIT $7",
        )
        .bind(allowed)
        .bind(agent_id)
        .bind(level)
        .bind(from)
        .bind(to)
        .bind(keyword)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let level = match row.get::<String, _>("level").as_str() {
                    "DEBUG" => crate::storage::LogLevel::DEBUG,
                    "WARN" => crate::storage::LogLevel::WARN,
                    "ERROR" => crate::storage::LogLevel::ERROR,
                    "FATAL" => crate::storage::LogLevel::FATAL,
                    _ => crate::storage::LogLevel::INFO,
                };
                crate::storage::LogEntry {
                    id: row.get::<i64, _>("id") as u64,
                    agent_id: row.get("agent_id"),
                    timestamp: row.get("timestamp"),
                    level,
                    source: row.get("source"),
                    message: row.get("message"),
                }
            })
            .collect())
    }

    pub async fn get_log_count(&self, allowed_agent_ids: Option<&[String]>) -> Result<usize> {
        let allowed = allowed_agent_ids.map(|ids| ids.to_vec());
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM logs WHERE ($1::text[] IS NULL OR agent_id = ANY($1))",
        )
        .bind(allowed)
        .fetch_one(&self.pool)
        .await?;
        Ok(count.max(0) as usize)
    }

    // ============ Alert Rules CRUD ============

    /// Create a new alert rule in the database
    pub async fn create_alert_rule(&self, rule: &AlertRule) -> Result<()> {
        let condition_type = match rule.condition {
            AlertCondition::GreaterThan => "greater_than",
            AlertCondition::LessThan => "less_than",
            AlertCondition::Equals => "equals",
            AlertCondition::NotEquals => "not_equals",
        };

        let severity = match rule.severity {
            AlertSeverity::Info => "info",
            AlertSeverity::Warning => "warning",
            AlertSeverity::Critical => "critical",
        };

        let channels_json = serde_json::to_value(&rule.channels).unwrap_or_default();

        sqlx::query(
            r#"
            INSERT INTO alert_rules (
                rule_id, name, description, metric, condition_type, threshold, 
                threshold_percent, duration_seconds, severity, channels, 
                agent_filter, cooldown_seconds, enabled, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            ON CONFLICT (rule_id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                metric = EXCLUDED.metric,
                condition_type = EXCLUDED.condition_type,
                threshold = EXCLUDED.threshold,
                threshold_percent = EXCLUDED.threshold_percent,
                duration_seconds = EXCLUDED.duration_seconds,
                severity = EXCLUDED.severity,
                channels = EXCLUDED.channels,
                agent_filter = EXCLUDED.agent_filter,
                cooldown_seconds = EXCLUDED.cooldown_seconds,
                enabled = EXCLUDED.enabled,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(&rule.id)
        .bind(&rule.name)
        .bind(&rule.description)
        .bind(&rule.metric)
        .bind(condition_type)
        .bind(rule.threshold)
        .bind(rule.threshold_percent)
        .bind(rule.duration_seconds as i32)
        .bind(severity)
        .bind(&channels_json)
        .bind(&rule.agent_filter)
        .bind(rule.cooldown_seconds as i32)
        .bind(rule.enabled)
        .bind(rule.created_at)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .context("Failed to create alert rule")?;

        Ok(())
    }

    /// Get an alert rule by its string ID
    pub async fn get_alert_rule(&self, rule_id: &str) -> Result<Option<AlertRule>> {
        let row = sqlx::query(
            r#"
            SELECT rule_id, name, description, metric, condition_type, threshold,
                   threshold_percent, duration_seconds, severity, channels,
                   agent_filter, cooldown_seconds, enabled, created_at, updated_at
            FROM alert_rules
            WHERE rule_id = $1
            "#,
        )
        .bind(rule_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch alert rule")?;

        if let Some(row) = row {
            Ok(Some(Self::row_to_alert_rule(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all alert rules from the database
    pub async fn list_alert_rules(&self) -> Result<Vec<AlertRule>> {
        let rows = sqlx::query(
            r#"
            SELECT rule_id, name, description, metric, condition_type, threshold,
                   threshold_percent, duration_seconds, severity, channels,
                   agent_filter, cooldown_seconds, enabled, created_at, updated_at
            FROM alert_rules
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list alert rules")?;

        let mut rules = Vec::new();
        for row in rows {
            rules.push(Self::row_to_alert_rule(row)?);
        }
        Ok(rules)
    }

    /// Update an alert rule in the database
    pub async fn update_alert_rule(&self, rule: &AlertRule) -> Result<bool> {
        let condition_type = match rule.condition {
            AlertCondition::GreaterThan => "greater_than",
            AlertCondition::LessThan => "less_than",
            AlertCondition::Equals => "equals",
            AlertCondition::NotEquals => "not_equals",
        };

        let severity = match rule.severity {
            AlertSeverity::Info => "info",
            AlertSeverity::Warning => "warning",
            AlertSeverity::Critical => "critical",
        };

        let channels_json = serde_json::to_value(&rule.channels).unwrap_or_default();

        let result = sqlx::query(
            r#"
            UPDATE alert_rules SET
                name = $2,
                description = $3,
                metric = $4,
                condition_type = $5,
                threshold = $6,
                threshold_percent = $7,
                duration_seconds = $8,
                severity = $9,
                channels = $10,
                agent_filter = $11,
                cooldown_seconds = $12,
                enabled = $13,
                updated_at = NOW()
            WHERE rule_id = $1
            "#,
        )
        .bind(&rule.id)
        .bind(&rule.name)
        .bind(&rule.description)
        .bind(&rule.metric)
        .bind(condition_type)
        .bind(rule.threshold)
        .bind(rule.threshold_percent)
        .bind(rule.duration_seconds as i32)
        .bind(severity)
        .bind(&channels_json)
        .bind(&rule.agent_filter)
        .bind(rule.cooldown_seconds as i32)
        .bind(rule.enabled)
        .execute(&self.pool)
        .await
        .context("Failed to update alert rule")?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete an alert rule from the database
    pub async fn delete_alert_rule(&self, rule_id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM alert_rules WHERE rule_id = $1")
            .bind(rule_id)
            .execute(&self.pool)
            .await
            .context("Failed to delete alert rule")?;

        Ok(result.rows_affected() > 0)
    }

    /// Toggle alert rule enabled status
    pub async fn toggle_alert_rule(&self, rule_id: &str) -> Result<Option<bool>> {
        let result = sqlx::query(
            r#"
            UPDATE alert_rules 
            SET enabled = NOT enabled, updated_at = NOW()
            WHERE rule_id = $1
            RETURNING enabled
            "#,
        )
        .bind(rule_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to toggle alert rule")?;

        Ok(result.map(|row| row.get::<bool, _>("enabled")))
    }

    fn row_to_alert_rule(row: sqlx::postgres::PgRow) -> Result<AlertRule> {
        let condition_type: Option<String> = row.get("condition_type");
        let condition = match condition_type.as_deref() {
            Some("greater_than") | None => AlertCondition::GreaterThan,
            Some("less_than") => AlertCondition::LessThan,
            Some("equals") => AlertCondition::Equals,
            Some("not_equals") => AlertCondition::NotEquals,
            Some(other) => {
                tracing::warn!(
                    "Unknown condition type: {}, defaulting to GreaterThan",
                    other
                );
                AlertCondition::GreaterThan
            }
        };

        let severity_str: String = row.get("severity");
        let severity = match severity_str.to_lowercase().as_str() {
            "info" => AlertSeverity::Info,
            "warning" => AlertSeverity::Warning,
            "critical" => AlertSeverity::Critical,
            _ => AlertSeverity::Warning,
        };

        let channels_json: serde_json::Value =
            row.try_get("channels").unwrap_or(serde_json::json!([]));
        let channels: Vec<String> = serde_json::from_value(channels_json).unwrap_or_default();

        Ok(AlertRule {
            id: row.try_get("rule_id").unwrap_or_default(),
            name: row.get("name"),
            description: row.get("description"),
            metric: row.try_get("metric").unwrap_or_default(),
            condition,
            threshold: row.try_get("threshold").unwrap_or(0.0),
            threshold_percent: row.get("threshold_percent"),
            duration_seconds: row.try_get::<i32, _>("duration_seconds").unwrap_or(0) as u64,
            severity,
            channels,
            enabled: row.try_get("enabled").unwrap_or(true),
            agent_filter: row.get("agent_filter"),
            created_at: row.try_get("created_at").unwrap_or_else(|_| Utc::now()),
            cooldown_seconds: row.try_get::<i32, _>("cooldown_seconds").unwrap_or(0) as u64,
        })
    }

    // ============ Alerts CRUD ============

    /// Create or update an alert in the database
    pub async fn upsert_alert(&self, alert: &Alert) -> Result<()> {
        let condition_type = match alert.condition {
            AlertCondition::GreaterThan => "greater_than",
            AlertCondition::LessThan => "less_than",
            AlertCondition::Equals => "equals",
            AlertCondition::NotEquals => "not_equals",
        };

        let severity = match alert.severity {
            AlertSeverity::Info => "info",
            AlertSeverity::Warning => "warning",
            AlertSeverity::Critical => "critical",
        };

        let state = match alert.state {
            AlertState::Pending => "pending",
            AlertState::Firing => "firing",
            AlertState::Resolved => "resolved",
        };

        sqlx::query(
            r#"
            INSERT INTO alerts (
                alert_id, rule_id, rule_name, agent_id, agent_name, severity,
                message, triggered_at, resolved_at, state, metric, current_value,
                threshold, condition_type, acknowledged, acknowledged_at,
                acknowledged_by, last_notification_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            ON CONFLICT (alert_id) DO UPDATE SET
                state = EXCLUDED.state,
                resolved_at = EXCLUDED.resolved_at,
                current_value = EXCLUDED.current_value,
                message = EXCLUDED.message,
                acknowledged = EXCLUDED.acknowledged,
                acknowledged_at = EXCLUDED.acknowledged_at,
                acknowledged_by = EXCLUDED.acknowledged_by,
                last_notification_at = EXCLUDED.last_notification_at
            "#,
        )
        .bind(&alert.id)
        .bind(&alert.rule_id)
        .bind(&alert.rule_name)
        .bind(&alert.agent_id)
        .bind(&alert.agent_name)
        .bind(severity)
        .bind(&alert.message)
        .bind(alert.triggered_at)
        .bind(alert.resolved_at)
        .bind(state)
        .bind(&alert.metric)
        .bind(alert.current_value)
        .bind(alert.threshold)
        .bind(condition_type)
        .bind(alert.acknowledged)
        .bind(alert.acknowledged_at)
        .bind(&alert.acknowledged_by)
        .bind(alert.last_notification_at)
        .execute(&self.pool)
        .await
        .context("Failed to upsert alert")?;

        Ok(())
    }

    /// Get an alert by its ID
    pub async fn get_alert(&self, alert_id: &str) -> Result<Option<Alert>> {
        let row = sqlx::query(
            r#"
            SELECT alert_id, rule_id, rule_name, agent_id, agent_name, severity,
                   message, triggered_at, resolved_at, state, metric, current_value,
                   threshold, condition_type, acknowledged, acknowledged_at,
                   acknowledged_by, last_notification_at
            FROM alerts
            WHERE alert_id = $1
            "#,
        )
        .bind(alert_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch alert")?;

        if let Some(row) = row {
            Ok(Some(Self::row_to_alert(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all alerts from the database
    pub async fn list_alerts(&self, limit: Option<i64>) -> Result<Vec<Alert>> {
        let query = if let Some(lim) = limit {
            format!(
                r#"
                SELECT alert_id, rule_id, rule_name, agent_id, agent_name, severity,
                       message, triggered_at, resolved_at, state, metric, current_value,
                       threshold, condition_type, acknowledged, acknowledged_at,
                       acknowledged_by, last_notification_at
                FROM alerts
                ORDER BY triggered_at DESC
                LIMIT {}
                "#,
                lim
            )
        } else {
            r#"
            SELECT alert_id, rule_id, rule_name, agent_id, agent_name, severity,
                   message, triggered_at, resolved_at, state, metric, current_value,
                   threshold, condition_type, acknowledged, acknowledged_at,
                   acknowledged_by, last_notification_at
            FROM alerts
            ORDER BY triggered_at DESC
            "#
            .to_string()
        };

        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list alerts")?;

        let mut alerts = Vec::new();
        for row in rows {
            alerts.push(Self::row_to_alert(row)?);
        }
        Ok(alerts)
    }

    /// Acknowledge an alert in the database
    pub async fn acknowledge_alert_db(
        &self,
        alert_id: &str,
        acknowledged_by: &str,
    ) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE alerts 
            SET acknowledged = true, acknowledged_at = NOW(), acknowledged_by = $2
            WHERE alert_id = $1
            "#,
        )
        .bind(alert_id)
        .bind(acknowledged_by)
        .execute(&self.pool)
        .await
        .context("Failed to acknowledge alert")?;

        Ok(result.rows_affected() > 0)
    }

    fn row_to_alert(row: sqlx::postgres::PgRow) -> Result<Alert> {
        let condition_type: Option<String> = row.try_get("condition_type").ok();
        let condition = match condition_type.as_deref() {
            Some("greater_than") | None => AlertCondition::GreaterThan,
            Some("less_than") => AlertCondition::LessThan,
            Some("equals") => AlertCondition::Equals,
            Some("not_equals") => AlertCondition::NotEquals,
            Some(_) => AlertCondition::GreaterThan,
        };

        let severity_str: String = row.get("severity");
        let severity = match severity_str.to_lowercase().as_str() {
            "info" => AlertSeverity::Info,
            "warning" => AlertSeverity::Warning,
            "critical" => AlertSeverity::Critical,
            _ => AlertSeverity::Warning,
        };

        let state_str: String = row.get("state");
        let state = match state_str.to_lowercase().as_str() {
            "pending" => AlertState::Pending,
            "firing" => AlertState::Firing,
            "resolved" => AlertState::Resolved,
            _ => AlertState::Firing,
        };

        Ok(Alert {
            id: row.get("alert_id"),
            rule_id: row.try_get("rule_id").unwrap_or_default(),
            rule_name: row.get("rule_name"),
            agent_id: row.get("agent_id"),
            agent_name: row.try_get("agent_name").unwrap_or_default(),
            state,
            metric: row.try_get("metric").unwrap_or_default(),
            current_value: row.try_get("current_value").unwrap_or(0.0),
            threshold: row.try_get("threshold").unwrap_or(0.0),
            condition,
            severity,
            message: row.get("message"),
            triggered_at: row.get("triggered_at"),
            resolved_at: row.get("resolved_at"),
            acknowledged: row.try_get("acknowledged").unwrap_or(false),
            acknowledged_at: row.get("acknowledged_at"),
            acknowledged_by: row.get("acknowledged_by"),
            last_notification_at: row.get("last_notification_at"),
        })
    }

    // ============ Production alerting ============

    pub async fn create_notification_channel(
        &self,
        request: &CreateNotificationChannelRequest,
    ) -> Result<NotificationChannel> {
        let id = uuid::Uuid::new_v4().to_string();
        let row = sqlx::query(
            r#"INSERT INTO notification_channels (
                id, name, channel_type, webhook_url, email_to, smtp_host, smtp_port,
                smtp_username, smtp_password, smtp_from, smtp_tls
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            RETURNING id, name, channel_type, webhook_url, email_to, smtp_host, smtp_port,
                      smtp_username, smtp_password, smtp_from, smtp_tls, enabled, created_at, updated_at"#,
        )
        .bind(id)
        .bind(request.name.trim())
        .bind(request.channel_type.as_str())
        .bind(request.webhook_url.as_deref())
        .bind(request.email_to.as_deref())
        .bind(request.smtp_host.as_deref())
        .bind(request.smtp_port)
        .bind(request.smtp_username.as_deref())
        .bind(request.smtp_password.as_deref())
        .bind(request.smtp_from.as_deref())
        .bind(request.smtp_tls)
        .fetch_one(&self.pool)
        .await
        .context("Failed to create notification channel")?;
        Self::row_to_notification_channel(row)
    }

    pub async fn list_notification_channels(&self) -> Result<Vec<NotificationChannel>> {
        let rows = sqlx::query(
            r#"SELECT id, name, channel_type, webhook_url, email_to, smtp_host, smtp_port,
                      smtp_username, smtp_password, smtp_from, smtp_tls, enabled, created_at, updated_at
               FROM notification_channels ORDER BY name"#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(Self::row_to_notification_channel)
            .collect()
    }

    pub async fn get_notification_channel(&self, id: &str) -> Result<Option<NotificationChannel>> {
        let row = sqlx::query(
            r#"SELECT id, name, channel_type, webhook_url, email_to, smtp_host, smtp_port,
                      smtp_username, smtp_password, smtp_from, smtp_tls, enabled, created_at, updated_at
               FROM notification_channels WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(Self::row_to_notification_channel).transpose()
    }

    pub async fn set_notification_channel_enabled(&self, id: &str, enabled: bool) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE notification_channels SET enabled = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .bind(enabled)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_notification_channel(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM notification_channels WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    fn row_to_notification_channel(row: sqlx::postgres::PgRow) -> Result<NotificationChannel> {
        let channel_type = match row.get::<String, _>("channel_type").as_str() {
            "slack" => NotificationChannelType::Slack,
            "discord" => NotificationChannelType::Discord,
            "email" => NotificationChannelType::Email,
            _ => NotificationChannelType::GenericWebhook,
        };
        Ok(NotificationChannel {
            id: row.get("id"),
            name: row.get("name"),
            channel_type,
            webhook_url: row.get("webhook_url"),
            email_to: row.get("email_to"),
            smtp_host: row.get("smtp_host"),
            smtp_port: row.get("smtp_port"),
            smtp_username: row.get("smtp_username"),
            smtp_password: row.get("smtp_password"),
            smtp_from: row.get("smtp_from"),
            smtp_tls: row.get("smtp_tls"),
            enabled: row.get("enabled"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    pub async fn create_alert_silence(
        &self,
        request: &CreateAlertSilenceRequest,
    ) -> Result<AlertSilence> {
        let id = uuid::Uuid::new_v4().to_string();
        let row = sqlx::query(
            r#"INSERT INTO alert_silences
               (id, name, reason, rule_id, agent_id, starts_at, ends_at, created_by)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
               RETURNING id, name, reason, rule_id, agent_id, starts_at, ends_at, created_by, created_at"#,
        )
        .bind(id)
        .bind(request.name.trim())
        .bind(request.reason.as_deref())
        .bind(request.rule_id.as_deref())
        .bind(request.agent_id.as_deref())
        .bind(request.starts_at)
        .bind(request.ends_at)
        .bind(request.created_by.trim())
        .fetch_one(&self.pool)
        .await?;
        Ok(Self::row_to_alert_silence(row))
    }

    pub async fn list_alert_silences(&self, active_only: bool) -> Result<Vec<AlertSilence>> {
        let rows = sqlx::query(
            r#"SELECT id, name, reason, rule_id, agent_id, starts_at, ends_at, created_by, created_at
               FROM alert_silences
               WHERE NOT $1 OR (starts_at <= NOW() AND ends_at > NOW())
               ORDER BY starts_at DESC"#,
        )
        .bind(active_only)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Self::row_to_alert_silence).collect())
    }

    pub async fn delete_alert_silence(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM alert_silences WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn is_alert_silenced(&self, rule_id: &str, agent_id: &str) -> Result<bool> {
        let silenced = sqlx::query_scalar::<_, bool>(
            r#"SELECT EXISTS(
                 SELECT 1 FROM alert_silences
                 WHERE starts_at <= NOW() AND ends_at > NOW()
                   AND (rule_id IS NULL OR rule_id = $1)
                   AND (agent_id IS NULL OR agent_id = $2)
               )"#,
        )
        .bind(rule_id)
        .bind(agent_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(silenced)
    }

    fn row_to_alert_silence(row: sqlx::postgres::PgRow) -> AlertSilence {
        AlertSilence {
            id: row.get("id"),
            name: row.get("name"),
            reason: row.get("reason"),
            rule_id: row.get("rule_id"),
            agent_id: row.get("agent_id"),
            starts_at: row.get("starts_at"),
            ends_at: row.get("ends_at"),
            created_by: row.get("created_by"),
            created_at: row.get("created_at"),
        }
    }

    pub async fn enqueue_notification(
        &self,
        alert_id: &str,
        channel: &NotificationChannel,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<i64> {
        let id = sqlx::query_scalar::<_, i64>(
            r#"INSERT INTO notification_deliveries
               (alert_id, channel_id, channel_name, event_type, status, payload)
               VALUES ($1,$2,$3,$4,'pending',$5) RETURNING id"#,
        )
        .bind(alert_id)
        .bind(&channel.id)
        .bind(&channel.name)
        .bind(event_type)
        .bind(payload)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn claim_due_notifications(&self, limit: i64) -> Result<Vec<PendingNotification>> {
        let rows = sqlx::query(
            r#"SELECT d.id, d.alert_id, d.channel_id, d.channel_name, d.event_type,
                      d.status, d.attempt_count, d.response_status, d.error_message,
                      d.created_at, d.delivered_at, d.next_attempt_at, d.last_attempt_at,
                      d.max_attempts, d.payload,
                      c.id AS c_id, c.name AS c_name, c.channel_type, c.webhook_url,
                      c.email_to, c.smtp_host, c.smtp_port, c.smtp_username, c.smtp_password,
                      c.smtp_from, c.smtp_tls, c.enabled, c.created_at AS c_created_at,
                      c.updated_at AS c_updated_at
               FROM notification_deliveries d
               JOIN notification_channels c ON c.id = d.channel_id
               WHERE d.status IN ('pending','failed')
                 AND d.attempt_count < d.max_attempts
                 AND d.next_attempt_at <= NOW()
                 AND c.enabled
               ORDER BY d.next_attempt_at
               LIMIT $1"#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let channel_type = match row.get::<String, _>("channel_type").as_str() {
                    "slack" => NotificationChannelType::Slack,
                    "discord" => NotificationChannelType::Discord,
                    "email" => NotificationChannelType::Email,
                    _ => NotificationChannelType::GenericWebhook,
                };
                Ok(PendingNotification {
                    delivery: NotificationDelivery {
                        id: row.get("id"),
                        alert_id: row.get("alert_id"),
                        channel_id: row.get("channel_id"),
                        channel_name: row.get("channel_name"),
                        event_type: row.get("event_type"),
                        status: row.get("status"),
                        attempt_count: row.get("attempt_count"),
                        response_status: row.get("response_status"),
                        error_message: row.get("error_message"),
                        created_at: row.get("created_at"),
                        delivered_at: row.get("delivered_at"),
                        next_attempt_at: row.get("next_attempt_at"),
                        last_attempt_at: row.get("last_attempt_at"),
                        max_attempts: row.get("max_attempts"),
                    },
                    payload: row.get("payload"),
                    channel: NotificationChannel {
                        id: row.get("c_id"),
                        name: row.get("c_name"),
                        channel_type,
                        webhook_url: row.get("webhook_url"),
                        email_to: row.get("email_to"),
                        smtp_host: row.get("smtp_host"),
                        smtp_port: row.get("smtp_port"),
                        smtp_username: row.get("smtp_username"),
                        smtp_password: row.get("smtp_password"),
                        smtp_from: row.get("smtp_from"),
                        smtp_tls: row.get("smtp_tls"),
                        enabled: row.get("enabled"),
                        created_at: row.get("c_created_at"),
                        updated_at: row.get("c_updated_at"),
                    },
                })
            })
            .collect()
    }

    pub async fn finish_notification_attempt(
        &self,
        id: i64,
        delivered: bool,
        response_status: Option<i32>,
        error_message: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"UPDATE notification_deliveries SET
                 status = CASE WHEN $2 THEN 'delivered' ELSE 'failed' END,
                 attempt_count = attempt_count + 1,
                 response_status = $3,
                 error_message = $4,
                 last_attempt_at = NOW(),
                 delivered_at = CASE WHEN $2 THEN NOW() ELSE delivered_at END,
                 next_attempt_at = CASE WHEN $2 THEN next_attempt_at
                   ELSE NOW() + make_interval(secs => LEAST(900, (5 * power(2, attempt_count))::int)) END
               WHERE id = $1"#,
        )
        .bind(id).bind(delivered).bind(response_status).bind(error_message)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn list_notification_deliveries(
        &self,
        limit: i64,
    ) -> Result<Vec<NotificationDelivery>> {
        let rows = sqlx::query(
            r#"SELECT id, alert_id, channel_id, channel_name, event_type, status,
                      attempt_count, response_status, error_message, created_at, delivered_at,
                      next_attempt_at, last_attempt_at, max_attempts
               FROM notification_deliveries ORDER BY created_at DESC LIMIT $1"#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| NotificationDelivery {
                id: row.get("id"),
                alert_id: row.get("alert_id"),
                channel_id: row.get("channel_id"),
                channel_name: row.get("channel_name"),
                event_type: row.get("event_type"),
                status: row.get("status"),
                attempt_count: row.get("attempt_count"),
                response_status: row.get("response_status"),
                error_message: row.get("error_message"),
                created_at: row.get("created_at"),
                delivered_at: row.get("delivered_at"),
                next_attempt_at: row.get("next_attempt_at"),
                last_attempt_at: row.get("last_attempt_at"),
                max_attempts: row.get("max_attempts"),
            })
            .collect())
    }

    pub async fn insert_incident_event(&self, event: &IncidentEvent) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO incident_events
               (event_id, alert_id, rule_id, agent_id, event_type, severity, title, description, metadata, occurred_at)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT (event_id) DO NOTHING"#,
        )
        .bind(&event.event_id).bind(&event.alert_id).bind(&event.rule_id).bind(&event.agent_id)
        .bind(&event.event_type).bind(&event.severity).bind(&event.title).bind(&event.description)
        .bind(&event.metadata).bind(event.occurred_at).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn list_incident_events(
        &self,
        agent_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<IncidentEvent>> {
        let rows = sqlx::query(
            r#"SELECT event_id, alert_id, rule_id, agent_id, event_type, severity,
                      title, description, metadata, occurred_at
               FROM incident_events WHERE ($1::text IS NULL OR agent_id = $1)
               ORDER BY occurred_at DESC LIMIT $2"#,
        )
        .bind(agent_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| IncidentEvent {
                event_id: row.get("event_id"),
                alert_id: row.get("alert_id"),
                rule_id: row.get("rule_id"),
                agent_id: row.get("agent_id"),
                event_type: row.get("event_type"),
                severity: row.get("severity"),
                title: row.get("title"),
                description: row.get("description"),
                metadata: row.get("metadata"),
                occurred_at: row.get("occurred_at"),
            })
            .collect())
    }

    // ============ Agent Requests CRUD ============

    /// Create a new agent request
    pub async fn create_agent_request(
        &self,
        payload: &crate::api::AgentRequestPayload,
    ) -> Result<i64> {
        let row = sqlx::query(
            r#"
            INSERT INTO agent_requests (
                company_name, contact_name, email, phone,
                agents_requested, use_case, message
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id
            "#,
        )
        .bind(&payload.company_name)
        .bind(&payload.contact_name)
        .bind(&payload.email)
        .bind(&payload.phone)
        .bind(payload.agents_requested)
        .bind(&payload.use_case)
        .bind(&payload.message)
        .fetch_one(&self.pool)
        .await
        .context("Failed to create agent request")?;

        Ok(row.get::<i64, _>("id"))
    }

    /// List agent requests with optional status filter
    pub async fn list_agent_requests(
        &self,
        status_filter: Option<&str>,
    ) -> Result<Vec<serde_json::Value>> {
        let query = if let Some(status) = status_filter {
            sqlx::query(
                r#"
                SELECT id, company_name, contact_name, email, phone,
                       agents_requested, use_case, message, status,
                       created_at, updated_at, reviewed_by, reviewed_at, notes
                FROM agent_requests
                WHERE status = $1
                ORDER BY created_at DESC
                "#,
            )
            .bind(status)
        } else {
            sqlx::query(
                r#"
                SELECT id, company_name, contact_name, email, phone,
                       agents_requested, use_case, message, status,
                       created_at, updated_at, reviewed_by, reviewed_at, notes
                FROM agent_requests
                ORDER BY created_at DESC
                "#,
            )
        };

        let rows = query
            .fetch_all(&self.pool)
            .await
            .context("Failed to list agent requests")?;

        let requests: Vec<serde_json::Value> = rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "id": row.get::<i64, _>("id"),
                    "company_name": row.get::<String, _>("company_name"),
                    "contact_name": row.get::<String, _>("contact_name"),
                    "email": row.get::<String, _>("email"),
                    "phone": row.get::<Option<String>, _>("phone"),
                    "agents_requested": row.get::<i32, _>("agents_requested"),
                    "use_case": row.get::<String, _>("use_case"),
                    "message": row.get::<Option<String>, _>("message"),
                    "status": row.get::<String, _>("status"),
                    "created_at": row.get::<DateTime<Utc>, _>("created_at").to_rfc3339(),
                    "updated_at": row.get::<DateTime<Utc>, _>("updated_at").to_rfc3339(),
                    "reviewed_by": row.get::<Option<String>, _>("reviewed_by"),
                    "reviewed_at": row.get::<Option<DateTime<Utc>>, _>("reviewed_at").map(|dt| dt.to_rfc3339()),
                    "notes": row.get::<Option<String>, _>("notes"),
                })
            })
            .collect();

        Ok(requests)
    }

    /// Update agent request status
    pub async fn update_agent_request_status(
        &self,
        request_id: i64,
        status: &str,
        notes: Option<&str>,
        reviewed_by: Option<&str>,
    ) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE agent_requests
            SET status = $2,
                notes = COALESCE($3, notes),
                reviewed_by = COALESCE($4, reviewed_by),
                reviewed_at = CASE WHEN $4 IS NOT NULL THEN NOW() ELSE reviewed_at END,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(request_id)
        .bind(status)
        .bind(notes)
        .bind(reviewed_by)
        .execute(&self.pool)
        .await
        .context("Failed to update agent request")?;

        Ok(result.rows_affected() > 0)
    }
}
