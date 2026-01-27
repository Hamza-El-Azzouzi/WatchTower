use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub agent: AgentConfig,
    pub collection: CollectionConfig,
    pub metrics: MetricsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
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
        }
    }
}
