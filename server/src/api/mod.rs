pub mod agent_control;
#[cfg(test)]
mod agent_control_tests;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, error, info};

use crate::alerts::manager::AlertManager;
use crate::auth::{
    AdminLoginRequest, AdminLoginResponse, AdminUser, AuthService, CreateApiKeyRequest,
};
use crate::db::Database;
use crate::storage::{Agent, DataPoint, MetricsPayload, StorageStats, TimeSeriesStore};
use crate::websocket::{WebSocketManager, WsMessage};

const MAX_AGENT_ID_LEN: usize = 128;
const MAX_METRICS_PER_PAYLOAD: usize = 512;
const MAX_METRIC_NAME_LEN: usize = 128;
const MAX_PROCESSES_PER_PAYLOAD: usize = 256;
const MAX_PROCESS_TEXT_LEN: usize = 96;
const MAX_MOUNTS_PER_PAYLOAD: usize = 128;
const MAX_INTERFACES_PER_PAYLOAD: usize = 128;
const MAX_SERVICES_PER_PAYLOAD: usize = 64;
const MAX_CONTAINERS_PER_PAYLOAD: usize = 64;
const MAX_TELEMETRY_TEXT_LEN: usize = 192;
const MAX_LOGS_PER_PAYLOAD: usize = 500;
const MAX_LOG_SOURCE_LEN: usize = 512;
const MAX_LOG_MESSAGE_LEN: usize = 16_384;

fn validate_metrics_payload(payload: &MetricsPayload) -> Result<(), ApiError> {
    if payload.delivery.as_ref().is_some_and(|delivery| {
        delivery.stream_id.len() != 32
            || !delivery
                .stream_id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || delivery.sequence == 0
            || delivery.sequence > i64::MAX as u64
    }) {
        return Err(ApiError::BadRequest("invalid delivery identity".into()));
    }
    let valid_identifier = |value: &str, max_len: usize| {
        !value.is_empty()
            && value.len() <= max_len
            && value.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':')
            })
    };

    if !valid_identifier(&payload.agent_id, MAX_AGENT_ID_LEN) {
        return Err(ApiError::BadRequest(
            "agent_id must be 1-128 characters using letters, numbers, '.', '_', ':' or '-'"
                .to_string(),
        ));
    }
    if payload.metrics.is_empty() || payload.metrics.len() > MAX_METRICS_PER_PAYLOAD {
        return Err(ApiError::BadRequest(format!(
            "metrics must contain between 1 and {MAX_METRICS_PER_PAYLOAD} entries"
        )));
    }
    if payload
        .metrics
        .iter()
        .any(|(name, value)| !valid_identifier(name, MAX_METRIC_NAME_LEN) || !value.is_finite())
    {
        return Err(ApiError::BadRequest(
            "metric names or values contain invalid data".to_string(),
        ));
    }
    if payload.processes.len() > MAX_PROCESSES_PER_PAYLOAD
        || payload.processes.iter().any(|process| {
            process.command.is_empty()
                || process.command.len() > MAX_PROCESS_TEXT_LEN
                || process.user.len() > MAX_PROCESS_TEXT_LEN
                || process.state.len() > 24
                || !process.cpu_percent.is_finite()
                || process.cpu_percent < 0.0
                || process.cpu_percent > 100_000.0
                || process.command.chars().any(char::is_control)
                || process.user.chars().any(char::is_control)
                || process.state.chars().any(char::is_control)
        })
    {
        return Err(ApiError::BadRequest(
            "process snapshot is too large or contains invalid data".to_string(),
        ));
    }

    let invalid_text = |value: &str| {
        value.is_empty()
            || value.len() > MAX_TELEMETRY_TEXT_LEN
            || value.chars().any(char::is_control)
    };
    let invalid_number = |value: f64| !value.is_finite() || !(0.0..=1.0e18).contains(&value);

    if payload.mounts.len() > MAX_MOUNTS_PER_PAYLOAD
        || payload.mounts.iter().any(|mount| {
            invalid_text(&mount.device)
                || invalid_text(&mount.mount_point)
                || invalid_text(&mount.filesystem)
                || invalid_number(mount.usage_percent)
                || invalid_number(mount.inode_usage_percent)
                || invalid_number(mount.read_bytes_per_sec)
                || invalid_number(mount.write_bytes_per_sec)
                || invalid_number(mount.read_iops)
                || invalid_number(mount.write_iops)
                || invalid_number(mount.average_latency_ms)
        })
    {
        return Err(ApiError::BadRequest(
            "mount telemetry is too large or contains invalid data".to_string(),
        ));
    }
    if payload.network_interfaces.len() > MAX_INTERFACES_PER_PAYLOAD
        || payload.network_interfaces.iter().any(|interface| {
            invalid_text(&interface.interface)
                || invalid_number(interface.rx_bytes_per_sec)
                || invalid_number(interface.tx_bytes_per_sec)
                || invalid_number(interface.rx_packets_per_sec)
                || invalid_number(interface.tx_packets_per_sec)
        })
    {
        return Err(ApiError::BadRequest(
            "network telemetry is too large or contains invalid data".to_string(),
        ));
    }
    if payload.services.len() > MAX_SERVICES_PER_PAYLOAD
        || payload.services.iter().any(|service| {
            invalid_text(&service.name)
                || invalid_text(&service.load_state)
                || invalid_text(&service.active_state)
                || invalid_text(&service.sub_state)
        })
    {
        return Err(ApiError::BadRequest(
            "service telemetry is too large or contains invalid data".to_string(),
        ));
    }
    if payload.containers.len() > MAX_CONTAINERS_PER_PAYLOAD
        || payload.containers.iter().any(|container| {
            invalid_text(&container.id)
                || invalid_text(&container.name)
                || invalid_text(&container.image)
                || invalid_text(&container.state)
                || invalid_text(&container.health)
                || invalid_number(container.cpu_percent)
                || invalid_number(container.memory_percent)
        })
    {
        return Err(ApiError::BadRequest(
            "container telemetry is too large or contains invalid data".to_string(),
        ));
    }

    Ok(())
}

fn validate_logs_payload(payload: &crate::storage::LogsPayload) -> Result<(), ApiError> {
    if payload.delivery.as_ref().is_some_and(|delivery| {
        delivery.stream_id.len() != 32
            || !delivery
                .stream_id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || delivery.sequence == 0
            || delivery.sequence > i64::MAX as u64
    }) {
        return Err(ApiError::BadRequest("invalid log delivery identity".into()));
    }
    if payload.agent_id.is_empty()
        || payload.agent_id.len() > MAX_AGENT_ID_LEN
        || payload.agent_id.chars().any(|character| {
            !(character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':'))
        })
    {
        return Err(ApiError::BadRequest("invalid log agent_id".to_string()));
    }
    if payload.logs.is_empty() || payload.logs.len() > MAX_LOGS_PER_PAYLOAD {
        return Err(ApiError::BadRequest(format!(
            "logs must contain between 1 and {MAX_LOGS_PER_PAYLOAD} entries"
        )));
    }
    if payload.logs.iter().any(|log| {
        log.source.len() > MAX_LOG_SOURCE_LEN
            || log.message.is_empty()
            || log.message.len() > MAX_LOG_MESSAGE_LEN
            || log.source.chars().any(|character| character == '\0')
            || log.message.chars().any(|character| character == '\0')
    }) {
        return Err(ApiError::BadRequest(
            "log source or message is invalid or too large".to_string(),
        ));
    }
    Ok(())
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub store: TimeSeriesStore,
    pub alert_manager: Arc<AlertManager>,
    pub database: Option<Arc<Database>>,
    pub auth_service: Option<Arc<AuthService>>,
    pub ws_manager: Arc<WebSocketManager>,
    metrics_persistence_interval: Duration,
    last_metrics_persistence: Arc<Mutex<HashMap<String, Instant>>>,
    last_incident_signal: Arc<Mutex<HashMap<String, Instant>>>,
}

