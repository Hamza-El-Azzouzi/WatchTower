use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, warn};

use crate::collector::process::ProcessSnapshot;
use crate::collector::{
    disk::MountSnapshot, docker::ContainerSnapshot, network::NetworkInterfaceSnapshot,
    service::ServiceSnapshot,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<crate::delivery::DeliveryIdentity>,
    pub agent_id: String,
    pub timestamp: DateTime<Utc>,
    pub metrics: HashMap<String, f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub processes: Vec<ProcessSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mounts: Vec<MountSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub network_interfaces: Vec<NetworkInterfaceSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub services: Vec<ServiceSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub containers: Vec<ContainerSnapshot>,
}

pub struct MetricsSender {
    client: Client,
    server_url: String,
    api_key: Option<String>,
    retry_attempts: u32,
    retry_delay: Duration,
}

impl MetricsSender {
    pub async fn send_queued(&self, payload: &crate::delivery::QueuedPayload) -> Result<()> {
        match payload {
            crate::delivery::QueuedPayload::Metrics(payload) => self.send_metrics(payload).await,
            crate::delivery::QueuedPayload::Logs(payload) => self.send_logs(payload).await,
        }
    }
    pub fn new(
        server_url: String,
        api_key: Option<String>,
        retry_attempts: u32,
        retry_delay_seconds: u64,
    ) -> Result<Self> {
        crate::control::validate_url(&server_url)?;
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            server_url,
            api_key,
            retry_attempts,
            retry_delay: Duration::from_secs(retry_delay_seconds),
        })
    }

    pub async fn send_metrics(&self, payload: &MetricsPayload) -> Result<()> {
        let endpoint = format!("{}/api/v1/metrics", self.server_url.trim_end_matches('/'));
        let mut last_error = None;

        for attempt in 1..=self.retry_attempts {
            debug!(
                "Sending metrics to {} (attempt {}/{})",
                endpoint, attempt, self.retry_attempts
            );

            let mut request = self
                .client
                .post(&endpoint)
                .query(&[("agent_id", payload.agent_id.as_str())])
                .json(payload);

            // Add Authorization header if API key is provided
            if let Some(ref api_key) = self.api_key {
                request = request.header("Authorization", format!("Bearer {}", api_key));
            }

            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        if payload.delivery.is_some() {
                            let acknowledgement = crate::control::bounded_json(response)
                                .await
                                .context("invalid durable acknowledgement")?;
                            if acknowledgement["durable"] != true
                                || acknowledgement["delivery"]
                                    != serde_json::to_value(&payload.delivery)?
                            {
                                return Err(anyhow::anyhow!(
                                    "server did not acknowledge durable delivery; retaining record"
                                ));
                            }
                        }
                        debug!("Metrics sent successfully: {}", status);
                        return Ok(());
                    } else if status == reqwest::StatusCode::FORBIDDEN {
                        // 403 Forbidden - API key invalid or agent limit reached
                        let error_msg = format!("HTTP {status}");
                        error!(
                            "API key is invalid, expired, or agent limit reached: {}",
                            error_msg
                        );
                        return Err(anyhow::anyhow!(
                            "upload authentication rejected; retaining queued metrics"
                        ));
                    } else {
                        let error_msg = format!("HTTP {status}");
                        warn!("Server returned error status {}: {}", status, error_msg);
                        last_error =
                            Some(anyhow::anyhow!("Server error: {} - {}", status, error_msg));
                    }
                }
                Err(e) => {
                    warn!("Failed to send metrics (attempt {}): {}", attempt, e);
                    last_error = Some(anyhow::anyhow!("Request failed: {}", e));
                }
            }

            // Wait before retrying (except on last attempt)
            if attempt < self.retry_attempts {
                let delay = self.retry_delay * attempt;
                debug!("Retrying in {:?}...", delay);
                tokio::time::sleep(delay).await;
            }
        }

        let error = last_error.unwrap_or_else(|| anyhow::anyhow!("All retry attempts failed"));
        error!(
            "Failed to send metrics after {} attempts",
            self.retry_attempts
        );
        Err(error)
    }

    pub async fn send_logs(&self, payload: &crate::collector::LogsPayload) -> Result<()> {
        let endpoint = format!("{}/api/v1/logs", self.server_url);
        let mut last_error = None;

        for attempt in 1..=self.retry_attempts {
            debug!(
                "Sending {} logs to {} (attempt {}/{})",
                payload.logs.len(),
                endpoint,
                attempt,
                self.retry_attempts
            );

            let mut request = self.client.post(&endpoint).json(payload);

            // Add Authorization header if API key is provided
            if let Some(ref api_key) = self.api_key {
                request = request.header("Authorization", format!("Bearer {}", api_key));
            }

            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        debug!("Logs sent successfully: {}", status);
                        if payload.delivery.is_some() {
                            let acknowledgement = crate::control::bounded_json(response)
                                .await
                                .context("invalid durable log acknowledgement")?;
                            if acknowledgement["durable"] != true
                                || acknowledgement["delivery"]
                                    != serde_json::to_value(&payload.delivery)?
                            {
                                return Err(anyhow::anyhow!(
                                    "server did not acknowledge durable logs; retaining record"
                                ));
                            }
                        }
                        return Ok(());
                    } else {
                        let error_msg = format!("HTTP {status}");
                        warn!("Server returned error status {}: {}", status, error_msg);
                        last_error =
                            Some(anyhow::anyhow!("Server error: {} - {}", status, error_msg));
                    }
                }
                Err(e) => {
                    warn!("Failed to send logs (attempt {}): {}", attempt, e);
                    last_error = Some(anyhow::anyhow!("Request failed: {}", e));
                }
            }

            // Wait before retrying (except on last attempt)
            if attempt < self.retry_attempts {
                let delay = self.retry_delay * attempt;
                debug!("Retrying in {:?}...", delay);
                tokio::time::sleep(delay).await;
            }
        }

        let error = last_error.unwrap_or_else(|| anyhow::anyhow!("All retry attempts failed"));
        error!("Failed to send logs after {} attempts", self.retry_attempts);
        Err(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sender_creation() {
        let sender = MetricsSender::new("http://localhost:8080".to_string(), None, 3, 2);
        assert!(sender.is_ok());
    }
    #[tokio::test]
    async fn requires_exact_durable_acknowledgement() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let payload = MetricsPayload {
            delivery: Some(crate::delivery::DeliveryIdentity {
                stream_id: "0123456789abcdef0123456789abcdef".into(),
                sequence: 1,
            }),
            agent_id: "test".into(),
            timestamp: Utc::now(),
            metrics: [("cpu_usage".into(), 1.0)].into(),
            processes: vec![],
            mounts: vec![],
            network_interfaces: vec![],
            services: vec![],
            containers: vec![],
        };
        for (body, expected) in [
            (serde_json::json!({"status":"accepted"}), false),
            (
                serde_json::json!({"durable":true,"delivery":{"stream_id":"wrong","sequence":1}}),
                false,
            ),
            (
                serde_json::json!({"durable":true,"delivery":payload.delivery}),
                true,
            ),
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut buffer = vec![0; 16384];
                let _ = socket.read(&mut buffer).await.unwrap();
                let body = body.to_string();
                let response=format!("HTTP/1.1 202 Accepted\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body);
                socket.write_all(response.as_bytes()).await.unwrap();
            });
            let sender = MetricsSender::new(format!("http://{address}"), None, 1, 0).unwrap();
            assert_eq!(sender.send_metrics(&payload).await.is_ok(), expected);
            server.await.unwrap();
        }
    }
}
