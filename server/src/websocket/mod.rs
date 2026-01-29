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
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    Metric {
        agent_id: String,
        metric_name: String,
        value: f64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    Log {
        agent_id: String,
        level: String,
        message: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    Alert {
        alert_id: String,
        agent_id: String,
        severity: String,
        message: String,
        state: String,
    },
    Heartbeat,
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
        if let Err(e) = self.metrics_tx.send(msg) {
            warn!("Failed to broadcast metric: {}", e);
        }
    }

    pub fn broadcast_log(&self, msg: WsMessage) {
        if let Err(e) = self.logs_tx.send(msg) {
            warn!("Failed to broadcast log: {}", e);
        }
    }

    pub fn broadcast_alert(&self, msg: WsMessage) {
        if let Err(e) = self.alerts_tx.send(msg) {
            warn!("Failed to broadcast alert: {}", e);
        }
    }
}

// WebSocket handler for metrics stream
pub async fn ws_metrics_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<crate::api::AppState>>,
) -> Response {
    let ws_manager = state.ws_manager.clone();
    ws.on_upgrade(|socket| handle_metrics_socket(socket, ws_manager))
}

async fn handle_metrics_socket(socket: WebSocket, ws_manager: Arc<WebSocketManager>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = ws_manager.metrics_tx.subscribe();

    info!("New WebSocket connection for metrics");

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

// WebSocket handler for logs stream
pub async fn ws_logs_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<crate::api::AppState>>,
) -> Response {
    let ws_manager = state.ws_manager.clone();
    ws.on_upgrade(|socket| handle_logs_socket(socket, ws_manager))
}

async fn handle_logs_socket(socket: WebSocket, ws_manager: Arc<WebSocketManager>) {
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
    let ws_manager = state.ws_manager.clone();
    ws.on_upgrade(|socket| handle_alerts_socket(socket, ws_manager))
}

async fn handle_alerts_socket(socket: WebSocket, ws_manager: Arc<WebSocketManager>) {
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
