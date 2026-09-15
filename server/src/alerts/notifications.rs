use std::{net::IpAddr, sync::Arc, time::Duration};

use anyhow::{anyhow, Context, Result};
use lettre::{
    message::Mailbox, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};
use reqwest::{redirect::Policy, Client, Url};
use serde_json::{json, Value};
use tracing::{error, info, warn};

use super::{AlertEventType, AlertTransition, NotificationChannel, NotificationChannelType};
use crate::db::Database;

#[derive(Clone)]
pub struct NotificationDispatcher {
    database: Arc<Database>,
    http: Client,
}

impl NotificationDispatcher {
    pub fn new(database: Arc<Database>) -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(Policy::none())
            .user_agent("WatchTower/0.1 alert-delivery")
            .build()?;
        Ok(Self { database, http })
    }

    pub async fn enqueue_transition(&self, transition: &AlertTransition) -> Result<usize> {
        if self
            .database
            .is_alert_silenced(&transition.alert.rule_id, &transition.alert.agent_id)
            .await?
        {
            info!(alert_id = %transition.alert.id, "Notification suppressed by an active silence");
            return Ok(0);
        }

        let mut channels = self.database.list_notification_channels().await?;
        channels.retain(|channel| channel.enabled);
        if !transition.channel_ids.is_empty() {
            channels.retain(|channel| transition.channel_ids.contains(&channel.id));
        } else if transition.alert.rule_id != "system-agent-offline" {
            channels.clear();
        }

        let payload = alert_payload(transition);
        for channel in &channels {
            self.database
                .enqueue_notification(
                    &transition.alert.id,
                    channel,
                    transition.event_type.as_str(),
                    &payload,
                )
                .await?;
        }
        Ok(channels.len())
    }

    pub async fn enqueue_test(&self, channel: &NotificationChannel) -> Result<i64> {
        let payload = json!({
            "event_type": "test",
            "title": "WatchTower test notification",
            "message": "This channel is configured correctly and can receive production alerts.",
            "severity": "info",
            "agent_id": "watchtower",
            "occurred_at": chrono::Utc::now(),
        });
        self.database
            .enqueue_notification(
                &format!("test-{}", uuid::Uuid::new_v4()),
                channel,
                AlertEventType::Test.as_str(),
                &payload,
            )
            .await
    }

    pub async fn run(self) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            if let Err(error) = self.process_due().await {
                error!(%error, "Notification delivery worker failed");
            }
        }
    }

    async fn process_due(&self) -> Result<()> {
        let deliveries = self.database.claim_due_notifications(20).await?;
        for pending in deliveries {
            let result = self.deliver(&pending.channel, &pending.payload).await;
            match result {
                Ok(status) => {
                    self.database
                        .finish_notification_attempt(pending.delivery.id, true, status, None)
                        .await?;
                    info!(delivery_id = pending.delivery.id, channel = %pending.channel.name, "Alert delivered");
                }
                Err(error) => {
                    let message = truncate_error(&error.to_string());
                    self.database
                        .finish_notification_attempt(
                            pending.delivery.id,
                            false,
                            None,
                            Some(&message),
                        )
                        .await?;
                    warn!(delivery_id = pending.delivery.id, channel = %pending.channel.name, %error, "Alert delivery failed; retry scheduled");
                }
            }
        }
        Ok(())
    }

    async fn deliver(&self, channel: &NotificationChannel, payload: &Value) -> Result<Option<i32>> {
        match channel.channel_type {
            NotificationChannelType::Email => {
                self.deliver_email(channel, payload).await?;
                Ok(None)
            }
            _ => self.deliver_webhook(channel, payload).await.map(Some),
        }
    }

    async fn deliver_webhook(&self, channel: &NotificationChannel, payload: &Value) -> Result<i32> {
        let url = channel
            .webhook_url
            .as_deref()
            .context("Webhook URL is missing")?;
        validate_webhook_url(url)?;
        let body = match channel.channel_type {
            NotificationChannelType::Slack => json!({
                "text": format!("*{}*\n{}", payload["title"].as_str().unwrap_or("WatchTower alert"), payload["message"].as_str().unwrap_or("")),
            }),
            NotificationChannelType::Discord => json!({
                "content": format!("**{}**\n{}", payload["title"].as_str().unwrap_or("WatchTower alert"), payload["message"].as_str().unwrap_or("")),
            }),
            _ => payload.clone(),
        };
        let response = self.http.post(url).json(&body).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(anyhow!("Webhook returned HTTP {}", status.as_u16()));
        }
        Ok(status.as_u16() as i32)
    }

    async fn deliver_email(&self, channel: &NotificationChannel, payload: &Value) -> Result<()> {
        let smtp_host = channel
            .smtp_host
            .as_deref()
            .context("SMTP host is missing")?;
        let from: Mailbox = channel
            .smtp_from
            .as_deref()
            .context("SMTP sender is missing")?
            .parse()?;
        let to: Mailbox = channel
            .email_to
            .as_deref()
            .context("Email recipient is missing")?
            .parse()?;
        let subject = payload["title"].as_str().unwrap_or("WatchTower alert");
        let message = Message::builder()
            .from(from)
            .to(to)
            .subject(subject)
            .body(payload["message"].as_str().unwrap_or("").to_string())?;

        let mut builder = if channel.smtp_tls {
            AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_host)?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(smtp_host)
        };
        builder = builder.port(channel.smtp_port.clamp(1, 65535) as u16);
        if let (Some(username), Some(password)) = (&channel.smtp_username, &channel.smtp_password) {
            builder = builder.credentials(Credentials::new(username.clone(), password.clone()));
        }
        builder.build().send(message).await?;
        Ok(())
    }
}