impl AppState {
    pub fn new(
        store: TimeSeriesStore,
        alert_manager: Arc<AlertManager>,
        database: Option<Arc<Database>>,
        auth_service: Option<Arc<AuthService>>,
        ws_manager: Arc<WebSocketManager>,
        metrics_persistence_interval_seconds: u64,
    ) -> Self {
        Self {
            store,
            alert_manager,
            database,
            auth_service,
            ws_manager,
            metrics_persistence_interval: Duration::from_secs(
                metrics_persistence_interval_seconds.max(1),
            ),
            last_metrics_persistence: Arc::new(Mutex::new(HashMap::new())),
            last_incident_signal: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn claim_incident_signal(&self, key: &str, cooldown: Duration) -> bool {
        let now = Instant::now();
        let mut signals = self
            .last_incident_signal
            .lock()
            .expect("incident signal lock poisoned");
        if signals
            .get(key)
            .is_some_and(|previous| previous.elapsed() < cooldown)
        {
            return false;
        }
        signals.insert(key.to_string(), now);
        true
    }

    fn claim_metrics_persistence(&self, agent_id: &str) -> bool {
        let now = Instant::now();
        let mut last = self
            .last_metrics_persistence
            .lock()
            .expect("metrics persistence lock poisoned");
        if last
            .get(agent_id)
            .is_some_and(|previous| previous.elapsed() < self.metrics_persistence_interval)
        {
            return false;
        }
        last.insert(agent_id.to_string(), now);
        true
    }
}

/// Custom error type for API responses
#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(serde_json::json!({
            "error": message
        }));

        (status, body).into_response()
    }
}

/// POST /api/v1/metrics - Ingest metrics from agents
pub async fn ingest_metrics(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<MetricsPayload>,
) -> Result<impl IntoResponse, ApiError> {
    validate_metrics_payload(&payload)?;

    debug!(
        "Received metrics from agent '{}' with {} metrics",
        payload.agent_id,
        payload.metrics.len()
    );

    // Check agent limit if auth is enabled and get api_key_id for registration
    let mut api_key_id: Option<i64> = None;
    if let Some(auth_service) = &state.auth_service {
        // Extract API key from headers
        let api_key = headers
            .get("X-API-Key")
            .or_else(|| headers.get("Authorization"))
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer ").or(Some(s)));

        if let Some(key) = api_key {
            // Validate with agent limit check
            match auth_service
                .validate_api_key_with_agent(key, Some(&payload.agent_id))
                .await
            {
                Ok(Some(key_id)) => {
                    // Valid and within limits, store key_id for agent registration
                    api_key_id = Some(key_id);
                }
                Ok(None) => {
                    return Err(ApiError::BadRequest(
                        "API key is invalid, expired, revoked, or agent limit reached".to_string(),
                    ));
                }
                Err(e) => {
                    error!("API key validation error: {}", e);
                    return Err(ApiError::InternalError("Authentication error".to_string()));
                }
            }
        }
    }

    if payload.delivery.is_some() {
        if api_key_id.is_none() {
            return Err(ApiError::Unauthorized(
                "durable uploads require an agent API key".into(),
            ));
        }
        let db = state.database.as_ref().ok_or_else(|| {
            ApiError::InternalError("durable delivery requires PostgreSQL".into())
        })?;
        db.register_agent(&payload.agent_id, api_key_id, None, None, None)
            .await
            .map_err(|_| ApiError::InternalError("agent registration failed".into()))?;
        let accepted = db.persist_delivery(&payload).await.map_err(|error| {
            error!("Durable upload failed: {error}");
            ApiError::InternalError("durable upload failed; retry with same sequence".into())
        })?;
        if !accepted || (Utc::now() - payload.timestamp).num_seconds() > 15 {
            // Historical replay must not replace live charts or fire stale alerts.
            return Ok((
                StatusCode::ACCEPTED,
                Json(
                    serde_json::json!({"status":"accepted","durable":true,"delivery":payload.delivery}),
                ),
            ));
        }
    }

    // Store metrics in-memory for fast queries
    state.store.insert_metrics(payload.clone());

    // Publish one coherent live sample before durable I/O. Browsers receive
    // realtime data without waiting for PostgreSQL.
    state.ws_manager.broadcast_metric(WsMessage::MetricBatch {
        agent_id: payload.agent_id.clone(),
        metrics: payload.metrics.clone(),
        timestamp: payload.timestamp,
    });
    if !payload.processes.is_empty() {
        state
            .ws_manager
            .broadcast_metric(WsMessage::ProcessSnapshot {
                agent_id: payload.agent_id.clone(),
                processes: payload.processes.clone(),
                timestamp: payload.timestamp,
            });

        if let Some(process) = payload
            .processes
            .iter()
            .max_by(|left, right| left.cpu_percent.total_cmp(&right.cpu_percent))
        {
            let signal_key = format!("process:{}:{}", payload.agent_id, process.pid);
            if process.cpu_percent >= 80.0
                && state.claim_incident_signal(&signal_key, Duration::from_secs(60))
            {
                if let Some(database) = &state.database {
                    let database = database.clone();
                    let event = crate::alerts::IncidentEvent {
                        event_id: uuid::Uuid::new_v4().to_string(),
                        alert_id: None,
                        rule_id: None,
                        agent_id: payload.agent_id.clone(),
                        event_type: "process_spike".to_string(),
                        severity: "warning".to_string(),
                        title: format!("CPU spike: {}", process.command),
                        description: format!(
                            "PID {} used {:.1}% CPU during this sample",
                            process.pid, process.cpu_percent
                        ),
                        metadata: serde_json::json!({
                            "pid": process.pid,
                            "command": process.command,
                            "user": process.user,
                            "cpu_percent": process.cpu_percent,
                            "memory_bytes": process.memory_bytes,
                        }),
                        occurred_at: payload.timestamp,
                    };
                    tokio::spawn(async move {
                        if let Err(error) = database.insert_incident_event(&event).await {
                            tracing::warn!(%error, "Failed to persist process spike event");
                        }
                    });
                }
            }
        }
    }

    state.ws_manager.broadcast_metric(WsMessage::HostTelemetry {
        agent_id: payload.agent_id.clone(),
        mounts: payload.mounts.clone(),
        network_interfaces: payload.network_interfaces.clone(),
        services: payload.services.clone(),
        containers: payload.containers.clone(),
        timestamp: payload.timestamp,
    });

    // Persist at a lower cadence in the background. This keeps historical data
    // durable without turning fast live samples into excessive database rows.
    if payload.delivery.is_none() && state.claim_metrics_persistence(&payload.agent_id) {
        if let Some(db) = &state.database {
            let db = db.clone();
            let agent_id = payload.agent_id.clone();
            let timestamp = payload.timestamp;
            let metrics: Vec<(String, f64)> = payload
                .metrics
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();

            tokio::spawn(async move {
                if let Err(e) = db
                    .register_agent(&agent_id, api_key_id, None, None, None)
                    .await
                {
                    error!("Failed to register agent in database: {}", e);
                    return;
                }
                if let Err(e) = db.insert_metrics(&agent_id, &metrics, timestamp).await {
                    error!("Failed to persist metrics to database: {}", e);
                }
            });
        }
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "status": "accepted", "durable": payload.delivery.is_some(), "delivery":payload.delivery
        })),
    ))
}

