use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub delivery: DeliveryConfig,
    pub agent: AgentConfig,
    pub collection: CollectionConfig,
    pub metrics: MetricsConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub database: Option<DatabaseConfig>,
    #[serde(default)]
    pub logs: Option<LogsConfig>,
    #[serde(default)]
    pub process_watch: ProcessWatchConfig,
    #[serde(default)]
    pub service_watch: ServiceWatchConfig,
    #[serde(default)]
    pub docker_monitor: DockerMonitorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryConfig {
    pub directory: std::path::PathBuf,
    pub max_bytes: u64,
    pub max_records: usize,
    #[serde(default = "default_replay_interval")]
    pub replay_interval_ms: u64,
}
fn default_replay_interval() -> u64 {
    100
}
impl Default for DeliveryConfig {
    fn default() -> Self {
        Self {
            directory: "./agent-spool".into(),
            max_bytes: 256 * 1024 * 1024,
            max_records: 100_000,
            replay_interval_ms: default_replay_interval(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceWatchConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_service_names")]
    pub names: Vec<String>,
    #[serde(default = "default_deep_collection_interval")]
    pub interval_seconds: u64,
}

impl Default for ServiceWatchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            names: default_service_names(),
            interval_seconds: default_deep_collection_interval(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerMonitorConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_docker_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_deep_collection_interval")]
    pub interval_seconds: u64,
}

impl Default for DockerMonitorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: default_docker_endpoint(),
            interval_seconds: default_deep_collection_interval(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessWatchConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Executable names to watch (case-insensitive exact match).
    #[serde(default)]
    pub names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_batch_interval")]
    pub batch_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub enabled: bool,
    #[serde(default = "default_database_interval")]
    pub interval_seconds: u64,
    pub db_type: String, // "postgres", "mysql"
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_server_url")]
    pub url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,
    #[serde(default = "default_retry_delay")]
    pub retry_delay_seconds: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: default_server_url(),
            api_key: None,
            retry_attempts: default_retry_attempts(),
            retry_delay_seconds: default_retry_delay(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    #[serde(default = "default_interval")]
    pub interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    #[serde(default = "default_true")]
    pub collect_cpu: bool,
    #[serde(default = "default_true")]
    pub collect_memory: bool,
    #[serde(default = "default_true")]
    pub collect_disk: bool,
    #[serde(default = "default_true")]
    pub collect_network: bool,
}

fn default_interval() -> u64 {
    10
}

fn default_true() -> bool {
    true
}

fn default_server_url() -> String {
    "http://localhost:8080".to_string()
}

fn default_retry_attempts() -> u32 {
    3
}

fn default_retry_delay() -> u64 {
    2
}

fn default_batch_size() -> usize {
    100
}

fn default_batch_interval() -> u64 {
    5
}

fn default_database_interval() -> u64 {
    15
}

fn default_deep_collection_interval() -> u64 {
    15
}

fn default_docker_endpoint() -> String {
    "file:///run/watchtower/docker-telemetry.json".to_string()
}

fn default_service_names() -> Vec<String> {
    ["watchtower-agent", "docker", "ssh"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn default() -> Self {
        Config {
            delivery: DeliveryConfig::default(),
            agent: AgentConfig {
                name: "default-agent".to_string(),
            },
            collection: CollectionConfig {
                interval_seconds: 10,
            },
            metrics: MetricsConfig {
                collect_cpu: true,
                collect_memory: true,
                collect_disk: true,
                collect_network: true,
            },
            server: ServerConfig {
                enabled: false,
                url: default_server_url(),
                api_key: None,
                retry_attempts: default_retry_attempts(),
                retry_delay_seconds: default_retry_delay(),
            },
            database: None,
            logs: None,
            process_watch: ProcessWatchConfig::default(),
            service_watch: ServiceWatchConfig::default(),
            docker_monitor: DockerMonitorConfig::default(),
        }
    }

    /// Apply the environment contract used by the container image.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(directory) = std::env::var("AGENT_SPOOL_DIRECTORY") {
            self.delivery.directory = directory.into();
        }
        if let Ok(value) = std::env::var("AGENT_SPOOL_MAX_BYTES") {
            if let Ok(limit) = value.parse::<u64>() {
                self.delivery.max_bytes = limit;
            }
        }
        if let Ok(value) = std::env::var("AGENT_SPOOL_MAX_RECORDS") {
            if let Ok(limit) = value.parse::<usize>() {
                self.delivery.max_records = limit;
            }
        }
        if let Ok(name) = std::env::var("AGENT_NAME") {
            if !name.trim().is_empty() {
                self.agent.name = name;
            }
        }
        if let Ok(url) = std::env::var("SERVER_URL") {
            if !url.trim().is_empty() {
                self.server.url = url;
                self.server.enabled = true;
            }
        }
        if let Ok(api_key) = std::env::var("API_KEY") {
            if !api_key.trim().is_empty() {
                self.server.api_key = Some(api_key);
            }
        }
        if let Ok(interval) = std::env::var("COLLECTION_INTERVAL") {
            if let Ok(interval) = interval.parse::<u64>() {
                if interval > 0 {
                    self.collection.interval_seconds = interval;
                }
            }
        }
        if let Ok(enabled) = std::env::var("PROCESS_WATCH_ENABLED") {
            self.process_watch.enabled = enabled.eq_ignore_ascii_case("true") || enabled == "1";
        }
        if let Ok(names) = std::env::var("PROCESS_WATCH_NAMES") {
            self.process_watch.names = names
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .collect();
        }

        if let Some(enabled) = env_bool("SERVICE_WATCH_ENABLED") {
            self.service_watch.enabled = enabled;
        }
        if let Ok(names) = std::env::var("SERVICE_WATCH_NAMES") {
            self.service_watch.names = split_csv(&names);
        }
        if let Ok(interval) = std::env::var("SERVICE_WATCH_INTERVAL_SECONDS") {
            if let Ok(interval) = interval.parse::<u64>() {
                self.service_watch.interval_seconds = interval.max(5);
            }
        }
        if let Some(enabled) = env_bool("DOCKER_MONITOR_ENABLED") {
            self.docker_monitor.enabled = enabled;
        }
        set_nonempty_env("DOCKER_MONITOR_ENDPOINT", &mut self.docker_monitor.endpoint);
        if let Ok(interval) = std::env::var("DOCKER_MONITOR_INTERVAL_SECONDS") {
            if let Ok(interval) = interval.parse::<u64>() {
                self.docker_monitor.interval_seconds = interval.max(5);
            }
        }

        if let Some(enabled) = env_bool("DB_MONITOR_ENABLED") {
            if enabled {
                let mut database = self.database.take().unwrap_or_else(|| DatabaseConfig {
                    enabled: true,
                    interval_seconds: default_database_interval(),
                    db_type: "postgres".to_string(),
                    host: "127.0.0.1".to_string(),
                    port: 5432,
                    database: "postgres".to_string(),
                    username: "postgres".to_string(),
                    password: String::new(),
                });
                database.enabled = true;
                set_nonempty_env("DB_MONITOR_TYPE", &mut database.db_type);
                set_nonempty_env("DB_MONITOR_HOST", &mut database.host);
                set_nonempty_env("DB_MONITOR_DATABASE", &mut database.database);
                set_nonempty_env("DB_MONITOR_USERNAME", &mut database.username);
                set_nonempty_env("DB_MONITOR_PASSWORD", &mut database.password);
                if let Ok(port) = std::env::var("DB_MONITOR_PORT") {
                    if let Ok(port) = port.parse::<u16>() {
                        if port > 0 {
                            database.port = port;
                        }
                    }
                }
                if let Ok(interval) = std::env::var("DB_MONITOR_INTERVAL_SECONDS") {
                    if let Ok(interval) = interval.parse::<u64>() {
                        if interval > 0 {
                            database.interval_seconds = interval;
                        }
                    }
                }
                self.database = Some(database);
            } else if let Some(database) = self.database.as_mut() {
                database.enabled = false;
            }
        }

        if let Some(enabled) = env_bool("LOG_COLLECTION_ENABLED") {
            if enabled {
                let mut logs = self.logs.take().unwrap_or_else(|| LogsConfig {
                    enabled: true,
                    paths: Vec::new(),
                    batch_size: default_batch_size(),
                    batch_interval_seconds: default_batch_interval(),
                });
                logs.enabled = true;
                if let Ok(paths) = std::env::var("LOG_PATHS") {
                    logs.paths = paths
                        .split(',')
                        .map(str::trim)
                        .filter(|path| !path.is_empty())
                        .map(str::to_string)
                        .collect();
                }
                if let Ok(size) = std::env::var("LOG_BATCH_SIZE") {
                    if let Ok(size) = size.parse::<usize>() {
                        if size > 0 {
                            logs.batch_size = size;
                        }
                    }
                }
                if let Ok(interval) = std::env::var("LOG_BATCH_INTERVAL_SECONDS") {
                    if let Ok(interval) = interval.parse::<u64>() {
                        if interval > 0 {
                            logs.batch_interval_seconds = interval;
                        }
                    }
                }
                self.logs = Some(logs);
            } else if let Some(logs) = self.logs.as_mut() {
                logs.enabled = false;
            }
        }
    }
}

fn env_bool(name: &str) -> Option<bool> {
    std::env::var(name).ok().map(|value| {
        value.eq_ignore_ascii_case("true") || value == "1" || value.eq_ignore_ascii_case("yes")
    })
}

fn set_nonempty_env(name: &str, target: &mut String) {
    if let Ok(value) = std::env::var(name) {
        if !value.trim().is_empty() {
            *target = value;
        }
    }
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_collection_interval_is_nonzero() {
        assert!(Config::default().collection.interval_seconds > 0);
    }
}
