use std::{net::IpAddr, sync::Arc, time::Duration};

use anyhow::{anyhow, Context, Result};
use lettre::{
    message::Mailbox,
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use reqwest::{redirect::Policy, Client, Url};
use serde_json::{json, Value};
use tracing::{error, info, warn};

use super::{AlertEventType, AlertTransition, NotificationChannel, NotificationChannelType};
use crate::db::Database;

#[derive(Clone)]
pub struct NotificationDispatcher {
    database: Arc<Database>,
}

impl NotificationDispatcher {
    pub fn new(database: Arc<Database>) -> Result<Self> {
        Ok(Self { database })
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
        let owner = self
            .database
            .agent_owners()
            .await?
            .get(&transition.alert.agent_id)
            .copied();
        channels.retain(|channel| {
            channel.enabled && owner.is_some() && channel.owner_api_key_id == owner
        });
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
        let parsed = Url::parse(url)?;
        let host = parsed.host_str().context("Webhook host missing")?;
        let addresses =
            public_addresses(host, parsed.port_or_known_default().unwrap_or(443)).await?;
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(Policy::none())
            .no_proxy()
            .resolve_to_addrs(host, &addresses)
            .user_agent("WatchTower/0.1 alert-delivery")
            .build()?;
        let response = client.post(url).json(&body).send().await?;
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

        validate_smtp_settings(smtp_host, channel.smtp_port, channel.smtp_tls)?;
        let addresses = public_addresses(smtp_host, channel.smtp_port as u16).await?;
        let tls = TlsParameters::new(smtp_host.to_owned())?;
        let mut builder =
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(addresses[0].ip().to_string())
                .port(channel.smtp_port as u16)
                .timeout(Some(Duration::from_secs(10)))
                .tls(if channel.smtp_port == 465 {
                    Tls::Wrapper(tls)
                } else {
                    Tls::Required(tls)
                });
        if let (Some(username), Some(password)) = (&channel.smtp_username, &channel.smtp_password) {
            builder = builder.credentials(Credentials::new(username.clone(), password.clone()));
        }
        builder.build().send(message).await?;
        Ok(())
    }
}

pub(crate) fn alert_payload(transition: &AlertTransition) -> Value {
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
    let host = url
        .host_str()
        .context("Webhook URL has no host")?
        .trim_start_matches('[')
        .trim_end_matches(']');
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

pub fn validate_smtp_settings(host: &str, port: i32, tls: bool) -> Result<()> {
    if !tls || !matches!(port, 465 | 587) || host.trim().is_empty() {
        return Err(anyhow!("SMTP requires TLS on port 465 or 587"));
    }
    validate_webhook_url(&format!("https://{host}/"))?;
    Ok(())
}

pub(crate) async fn public_addresses(host: &str, port: u16) -> Result<Vec<std::net::SocketAddr>> {
    let addresses: Vec<_> = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::net::lookup_host((host, port)),
    )
    .await??
    .collect();
    if addresses.is_empty()
        || addresses
            .iter()
            .any(|address| is_private_address(address.ip()))
    {
        return Err(anyhow!(
            "Notification destination must resolve exclusively to public addresses"
        ));
    }
    Ok(addresses)
}

fn is_private_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_unspecified()
                || ip.is_multicast()
                || ip.is_documentation()
                || ip.octets()[0] == 0
                || (ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
                || ip.octets()[0] >= 240
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
                || ip.is_multicast()
                || ip
                    .to_ipv4_mapped()
                    .is_some_and(|ip| is_private_address(IpAddr::V4(ip)))
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
        assert!(validate_webhook_url("https://[::1]/hook").is_err());
        assert!(validate_webhook_url("https://[::ffff:127.0.0.1]/hook").is_err());
        assert!(validate_webhook_url("https://169.254.169.254/hook").is_err());
        assert!(validate_webhook_url("https://hooks.slack.com/services/test").is_ok());
    }

    #[test]
    fn requires_secure_public_smtp_settings() {
        assert!(validate_smtp_settings("smtp.example.com", 587, true).is_ok());
        assert!(validate_smtp_settings("smtp.example.com", 465, true).is_ok());
        assert!(validate_smtp_settings("smtp.example.com", 587, false).is_err());
        assert!(validate_smtp_settings("smtp.example.com", 25, true).is_err());
        assert!(validate_smtp_settings("127.0.0.1", 587, true).is_err());
    }
}