#[cfg(test)]
mod payload_validation_tests {
    use super::*;
    use std::collections::HashMap;

    fn payload(agent_id: &str, metrics: HashMap<String, f64>) -> MetricsPayload {
        MetricsPayload {
            delivery: None,
            agent_id: agent_id.to_string(),
            timestamp: Utc::now(),
            metrics,
            processes: Vec::new(),
            mounts: Vec::new(),
            network_interfaces: Vec::new(),
            services: Vec::new(),
            containers: Vec::new(),
        }
    }

    #[test]
    fn accepts_process_metrics() {
        let metrics = HashMap::from([("process_postgres_running".to_string(), 1.0)]);
        assert!(validate_metrics_payload(&payload("agent-01", metrics)).is_ok());
    }

    #[test]
    fn rejects_log_injection_and_excessive_cardinality() {
        let metric = HashMap::from([("cpu_usage".to_string(), 1.0)]);
        assert!(validate_metrics_payload(&payload("agent\nforged", metric)).is_err());

        let too_many = (0..=MAX_METRICS_PER_PAYLOAD)
            .map(|index| (format!("metric_{index}"), index as f64))
            .collect();
        assert!(validate_metrics_payload(&payload("agent-01", too_many)).is_err());
    }

    #[test]
    fn rejects_non_finite_structured_telemetry() {
        let mut payload = payload("agent-01", HashMap::from([("cpu_usage".to_string(), 1.0)]));
        payload.mounts.push(crate::storage::MountSnapshot {
            device: "/dev/sda1".to_string(),
            mount_point: "/".to_string(),
            filesystem: "ext4".to_string(),
            used_bytes: 1,
            available_bytes: 1,
            total_bytes: 2,
            usage_percent: f64::NAN,
            inodes_used: 1,
            inodes_total: 2,
            inode_usage_percent: 50.0,
            read_bytes_per_sec: 0.0,
            write_bytes_per_sec: 0.0,
            read_iops: 0.0,
            write_iops: 0.0,
            average_latency_ms: 0.0,
        });
        assert!(validate_metrics_payload(&payload).is_err());
    }
}

/// Query parameters for GET /api/v1/metrics
#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    pub agent_id: String,
    pub metric: String,
    #[serde(default)]
    pub from: Option<DateTime<Utc>>,
    #[serde(default)]
    pub to: Option<DateTime<Utc>>,
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Response for metrics query
#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub agent_id: String,
    pub metric: String,
    pub data_points: Vec<DataPoint>,
    pub count: usize,
}

/// GET /api/v1/metrics - Query metrics
pub async fn query_metrics(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<MetricsQuery>,
) -> Result<Json<MetricsResponse>, ApiError> {
    ensure_agent_access(&state, &headers, &params.agent_id).await?;
    info!(
        "Querying metrics for agent '{}', metric '{}'",
        params.agent_id, params.metric
    );

    // First try in-memory store for recent data
    let mut data_points =
        state
            .store
            .query(&params.agent_id, &params.metric, params.from, params.to);

    // If in-memory store doesn't have enough data and database is available,
    // fallback to database for historical data
    let requested_limit = params.limit.unwrap_or(100);
    if data_points.len() < requested_limit {
        if let Some(db) = &state.database {
            match db
                .query_metrics(
                    &params.agent_id,
                    &params.metric,
                    params.from,
                    params.to,
                    Some(requested_limit),
                )
                .await
            {
                Ok(db_points) => {
                    // Use database results if we got more data
                    if db_points.len() > data_points.len() {
                        data_points = db_points;
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to query metrics from database: {}", e);
                    // Continue with in-memory data
                }
            }
        }
    }

    // Apply limit if specified
    if let Some(limit) = params.limit {
        if data_points.len() > limit {
            let start = data_points.len() - limit;
            data_points = data_points.drain(start..).collect();
        }
    }

    let count = data_points.len();

    Ok(Json(MetricsResponse {
        agent_id: params.agent_id,
        metric: params.metric,
        data_points,
        count,
    }))
}

/// Query parameters for GET /api/v1/metrics/latest
#[derive(Debug, Deserialize)]
pub struct LatestMetricsQuery {
    pub agent_id: String,
}

/// Response for latest metrics
#[derive(Debug, Serialize)]
pub struct LatestMetricsResponse {
    pub agent_id: String,
    pub metrics: Vec<LatestMetric>,
}

#[derive(Debug, Serialize)]
pub struct LatestMetric {
    pub name: String,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
}

/// GET /api/v1/metrics/latest - Get latest values for all metrics of an agent
pub async fn get_latest_metrics(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<LatestMetricsQuery>,
) -> Result<Json<LatestMetricsResponse>, ApiError> {
    ensure_agent_access(&state, &headers, &params.agent_id).await?;
    info!("Getting latest metrics for agent '{}'", params.agent_id);

    let metric_names = state.store.get_agent_metrics(&params.agent_id);
    let mut metrics = Vec::new();

    // Try in-memory store first
    for metric_name in &metric_names {
        if let Some(data_point) = state.store.get_latest(&params.agent_id, metric_name) {
            metrics.push(LatestMetric {
                name: metric_name.clone(),
                value: data_point.value,
                timestamp: data_point.timestamp,
            });
        }
    }

    // If in-memory is empty, fallback to database
    if metrics.is_empty() {
        if let Some(db) = &state.database {
            match db.get_latest_metrics(&params.agent_id).await {
                Ok(db_metrics) => {
                    for (name, value, timestamp) in db_metrics {
                        metrics.push(LatestMetric {
                            name,
                            value,
                            timestamp,
                        });
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to get latest metrics from database: {}", e);
                }
            }
        }
    }

    Ok(Json(LatestMetricsResponse {
        agent_id: params.agent_id,
        metrics,
    }))
}

/// GET /api/v1/agents - List all registered agents (filtered by API key)
pub async fn list_agents(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<Agent>>, ApiError> {
    info!("Listing registered agents");

    let all_agents = state.store.get_agents();

    let filtered_agents = match request_principal(&state, &headers).await? {
        RequestPrincipal::Admin(_) => all_agents,
        RequestPrincipal::ApiKey(key_id) => {
            let allowed = tenant_agent_ids(&state, key_id).await?;
            all_agents
                .into_iter()
                .filter(|agent| allowed.contains(&agent.id))
                .collect()
        }
    };

    Ok(Json(filtered_agents))
}

/// GET /api/v1/agents/:agent_id - Get specific agent details
pub async fn get_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
) -> Result<Json<Agent>, ApiError> {
    info!("Getting agent details for '{}'", agent_id);
    ensure_agent_access(&state, &headers, &agent_id).await?;

    state
        .store
        .get_agent(&agent_id)
        .map(Json)
        .ok_or_else(|| ApiError::BadRequest(format!("Agent '{}' not found", agent_id)))
}

/// GET /api/v1/stats - Get storage statistics (filtered by API key)
pub async fn get_stats(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<StorageStats>, ApiError> {
    info!("Getting storage statistics");

    let stats = match request_principal(&state, &headers).await? {
        RequestPrincipal::Admin(_) => state.store.get_stats(),
        RequestPrincipal::ApiKey(key_id) => {
            let allowed = tenant_agent_ids(&state, key_id).await?;
            state.store.get_stats_for_agents(&allowed)
        }
    };

    Ok(Json(stats))
}

/// GET /health - Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": Utc::now()
    }))
}

