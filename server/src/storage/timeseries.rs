use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

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
}

impl TimeSeriesStore {
    pub fn new(max_points_per_metric: usize) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            agents: Arc::new(RwLock::new(HashMap::new())),
            max_points_per_metric,
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
        let mut data = self.data.write().unwrap();

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
        let data = self.data.read().unwrap();

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
        let data = self.data.read().unwrap();

        data.get(agent_id)
            .and_then(|agent_data| agent_data.get(metric_name))
            .and_then(|points| points.last().cloned())
    }

    /// Get all available metrics for an agent
    pub fn get_agent_metrics(&self, agent_id: &str) -> Vec<String> {
        let data = self.data.read().unwrap();

        data.get(agent_id)
            .map(|agent_data| agent_data.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Register or update an agent
    pub fn register_agent(&self, agent_id: &str, agent_name: &str) {
        let mut agents = self.agents.write().unwrap();

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
        let mut agents = self.agents.write().unwrap();

        // Update status for all agents
        for agent in agents.values_mut() {
            agent.update_status();
        }

        agents.values().cloned().collect()
    }

    /// Get a specific agent
    pub fn get_agent(&self, agent_id: &str) -> Option<Agent> {
        let mut agents = self.agents.write().unwrap();

        agents.get_mut(agent_id).map(|agent| {
            agent.update_status();
            agent.clone()
        })
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> StorageStats {
        let data = self.data.read().unwrap();
        let agents = self.agents.read().unwrap();

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
