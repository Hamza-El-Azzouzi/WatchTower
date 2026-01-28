use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

use crate::auth::AuthService;

/// Extract API key from Authorization header
fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|auth| {
            if auth.starts_with("Bearer ") {
                Some(auth[7..].to_string())
            } else {
                None
            }
        })
}

/// Middleware to validate API keys with agent limit enforcement
pub async fn auth_middleware(
    State(auth_service): State<Arc<AuthService>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let api_key = match extract_api_key(&headers) {
        Some(key) => key,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Missing or invalid Authorization header. Expected: Authorization: Bearer <api-key>"
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
