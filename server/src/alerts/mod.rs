use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod manager;
pub mod notifications;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    #[serde(default)]
    pub owner_api_key_id: Option<i64>,
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub metric: String,
    pub condition: AlertCondition,
    pub threshold: f64,
    #[serde(default)]
    pub threshold_percent: Option<f64>, // For percentage of max (e.g., db connections)
    pub duration_seconds: u64,
    pub severity: AlertSeverity,
    pub channels: Vec<String>,
    pub enabled: bool,
    #[serde(default)]
    pub agent_filter: Option<String>, // Optional: only alert for specific agent
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub cooldown_seconds: u64, // Prevent notification spam
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AlertCondition {
    GreaterThan,
    LessThan,
    Equals,
    NotEquals,
}

impl AlertCondition {
    pub fn evaluate(&self, value: f64, threshold: f64) -> bool {
        match self {
            AlertCondition::GreaterThan => value > threshold,
            AlertCondition::LessThan => value < threshold,
            AlertCondition::Equals => (value - threshold).abs() < f64::EPSILON,
            AlertCondition::NotEquals => (value - threshold).abs() >= f64::EPSILON,
        }
    }

    pub fn symbol(&self) -> &str {
        match self {
            AlertCondition::GreaterThan => ">",
            AlertCondition::LessThan => "<",
            AlertCondition::Equals => "=",
            AlertCondition::NotEquals => "!=",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl AlertSeverity {
    pub fn emoji(&self) -> &str {
        match self {
            AlertSeverity::Info => "ℹ️",
            AlertSeverity::Warning => "⚠️",
            AlertSeverity::Critical => "🚨",
        }
    }

    #[allow(dead_code)]
    pub fn color(&self) -> u32 {
        match self {
            AlertSeverity::Info => 0x3b82f6,     // Blue
            AlertSeverity::Warning => 0xf59e0b,  // Yellow
            AlertSeverity::Critical => 0xef4444, // Red
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub agent_id: String,
    pub agent_name: String,
    pub state: AlertState,
    pub metric: String,
    pub current_value: f64,
    pub threshold: f64,
    pub condition: AlertCondition,
    pub severity: AlertSeverity,
    pub message: String,
    pub triggered_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub acknowledged: bool,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<String>,
    pub last_notification_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AlertEventType {
    Firing,
    Resolved,
    Test,
}

impl AlertEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Firing => "firing",
            Self::Resolved => "resolved",
            Self::Test => "test",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AlertTransition {
    pub alert: Alert,
    pub event_type: AlertEventType,
    pub channel_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannelType {
    GenericWebhook,
    Slack,
    Discord,
    Email,
}

impl NotificationChannelType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GenericWebhook => "generic_webhook",
            Self::Slack => "slack",
            Self::Discord => "discord",
            Self::Email => "email",
        }
    }
}

#[derive(Debug, Clone)]
pub struct NotificationChannel {
    pub owner_api_key_id: Option<i64>,
    pub id: String,
    pub name: String,
    pub channel_type: NotificationChannelType,
    pub webhook_url: Option<String>,
    pub email_to: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: i32,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_from: Option<String>,
    pub smtp_tls: bool,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NotificationChannelView {
    pub id: String,
    pub name: String,
    pub channel_type: NotificationChannelType,
    pub destination: String,
    pub enabled: bool,
    pub configured: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&NotificationChannel> for NotificationChannelView {
    fn from(channel: &NotificationChannel) -> Self {
        let destination = channel
            .email_to
            .clone()
            .or_else(|| {
                channel.webhook_url.as_ref().map(|url| {
                    reqwest::Url::parse(url)
                        .ok()
                        .and_then(|parsed| parsed.host_str().map(str::to_string))
                        .unwrap_or_else(|| "configured webhook".to_string())
                })
            })
            .unwrap_or_else(|| "not configured".to_string());
        let configured = match channel.channel_type {
            NotificationChannelType::Email => {
                channel.email_to.is_some()
                    && channel.smtp_host.is_some()
                    && channel.smtp_from.is_some()
            }
            _ => channel.webhook_url.is_some(),
        };
        Self {
            id: channel.id.clone(),
            name: channel.name.clone(),
            channel_type: channel.channel_type.clone(),
            destination,
            enabled: channel.enabled,
            configured,
            created_at: channel.created_at,
            updated_at: channel.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateNotificationChannelRequest {
    pub name: String,
    pub channel_type: NotificationChannelType,
    pub webhook_url: Option<String>,
    pub email_to: Option<String>,
    pub smtp_host: Option<String>,
    #[serde(default = "default_smtp_port")]
    pub smtp_port: i32,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_from: Option<String>,
    #[serde(default = "default_true")]
    pub smtp_tls: bool,
}

fn default_smtp_port() -> i32 {
    587
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSilence {
    #[serde(default)]
    pub owner_api_key_id: Option<i64>,
    pub id: String,
    pub name: String,
    pub reason: Option<String>,
    pub rule_id: Option<String>,
    pub agent_id: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAlertSilenceRequest {
    pub name: String,
    pub reason: Option<String>,
    pub rule_id: Option<String>,
    pub agent_id: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationDelivery {
    pub id: i64,
    pub alert_id: String,
    pub channel_id: Option<String>,
    pub channel_name: String,
    pub event_type: String,
    pub status: String,
    pub attempt_count: i32,
    pub response_status: Option<i32>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub next_attempt_at: DateTime<Utc>,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub max_attempts: i32,
}

#[derive(Debug, Clone)]
pub struct PendingNotification {
    pub delivery: NotificationDelivery,
    pub payload: serde_json::Value,
    pub channel: NotificationChannel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentEvent {
    pub event_id: String,
    pub alert_id: Option<String>,
    pub rule_id: Option<String>,
    pub agent_id: String,
    pub event_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub metadata: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AlertState {
    Pending,  // Condition met but waiting for duration
    Firing,   // Alert actively triggered
    Resolved, // Condition no longer met
}

impl Alert {
    pub fn new(rule: &AlertRule, agent_id: String, agent_name: String, current_value: f64) -> Self {
        // Every firing lifecycle is a distinct incident. The evaluator separately
        // correlates active alerts by rule and agent.
        let id = uuid::Uuid::new_v4().to_string();
        let message = format!(
            "{} {} on {} is {} {} (threshold: {} {})",
            rule.severity.emoji(),
            rule.name,
            agent_name,
            rule.metric,
            current_value,
            rule.condition.symbol(),
            rule.threshold
        );

        Alert {
            id,
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            agent_id,
            agent_name,
            state: AlertState::Pending,
            metric: rule.metric.clone(),
            current_value,
            threshold: rule.threshold,
            condition: rule.condition.clone(),
            severity: rule.severity.clone(),
            message,
            triggered_at: Utc::now(),
            resolved_at: None,
            acknowledged: false,
            acknowledged_at: None,
            acknowledged_by: None,
            last_notification_at: None,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, AlertState::Pending | AlertState::Firing)
    }

    pub fn should_notify(&self, cooldown_seconds: u64) -> bool {
        if self.state != AlertState::Firing {
            return false;
        }

        if let Some(last_notification) = self.last_notification_at {
            let elapsed = (Utc::now() - last_notification).num_seconds() as u64;
            elapsed >= cooldown_seconds
        } else {
            true
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvaluation {
    pub rule_id: String,
    pub agent_id: String,
    pub first_triggered_at: DateTime<Utc>,
    pub last_evaluated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAlertRuleRequest {
    pub name: String,
    pub description: Option<String>,
    pub metric: String,
    pub condition: AlertCondition,
    pub threshold: f64,
    pub threshold_percent: Option<f64>,
    pub duration_seconds: u64,
    pub severity: AlertSeverity,
    pub channels: Vec<String>,
    pub agent_filter: Option<String>,
    #[serde(default = "default_cooldown")]
    pub cooldown_seconds: u64,
}

fn default_cooldown() -> u64 {
    300 // 5 minutes default cooldown
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAlertRuleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub metric: Option<String>,
    pub condition: Option<AlertCondition>,
    pub threshold: Option<f64>,
    pub duration_seconds: Option<u64>,
    pub severity: Option<AlertSeverity>,
    pub channels: Option<Vec<String>>,
    pub enabled: Option<bool>,
    pub cooldown_seconds: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AcknowledgeAlertRequest {
    pub acknowledged_by: String,
}

#[derive(Debug, Serialize)]
pub struct AlertsResponse {
    pub active_alerts: Vec<Alert>,
    pub recent_alerts: Vec<Alert>,
    pub total_active: usize,
}

#[derive(Debug, Serialize)]
pub struct AlertRulesResponse {
    pub rules: Vec<AlertRule>,
    pub total: usize,
}
