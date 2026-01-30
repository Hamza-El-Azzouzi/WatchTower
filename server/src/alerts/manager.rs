use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid::Uuid;

use super::{
    Alert, AlertEvaluation, AlertRule, AlertState, CreateAlertRuleRequest, UpdateAlertRuleRequest,
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
    pub async fn evaluate_alerts(&self) -> Vec<Alert> {
        let rules = self.rules_cache.read().await;
        let mut alerts = self.alerts_cache.write().await;
        let mut evaluations = self.evaluations.write().await;
        let mut triggered_alerts: Vec<Alert> = Vec::new();

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
                                        // Save to database
                                        self.persist_alert(alert.clone());
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
                                    // Save to database
                                    self.persist_alert(alert.clone());
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
                            // Save to database
                            self.persist_alert(alert.clone());
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
                                // Save to database
                                self.persist_alert(alert.clone());
                            }
                        }
                    }
                }
            }
        }

        triggered_alerts
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
            tokio::spawn(async move {
                if let Err(e) = db.acknowledge_alert_db(&aid, &acknowledged_by).await {
                    error!("Failed to acknowledge alert in database: {}", e);
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
