use crate::config::DatabaseConfig;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

// Track previous stats for rate calculation
#[derive(Debug, Clone)]
struct PreviousStats {
    xact_commit: u64,
    xact_rollback: u64,
    timestamp: Instant,
}

#[derive(Debug, Clone)]
pub struct DatabaseMetrics {
    pub connections_active: u32,
    pub connections_idle: u32,
    pub connections_max: u32,
    pub queries_per_second: f64,
    pub slow_queries: u64,
    pub cache_hit_ratio: f64,
    pub transactions_committed: u64,
    pub transactions_rolled_back: u64,
    pub database_size_bytes: u64,
    pub locks_waiting: u32,
}

impl DatabaseMetrics {
    pub fn to_hashmap(&self) -> HashMap<String, f64> {
        let mut map = HashMap::new();
        map.insert(
            "db_connections_active".to_string(),
            self.connections_active as f64,
        );
        map.insert(
            "db_connections_idle".to_string(),
            self.connections_idle as f64,
        );
        map.insert(
            "db_connections_max".to_string(),
            self.connections_max as f64,
        );
        map.insert("db_queries_per_second".to_string(), self.queries_per_second);
        map.insert("db_slow_queries".to_string(), self.slow_queries as f64);
        map.insert("db_cache_hit_ratio".to_string(), self.cache_hit_ratio);
        map.insert(
            "db_transactions_committed".to_string(),
            self.transactions_committed as f64,
        );
        map.insert(
            "db_transactions_rolled_back".to_string(),
            self.transactions_rolled_back as f64,
        );
        map.insert(
            "db_database_size_bytes".to_string(),
            self.database_size_bytes as f64,
        );
        map.insert("db_locks_waiting".to_string(), self.locks_waiting as f64);
        map
    }
}

pub struct DatabaseCollector {
    config: DatabaseConfig,
    previous_stats: Arc<Mutex<Option<PreviousStats>>>,
}

impl DatabaseCollector {
    pub fn new(config: DatabaseConfig) -> Result<Self> {
        Ok(Self {
            config,
            previous_stats: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn collect(&self) -> Result<DatabaseMetrics> {
        match self.config.db_type.as_str() {
            "postgres" => self.collect_postgres().await,
            "mysql" => self.collect_mysql().await,
            _ => Err(anyhow::anyhow!(
                "Unsupported database type: {}",
                self.config.db_type
            )),
        }
    }

    #[cfg(feature = "postgres")]
    async fn collect_postgres(&self) -> Result<DatabaseMetrics> {
        use tokio_postgres::{Client, NoTls};

        let conn_string = format!(
            "host={} port={} dbname={} user={} password={}",
            self.config.host,
            self.config.port,
            self.config.database,
            self.config.username,
            self.config.password
        );

        let (client, connection) = tokio_postgres::connect(&conn_string, NoTls)
            .await
            .context("Failed to connect to PostgreSQL")?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("PostgreSQL connection error: {}", e);
            }
        });

        let metrics = self.query_postgres_metrics(&client).await?;
        Ok(metrics)
    }

