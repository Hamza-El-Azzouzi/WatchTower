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
        }
    }
}
