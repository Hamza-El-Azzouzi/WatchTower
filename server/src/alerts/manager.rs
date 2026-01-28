use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{
    Alert, AlertEvaluation, AlertRule, AlertState, CreateAlertRuleRequest, UpdateAlertRuleRequest,
};
use crate::storage::TimeSeriesStore;

pub struct AlertManager {
    rules: Arc<RwLock<HashMap<String, AlertRule>>>,
    alerts: Arc<RwLock<HashMap<String, Alert>>>,
    evaluations: Arc<RwLock<HashMap<String, AlertEvaluation>>>, // Key: "rule_id:agent_id"
    metrics_storage: TimeSeriesStore,
}

impl AlertManager {
    pub fn new(metrics_storage: TimeSeriesStore) -> Self {
        Self {
            rules: Arc::new(RwLock::new(HashMap::new())),
            alerts: Arc::new(RwLock::new(HashMap::new())),
            evaluations: Arc::new(RwLock::new(HashMap::new())),
            metrics_storage,
        }
    }

    // Alert Rule Management
    pub async fn create_rule(&self, req: CreateAlertRuleRequest) -> AlertRule {
        let rule = AlertRule {
            id: Uuid::new_v4().to_string(),
            name: req.name,
            description: req.description,
            metric: req.metric,
            condition: req.condition,
            threshold: req.threshold,
            threshold_percent: req.threshold_percent,
            duration_seconds: req.duration_seconds,
            severity: req.severity,
            channels: req.channels,
            enabled: true,
            agent_filter: req.agent_filter,
            created_at: Utc::now(),
            cooldown_seconds: req.cooldown_seconds,
        };

        let mut rules = self.rules.write().await;
        rules.insert(rule.id.clone(), rule.clone());
        rule
    }

    pub async fn get_rule(&self, rule_id: &str) -> Option<AlertRule> {
        let rules = self.rules.read().await;
        rules.get(rule_id).cloned()
    }

    pub async fn list_rules(&self) -> Vec<AlertRule> {
        let rules = self.rules.read().await;
        rules.values().cloned().collect()
    }

    pub async fn update_rule(
        &self,
        rule_id: &str,
        req: UpdateAlertRuleRequest,
    ) -> Option<AlertRule> {
        let mut rules = self.rules.write().await;

        if let Some(rule) = rules.get_mut(rule_id) {
            if let Some(name) = req.name {
                rule.name = name;
            }
            if let Some(description) = req.description {
                rule.description = Some(description);
            }
            if let Some(metric) = req.metric {
                rule.metric = metric;
            }
            if let Some(condition) = req.condition {
                rule.condition = condition;
            }
            if let Some(threshold) = req.threshold {
                rule.threshold = threshold;
            }
            if let Some(duration) = req.duration_seconds {
                rule.duration_seconds = duration;
            }
            if let Some(severity) = req.severity {
                rule.severity = severity;
            }
            if let Some(channels) = req.channels {
                rule.channels = channels;
            }
            if let Some(enabled) = req.enabled {
                rule.enabled = enabled;
            }
            if let Some(cooldown) = req.cooldown_seconds {
                rule.cooldown_seconds = cooldown;
            }

            Some(rule.clone())
        } else {
            None
        }
    }

    pub async fn delete_rule(&self, rule_id: &str) -> bool {
        let mut rules = self.rules.write().await;
        rules.remove(rule_id).is_some()
    }

    pub async fn toggle_rule(&self, rule_id: &str) -> Option<AlertRule> {
        let mut rules = self.rules.write().await;
        if let Some(rule) = rules.get_mut(rule_id) {
            rule.enabled = !rule.enabled;
            Some(rule.clone())
        } else {
            None
        }
    }

