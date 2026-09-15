use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid::Uuid;

use super::{
    Alert, AlertCondition, AlertEvaluation, AlertEventType, AlertRule, AlertSeverity, AlertState,
    AlertTransition, CreateAlertRuleRequest, IncidentEvent, UpdateAlertRuleRequest,
};
use crate::db::Database;
use crate::storage::TimeSeriesStore;

pub struct AlertManager {
    // In-memory cache for fast evaluation - synced with database
    rules_cache: Arc<RwLock<HashMap<String, AlertRule>>>,
    alerts_cache: Arc<RwLock<HashMap<String, Alert>>>,
    evaluations: Arc<RwLock<HashMap<String, AlertEvaluation>>>, // Key: "rule_id:agent_id"
    metrics_storage: TimeSeriesStore,
    database: Arc<Database>,
}

impl AlertManager {
    pub async fn new(metrics_storage: TimeSeriesStore, database: Arc<Database>) -> Self {
        let manager = Self {
            rules_cache: Arc::new(RwLock::new(HashMap::new())),
            alerts_cache: Arc::new(RwLock::new(HashMap::new())),
            evaluations: Arc::new(RwLock::new(HashMap::new())),
            metrics_storage,
            database,
        };

        // Load existing rules and alerts from database
        if let Err(e) = manager.load_from_database().await {
            error!("Failed to load alerts from database: {}", e);
        }

        manager
    }

    /// Load rules and alerts from database into memory cache
    async fn load_from_database(&self) -> anyhow::Result<()> {
        // Load alert rules
        let rules = self.database.list_alert_rules().await?;
        let mut rules_cache = self.rules_cache.write().await;
        for rule in rules {
            info!(
                "Loaded alert rule from database: {} ({})",
                rule.name, rule.id
            );
            rules_cache.insert(rule.id.clone(), rule);
        }
        drop(rules_cache);

        // Load alerts
        let alerts = self.database.list_alerts(Some(1000)).await?;
        let mut alerts_cache = self.alerts_cache.write().await;
        for alert in alerts {
            alerts_cache.insert(alert.id.clone(), alert);
        }
        info!(
            "Loaded {} alert rules and {} alerts from database",
            self.rules_cache.read().await.len(),
            alerts_cache.len()
        );

        Ok(())
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

        // Save to database first
        if let Err(e) = self.database.create_alert_rule(&rule).await {
            error!("Failed to save alert rule to database: {}", e);
        } else {
            info!("Saved alert rule to database: {} ({})", rule.name, rule.id);
        }

        // Update in-memory cache
        let mut rules = self.rules_cache.write().await;
        rules.insert(rule.id.clone(), rule.clone());
        rule
    }

    pub async fn get_rule(&self, rule_id: &str) -> Option<AlertRule> {
        // First check cache
        let rules = self.rules_cache.read().await;
        if let Some(rule) = rules.get(rule_id) {
            return Some(rule.clone());
        }
        drop(rules);

        // If not in cache, try database
        match self.database.get_alert_rule(rule_id).await {
            Ok(Some(rule)) => {
                // Update cache
                let mut rules = self.rules_cache.write().await;
                rules.insert(rule.id.clone(), rule.clone());
                Some(rule)
            }
            Ok(None) => None,
            Err(e) => {
                error!("Failed to get alert rule from database: {}", e);
                None
            }
        }
    }

    pub async fn list_rules(&self) -> Vec<AlertRule> {
        let rules = self.rules_cache.read().await;
        rules.values().cloned().collect()
    }

