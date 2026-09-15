use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use hyper::{body::to_bytes, Body, Client};
use hyperlocal::{UnixClientExt, UnixConnector, Uri};
use reqwest::Client as HttpClient;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf, time::Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSnapshot {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub health: String,
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    pub memory_limit_bytes: u64,
    pub memory_percent: f64,
    pub restart_count: u64,
    pub run_time_seconds: u64,
}

enum DockerSource {
    LoopbackHttp {
        client: HttpClient,
        endpoint: String,
    },
    SnapshotFile(PathBuf),
    UnixSocket(PathBuf),
}

pub struct DockerCollector {
    source: DockerSource,
}

impl DockerCollector {
    pub fn new(endpoint: String) -> Result<Self> {
        if let Some(path) = endpoint.strip_prefix("file://") {
            let path = PathBuf::from(path);
            if !path.is_absolute() {
                anyhow::bail!("Docker snapshot path must be absolute");
            }
            return Ok(Self {
                source: DockerSource::SnapshotFile(path),
            });
        }

        let endpoint = endpoint.trim_end_matches('/').to_string();
        if !endpoint.starts_with("http://127.0.0.1:") && !endpoint.starts_with("http://localhost:")
        {
            anyhow::bail!("Docker metrics endpoint must be a local snapshot or loopback HTTP");
        }
        Ok(Self {
            source: DockerSource::LoopbackHttp {
                client: HttpClient::builder()
                    .timeout(Duration::from_secs(5))
                    .build()?,
                endpoint,
            },
        })
    }

    pub fn from_unix_socket(socket: PathBuf) -> Self {
        Self {
            source: DockerSource::UnixSocket(socket),
        }
    }

    pub async fn collect(&self) -> Result<Vec<ContainerSnapshot>> {
        if let DockerSource::SnapshotFile(path) = &self.source {
            let contents = fs::read(path)
                .with_context(|| format!("Failed to read Docker snapshot {}", path.display()))?;
            let mut snapshots: Vec<ContainerSnapshot> =
                serde_json::from_slice(&contents).context("Invalid Docker telemetry snapshot")?;
            snapshots.truncate(32);
            snapshots.sort_by(|left, right| left.name.cmp(&right.name));
            return Ok(snapshots);
        }

        let containers: Vec<ContainerSummary> = self
            .get_json("/containers/json?all=1")
            .await
            .context("Invalid Docker container list response")?;
        let mut snapshots = Vec::new();
        for container in containers.into_iter().take(32) {
            if !is_container_id(&container.id) {
                continue;
            }
            let inspect: ContainerInspect = self
                .get_json(&format!("/containers/{}/json", container.id))
                .await?;
            let stats = self
                .get_json::<ContainerStats>(&format!(
                    "/containers/{}/stats?stream=false",
                    container.id
                ))
                .await
                .ok();
            let (cpu_percent, memory_bytes, memory_limit_bytes, memory_percent) = stats
                .as_ref()
                .map(resource_usage)
                .unwrap_or((0.0, 0, 0, 0.0));
            let state = inspect.state.status.clone().unwrap_or(container.state);
            let health = inspect
                .state
                .health
                .as_ref()
                .and_then(|health| health.status.clone())
                .unwrap_or_else(|| {
                    if state == "running" {
                        "none".to_string()
                    } else {
                        state.clone()
                    }
                });
            let started_at = inspect
                .state
                .started_at
                .as_deref()
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&Utc));
            snapshots.push(ContainerSnapshot {
                id: container.id.chars().take(12).collect(),
                name: container
                    .names
                    .first()
                    .map(|name| name.trim_start_matches('/').to_string())
                    .unwrap_or_else(|| container.id.chars().take(12).collect()),
                image: container.image.chars().take(160).collect(),
                state,
                health,
                cpu_percent,
                memory_bytes,
                memory_limit_bytes,
                memory_percent,
                restart_count: inspect.restart_count,
                run_time_seconds: started_at
                    .map(|started| {
                        Utc::now()
                            .signed_duration_since(started)
                            .num_seconds()
                            .max(0) as u64
                    })
                    .unwrap_or(0),
            });
        }
        snapshots.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(snapshots)
    }

    async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        match &self.source {
            DockerSource::LoopbackHttp { client, endpoint } => Ok(client
                .get(format!("{endpoint}{path}"))
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?),
            DockerSource::UnixSocket(socket) => {
                let client: Client<UnixConnector, Body> = Client::unix();
                let uri: hyper::Uri = Uri::new(socket, path).into();
                let response = client.get(uri).await?;
                if !response.status().is_success() {
                    anyhow::bail!("Docker API returned {} for {path}", response.status());
                }
                let body = to_bytes(response.into_body()).await?;
                Ok(serde_json::from_slice(&body)?)
            }
            DockerSource::SnapshotFile(_) => {
                anyhow::bail!("snapshot files do not expose Docker API routes")
            }
        }
    }
}