// ============ Logs API Endpoints ============

/// POST /api/v1/logs - Ingest logs from agents
pub async fn ingest_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<crate::storage::LogsPayload>,
) -> Result<impl IntoResponse, ApiError> {
    validate_logs_payload(&payload)?;
    info!(
        "Received {} logs from agent '{}'",
        payload.logs.len(),
        payload.agent_id
    );

    let api_key = extract_api_key(&headers)
        .ok_or_else(|| ApiError::Unauthorized("Missing API key".to_string()))?;
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Authentication is not available".to_string()))?;
    let api_key_id = auth_service
        .validate_api_key_with_agent(api_key, Some(&payload.agent_id))
        .await
        .map_err(|_| ApiError::InternalError("Authentication service error".to_string()))?
        .ok_or_else(|| {
            ApiError::Forbidden("Agent ID is unavailable to this API key".to_string())
        })?;

    if payload.delivery.is_some() {
        let db = state
            .database
            .as_ref()
            .ok_or_else(|| ApiError::InternalError("durable logs require PostgreSQL".into()))?;
        db.register_agent(&payload.agent_id, Some(api_key_id), None, None, None)
            .await
            .map_err(|_| ApiError::InternalError("agent registration failed".into()))?;
        let accepted = db.persist_log_delivery(&payload).await.map_err(|error| {
            error!("Durable log persistence failed: {error}");
            ApiError::InternalError("durable log persistence failed; retry same sequence".into())
        })?;
        if !accepted {
            return Ok((
                StatusCode::ACCEPTED,
                Json(
                    serde_json::json!({"status":"accepted","durable":true,"delivery":payload.delivery}),
                ),
            ));
        }
    }

    // Store logs in-memory for fast queries
    state.store.insert_logs(payload.clone());

    // Persist to database if available
    if let Some(db) = state
        .database
        .as_ref()
        .filter(|_| payload.delivery.is_none())
    {
        db.register_agent(&payload.agent_id, Some(api_key_id), None, None, None)
            .await
            .map_err(|e| {
                error!("Failed to register log agent: {}", e);
                ApiError::InternalError("Failed to register agent".to_string())
            })?;
        for log in &payload.logs {
            if let Err(e) = db
                .insert_log(
                    &payload.agent_id,
                    &log.level.to_string(),
                    &log.source,
                    &log.message,
                    log.timestamp,
                )
                .await
            {
                error!("Failed to persist log to database: {}", e);
                // Continue anyway - logs are still in memory
            }
        }
    }

    for log in &payload.logs {
        state.ws_manager.broadcast_log(WsMessage::Log {
            agent_id: payload.agent_id.clone(),
            level: log.level.to_string(),
            message: log.message.clone(),
            source: log.source.clone(),
            timestamp: log.timestamp,
        });

        if matches!(
            log.level,
            crate::storage::LogLevel::ERROR | crate::storage::LogLevel::FATAL
        ) {
            let signal_key = format!("log:{}:{}", payload.agent_id, log.source);
            if state.claim_incident_signal(&signal_key, Duration::from_secs(30)) {
                if let Some(database) = &state.database {
                    let database = database.clone();
                    let event = crate::alerts::IncidentEvent {
                        event_id: uuid::Uuid::new_v4().to_string(),
                        alert_id: None,
                        rule_id: None,
                        agent_id: payload.agent_id.clone(),
                        event_type: "log_error".to_string(),
                        severity: if log.level == crate::storage::LogLevel::FATAL {
                            "critical"
                        } else {
                            "warning"
                        }
                        .to_string(),
                        title: format!("{} log from {}", log.level, log.source),
                        description: log.message.chars().take(1000).collect(),
                        metadata: serde_json::json!({ "source": log.source, "level": log.level.to_string() }),
                        occurred_at: log.timestamp,
                    };
                    tokio::spawn(async move {
                        if let Err(error) = database.insert_incident_event(&event).await {
                            tracing::warn!(%error, "Failed to persist log incident event");
                        }
                    });
                }
            }
        }
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "status": "accepted", "durable": payload.delivery.is_some(), "delivery":payload.delivery
        })),
    ))
}

/// Query parameters for GET /api/v1/logs
#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub from: Option<DateTime<Utc>>,
    #[serde(default)]
    pub to: Option<DateTime<Utc>>,
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Response for logs query
#[derive(Debug, Serialize)]
pub struct LogsResponse {
    pub logs: Vec<crate::storage::LogEntry>,
    pub count: usize,
    pub total_count: usize,
}

/// GET /api/v1/logs - Query logs
pub async fn query_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<LogsQuery>,
) -> Result<Json<LogsResponse>, ApiError> {
    info!(
        "Querying logs - agent: {:?}, level: {:?}, keyword: {:?}",
        params.agent_id, params.level, params.keyword
    );

    // Parse log level if provided
    let level = params
        .level
        .as_ref()
        .and_then(|l| match l.to_uppercase().as_str() {
            "DEBUG" => Some(crate::storage::LogLevel::DEBUG),
            "INFO" => Some(crate::storage::LogLevel::INFO),
            "WARN" => Some(crate::storage::LogLevel::WARN),
            "ERROR" => Some(crate::storage::LogLevel::ERROR),
            "FATAL" => Some(crate::storage::LogLevel::FATAL),
            _ => None,
        });

    let allowed_agent_ids = match request_principal(&state, &headers).await? {
        RequestPrincipal::Admin(_) => None,
        RequestPrincipal::ApiKey(key_id) => Some(tenant_agent_ids(&state, key_id).await?),
    };
    if let Some(agent_id) = params.agent_id.as_deref() {
        if allowed_agent_ids
            .as_ref()
            .is_some_and(|allowed| !allowed.iter().any(|id| id == agent_id))
        {
            return Err(ApiError::Forbidden(
                "Agent is not owned by this tenant".to_string(),
            ));
        }
    }

    let limit = params.limit.unwrap_or(100).clamp(1, 1000);
    let memory_logs = state.store.query_logs(
        allowed_agent_ids.as_deref(),
        params.agent_id.as_deref(),
        level.clone(),
        params.from,
        params.to,
        params.keyword.as_deref(),
        Some(limit),
    );

    let memory_total = allowed_agent_ids.as_ref().map_or_else(
        || state.store.get_log_count(),
        |allowed| state.store.get_log_count_for_agents(allowed),
    );

    let (logs, total_count) = if let Some(database) = &state.database {
        let level_string = level.as_ref().map(ToString::to_string);
        match database
            .query_logs(
                allowed_agent_ids.as_deref(),
                params.agent_id.as_deref(),
                level_string.as_deref(),
                params.from,
                params.to,
                params.keyword.as_deref(),
                limit,
            )
            .await
        {
            Ok(logs) => {
                let total = database
                    .get_log_count(allowed_agent_ids.as_deref())
                    .await
                    .unwrap_or(memory_total);
                (logs, total)
            }
            Err(error) => {
                tracing::warn!("Failed to query durable logs: {}", error);
                (memory_logs, memory_total)
            }
        }
    } else {
        (memory_logs, memory_total)
    };
    let count = logs.len();

    Ok(Json(LogsResponse {
        logs,
        count,
        total_count,
    }))
}

