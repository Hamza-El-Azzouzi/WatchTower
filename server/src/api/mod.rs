use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::storage::{Agent, DataPoint, MetricsPayload, StorageStats, TimeSeriesStore};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub store: TimeSeriesStore,
}

impl AppState {
    pub fn new(store: TimeSeriesStore) -> Self {
        Self { store }
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

    let mut data_points = state
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
