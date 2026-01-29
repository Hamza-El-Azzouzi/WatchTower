#![allow(dead_code)] // Some methods are for future use

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Log level for log entries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum LogLevel {
    DEBUG,
    INFO,
    WARN,
    ERROR,
    FATAL,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::DEBUG => write!(f, "DEBUG"),
            LogLevel::INFO => write!(f, "INFO"),
            LogLevel::WARN => write!(f, "WARN"),
            LogLevel::ERROR => write!(f, "ERROR"),
            LogLevel::FATAL => write!(f, "FATAL"),
        }
    }
}

/// A single log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: u64,
    pub agent_id: String,
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub source: String, // File path
    pub message: String,
}

/// Logs payload received from agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsPayload {
    pub agent_id: String,
    pub logs: Vec<LogEntryInput>,
}

/// Log entry input from agents (without id)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntryInput {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub source: String,
    pub message: String,
}

/// A single data point in the time series
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

/// Metrics payload received from agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsPayload {
    pub agent_id: String,
    pub timestamp: DateTime<Utc>,
    pub metrics: HashMap<String, f64>,
}

/// Information about a registered agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub last_seen: DateTime<Utc>,
    pub status: AgentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    Healthy,     // Last seen < 1 minute ago
    Degraded,    // Last seen 1-5 minutes ago
    Unreachable, // Last seen > 5 minutes ago
}

impl Agent {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            last_seen: Utc::now(),
            status: AgentStatus::Healthy,
        }
    }

    pub fn update_last_seen(&mut self) {
        self.last_seen = Utc::now();
        self.update_status();
    }

    pub fn update_status(&mut self) {
        let elapsed = Utc::now().signed_duration_since(self.last_seen);
        self.status = if elapsed.num_seconds() < 60 {
            AgentStatus::Healthy
        } else if elapsed.num_seconds() < 300 {
            AgentStatus::Degraded
        } else {
            AgentStatus::Unreachable
        };
    }
}

// Type alias to simplify complex nested HashMap
type MetricData = HashMap<String, Vec<DataPoint>>;
type AgentMetrics = HashMap<String, MetricData>;

/// Thread-safe in-memory time-series storage
#[derive(Clone)]
pub struct TimeSeriesStore {
    // agent_id -> metric_name -> Vec<DataPoint>
    data: Arc<RwLock<AgentMetrics>>,
    // agent_id -> Agent
    agents: Arc<RwLock<HashMap<String, Agent>>>,
    max_points_per_metric: usize,
    // Logs storage: Vec<LogEntry> sorted by timestamp desc
    logs: Arc<RwLock<Vec<LogEntry>>>,
    // Next log ID
    next_log_id: Arc<RwLock<u64>>,
    // Maximum logs to store
    max_logs: usize,
}

