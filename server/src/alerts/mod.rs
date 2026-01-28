use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
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
#[serde(rename_all = "lowercase")]
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
pub enum AlertState {
    Pending,  // Condition met but waiting for duration
    Firing,   // Alert actively triggered
    Resolved, // Condition no longer met
}

impl Alert {
    pub fn new(rule: &AlertRule, agent_id: String, agent_name: String, current_value: f64) -> Self {
        // Use composite key as ID so it matches HashMap storage key
        let id = format!("{}:{}", rule.id, agent_id);
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
