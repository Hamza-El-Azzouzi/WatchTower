#![allow(dead_code)] // Some methods are for future use

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

use crate::storage::{Agent, DataPoint, ProcessSnapshot, TimeSeriesStore};

#[derive(Clone)]
enum AccessScope {
    Admin,
    Tenant(Vec<String>),
}

impl AccessScope {
    fn allows(&self, agent_id: &str) -> bool {
        match self {
            Self::Admin => true,
            Self::Tenant(agent_ids) => agent_ids.iter().any(|id| id == agent_id),
        }
    }

    fn allows_message(&self, message: &WsMessage) -> bool {
        match message {
            WsMessage::Metric { agent_id, .. }
            | WsMessage::MetricBatch { agent_id, .. }
            | WsMessage::ProcessSnapshot { agent_id, .. }
            | WsMessage::Log { agent_id, .. }
            | WsMessage::Alert { agent_id, .. }
            | WsMessage::HistoricalMetrics { agent_id, .. } => self.allows(agent_id),
            WsMessage::Heartbeat | WsMessage::InitialState { .. } => true,
        }
    }
}

async fn authenticate_socket(
    socket: &mut WebSocket,
    state: &crate::api::AppState,
) -> Option<AccessScope> {
    let received = tokio::time::timeout(std::time::Duration::from_secs(5), socket.recv())
        .await
        .ok()?;
    let message = received?.ok()?;
    let Message::Text(text) = message else {
        return None;
    };
    let payload: serde_json::Value = serde_json::from_str(&text).ok()?;
    if payload.get("type")?.as_str()? != "authenticate" {
        return None;
    }
    let token = payload.get("token")?.as_str()?;
    let auth = state.auth_service.as_ref()?;

    if auth
        .validate_admin_token(token)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return Some(AccessScope::Admin);
    }
    let key_id = auth.validate_api_key(token).await.ok().flatten()?;
    let agent_ids = state
        .database
        .as_ref()?
        .get_agents_by_api_key(key_id)
        .await
        .ok()?;
    Some(AccessScope::Tenant(agent_ids))
}

// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    // Real-time metric update
    Metric {
        agent_id: String,
        metric_name: String,
        value: f64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    MetricBatch {
        agent_id: String,
        metrics: HashMap<String, f64>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    ProcessSnapshot {
        agent_id: String,
        processes: Vec<ProcessSnapshot>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    // Real-time log entry
    Log {
        agent_id: String,
        level: String,
        message: String,
        source: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    // Real-time alert
    Alert {
        alert_id: String,
        agent_id: String,
        severity: String,
        message: String,
        state: String,
    },
    // Heartbeat to keep connection alive
    Heartbeat,
    // Initial state snapshot sent on connection
    InitialState {
        agents: Vec<AgentSnapshot>,
        metrics: Vec<MetricSnapshot>,
    },
    // Historical metrics batch for a specific agent
    HistoricalMetrics {
        agent_id: String,
        metric_name: String,
        data_points: Vec<DataPointSnapshot>,
    },
}

// Snapshot types for initial state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSnapshot {
    pub id: String,
    pub name: String,
    pub status: String,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSnapshot {
    pub agent_id: String,
    pub metric_name: String,
    pub latest_value: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPointSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub value: f64,
}

impl From<&Agent> for AgentSnapshot {
    fn from(agent: &Agent) -> Self {
        Self {
            id: agent.id.clone(),
            name: agent.name.clone(),
            status: format!("{:?}", agent.status),
            last_seen: agent.last_seen,
        }
    }
}

impl From<&DataPoint> for DataPointSnapshot {
    fn from(dp: &DataPoint) -> Self {
        Self {
            timestamp: dp.timestamp,
            value: dp.value,
        }
    }
}

#[derive(Clone)]
pub struct WebSocketManager {
    metrics_tx: broadcast::Sender<WsMessage>,
    logs_tx: broadcast::Sender<WsMessage>,
    alerts_tx: broadcast::Sender<WsMessage>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        let (metrics_tx, _) = broadcast::channel(1000);
        let (logs_tx, _) = broadcast::channel(1000);
        let (alerts_tx, _) = broadcast::channel(1000);

        Self {
            metrics_tx,
            logs_tx,
            alerts_tx,
        }
    }

    pub fn broadcast_metric(&self, msg: WsMessage) {
        // Having no active dashboard subscribers is normal, not an error.
        let _ = self.metrics_tx.send(msg);
    }

    pub fn broadcast_log(&self, msg: WsMessage) {
        let _ = self.logs_tx.send(msg);
    }

    pub fn broadcast_alert(&self, msg: WsMessage) {
        let _ = self.alerts_tx.send(msg);
    }
}

// WebSocket handler for metrics stream
pub async fn ws_metrics_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<crate::api::AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_metrics_socket(socket, state))
}