// ============ Alert API Endpoints ============

/// POST /api/v1/alert-rules - Create a new alert rule
pub async fn create_alert_rule(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<crate::alerts::CreateAlertRuleRequest>,
) -> Result<Json<crate::alerts::AlertRule>, ApiError> {
    require_admin(&state, &headers).await?;
    info!("Creating new alert rule: {}", payload.name);
    let rule = state.alert_manager.create_rule(payload).await;
    Ok(Json(rule))
}

/// GET /api/v1/alert-rules - List all alert rules
pub async fn list_alert_rules(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<crate::alerts::AlertRulesResponse>, ApiError> {
    require_admin(&state, &headers).await?;
    let rules = state.alert_manager.list_rules().await;
    let total = rules.len();
    Ok(Json(crate::alerts::AlertRulesResponse { rules, total }))
}

#[derive(Debug, Deserialize)]
pub struct EffectiveRulesQuery {
    pub agent_id: String,
}

/// GET /api/v1/alert-rules/effective - Enabled rules applicable to one agent.
pub async fn list_effective_alert_rules(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<EffectiveRulesQuery>,
) -> Result<Json<crate::alerts::AlertRulesResponse>, ApiError> {
    ensure_agent_access(&state, &headers, &query.agent_id).await?;
    let rules: Vec<_> = state
        .alert_manager
        .list_rules()
        .await
        .into_iter()
        .filter(|rule| rule.enabled)
        .filter(|rule| {
            rule.agent_filter
                .as_deref()
                .is_none_or(|agent| agent == query.agent_id)
        })
        .collect();
    let total = rules.len();
    Ok(Json(crate::alerts::AlertRulesResponse { rules, total }))
}

/// GET /api/v1/alert-rules/:id - Get specific alert rule
pub async fn get_alert_rule(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(rule_id): Path<String>,
) -> Result<Json<crate::alerts::AlertRule>, ApiError> {
    require_admin(&state, &headers).await?;
    state
        .alert_manager
        .get_rule(&rule_id)
        .await
        .map(Json)
        .ok_or_else(|| ApiError::BadRequest(format!("Alert rule {} not found", rule_id)))
}

/// PUT /api/v1/alert-rules/:id - Update alert rule
pub async fn update_alert_rule(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(rule_id): Path<String>,
    Json(payload): Json<crate::alerts::UpdateAlertRuleRequest>,
) -> Result<Json<crate::alerts::AlertRule>, ApiError> {
    require_admin(&state, &headers).await?;
    info!("Updating alert rule: {}", rule_id);
    state
        .alert_manager
        .update_rule(&rule_id, payload)
        .await
        .map(Json)
        .ok_or_else(|| ApiError::BadRequest(format!("Alert rule {} not found", rule_id)))
}

/// DELETE /api/v1/alert-rules/:id - Delete alert rule
pub async fn delete_alert_rule(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(rule_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    require_admin(&state, &headers).await?;
    info!("Deleting alert rule: {}", rule_id);
    if state.alert_manager.delete_rule(&rule_id).await {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::BadRequest(format!(
            "Alert rule {} not found",
            rule_id
        )))
    }
}

/// POST /api/v1/alert-rules/:id/toggle - Toggle alert rule enabled/disabled
pub async fn toggle_alert_rule(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(rule_id): Path<String>,
) -> Result<Json<crate::alerts::AlertRule>, ApiError> {
    require_admin(&state, &headers).await?;
    info!("Toggling alert rule: {}", rule_id);
    state
        .alert_manager
        .toggle_rule(&rule_id)
        .await
        .map(Json)
        .ok_or_else(|| ApiError::BadRequest(format!("Alert rule {} not found", rule_id)))
}

/// GET /api/v1/alerts - Get all alerts (filtered by API key)
pub async fn list_alerts(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<crate::alerts::AlertsResponse>, ApiError> {
    let all_alerts = state.alert_manager.get_all_alerts().await;
    let filtered_alerts = match request_principal(&state, &headers).await? {
        RequestPrincipal::Admin(_) => all_alerts,
        RequestPrincipal::ApiKey(key_id) => {
            let allowed = tenant_agent_ids(&state, key_id).await?;
            all_alerts
                .into_iter()
                .filter(|alert| allowed.contains(&alert.agent_id))
                .collect()
        }
    };

    let active_alerts: Vec<_> = filtered_alerts
        .iter()
        .filter(|a| a.is_active())
        .cloned()
        .collect();
    let recent_alerts: Vec<_> = filtered_alerts
        .iter()
        .filter(|a| a.state == crate::alerts::AlertState::Resolved)
        .take(50)
        .cloned()
        .collect();

    Ok(Json(crate::alerts::AlertsResponse {
        total_active: active_alerts.len(),
        active_alerts,
        recent_alerts,
    }))
}

/// GET /api/v1/alerts/:id - Get specific alert
#[allow(dead_code)]
pub async fn get_alert(
    State(state): State<Arc<AppState>>,
    Path(alert_id): Path<String>,
) -> Result<Json<crate::alerts::Alert>, ApiError> {
    state
        .alert_manager
        .get_alert(&alert_id)
        .await
        .map(Json)
        .ok_or_else(|| ApiError::BadRequest(format!("Alert {} not found", alert_id)))
}

/// POST /api/v1/alerts/:id/acknowledge - Acknowledge an alert
pub async fn acknowledge_alert(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(alert_id): Path<String>,
    Json(_payload): Json<crate::alerts::AcknowledgeAlertRequest>,
) -> Result<Json<crate::alerts::Alert>, ApiError> {
    let alert = state
        .alert_manager
        .get_alert(&alert_id)
        .await
        .ok_or_else(|| ApiError::BadRequest(format!("Alert {} not found", alert_id)))?;
    let acknowledged_by = match request_principal(&state, &headers).await? {
        RequestPrincipal::Admin(username) => username,
        RequestPrincipal::ApiKey(key_id) => {
            if !database(&state)?
                .agent_belongs_to_api_key(&alert.agent_id, key_id)
                .await
                .map_err(|_| {
                    ApiError::InternalError("Failed to verify agent ownership".to_string())
                })?
            {
                return Err(ApiError::Forbidden(
                    "Agent is not owned by this tenant".to_string(),
                ));
            }
            format!("api-key-{key_id}")
        }
    };
    info!("Acknowledging alert: {} by {}", alert_id, acknowledged_by);
    state
        .alert_manager
        .acknowledge_alert(&alert_id, acknowledged_by)
        .await
        .map(Json)
        .ok_or_else(|| ApiError::BadRequest(format!("Alert {} not found", alert_id)))
}

// ============ Notification, silence, and incident API ============

fn database(state: &AppState) -> Result<Arc<Database>, ApiError> {
    state
        .database
        .clone()
        .ok_or_else(|| ApiError::InternalError("Database is unavailable".to_string()))
}

pub async fn create_notification_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<crate::alerts::CreateNotificationChannelRequest>,
) -> Result<Json<crate::alerts::NotificationChannelView>, ApiError> {
    require_admin(&state, &headers).await?;
    if payload.name.trim().is_empty() {
        return Err(ApiError::BadRequest("Channel name is required".to_string()));
    }
    if !matches!(
        payload.channel_type,
        crate::alerts::NotificationChannelType::Email
    ) {
        let url = payload
            .webhook_url
            .as_deref()
            .ok_or_else(|| ApiError::BadRequest("Webhook URL is required".to_string()))?;
        crate::alerts::notifications::validate_webhook_url(url)
            .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    } else if payload.email_to.as_deref().is_none_or(str::is_empty)
        || payload.smtp_host.as_deref().is_none_or(str::is_empty)
        || payload.smtp_from.as_deref().is_none_or(str::is_empty)
    {
        return Err(ApiError::BadRequest(
            "Email recipient, SMTP host, and sender are required".to_string(),
        ));
    }
    let channel = database(&state)?
        .create_notification_channel(&payload)
        .await
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    Ok(Json((&channel).into()))
}

pub async fn list_notification_channels(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<crate::alerts::NotificationChannelView>>, ApiError> {
    require_admin(&state, &headers).await?;
    let channels = database(&state)?
        .list_notification_channels()
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?;
    Ok(Json(channels.iter().map(Into::into).collect()))
}

#[derive(Debug, Deserialize)]
pub struct SetEnabledRequest {
    pub enabled: bool,
}

pub async fn set_notification_channel_enabled(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<SetEnabledRequest>,
) -> Result<StatusCode, ApiError> {
    require_admin(&state, &headers).await?;
    if database(&state)?
        .set_notification_channel_enabled(&id, payload.enabled)
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::BadRequest(
            "Notification channel not found".to_string(),
        ))
    }
}