    // Alert Evaluation
    pub async fn evaluate_alerts(&self) -> Vec<Alert> {
        let rules = self.rules.read().await;
        let mut alerts = self.alerts.write().await;
        let mut evaluations = self.evaluations.write().await;
        let mut triggered_alerts = Vec::new();

        for rule in rules.values() {
            if !rule.enabled {
                continue;
            }

            // Get all agents
            let agents = self.metrics_storage.get_agents();

            for agent in agents {
                // Apply agent filter if specified
                if let Some(filter) = &rule.agent_filter {
                    if &agent.id != filter {
                        continue;
                    }
                }

                // Get latest metrics for this agent
                let metric_value = self
                    .metrics_storage
                    .get_latest(&agent.id, &rule.metric)
                    .map(|dp| dp.value);

                if let Some(value) = metric_value {
                    let eval_key = format!("{}:{}", rule.id, agent.id);
                    let condition_met = self.evaluate_condition(rule, value, &agent.id);

                    if condition_met {
                        // Check if we have an existing evaluation
                        let now = Utc::now();

                        if let Some(eval) = evaluations.get_mut(&eval_key) {
                            eval.last_evaluated_at = now;

                            // Check if duration threshold is met
                            let duration_elapsed =
                                (now - eval.first_triggered_at).num_seconds() as u64;

                            if duration_elapsed >= rule.duration_seconds {
                                // Find or create alert
                                let alert_id = format!("{}:{}", rule.id, agent.id);

                                if let Some(alert) = alerts.get_mut(&alert_id) {
                                    // Update existing alert
                                    alert.current_value = value;

                                    if alert.state == AlertState::Pending {
                                        alert.state = AlertState::Firing;
                                        triggered_alerts.push(alert.clone());
                                    } else if alert.state == AlertState::Firing {
                                        // Check if we should re-notify based on cooldown
                                        if alert.should_notify(rule.cooldown_seconds) {
                                            triggered_alerts.push(alert.clone());
                                        }
                                    }
                                } else {
                                    // Create new alert in firing state
                                    let mut alert = Alert::new(
                                        rule,
                                        agent.id.clone(),
                                        agent.name.clone(),
                                        value,
                                    );
                                    alert.state = AlertState::Firing;
                                    alerts.insert(alert_id, alert.clone());
                                    triggered_alerts.push(alert);
                                }
                            }
                        } else {
                            // First time condition is met - create evaluation record
                            evaluations.insert(
                                eval_key.clone(),
                                AlertEvaluation {
                                    rule_id: rule.id.clone(),
                                    agent_id: agent.id.clone(),
                                    first_triggered_at: now,
                                    last_evaluated_at: now,
                                },
                            );

                            // Create pending alert
                            let alert_id = format!("{}:{}", rule.id, agent.id);
                            let alert =
                                Alert::new(rule, agent.id.clone(), agent.name.clone(), value);
                            alerts.insert(alert_id, alert);
                        }
                    } else {
                        // Condition no longer met - resolve alert
                        let eval_key = format!("{}:{}", rule.id, agent.id);
                        evaluations.remove(&eval_key);

                        let alert_id = format!("{}:{}", rule.id, agent.id);
                        if let Some(alert) = alerts.get_mut(&alert_id) {
                            if alert.is_active() {
                                alert.state = AlertState::Resolved;
                                alert.resolved_at = Some(Utc::now());
                            }
                        }
                    }
                }
            }
        }

        triggered_alerts
    }

    fn evaluate_condition(&self, rule: &AlertRule, value: f64, agent_id: &str) -> bool {
        // If threshold_percent is specified, calculate actual threshold
        let threshold = if let Some(percent) = rule.threshold_percent {
            // Find the max value metric (e.g., db_connections_max)
            let max_metric_name = format!("{}_max", rule.metric.trim_end_matches("_active"));

            if let Some(max_value) = self.metrics_storage.get_latest(agent_id, &max_metric_name) {
                (max_value.value * percent) / 100.0
            } else {
                rule.threshold
            }
        } else {
            rule.threshold
        };

        rule.condition.evaluate(value, threshold)
    }

    // Alert Management
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts.values().filter(|a| a.is_active()).cloned().collect()
    }

    pub async fn get_all_alerts(&self) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        let mut all: Vec<Alert> = alerts.values().cloned().collect();
        all.sort_by(|a, b| b.triggered_at.cmp(&a.triggered_at));
        all
    }

    pub async fn get_alert(&self, alert_id: &str) -> Option<Alert> {
        let alerts = self.alerts.read().await;
        alerts.get(alert_id).cloned()
    }

    pub async fn acknowledge_alert(
        &self,
        alert_id: &str,
        acknowledged_by: String,
    ) -> Option<Alert> {
        let mut alerts = self.alerts.write().await;

        if let Some(alert) = alerts.get_mut(alert_id) {
            alert.acknowledged = true;
            alert.acknowledged_at = Some(Utc::now());
            alert.acknowledged_by = Some(acknowledged_by);
            Some(alert.clone())
        } else {
            None
        }
    }

    pub async fn mark_notified(&self, alert_id: &str) {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.get_mut(alert_id) {
            alert.last_notification_at = Some(Utc::now());
        }
    }

    pub async fn cleanup_old_alerts(&self, max_age_hours: i64) {
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours);
        let mut alerts = self.alerts.write().await;

        alerts.retain(|_, alert| {
            if alert.state == AlertState::Resolved {
                if let Some(resolved_at) = alert.resolved_at {
                    resolved_at > cutoff
                } else {
                    true
                }
            } else {
                true
            }
        });
    }
}