fn alert_payload(transition: &AlertTransition) -> Value {
    let alert = &transition.alert;
    let title = match transition.event_type {
        AlertEventType::Resolved => {
            format!("Recovered: {} on {}", alert.rule_name, alert.agent_name)
        }
        _ => format!(
            "{} alert: {} on {}",
            severity_label(&alert.severity),
            alert.rule_name,
            alert.agent_name
        ),
    };
    json!({
        "event_type": transition.event_type.as_str(),
        "title": title,
        "message": alert.message,
        "alert_id": alert.id,
        "rule_id": alert.rule_id,
        "rule_name": alert.rule_name,
        "agent_id": alert.agent_id,
        "agent_name": alert.agent_name,
        "metric": alert.metric,
        "current_value": alert.current_value,
        "threshold": alert.threshold,
        "severity": severity_label(&alert.severity).to_lowercase(),
        "state": format!("{:?}", alert.state).to_lowercase(),
        "occurred_at": chrono::Utc::now(),
    })
}

fn severity_label(severity: &super::AlertSeverity) -> &'static str {
    match severity {
        super::AlertSeverity::Info => "Info",
        super::AlertSeverity::Warning => "Warning",
        super::AlertSeverity::Critical => "Critical",
    }
}

pub fn validate_webhook_url(value: &str) -> Result<()> {
    let url = Url::parse(value).context("Invalid webhook URL")?;
    if url.scheme() != "https" {
        return Err(anyhow!("Webhook URL must use HTTPS"));
    }
    let host = url.host_str().context("Webhook URL has no host")?;
    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".local") {
        return Err(anyhow!("Local webhook destinations are not allowed"));
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_private_address(ip) {
            return Err(anyhow!("Private webhook destinations are not allowed"));
        }
    }
    Ok(())
}

fn is_private_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_unspecified()
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
        }
    }
}

fn truncate_error(value: &str) -> String {
    value.chars().take(1000).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_webhook_destinations() {
        assert!(validate_webhook_url("http://example.com/hook").is_err());
        assert!(validate_webhook_url("https://127.0.0.1/hook").is_err());
        assert!(validate_webhook_url("https://hooks.slack.com/services/test").is_ok());
    }
}