async fn handle_metrics_socket(mut socket: WebSocket, state: Arc<crate::api::AppState>) {
    let Some(scope) = authenticate_socket(&mut socket, &state).await else {
        warn!("Rejected unauthenticated metrics WebSocket");
        return;
    };
    let ws_manager = state.ws_manager.clone();
    let store = state.store.clone();
    let (mut sender, mut receiver) = socket.split();
    let mut rx = ws_manager.metrics_tx.subscribe();

    info!("New WebSocket connection for metrics - sending initial state");

    // Send initial state snapshot immediately on connect
    let initial_state = build_initial_state(&store, &scope);
    if let Ok(json) = serde_json::to_string(&initial_state) {
        if sender.send(Message::Text(json)).await.is_err() {
            error!("Failed to send initial state");
            return;
        }
        info!(
            "Sent initial state with {} agents, {} metrics",
            match &initial_state {
                WsMessage::InitialState { agents, metrics } =>
                    format!("{}, {}", agents.len(), metrics.len()),
                _ => "?".to_string(),
            },
            ""
        );
    }

    // Send heartbeat and forward messages
    let mut send_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if sender.send(Message::Text(
                        serde_json::to_string(&WsMessage::Heartbeat)
                            .expect("heartbeat serialization failed")
                    )).await.is_err() {
                        break;
                    }
                }
                msg = rx.recv() => {
                    match msg {
                        Ok(msg) => {
                            if !scope.allows_message(&msg) {
                                continue;
                            }
                            if let Ok(json) = serde_json::to_string(&msg) {
                                if sender.send(Message::Text(json)).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Broadcast receive error: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    });

    // Handle incoming messages (pings, close, etc.)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Close(_) = msg {
                break;
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    info!("WebSocket connection closed for metrics");
}

// Build initial state snapshot from store
fn build_initial_state(store: &TimeSeriesStore, scope: &AccessScope) -> WsMessage {
    let agents: Vec<_> = store
        .get_agents()
        .into_iter()
        .filter(|agent| scope.allows(&agent.id))
        .collect();
    let agent_snapshots: Vec<AgentSnapshot> = agents.iter().map(|a| a.into()).collect();
    let mut metric_snapshots: Vec<MetricSnapshot> = Vec::new();

    // Get latest value for each metric for each agent
    for agent in &agents {
        let metric_names = store.get_agent_metrics(&agent.id);
        for metric_name in metric_names {
            if let Some(dp) = store.get_latest(&agent.id, &metric_name) {
                metric_snapshots.push(MetricSnapshot {
                    agent_id: agent.id.clone(),
                    metric_name,
                    latest_value: dp.value,
                    timestamp: dp.timestamp,
                });
            }
        }
    }

    WsMessage::InitialState {
        agents: agent_snapshots,
        metrics: metric_snapshots,
    }
}

// WebSocket handler for logs stream
pub async fn ws_logs_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<crate::api::AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_logs_socket(socket, state))
}

async fn handle_logs_socket(mut socket: WebSocket, state: Arc<crate::api::AppState>) {
    let Some(scope) = authenticate_socket(&mut socket, &state).await else {
        warn!("Rejected unauthenticated logs WebSocket");
        return;
    };
    let ws_manager = state.ws_manager.clone();
    let (mut sender, mut receiver) = socket.split();
    let mut rx = ws_manager.logs_tx.subscribe();

    info!("New WebSocket connection for logs");

    let mut send_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if sender.send(Message::Text(
                        serde_json::to_string(&WsMessage::Heartbeat)
                            .expect("heartbeat serialization failed")
                    )).await.is_err() {
                        break;
                    }
                }
                msg = rx.recv() => {
                    match msg {
                        Ok(msg) => {
                            if !scope.allows_message(&msg) {
                                continue;
                            }
                            if let Ok(json) = serde_json::to_string(&msg) {
                                if sender.send(Message::Text(json)).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Broadcast receive error: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Close(_) = msg {
                break;
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    info!("WebSocket connection closed for logs");
}

// WebSocket handler for alerts stream
pub async fn ws_alerts_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<crate::api::AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_alerts_socket(socket, state))
}

async fn handle_alerts_socket(mut socket: WebSocket, state: Arc<crate::api::AppState>) {
    let Some(scope) = authenticate_socket(&mut socket, &state).await else {
        warn!("Rejected unauthenticated alerts WebSocket");
        return;
    };
    let ws_manager = state.ws_manager.clone();
    let (mut sender, mut receiver) = socket.split();
    let mut rx = ws_manager.alerts_tx.subscribe();

    info!("New WebSocket connection for alerts");

    let mut send_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if sender.send(Message::Text(
                        serde_json::to_string(&WsMessage::Heartbeat)
                            .expect("heartbeat serialization failed")
                    )).await.is_err() {
                        break;
                    }
                }
                msg = rx.recv() => {
                    match msg {
                        Ok(msg) => {
                            if !scope.allows_message(&msg) {
                                continue;
                            }
                            if let Ok(json) = serde_json::to_string(&msg) {
                                if sender.send(Message::Text(json)).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Broadcast receive error: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Close(_) = msg {
                break;
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }

    info!("WebSocket connection closed for alerts");
}