impl TimeSeriesStore {
    pub fn new(max_points_per_metric: usize) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            agents: Arc::new(RwLock::new(HashMap::new())),
            max_points_per_metric,
            logs: Arc::new(RwLock::new(Vec::new())),
            next_log_id: Arc::new(RwLock::new(1)),
            max_logs: 10000, // Keep last 10k logs
        }
    }

    /// Insert a data point for a specific agent and metric
    pub fn insert(
        &self,
        agent_id: String,
        metric_name: String,
        timestamp: DateTime<Utc>,
        value: f64,
    ) {
        let mut data = self.data.write().expect("metrics data lock poisoned");

        let agent_data = data.entry(agent_id).or_default();
        let points = agent_data.entry(metric_name).or_default();

        points.push(DataPoint { timestamp, value });

        // Keep only recent points to prevent memory overflow
        if points.len() > self.max_points_per_metric {
            points.drain(0..points.len() - self.max_points_per_metric);
        }

        // Sort by timestamp
        points.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    }

    /// Insert multiple metrics from a payload
    pub fn insert_metrics(&self, payload: MetricsPayload) {
        // Register or update agent
        self.register_agent(&payload.agent_id, &payload.agent_id);

        // Insert all metrics
        for (metric_name, value) in payload.metrics {
            self.insert(
                payload.agent_id.clone(),
                metric_name,
                payload.timestamp,
                value,
            );
        }
    }

    /// Query data points for a specific agent and metric within a time range
    pub fn query(
        &self,
        agent_id: &str,
        metric_name: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Vec<DataPoint> {
        let data = self.data.read().expect("metrics data lock poisoned");

        data.get(agent_id)
            .and_then(|agent_data| agent_data.get(metric_name))
            .map(|points| {
                points
                    .iter()
                    .filter(|p| {
                        let after_from = from.is_none_or(|f| p.timestamp >= f);
                        let before_to = to.is_none_or(|t| p.timestamp <= t);
                        after_from && before_to
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the latest value for a specific metric
    pub fn get_latest(&self, agent_id: &str, metric_name: &str) -> Option<DataPoint> {
        let data = self.data.read().expect("metrics data lock poisoned");

        data.get(agent_id)
            .and_then(|agent_data| agent_data.get(metric_name))
            .and_then(|points| points.last().cloned())
    }

    /// Get all available metrics for an agent
    pub fn get_agent_metrics(&self, agent_id: &str) -> Vec<String> {
        let data = self.data.read().expect("metrics data lock poisoned");

        data.get(agent_id)
            .map(|agent_data| agent_data.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Register or update an agent
    pub fn register_agent(&self, agent_id: &str, agent_name: &str) {
        let mut agents = self.agents.write().expect("agents lock poisoned");

        if let Some(agent) = agents.get_mut(agent_id) {
            agent.update_last_seen();
        } else {
            agents.insert(
                agent_id.to_string(),
                Agent::new(agent_id.to_string(), agent_name.to_string()),
            );
        }
    }

    /// Get all registered agents
    pub fn get_agents(&self) -> Vec<Agent> {
        let mut agents = self.agents.write().expect("agents lock poisoned");

        // Update status for all agents
        for agent in agents.values_mut() {
            agent.update_status();
        }

        agents.values().cloned().collect()
    }

    /// Get a specific agent
    pub fn get_agent(&self, agent_id: &str) -> Option<Agent> {
        let mut agents = self.agents.write().expect("agents lock poisoned");

        agents.get_mut(agent_id).map(|agent| {
            agent.update_status();
            agent.clone()
        })
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> StorageStats {
        let data = self.data.read().expect("metrics data lock poisoned");
        let agents = self.agents.read().expect("agents lock poisoned");

        let total_agents = agents.len();
        let mut total_metrics = 0;
        let mut total_data_points = 0;

        for agent_data in data.values() {
            total_metrics += agent_data.len();
            for points in agent_data.values() {
                total_data_points += points.len();
            }
        }

        StorageStats {
            total_agents,
            total_metrics,
            total_data_points,
        }
    }

    /// Insert logs from agents
    pub fn insert_logs(&self, payload: LogsPayload) {
        // Register or update agent
        self.register_agent(&payload.agent_id, &payload.agent_id);

        let mut logs = self.logs.write().expect("logs lock poisoned");
        let mut next_id = self.next_log_id.write().expect("log id lock poisoned");

        // Convert input logs to log entries with IDs
        for log_input in payload.logs {
            let log_entry = LogEntry {
                id: *next_id,
                agent_id: payload.agent_id.clone(),
                timestamp: log_input.timestamp,
                level: log_input.level,
                source: log_input.source,
                message: log_input.message,
            };

            logs.push(log_entry);
            *next_id += 1;
        }

        // Keep only the most recent logs
        if logs.len() > self.max_logs {
            let excess = logs.len() - self.max_logs;
            logs.drain(0..excess);
        }

        // Sort by timestamp descending (most recent first)
        logs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    }

    /// Query logs with filters
    pub fn query_logs(
        &self,
        agent_id: Option<&str>,
        level: Option<LogLevel>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        keyword: Option<&str>,
        limit: Option<usize>,
    ) -> Vec<LogEntry> {
        let logs = self.logs.read().expect("logs lock poisoned");

        let filtered: Vec<LogEntry> = logs
            .iter()
            .filter(|log| {
                // Filter by agent_id
                if let Some(agent) = agent_id {
                    if log.agent_id != agent {
                        return false;
                    }
                }

                // Filter by level
                if let Some(ref lvl) = level {
                    if &log.level != lvl {
                        return false;
                    }
                }

                // Filter by time range
                if let Some(f) = from {
                    if log.timestamp < f {
                        return false;
                    }
                }

                if let Some(t) = to {
                    if log.timestamp > t {
                        return false;
                    }
                }

                // Filter by keyword in message
                if let Some(kw) = keyword {
                    if !log.message.to_lowercase().contains(&kw.to_lowercase()) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Apply limit
        if let Some(lim) = limit {
            filtered.into_iter().take(lim).collect()
        } else {
            filtered
        }
    }

    /// Get total log count
    pub fn get_log_count(&self) -> usize {
        let logs = self.logs.read().expect("logs lock poisoned");
        logs.len()
    }

    /// Clean up old logs (older than specified duration in seconds)
    pub fn cleanup_old_logs(&self, max_age_seconds: i64) {
        let mut logs = self.logs.write().expect("logs lock poisoned");
        let cutoff = Utc::now() - chrono::Duration::seconds(max_age_seconds);

        logs.retain(|log| log.timestamp > cutoff);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_agents: usize,
    pub total_metrics: usize,
    pub total_data_points: usize,
}

impl Default for TimeSeriesStore {
    fn default() -> Self {
        Self::new(10_000) // Default: keep 10k points per metric
    }
}