pub async fn delete_notification_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    require_admin(&state, &headers).await?;
    if database(&state)?
        .delete_notification_channel(&id)
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::BadRequest(
            "Notification channel not found".to_string(),
        ))
    }
}

pub async fn test_notification_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &headers).await?;
    let db = database(&state)?;
    let channel = db
        .get_notification_channel(&id)
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?
        .ok_or_else(|| ApiError::BadRequest("Notification channel not found".to_string()))?;
    let dispatcher = crate::alerts::notifications::NotificationDispatcher::new(db)
        .map_err(|error| ApiError::InternalError(error.to_string()))?;
    let delivery_id = dispatcher
        .enqueue_test(&channel)
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "delivery_id": delivery_id })),
    ))
}

#[derive(Debug, Deserialize)]
pub struct DeliveryQuery {
    pub limit: Option<i64>,
}

pub async fn list_notification_deliveries(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<DeliveryQuery>,
) -> Result<Json<Vec<crate::alerts::NotificationDelivery>>, ApiError> {
    require_admin(&state, &headers).await?;
    let deliveries = database(&state)?
        .list_notification_deliveries(query.limit.unwrap_or(100).clamp(1, 500))
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?;
    Ok(Json(deliveries))
}

pub async fn create_alert_silence(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<crate::alerts::CreateAlertSilenceRequest>,
) -> Result<Json<crate::alerts::AlertSilence>, ApiError> {
    if payload.ends_at <= payload.starts_at {
        return Err(ApiError::BadRequest(
            "Silence end must be after its start".to_string(),
        ));
    }
    let admin = require_admin(&state, &headers).await?;
    let payload = crate::alerts::CreateAlertSilenceRequest {
        created_by: admin.username,
        ..payload
    };
    let silence = database(&state)?
        .create_alert_silence(&payload)
        .await
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    Ok(Json(silence))
}

#[derive(Debug, Deserialize)]
pub struct SilencesQuery {
    #[serde(default)]
    pub active_only: bool,
}

pub async fn list_alert_silences(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<SilencesQuery>,
) -> Result<Json<Vec<crate::alerts::AlertSilence>>, ApiError> {
    require_admin(&state, &headers).await?;
    let silences = database(&state)?
        .list_alert_silences(query.active_only)
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?;
    Ok(Json(silences))
}

pub async fn delete_alert_silence(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    require_admin(&state, &headers).await?;
    if database(&state)?
        .delete_alert_silence(&id)
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::BadRequest("Silence not found".to_string()))
    }
}

#[derive(Debug, Deserialize)]
pub struct IncidentQuery {
    pub agent_id: Option<String>,
    pub limit: Option<i64>,
}

pub async fn list_incident_timeline(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<IncidentQuery>,
) -> Result<Json<Vec<crate::alerts::IncidentEvent>>, ApiError> {
    match request_principal(&state, &headers).await? {
        RequestPrincipal::Admin(_) => {}
        RequestPrincipal::ApiKey(key_id) => {
            let agent_id = query
                .agent_id
                .as_deref()
                .ok_or_else(|| ApiError::BadRequest("agent_id is required".to_string()))?;
            let allowed = tenant_agent_ids(&state, key_id).await?;
            if !allowed.iter().any(|allowed_id| allowed_id == agent_id) {
                return Err(ApiError::Forbidden(
                    "Agent is not owned by this tenant".to_string(),
                ));
            }
        }
    }
    let events = database(&state)?
        .list_incident_events(
            query.agent_id.as_deref(),
            query.limit.unwrap_or(100).clamp(1, 500),
        )
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?;
    Ok(Json(events))
}

/// GET /api/v1/health - System health check endpoint
pub async fn system_health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut status = serde_json::json!({
        "status": "healthy",
        "storage": "ok",
    });

    // Check database health if enabled
    if let Some(db) = &state.database {
        match db.health_check().await {
            Ok(_) => {
                status["database"] = serde_json::json!("ok");
            }
            Err(e) => {
                error!("Database health check failed: {}", e);
                status["database"] = serde_json::json!("unhealthy");
                status["status"] = serde_json::json!("degraded");
            }
        }
    } else {
        status["database"] = serde_json::json!("disabled");
    }

    Json(status)
}