    pub async fn update_rule(
        &self,
        rule_id: &str,
        req: UpdateAlertRuleRequest,
    ) -> Option<AlertRule> {
        let mut rules = self.rules_cache.write().await;

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

            // Save to database
            let updated_rule = rule.clone();
            let db = self.database.clone();
            tokio::spawn(async move {
                if let Err(e) = db.update_alert_rule(&updated_rule).await {
                    error!("Failed to update alert rule in database: {}", e);
                }
            });

            Some(rule.clone())
        } else {
            None
        }
    }

    pub async fn delete_rule(&self, rule_id: &str) -> bool {
        // Delete from database first
        if let Err(e) = self.database.delete_alert_rule(rule_id).await {
            error!("Failed to delete alert rule from database: {}", e);
        }

        let mut rules = self.rules_cache.write().await;
        rules.remove(rule_id).is_some()
    }

    pub async fn toggle_rule(&self, rule_id: &str) -> Option<AlertRule> {
        let mut rules = self.rules_cache.write().await;
        if let Some(rule) = rules.get_mut(rule_id) {
            rule.enabled = !rule.enabled;

            // Save to database
            let updated_rule = rule.clone();
            let db = self.database.clone();
            tokio::spawn(async move {
                if let Err(e) = db.update_alert_rule(&updated_rule).await {
                    error!("Failed to toggle alert rule in database: {}", e);
                }
            });

            Some(rule.clone())
        } else {
            None
        }
    }

    // Alert Evaluation
    pub async fn evaluate_alerts(&self, offline_after_seconds: u64) -> Vec<AlertTransition> {
        let rules: Vec<AlertRule> = self.rules_cache.read().await.values().cloned().collect();
        let mut alerts = self.alerts_cache.write().await;
        let mut evaluations = self.evaluations.write().await;
        let mut transitions = Vec::new();
        let agents = self.metrics_storage.get_agents();

        for rule in &rules {
            if !rule.enabled {
                continue;
            }

            for agent in &agents {
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
                                if let Some(alert) = alerts.values_mut().find(|alert| {
                                    alert.rule_id == rule.id
                                        && alert.agent_id == agent.id
                                        && alert.is_active()
                                }) {
                                    alert.current_value = value;
                                    if alert.state == AlertState::Pending {
                                        alert.state = AlertState::Firing;
                                        alert.triggered_at = now;
                                        self.persist_alert(alert.clone());
                                        self.record_incident(alert, "firing");
                                        transitions.push(AlertTransition {
                                            alert: alert.clone(),
                                            event_type: AlertEventType::Firing,
                                            channel_ids: rule.channels.clone(),
                                        });
                                    } else if alert.should_notify(rule.cooldown_seconds) {
                                        transitions.push(AlertTransition {
                                            alert: alert.clone(),
                                            event_type: AlertEventType::Firing,
                                            channel_ids: rule.channels.clone(),
                                        });
                                    }
                                }
                            }
                        } else {
                            let existing_id = alerts
                                .values()
                                .find(|alert| {
                                    alert.rule_id == rule.id
                                        && alert.agent_id == agent.id
                                        && alert.is_active()
                                })
                                .map(|alert| alert.id.clone());

                            if let Some(existing_id) = existing_id {
                                // Rehydrate evaluator state after a server restart instead of
                                // opening a duplicate incident for the same active condition.
                                if let Some(alert) = alerts.get_mut(&existing_id) {
                                    alert.current_value = value;
                                    evaluations.insert(
                                        eval_key.clone(),
                                        AlertEvaluation {
                                            rule_id: rule.id.clone(),
                                            agent_id: agent.id.clone(),
                                            first_triggered_at: alert.triggered_at,
                                            last_evaluated_at: now,
                                        },
                                    );
                                    if alert.should_notify(rule.cooldown_seconds) {
                                        transitions.push(AlertTransition {
                                            alert: alert.clone(),
                                            event_type: AlertEventType::Firing,
                                            channel_ids: rule.channels.clone(),
                                        });
                                    }
                                }
                            } else {
                                evaluations.insert(
                                    eval_key.clone(),
                                    AlertEvaluation {
                                        rule_id: rule.id.clone(),
                                        agent_id: agent.id.clone(),
                                        first_triggered_at: now,
                                        last_evaluated_at: now,
                                    },
                                );

                                let mut alert =
                                    Alert::new(rule, agent.id.clone(), agent.name.clone(), value);
                                let fires_immediately = rule.duration_seconds == 0;
                                if fires_immediately {
                                    alert.state = AlertState::Firing;
                                }
                                self.persist_alert(alert.clone());
                                self.record_incident(
                                    &alert,
                                    if fires_immediately {
                                        "firing"
                                    } else {
                                        "pending"
                                    },
                                );
                                if fires_immediately {
                                    transitions.push(AlertTransition {
                                        alert: alert.clone(),
                                        event_type: AlertEventType::Firing,
                                        channel_ids: rule.channels.clone(),
                                    });
                                }
                                alerts.insert(alert.id.clone(), alert);
                            }
                        }
                    } else {
                        let eval_key = format!("{}:{}", rule.id, agent.id);
                        evaluations.remove(&eval_key);
                        if let Some(alert) = alerts.values_mut().find(|alert| {
                            alert.rule_id == rule.id
                                && alert.agent_id == agent.id
                                && alert.is_active()
                        }) {
                            let was_firing = alert.state == AlertState::Firing;
                            alert.state = AlertState::Resolved;
                            alert.resolved_at = Some(Utc::now());
                            alert.current_value = value;
                            alert.message = format!(
                                "✅ {} on {} recovered: {} is {} (threshold {} {})",
                                alert.rule_name,
                                alert.agent_name,
                                alert.metric,
                                value,
                                alert.condition.symbol(),
                                alert.threshold
                            );
                            self.persist_alert(alert.clone());
                            self.record_incident(alert, "recovery");
                            if was_firing {
                                transitions.push(AlertTransition {
                                    alert: alert.clone(),
                                    event_type: AlertEventType::Resolved,
                                    channel_ids: rule.channels.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        self.evaluate_offline_agents(
            &agents,
            offline_after_seconds,
            &mut alerts,
            &mut transitions,
        );

        transitions
    }

    fn evaluate_offline_agents(
        &self,
        agents: &[crate::storage::Agent],
        offline_after_seconds: u64,
        alerts: &mut HashMap<String, Alert>,
        transitions: &mut Vec<AlertTransition>,
    ) {
        const RULE_ID: &str = "system-agent-offline";
        for agent in agents {
            let age = Utc::now()
                .signed_duration_since(agent.last_seen)
                .num_seconds()
                .max(0) as u64;
            let active_id = alerts
                .values()
                .find(|alert| {
                    alert.rule_id == RULE_ID && alert.agent_id == agent.id && alert.is_active()
                })
                .map(|alert| alert.id.clone());

            if age >= offline_after_seconds {
                if active_id.is_none() {
                    let alert = Alert {
                        id: Uuid::new_v4().to_string(),
                        rule_id: RULE_ID.to_string(),
                        rule_name: "Agent offline".to_string(),
                        agent_id: agent.id.clone(),
                        agent_name: agent.name.clone(),
                        state: AlertState::Firing,
                        metric: "agent_heartbeat_age_seconds".to_string(),
                        current_value: age as f64,
                        threshold: offline_after_seconds as f64,
                        condition: AlertCondition::GreaterThan,
                        severity: AlertSeverity::Critical,
                        message: format!(
                            "Agent {} has not reported for {} seconds",
                            agent.name, age
                        ),
                        triggered_at: Utc::now(),
                        resolved_at: None,
                        acknowledged: false,
                        acknowledged_at: None,
                        acknowledged_by: None,
                        last_notification_at: None,
                    };
                    self.persist_alert(alert.clone());
                    self.record_incident(&alert, "firing");
                    transitions.push(AlertTransition {
                        alert: alert.clone(),
                        event_type: AlertEventType::Firing,
                        channel_ids: Vec::new(), // Built-in health alerts use all enabled channels.
                    });
                    alerts.insert(alert.id.clone(), alert);
                } else if let Some(alert) = active_id.and_then(|id| alerts.get_mut(&id)) {
                    alert.current_value = age as f64;
                    if alert.should_notify(300) {
                        transitions.push(AlertTransition {
                            alert: alert.clone(),
                            event_type: AlertEventType::Firing,
                            channel_ids: Vec::new(),
                        });
                    }
                }
            } else if let Some(alert) = active_id.and_then(|id| alerts.get_mut(&id)) {
                alert.state = AlertState::Resolved;
                alert.current_value = age as f64;
                alert.resolved_at = Some(Utc::now());
                alert.message = format!("Agent {} resumed reporting", agent.name);
                self.persist_alert(alert.clone());
                self.record_incident(alert, "recovery");
                transitions.push(AlertTransition {
                    alert: alert.clone(),
                    event_type: AlertEventType::Resolved,
                    channel_ids: Vec::new(),
                });
            }
        }
    }

    /// Persist alert to database in background
    fn persist_alert(&self, alert: Alert) {
        let db = self.database.clone();
        tokio::spawn(async move {
            if let Err(e) = db.upsert_alert(&alert).await {
                error!("Failed to persist alert to database: {}", e);
            }
        });
    }

    fn record_incident(&self, alert: &Alert, event_type: &str) {
        let db = self.database.clone();
        let event = IncidentEvent {
            event_id: Uuid::new_v4().to_string(),
            alert_id: Some(alert.id.clone()),
            rule_id: Some(alert.rule_id.clone()),
            agent_id: alert.agent_id.clone(),
            event_type: event_type.to_string(),
            severity: format!("{:?}", alert.severity).to_lowercase(),
            title: match event_type {
                "recovery" => format!("Recovered: {}", alert.rule_name),
                "pending" => format!("Pending: {}", alert.rule_name),
                _ => format!("Firing: {}", alert.rule_name),
            },
            description: alert.message.clone(),
            metadata: serde_json::json!({
                "metric": alert.metric,
                "value": alert.current_value,
                "threshold": alert.threshold,
            }),
            occurred_at: Utc::now(),
        };
        tokio::spawn(async move {
            if let Err(error) = db.insert_incident_event(&event).await {
                error!(%error, "Failed to persist incident event");
            }
        });
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
    #[allow(dead_code)]
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let alerts = self.alerts_cache.read().await;
        alerts.values().filter(|a| a.is_active()).cloned().collect()
    }

    pub async fn get_all_alerts(&self) -> Vec<Alert> {
        let alerts = self.alerts_cache.read().await;
        let mut all: Vec<Alert> = alerts.values().cloned().collect();
        all.sort_by(|a, b| b.triggered_at.cmp(&a.triggered_at));
        all
    }

    #[allow(dead_code)]
    pub async fn get_alert(&self, alert_id: &str) -> Option<Alert> {
        // First check cache
        let alerts = self.alerts_cache.read().await;
        if let Some(alert) = alerts.get(alert_id) {
            return Some(alert.clone());
        }
        drop(alerts);

        // Try database
        match self.database.get_alert(alert_id).await {
            Ok(Some(alert)) => {
                let mut alerts = self.alerts_cache.write().await;
                alerts.insert(alert.id.clone(), alert.clone());
                Some(alert)
            }
            Ok(None) => None,
            Err(e) => {
                error!("Failed to get alert from database: {}", e);
                None
            }
        }
    }

    pub async fn acknowledge_alert(
        &self,
        alert_id: &str,
        acknowledged_by: String,
    ) -> Option<Alert> {
        let mut alerts = self.alerts_cache.write().await;

        if let Some(alert) = alerts.get_mut(alert_id) {
            alert.acknowledged = true;
            alert.acknowledged_at = Some(Utc::now());
            alert.acknowledged_by = Some(acknowledged_by.clone());

            // Save to database
            let db = self.database.clone();
            let aid = alert_id.to_string();
            let acknowledged_by_db = acknowledged_by.clone();
            tokio::spawn(async move {
                if let Err(e) = db.acknowledge_alert_db(&aid, &acknowledged_by_db).await {
                    error!("Failed to acknowledge alert in database: {}", e);
                }
            });

            let event_db = self.database.clone();
            let event = IncidentEvent {
                event_id: Uuid::new_v4().to_string(),
                alert_id: Some(alert.id.clone()),
                rule_id: Some(alert.rule_id.clone()),
                agent_id: alert.agent_id.clone(),
                event_type: "acknowledged".to_string(),
                severity: format!("{:?}", alert.severity).to_lowercase(),
                title: format!("Acknowledged: {}", alert.rule_name),
                description: format!("Alert acknowledged by {}", acknowledged_by),
                metadata: serde_json::json!({ "acknowledged_by": acknowledged_by }),
                occurred_at: Utc::now(),
            };
            tokio::spawn(async move {
                if let Err(error) = event_db.insert_incident_event(&event).await {
                    error!(%error, "Failed to persist acknowledgement event");
                }
            });

            Some(alert.clone())
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub async fn mark_notified(&self, alert_id: &str) {
        let mut alerts = self.alerts_cache.write().await;
        if let Some(alert) = alerts.get_mut(alert_id) {
            alert.last_notification_at = Some(Utc::now());
            // Persist the update
            self.persist_alert(alert.clone());
        }
    }

    pub async fn cleanup_old_alerts(&self, max_age_hours: i64) {
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours);
        let mut alerts = self.alerts_cache.write().await;

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

        // Database cleanup is handled separately by Database::cleanup_old_alerts
    }
}