    #[cfg(feature = "postgres")]
    async fn query_postgres_metrics(
        &self,
        client: &tokio_postgres::Client,
    ) -> Result<DatabaseMetrics> {
        // Query connection stats
        let conn_row = client
            .query_one(
                "SELECT 
                    (SELECT count(*) FROM pg_stat_activity WHERE state = 'active') as active,
                    (SELECT count(*) FROM pg_stat_activity WHERE state = 'idle') as idle,
                    (SELECT setting::int FROM pg_settings WHERE name = 'max_connections') as max_conn",
                &[],
            )
            .await?;

        // Query database stats
        let stats_row = client
            .query_one(
                "SELECT 
                    xact_commit,
                    xact_rollback,
                    blks_hit,
                    blks_read,
                    pg_database_size(current_database()) as db_size
                FROM pg_stat_database 
                WHERE datname = current_database()",
                &[],
            )
            .await?;

        // Query slow queries count (queries taking > 1 second)
        let slow_queries_row = client
            .query_one(
                "SELECT count(*) as slow FROM pg_stat_activity 
                WHERE state = 'active' AND (now() - query_start) > interval '1 second'",
                &[],
            )
            .await?;

        // Query locks
        let locks_row = client
            .query_one(
                "SELECT count(*) as waiting_locks FROM pg_locks WHERE NOT granted",
                &[],
            )
            .await?;

        let blks_hit: i64 = stats_row.get("blks_hit");
        let blks_read: i64 = stats_row.get("blks_read");
        let cache_hit_ratio = if blks_hit + blks_read > 0 {
            (blks_hit as f64 / (blks_hit + blks_read) as f64) * 100.0
        } else {
            0.0
        };

        // Calculate queries per second from transaction rate
        let xact_commit: i64 = stats_row.get("xact_commit");
        let xact_rollback: i64 = stats_row.get("xact_rollback");
        let total_transactions = (xact_commit + xact_rollback) as u64;
        let now = Instant::now();

        let queries_per_second = {
            let mut prev = self.previous_stats.lock().await;

            let qps = if let Some(ref previous) = *prev {
                let elapsed = now.duration_since(previous.timestamp).as_secs_f64();
                if elapsed > 0.0 {
                    let transaction_diff = total_transactions
                        .saturating_sub(previous.xact_commit + previous.xact_rollback);
                    transaction_diff as f64 / elapsed
                } else {
                    0.0
                }
            } else {
                0.0
            };

            // Update previous stats for next calculation
            *prev = Some(PreviousStats {
                xact_commit: xact_commit as u64,
                xact_rollback: xact_rollback as u64,
                timestamp: now,
            });

            qps
        };

        Ok(DatabaseMetrics {
            connections_active: conn_row.get::<_, i64>("active") as u32,
            connections_idle: conn_row.get::<_, i64>("idle") as u32,
            connections_max: conn_row.get::<_, i32>("max_conn") as u32,
            queries_per_second,
            slow_queries: slow_queries_row.get::<_, i64>("slow") as u64,
            cache_hit_ratio,
            transactions_committed: stats_row.get::<_, i64>("xact_commit") as u64,
            transactions_rolled_back: stats_row.get::<_, i64>("xact_rollback") as u64,
            database_size_bytes: stats_row.get::<_, i64>("db_size") as u64,
            locks_waiting: locks_row.get::<_, i64>("waiting_locks") as u32,
        })
    }

    #[cfg(not(feature = "postgres"))]
    async fn collect_postgres(&self) -> Result<DatabaseMetrics> {
        Err(anyhow::anyhow!(
            "PostgreSQL support not compiled. Enable 'postgres' feature"
        ))
    }

    #[cfg(feature = "mysql")]
    async fn collect_mysql(&self) -> Result<DatabaseMetrics> {
        use mysql_async::prelude::*;
        use mysql_async::{OptsBuilder, Pool};

        let opts = OptsBuilder::default()
            .ip_or_hostname(self.config.host.clone())
            .tcp_port(self.config.port)
            .db_name(Some(self.config.database.clone()))
            .user(Some(self.config.username.clone()))
            .pass(Some(self.config.password.clone()));

        let pool = Pool::new(opts);
        let mut conn = pool.get_conn().await?;

        // Get connection stats
        let status: HashMap<String, String> = conn
            .query("SHOW STATUS WHERE Variable_name IN ('Threads_connected', 'Max_used_connections', 'Slow_queries', 'Com_commit', 'Com_rollback')")
            .await?;

        // Get max connections
        let max_conn: u32 = conn
            .query_first("SHOW VARIABLES LIKE 'max_connections'")
            .await?
            .unwrap_or(100);

        // Get database size
        let db_size: u64 = conn
            .query_first(&format!(
                "SELECT SUM(data_length + index_length) as size FROM information_schema.TABLES WHERE table_schema = '{}'",
                self.config.database
            ))
            .await?
            .unwrap_or(0);

        Ok(DatabaseMetrics {
            connections_active: status
                .get("Threads_connected")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            connections_idle: 0, // Not directly available in MySQL
            connections_max: max_conn,
            queries_per_second: 0.0,
            slow_queries: status
                .get("Slow_queries")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            cache_hit_ratio: 0.0, // Would need to calculate from Qcache stats
            transactions_committed: status
                .get("Com_commit")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            transactions_rolled_back: status
                .get("Com_rollback")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            database_size_bytes: db_size,
            locks_waiting: 0, // Would need to query information_schema.innodb_locks
        })
    }

    #[cfg(not(feature = "mysql"))]
    async fn collect_mysql(&self) -> Result<DatabaseMetrics> {
        Err(anyhow::anyhow!(
            "MySQL support not compiled. Enable 'mysql' feature"
        ))
    }
}
