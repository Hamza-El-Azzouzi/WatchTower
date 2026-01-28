use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::alerts::manager::AlertManager;
use crate::auth::{AuthService, CreateApiKeyRequest};
use crate::db::Database;
use crate::storage::{Agent, DataPoint, MetricsPayload, StorageStats, TimeSeriesStore};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub store: TimeSeriesStore,
    pub alert_manager: Arc<AlertManager>,
    pub database: Option<Arc<Database>>,
    pub auth_service: Option<Arc<AuthService>>,
}

impl AppState {
    pub fn new(
        store: TimeSeriesStore,
        alert_manager: Arc<AlertManager>,
        database: Option<Arc<Database>>,
        auth_service: Option<Arc<AuthService>>,
    ) -> Self {
        Self {
            store,
            alert_manager,
            database,
            auth_service,
        }
    }
}

/// Custom error type for API responses
#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
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
    Json(payload): Json<MetricsPayload>,
) -> Result<impl IntoResponse, ApiError> {
    info!(
        "Received metrics from agent '{}' with {} metrics",
        payload.agent_id,
        payload.metrics.len()
    );

    state.store.insert_metrics(payload);

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
    Query(params): Query<MetricsQuery>,
) -> Result<Json<MetricsResponse>, ApiError> {
    info!(
        "Querying metrics for agent '{}', metric '{}'",
        params.agent_id, params.metric
    );

    let mut data_points =
        state
            .store
            .query(&params.agent_id, &params.metric, params.from, params.to);

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
    Query(params): Query<LatestMetricsQuery>,
) -> Result<Json<LatestMetricsResponse>, ApiError> {
    info!("Getting latest metrics for agent '{}'", params.agent_id);

    let metric_names = state.store.get_agent_metrics(&params.agent_id);
    let mut metrics = Vec::new();

    for metric_name in metric_names {
        if let Some(data_point) = state.store.get_latest(&params.agent_id, &metric_name) {
            metrics.push(LatestMetric {
                name: metric_name,
                value: data_point.value,
                timestamp: data_point.timestamp,
            });
        }
    }

    Ok(Json(LatestMetricsResponse {
        agent_id: params.agent_id,
        metrics,
    }))
}

/// GET /api/v1/agents - List all registered agents
pub async fn list_agents(State(state): State<Arc<AppState>>) -> Json<Vec<Agent>> {
    info!("Listing all registered agents");
    let agents = state.store.get_agents();
    Json(agents)
}

/// GET /api/v1/agents/:agent_id - Get specific agent details
#[derive(Debug, Deserialize)]
pub struct AgentQuery {
    pub agent_id: String,
}

pub async fn get_agent(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AgentQuery>,
) -> Result<Json<Agent>, ApiError> {
    info!("Getting agent details for '{}'", params.agent_id);

    state
        .store
        .get_agent(&params.agent_id)
        .map(Json)
        .ok_or_else(|| ApiError::BadRequest(format!("Agent '{}' not found", params.agent_id)))
}

/// GET /api/v1/stats - Get storage statistics
pub async fn get_stats(State(state): State<Arc<AppState>>) -> Json<StorageStats> {
    info!("Getting storage statistics");
    let stats = state.store.get_stats();
    Json(stats)
}

/// GET /health - Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": Utc::now()
    }))
}

// ============ Alert API Endpoints ============

/// POST /api/v1/alert-rules - Create a new alert rule
pub async fn create_alert_rule(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<crate::alerts::CreateAlertRuleRequest>,
) -> Result<Json<crate::alerts::AlertRule>, ApiError> {
    info!("Creating new alert rule: {}", payload.name);
    let rule = state.alert_manager.create_rule(payload).await;
    Ok(Json(rule))
}

/// GET /api/v1/alert-rules - List all alert rules
pub async fn list_alert_rules(
    State(state): State<Arc<AppState>>,
) -> Json<crate::alerts::AlertRulesResponse> {
    let rules = state.alert_manager.list_rules().await;
    let total = rules.len();
    Json(crate::alerts::AlertRulesResponse { rules, total })
}

/// GET /api/v1/alert-rules/:id - Get specific alert rule
pub async fn get_alert_rule(
    State(state): State<Arc<AppState>>,
    Path(rule_id): Path<String>,
) -> Result<Json<crate::alerts::AlertRule>, ApiError> {
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
    Path(rule_id): Path<String>,
    Json(payload): Json<crate::alerts::UpdateAlertRuleRequest>,
) -> Result<Json<crate::alerts::AlertRule>, ApiError> {
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
    Path(rule_id): Path<String>,
) -> Result<StatusCode, ApiError> {
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
    Path(rule_id): Path<String>,
) -> Result<Json<crate::alerts::AlertRule>, ApiError> {
    info!("Toggling alert rule: {}", rule_id);
    state
        .alert_manager
        .toggle_rule(&rule_id)
        .await
        .map(Json)
        .ok_or_else(|| ApiError::BadRequest(format!("Alert rule {} not found", rule_id)))
}

/// GET /api/v1/alerts - Get all alerts
pub async fn list_alerts(
    State(state): State<Arc<AppState>>,
) -> Json<crate::alerts::AlertsResponse> {
    let all_alerts = state.alert_manager.get_all_alerts().await;
    let active_alerts: Vec<_> = all_alerts
        .iter()
        .filter(|a| a.is_active())
        .cloned()
        .collect();
    let recent_alerts: Vec<_> = all_alerts
        .iter()
        .filter(|a| a.state == crate::alerts::AlertState::Resolved)
        .take(50)
        .cloned()
        .collect();

    Json(crate::alerts::AlertsResponse {
        total_active: active_alerts.len(),
        active_alerts,
        recent_alerts,
    })
}

/// GET /api/v1/alerts/:id - Get specific alert
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
    Path(alert_id): Path<String>,
    Json(payload): Json<crate::alerts::AcknowledgeAlertRequest>,
) -> Result<Json<crate::alerts::Alert>, ApiError> {
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

/// POST /api/v1/auth/keys - Create a new API key
pub async fn create_api_key(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let auth_service = state
        .auth_service
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Authentication is not enabled".to_string()))?;

    match auth_service.create_api_key(request, "admin").await {
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

/// GET /api/v1/auth/keys - List all API keys
pub async fn list_api_keys(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
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

/// DELETE /api/v1/auth/keys/:key_id - Revoke an API key
pub async fn revoke_api_key(
    State(state): State<Arc<AppState>>,
    Path(key_id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
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
