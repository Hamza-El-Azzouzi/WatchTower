use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

use crate::auth::AuthService;

/// Extract API key from Authorization or X-API-Key header
fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    // Try X-API-Key header first (preferred for agents)
    if let Some(key) = headers.get("X-API-Key").and_then(|v| v.to_str().ok()) {
        return Some(key.to_string());
    }

    // Fall back to Authorization: Bearer format
    headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|auth| auth.strip_prefix("Bearer ").map(|s| s.to_string()))
}

/// Extract admin token from X-Admin-Token header
fn extract_admin_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("X-Admin-Token")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Middleware to validate API keys with agent limit enforcement
/// Also accepts admin tokens for dashboard access
pub async fn auth_middleware(
    State(auth_service): State<Arc<AuthService>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    // First check for admin token via X-Admin-Token header
    if let Some(admin_token) = extract_admin_token(&headers) {
        if admin_token.starts_with("admin_") && admin_token.len() > 10 {
            // Valid admin token format - allow access
            return Ok(next.run(request).await);
        }
    }

    // Also check for admin token via Authorization: Bearer header
    if let Some(bearer_token) = extract_api_key(&headers) {
        if bearer_token.starts_with("admin_") && bearer_token.len() > 10 {
            // Valid admin token format via Bearer - allow access
            return Ok(next.run(request).await);
        }
    }

    let api_key = match extract_api_key(&headers) {
        Some(key) => key,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Missing or invalid API key. Expected: X-API-Key header or Authorization: Bearer <api-key>"
                })),
            ));
        }
    };

    // Extract agent_id from request (query params or body)
    let agent_id = request.uri().query().and_then(|q| {
        q.split('&')
            .find(|p| p.starts_with("agent_id="))
            .map(|p| p.strip_prefix("agent_id=").unwrap_or("").to_string())
    });

    // Validate with agent limit enforcement
    match auth_service
        .validate_api_key_with_agent(&api_key, agent_id.as_deref())
        .await
    {
        Ok(Some(_)) => Ok(next.run(request).await),
        Ok(None) => Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Invalid, expired, or agent limit reached for this API key"
            })),
        )),
        Err(e) => {
            tracing::error!("Auth validation error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Authentication service error"
                })),
            ))
        }
    }
}
