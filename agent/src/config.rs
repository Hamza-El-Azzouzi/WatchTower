use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
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

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn default() -> Self {
        Config {
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
        }
    }

    /// Apply the environment contract used by the container image.
    pub fn apply_env_overrides(&mut self) {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_collection_interval_is_nonzero() {
        assert!(Config::default().collection.interval_seconds > 0);
    }
}