/// GET /api/v1/stats - Get database statistics
#[allow(dead_code)]
pub async fn get_database_stats(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    if let Some(db) = &state.database {
        match db.get_stats().await {
            Ok(stats) => Ok(Json(stats)),
            Err(e) => {
                error!("Failed to get database stats: {}", e);
                Err(ApiError::InternalError(format!(
                    "Failed to get database stats: {}",
                    e
                )))
            }
        }
    } else {
        Err(ApiError::BadRequest("Database is not enabled".to_string()))
    }
}

// ============================================================================
// API Key Management Endpoints
// ============================================================================

/// POST /api/v1/auth/keys - Create a new API key (admin only)
pub async fn create_api_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate admin token
    let admin = require_admin(&state, &headers).await?;

    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Authentication is not enabled".to_string()))?;

    match auth_service.create_api_key(request, &admin.username).await {
        Ok(response) => {
            info!("Created new API key: {}", response.name);
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to create API key: {}", e);
            Err(ApiError::InternalError(format!(
                "Failed to create API key: {}",
                e
            )))
        }
    }
}

/// GET /api/v1/auth/validate - Validate an API key (public endpoint for login)
pub async fn validate_api_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Authentication is not enabled".to_string()))?;

    // Extract API key from headers
    let api_key = headers
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::BadRequest("Missing API key".to_string()))?;

    // Validate the key
    match auth_service.validate_api_key(api_key).await {
        Ok(Some(_key_info)) => {
            info!("API key validated successfully");
            Ok(Json(serde_json::json!({
                "valid": true,
                "message": "API key is valid"
            })))
        }
        Ok(None) => {
            info!("Invalid API key attempted");
            Err(ApiError::BadRequest(
                "Invalid or expired API key".to_string(),
            ))
        }
        Err(e) => {
            error!("API key validation error: {}", e);
            Err(ApiError::InternalError(format!("Validation failed: {}", e)))
        }
    }
}

/// GET /api/v1/auth/keys - List all API keys (admin only)
pub async fn list_api_keys(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    // Validate admin token
    require_admin(&state, &headers).await?;

    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Authentication is not enabled".to_string()))?;

    match auth_service.list_api_keys().await {
        Ok(keys) => {
            // Don't expose the actual key values in the list
            let safe_keys: Vec<_> = keys
                .into_iter()
                .map(|mut key| {
                    key.key = format!("{}...", &key.key[..12]); // Show only first 12 chars
                    key
                })
                .collect();
            Ok(Json(safe_keys))
        }
        Err(e) => {
            error!("Failed to list API keys: {}", e);
            Err(ApiError::InternalError(format!(
                "Failed to list API keys: {}",
                e
            )))
        }
    }
}

/// DELETE /api/v1/auth/keys/:key_id - Revoke an API key (admin only)
pub async fn revoke_api_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(key_id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate admin token
    require_admin(&state, &headers).await?;

    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Authentication is not enabled".to_string()))?;

    match auth_service.revoke_api_key(key_id).await {
        Ok(true) => {
            info!("Revoked API key ID: {}", key_id);
            Ok(Json(serde_json::json!({
                "success": true,
                "message": "API key revoked successfully"
            })))
        }
        Ok(false) => Err(ApiError::BadRequest(format!(
            "API key {} not found or already revoked",
            key_id
        ))),
        Err(e) => {
            error!("Failed to revoke API key: {}", e);
            Err(ApiError::InternalError(format!(
                "Failed to revoke API key: {}",
                e
            )))
        }
    }
}

// ============================================================================
// Admin Authentication Endpoints
// ============================================================================

/// POST /api/v1/admin/login - Admin login
pub async fn admin_login(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AdminLoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Authentication is not enabled".to_string()))?;

    match auth_service
        .authenticate_admin(&request.username, &request.password)
        .await
    {
        Ok(Some(admin)) => {
            let token = AuthService::generate_admin_token(&admin).map_err(|e| {
                error!("Failed to issue admin token: {}", e);
                ApiError::InternalError("Admin token configuration is invalid".to_string())
            })?;
            info!("Admin user '{}' logged in successfully", admin.username);
            Ok(Json(AdminLoginResponse {
                username: admin.username,
                full_name: admin.full_name,
                token,
            }))
        }
        Ok(None) => Err(ApiError::BadRequest(
            "Invalid username or password".to_string(),
        )),
        Err(e) => {
            error!("Admin login error: {}", e);
            Err(ApiError::InternalError("Login failed".to_string()))
        }
    }
}

/// GET /api/v1/admin/validate - Validate admin session
pub async fn admin_validate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let admin = require_admin(&state, &headers).await?;
    Ok(Json(serde_json::json!({
        "valid": true,
        "username": admin.username
    })))
}

/// POST /api/v1/admin/change-password - Change admin password
pub async fn admin_change_password(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ChangePasswordRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate admin token
    let admin = require_admin(&state, &headers).await?;

    if admin.username != request.username {
        return Err(ApiError::Forbidden(
            "Admins may only change their own password".to_string(),
        ));
    }

    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Authentication is not enabled".to_string()))?;

    match auth_service
        .change_admin_password(
            &request.username,
            &request.old_password,
            &request.new_password,
        )
        .await
    {
        Ok(()) => {
            info!("Admin '{}' changed password successfully", request.username);
            Ok(Json(serde_json::json!({
                "success": true,
                "message": "Password changed successfully"
            })))
        }
        Err(e) => {
            error!("Password change error: {}", e);
            Err(ApiError::InternalError(
                "Password change failed".to_string(),
            ))
        }
    }
}

/// POST /api/v1/admin/users - Create new admin user
pub async fn admin_create_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateAdminRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate admin token
    require_admin(&state, &headers).await?;

    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Authentication is not enabled".to_string()))?;

    match auth_service
        .create_admin(
            &request.username,
            &request.password,
            request.email.clone(),
            request.full_name.clone(),
        )
        .await
    {
        Ok(user) => {
            info!("Created new admin user '{}'", request.username);
            Ok(Json(serde_json::json!({
                "success": true,
                "username": user.username,
                "username": request.username
            })))
        }
        Err(e) => {
            error!("Admin creation error: {}", e);
            Err(ApiError::BadRequest(format!(
                "Failed to create admin: {}",
                e
            )))
        }
    }
}

/// GET /api/v1/admin/users - List all admin users
pub async fn admin_list_users(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    // Validate admin token
    require_admin(&state, &headers).await?;

    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Authentication is not enabled".to_string()))?;

    match auth_service.list_admins().await {
        Ok(admins) => Ok(Json(admins)),
        Err(e) => {
            error!("Failed to list admins: {}", e);
            Err(ApiError::InternalError("Failed to list admins".to_string()))
        }
    }
}

/// POST /api/v1/admin/users/:username/deactivate - Deactivate admin user
pub async fn admin_deactivate_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(username): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate admin token
    require_admin(&state, &headers).await?;

    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Authentication is not enabled".to_string()))?;

    match auth_service.deactivate_admin(&username).await {
        Ok(_) => {
            info!("Deactivated admin user '{}'", username);
            Ok(Json(serde_json::json!({
                "success": true,
                "message": format!("User '{}' deactivated", username)
            })))
        }
        Err(e) => {
            error!("Failed to deactivate admin: {}", e);
            Err(ApiError::InternalError(
                "Failed to deactivate admin".to_string(),
            ))
        }
    }
}

