use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub retention: RetentionConfig,
    #[serde(default)]
    pub aggregation: AggregationConfig,
    #[serde(default)]
    pub alerts: AlertsConfig,
    #[serde(default)]
    pub websocket: WebSocketConfig,
    #[serde(default)]
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(default = "default_max_points")]
    pub max_points_per_metric: usize,
    #[serde(default = "default_persistence_interval")]
    pub persistence_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_database_url")]
    pub url: String,

    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    #[serde(default = "default_raw_metrics_hours")]
    pub raw_metrics_hours: i64,

    #[serde(default = "default_minute_aggregates_days")]
    pub minute_aggregates_days: i64,

    #[serde(default = "default_hour_aggregates_days")]
    pub hour_aggregates_days: i64,

    #[serde(default = "default_alerts_retention_days")]
    pub alerts_days: i64,

    #[serde(default = "default_cleanup_interval_hours")]
    pub cleanup_interval_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationConfig {
    #[serde(default = "default_minute_interval_hours")]
    pub minute_interval_hours: u64,

    #[serde(default = "default_hour_interval_hours")]
    pub hour_interval_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertsConfig {
    #[serde(default = "default_check_interval_seconds")]
    pub check_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default = "default_false")]
    pub require_api_key: bool,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_max_points() -> usize {
    10_000
}

fn default_persistence_interval() -> u64 {
    15
}

fn default_database_url() -> String {
    "sqlite:monitoring.db".to_string()
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_raw_metrics_hours() -> i64 {
    24 // 1 day
}

fn default_minute_aggregates_days() -> i64 {
    7 // 1 week
}

fn default_hour_aggregates_days() -> i64 {
    30 // 1 month
}

fn default_alerts_retention_days() -> i64 {
    90 // 3 months
}

fn default_cleanup_interval_hours() -> u64 {
    1 // Every hour
}

fn default_minute_interval_hours() -> u64 {
    1 // Aggregate to minutes every hour
}

fn default_hour_interval_hours() -> u64 {
    24 // Aggregate to hours daily
}

fn default_check_interval_seconds() -> u64 {
    30 // Check alerts every 30 seconds
}

fn default_max_connections() -> usize {
    1000
}

fn default_heartbeat_interval() -> u64 {
    30 // 30 seconds
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: default_database_url(),
            enabled: default_true(),
        }
    }
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            raw_metrics_hours: default_raw_metrics_hours(),
            minute_aggregates_days: default_minute_aggregates_days(),
            hour_aggregates_days: default_hour_aggregates_days(),
            alerts_days: default_alerts_retention_days(),
            cleanup_interval_hours: default_cleanup_interval_hours(),
        }
    }
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self {
            minute_interval_hours: default_minute_interval_hours(),
            hour_interval_hours: default_hour_interval_hours(),
        }
    }
}

impl Default for AlertsConfig {
    fn default() -> Self {
        Self {
            check_interval_seconds: default_check_interval_seconds(),
        }
    }
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_connections: default_max_connections(),
            heartbeat_interval: default_heartbeat_interval(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            require_api_key: default_false(),
        }
    }
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        let mut config: Config = toml::from_str(&contents)?;

        // Override with environment variables if present
        config.apply_env_overrides();

        Ok(config)
    }

    /// Apply environment variable overrides to configuration
    pub fn apply_env_overrides(&mut self) {
        // Server config
        if let Ok(host) = std::env::var("SERVER_HOST") {
            self.server.host = host;
        }
        if let Ok(port) = std::env::var("SERVER_PORT") {
            if let Ok(port_num) = port.parse() {
                self.server.port = port_num;
            }
        }

        // Database config
        if let Ok(enabled) = std::env::var("DATABASE_ENABLED") {
            self.database.enabled = enabled.eq_ignore_ascii_case("true");
        }
        if let Ok(url) = std::env::var("DATABASE_URL") {
            self.database.url = url;
        }

        // Auth config
        if let Ok(enabled) = std::env::var("AUTH_ENABLED") {
            self.auth.enabled = enabled.eq_ignore_ascii_case("true");
        }
        if let Ok(require) = std::env::var("AUTH_REQUIRE_API_KEY") {
            self.auth.require_api_key = require.eq_ignore_ascii_case("true");
        }

        // Retention config
        if let Ok(hours) = std::env::var("RETENTION_RAW_METRICS_HOURS") {
            if let Ok(h) = hours.parse() {
                self.retention.raw_metrics_hours = h;
            }
        }
        if let Ok(days) = std::env::var("RETENTION_MINUTE_AGGREGATES_DAYS") {
            if let Ok(d) = days.parse() {
                self.retention.minute_aggregates_days = d;
            }
        }
        if let Ok(days) = std::env::var("RETENTION_HOUR_AGGREGATES_DAYS") {
            if let Ok(d) = days.parse() {
                self.retention.hour_aggregates_days = d;
            }
        }
        if let Ok(days) = std::env::var("RETENTION_ALERTS_DAYS") {
            if let Ok(d) = days.parse() {
                self.retention.alerts_days = d;
            }
        }
        if let Ok(interval) = std::env::var("RETENTION_CLEANUP_INTERVAL_HOURS") {
            if let Ok(i) = interval.parse() {
                self.retention.cleanup_interval_hours = i;
            }
        }

        // Storage config
        if let Ok(max_points) = std::env::var("STORAGE_MAX_POINTS") {
            if let Ok(mp) = max_points.parse() {
                self.storage.max_points_per_metric = mp;
            }
        }
        if let Ok(interval) = std::env::var("METRICS_PERSIST_INTERVAL_SECONDS") {
            if let Ok(interval) = interval.parse::<u64>() {
                if interval > 0 {
                    self.storage.persistence_interval_seconds = interval;
                }
            }
        }
    }

    pub fn default() -> Self {
        let mut config = Config {
            server: ServerConfig {
                host: default_host(),
                port: default_port(),
            },
            storage: StorageConfig {
                max_points_per_metric: default_max_points(),
                persistence_interval_seconds: default_persistence_interval(),
            },
            database: DatabaseConfig::default(),
            retention: RetentionConfig::default(),
            aggregation: AggregationConfig::default(),
            alerts: AlertsConfig::default(),
            websocket: WebSocketConfig::default(),
            auth: AuthConfig::default(),
        };

        // Apply environment variable overrides
        config.apply_env_overrides();

        config
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

#[cfg(test)]
mod tests;