fn is_container_id(value: &str) -> bool {
    value.len() >= 12
        && value.len() <= 128
        && value.chars().all(|character| character.is_ascii_hexdigit())
}

#[derive(Deserialize)]
struct ContainerSummary {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Names", default)]
    names: Vec<String>,
    #[serde(rename = "Image", default)]
    image: String,
    #[serde(rename = "State", default)]
    state: String,
}

#[derive(Deserialize, Default)]
struct ContainerInspect {
    #[serde(rename = "RestartCount", default)]
    restart_count: u64,
    #[serde(rename = "State", default)]
    state: ContainerState,
}

#[derive(Deserialize, Default)]
struct ContainerState {
    #[serde(rename = "Status")]
    status: Option<String>,
    #[serde(rename = "StartedAt")]
    started_at: Option<String>,
    #[serde(rename = "Health")]
    health: Option<ContainerHealth>,
}

#[derive(Deserialize)]
struct ContainerHealth {
    #[serde(rename = "Status")]
    status: Option<String>,
}

#[derive(Deserialize, Default)]
struct ContainerStats {
    #[serde(default)]
    cpu_stats: CpuStats,
    #[serde(default)]
    precpu_stats: CpuStats,
    #[serde(default)]
    memory_stats: MemoryStats,
}

#[derive(Deserialize, Default)]
struct CpuStats {
    #[serde(default)]
    cpu_usage: CpuUsage,
    #[serde(default)]
    system_cpu_usage: u64,
    #[serde(default)]
    online_cpus: u64,
}

#[derive(Deserialize, Default)]
struct CpuUsage {
    #[serde(default)]
    total_usage: u64,
}

#[derive(Deserialize, Default)]
struct MemoryStats {
    #[serde(default)]
    usage: u64,
    #[serde(default)]
    limit: u64,
    #[serde(default)]
    stats: HashMap<String, u64>,
}

fn resource_usage(stats: &ContainerStats) -> (f64, u64, u64, f64) {
    let cpu_delta = stats
        .cpu_stats
        .cpu_usage
        .total_usage
        .saturating_sub(stats.precpu_stats.cpu_usage.total_usage);
    let system_delta = stats
        .cpu_stats
        .system_cpu_usage
        .saturating_sub(stats.precpu_stats.system_cpu_usage);
    let cpus = stats.cpu_stats.online_cpus.max(1);
    let cpu_percent = if system_delta > 0 {
        cpu_delta as f64 / system_delta as f64 * cpus as f64 * 100.0
    } else {
        0.0
    };
    let cache = stats
        .memory_stats
        .stats
        .get("inactive_file")
        .or_else(|| stats.memory_stats.stats.get("cache"))
        .copied()
        .unwrap_or(0);
    let memory = stats.memory_stats.usage.saturating_sub(cache);
    let limit = stats.memory_stats.limit;
    let memory_percent = if limit > 0 {
        memory as f64 * 100.0 / limit as f64
    } else {
        0.0
    };
    (cpu_percent, memory, limit, memory_percent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_non_loopback_docker_endpoints() {
        assert!(DockerCollector::new("http://10.0.0.2:2375".to_string()).is_err());
    }

    #[test]
    fn accepts_absolute_snapshot_path() {
        assert!(DockerCollector::new("file:///run/watchtower/docker.json".to_string()).is_ok());
        assert!(DockerCollector::new("file://relative.json".to_string()).is_err());
    }

    #[test]
    fn validates_container_ids_before_building_paths() {
        assert!(is_container_id("0123456789abcdef"));
        assert!(!is_container_id("../../secrets"));
    }
}
