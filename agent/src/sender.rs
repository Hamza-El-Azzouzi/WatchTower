use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsPayload {
    pub agent_id: String,
    pub timestamp: DateTime<Utc>,
    pub metrics: HashMap<String, f64>,
}

pub struct MetricsSender {
    client: Client,
    server_url: String,
    api_key: Option<String>,
    retry_attempts: u32,
    retry_delay: Duration,
}

impl MetricsSender {
    pub fn new(
        server_url: String,
        api_key: Option<String>,
        retry_attempts: u32,
        retry_delay_seconds: u64,
    ) -> Result<Self> {
        let client = Client::builder()
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
        let endpoint = format!(
            "{}/api/v1/metrics?agent_id={}",
            self.server_url, payload.agent_id
        );
        let mut last_error = None;

        for attempt in 1..=self.retry_attempts {
            debug!(
                "Sending metrics to {} (attempt {}/{})",
                endpoint, attempt, self.retry_attempts
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
                        debug!("Metrics sent successfully: {}", status);
                        return Ok(());
                    } else {
                        let error_msg = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
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
                        return Ok(());
                    } else {
                        let error_msg = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
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
}
