use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::alerts::manager::AlertManager;
use crate::auth::{AdminLoginRequest, AdminLoginResponse, AdminUser, AuthService, CreateApiKeyRequest};
use crate::db::Database;
use crate::storage::{Agent, DataPoint, MetricsPayload, StorageStats, TimeSeriesStore};
use crate::websocket::{WebSocketManager, WsMessage};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub store: TimeSeriesStore,
    pub alert_manager: Arc<AlertManager>,
    pub database: Option<Arc<Database>>,
    pub auth_service: Option<Arc<AuthService>>,
    pub ws_manager: Arc<WebSocketManager>,
}

impl AppState {
    pub fn new(
        store: TimeSeriesStore,
        alert_manager: Arc<AlertManager>,
        database: Option<Arc<Database>>,
        auth_service: Option<Arc<AuthService>>,
        ws_manager: Arc<WebSocketManager>,
    ) -> Self {
        Self {
            store,
            alert_manager,
            database,
            auth_service,
            ws_manager,
        }
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
    info!(
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

    // Store metrics in-memory for fast queries
    state.store.insert_metrics(payload.clone());

    // Persist to database if available
    if let Some(db) = &state.database {
        let metrics: Vec<(String, f64)> = payload
            .metrics
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();

        // Register/update agent in database
        if let Err(e) = db
            .register_agent(
                &payload.agent_id,
                api_key_id,
                None, // hostname could be extracted from agent metadata
                None, // os
                None, // arch
            )
            .await
        {
            error!("Failed to register agent in database: {}", e);
        }

        // Insert metrics
        if let Err(e) = db
            .insert_metrics(&payload.agent_id, &metrics, payload.timestamp)
            .await
        {
            error!("Failed to persist metrics to database: {}", e);
            // Continue anyway - metrics are still in memory
        }
    }

    // Broadcast each metric via WebSocket
    for (metric_name, value) in &payload.metrics {
        state.ws_manager.broadcast_metric(WsMessage::Metric {
            agent_id: payload.agent_id.clone(),
            metric_name: metric_name.clone(),
            value: *value,
            timestamp: payload.timestamp,
        });
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "status": "accepted"
        })),
    ))
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
        RequestPrincipal::Admin => all_agents,
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
        RequestPrincipal::Admin => state.store.get_stats(),
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
        .ok_or_else(|| ApiError::Forbidden("Agent ID is unavailable to this API key".to_string()))?;

    // Store logs in-memory for fast queries
    state.store.insert_logs(payload.clone());

    // Persist to database if available
    if let Some(db) = &state.database {
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
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "status": "accepted"
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
        RequestPrincipal::Admin => None,
        RequestPrincipal::ApiKey(key_id) => Some(tenant_agent_ids(&state, key_id).await?),
    };
    if let Some(agent_id) = params.agent_id.as_deref() {
        if allowed_agent_ids
            .as_ref()
            .is_some_and(|allowed| !allowed.iter().any(|id| id == agent_id))
        {
            return Err(ApiError::Forbidden("Agent is not owned by this tenant".to_string()));
        }
    }

    let logs = state.store.query_logs(
        allowed_agent_ids.as_deref(),
        params.agent_id.as_deref(),
        level,
        params.from,
        params.to,
        params.keyword.as_deref(),
        params.limit,
    );

    let count = logs.len();
    let total_count = allowed_agent_ids.as_ref().map_or_else(
        || state.store.get_log_count(),
        |allowed| state.store.get_log_count_for_agents(allowed),
    );

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
        RequestPrincipal::Admin => all_alerts,
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
    Json(payload): Json<crate::alerts::AcknowledgeAlertRequest>,
) -> Result<Json<crate::alerts::Alert>, ApiError> {
    let alert = state
        .alert_manager
        .get_alert(&alert_id)
        .await
        .ok_or_else(|| ApiError::BadRequest(format!("Alert {} not found", alert_id)))?;
    ensure_agent_access(&state, &headers, &alert.agent_id).await?;
    info!(
        "Acknowledging alert: {} by {}",
        alert_id, payload.acknowledged_by
    );
    state
        .alert_manager
        .acknowledge_alert(&alert_id, payload.acknowledged_by)
        .await
        .map(Json)
        .ok_or_else(|| ApiError::BadRequest(format!("Alert {} not found", alert_id)))
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
    Admin,
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
        if auth_service
            .validate_admin_token(token)
            .await
            .map_err(|_| ApiError::InternalError("Authentication service error".to_string()))?
            .is_some()
        {
            return Ok(RequestPrincipal::Admin);
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
        RequestPrincipal::Admin => Ok(()),
        RequestPrincipal::ApiKey(key_id) => {
            let database = state
                .database
                .as_ref()
                .ok_or_else(|| ApiError::InternalError("Database is not available".to_string()))?;
            if database
                .agent_belongs_to_api_key(agent_id, key_id)
                .await
                .map_err(|_| ApiError::InternalError("Failed to verify agent ownership".to_string()))?
            {
                Ok(())
            } else {
                Err(ApiError::Forbidden("Agent is not owned by this tenant".to_string()))
            }
        }
    }
}

async fn require_admin(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AdminUser, ApiError> {
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