/// POST /api/v1/admin/users/:username/activate - Activate admin user
pub async fn admin_activate_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(username): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate admin token
    require_admin(&state, &headers).await?;

    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Authentication is not enabled".to_string()))?;

    match auth_service.activate_admin(&username).await {
        Ok(_) => {
            info!("Activated admin user '{}'", username);
            Ok(Json(serde_json::json!({
                "success": true,
                "message": format!("User '{}' activated", username)
            })))
        }
        Err(e) => {
            error!("Failed to activate admin: {}", e);
            Err(ApiError::InternalError(
                "Failed to activate admin".to_string(),
            ))
        }
    }
}

fn extract_admin_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("X-Admin-Token")
        .or_else(|| headers.get("Authorization"))
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").or(Some(s)))
}

enum RequestPrincipal {
    Admin(String),
    ApiKey(i64),
}

fn extract_api_key(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
        .or_else(|| {
            headers
                .get("Authorization")
                .and_then(|h| h.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
        })
}

async fn request_principal(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<RequestPrincipal, ApiError> {
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Authentication is not available".to_string()))?;

    if let Some(token) = extract_admin_token(headers) {
        if let Some(admin) = auth_service
            .validate_admin_token(token)
            .await
            .map_err(|_| ApiError::InternalError("Authentication service error".to_string()))?
        {
            return Ok(RequestPrincipal::Admin(admin.username));
        }
    }

    let key = extract_api_key(headers)
        .ok_or_else(|| ApiError::Unauthorized("Missing API key".to_string()))?;
    let key_id = auth_service
        .validate_api_key(key)
        .await
        .map_err(|_| ApiError::InternalError("Authentication service error".to_string()))?
        .ok_or_else(|| ApiError::Unauthorized("Invalid or expired API key".to_string()))?;
    Ok(RequestPrincipal::ApiKey(key_id))
}

async fn tenant_agent_ids(state: &AppState, key_id: i64) -> Result<Vec<String>, ApiError> {
    state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database is not available".to_string()))?
        .get_agents_by_api_key(key_id)
        .await
        .map_err(|e| {
            error!("Failed to resolve tenant agents: {}", e);
            ApiError::InternalError("Failed to resolve tenant ownership".to_string())
        })
}

async fn ensure_agent_access(
    state: &AppState,
    headers: &HeaderMap,
    agent_id: &str,
) -> Result<(), ApiError> {
    match request_principal(state, headers).await? {
        RequestPrincipal::Admin(_) => Ok(()),
        RequestPrincipal::ApiKey(key_id) => {
            let database = state
                .database
                .as_ref()
                .ok_or_else(|| ApiError::InternalError("Database is not available".to_string()))?;
            if database
                .agent_belongs_to_api_key(agent_id, key_id)
                .await
                .map_err(|_| {
                    ApiError::InternalError("Failed to verify agent ownership".to_string())
                })?
            {
                Ok(())
            } else {
                Err(ApiError::Forbidden(
                    "Agent is not owned by this tenant".to_string(),
                ))
            }
        }
    }
}

async fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<AdminUser, ApiError> {
    let token = extract_admin_token(headers)
        .ok_or_else(|| ApiError::Unauthorized("Missing admin token".to_string()))?;
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Authentication is not enabled".to_string()))?;

    auth_service
        .validate_admin_token(token)
        .await
        .map_err(|e| {
            error!("Admin token validation failed: {}", e);
            ApiError::InternalError("Authentication service error".to_string())
        })?
        .ok_or_else(|| ApiError::Unauthorized("Invalid or expired admin token".to_string()))
}

/// Request to change admin password
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub username: String,
    pub old_password: String,
    pub new_password: String,
}

/// Request to create new admin
#[derive(Debug, Deserialize)]
pub struct CreateAdminRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
    pub full_name: Option<String>,
}

// ============ Agent Request API ============

/// Request for agent quota
#[derive(Debug, Deserialize)]
pub struct AgentRequestPayload {
    pub company_name: String,
    pub contact_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub agents_requested: i32,
    pub use_case: String,
    pub message: Option<String>,
}

/// Response for agent request submission
#[derive(Debug, Serialize)]
pub struct AgentRequestResponse {
    pub id: i64,
    pub message: String,
}

/// POST /api/v1/requests/agents - Submit an agent quota request (public endpoint)
pub async fn submit_agent_request(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AgentRequestPayload>,
) -> Result<impl IntoResponse, ApiError> {
    info!(
        "Received agent request from '{}' ({}) for {} agents",
        payload.company_name, payload.email, payload.agents_requested
    );

    let database = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    match database.create_agent_request(&payload).await {
        Ok(id) => {
            info!(
                "Created agent request #{} for {} ({})",
                id, payload.company_name, payload.email
            );
            Ok(Json(AgentRequestResponse {
                id,
                message: "Request submitted successfully. We will contact you shortly.".to_string(),
            }))
        }
        Err(e) => {
            error!("Failed to create agent request: {}", e);
            Err(ApiError::InternalError(
                "Failed to submit request. Please try again.".to_string(),
            ))
        }
    }
}

/// GET /api/v1/requests/agents - List all agent requests (admin only)
pub async fn list_agent_requests(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    // Verify admin token
    let _admin = require_admin(&state, &headers).await?;

    let database = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    let status_filter = params.get("status").map(|s| s.as_str());

    match database.list_agent_requests(status_filter).await {
        Ok(requests) => Ok(Json(serde_json::json!({
            "requests": requests,
            "total": requests.len()
        }))),
        Err(e) => {
            error!("Failed to list agent requests: {}", e);
            Err(ApiError::InternalError(
                "Failed to fetch requests".to_string(),
            ))
        }
    }
}

/// PUT /api/v1/requests/agents/:id/status - Update request status (admin only)
pub async fn update_agent_request_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(request_id): Path<i64>,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl IntoResponse, ApiError> {
    // Verify admin token
    let admin = require_admin(&state, &headers).await?;

    let database = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    let status = payload
        .get("status")
        .and_then(|s| s.as_str())
        .ok_or_else(|| ApiError::BadRequest("Status is required".to_string()))?;

    let notes = payload.get("notes").and_then(|n| n.as_str());
    let reviewed_by = Some(admin.username.as_str());

    match database
        .update_agent_request_status(request_id, status, notes, reviewed_by)
        .await
    {
        Ok(updated) => {
            if updated {
                info!(
                    "Updated agent request #{} to status '{}'",
                    request_id, status
                );
                Ok(Json(serde_json::json!({
                    "success": true,
                    "message": format!("Request #{} updated to {}", request_id, status)
                })))
            } else {
                Err(ApiError::BadRequest(format!(
                    "Request #{} not found",
                    request_id
                )))
            }
        }
        Err(e) => {
            error!("Failed to update agent request: {}", e);
            Err(ApiError::InternalError(
                "Failed to update request".to_string(),
            ))
        }
    }
}
