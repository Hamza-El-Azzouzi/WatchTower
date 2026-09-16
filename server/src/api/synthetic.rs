use super::*;
use crate::synthetic::CheckSpec;
use sqlx::Row;

fn db_error(error: impl std::fmt::Display) -> ApiError {
    ApiError::InternalError(error.to_string())
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(spec): Json<CheckSpec>,
) -> Result<impl IntoResponse, ApiError> {
    let owner = require_tenant(&state, &headers).await?;
    spec.validate()
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    validate_owned_channels(&state, owner, &spec.channels).await?;
    let db = database(&state)?;
    let mut tx = db.pool().begin().await.map_err(db_error)?;
    // Serialize tenant quota decisions without locking unrelated enterprises.
    sqlx::query("SELECT id FROM api_keys WHERE id=$1 FOR UPDATE")
        .bind(owner)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_error)?;
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM synthetic_checks WHERE owner_api_key_id=$1 AND deleted_at IS NULL",
    )
    .bind(owner)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_error)?;
    if count >= 50 {
        return Err(ApiError::BadRequest(
            "Maximum 50 synthetic checks per enterprise key".into(),
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let agent = format!("synthetic:{id}");
    sqlx::query("INSERT INTO agents(id,name,agent_type,api_key_id) VALUES($1,$2,'synthetic',$3)")
        .bind(&agent)
        .bind(&spec.name)
        .bind(owner)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
    sqlx::query(
        "INSERT INTO synthetic_checks(id,owner_api_key_id,agent_id,spec) VALUES($1,$2,$3,$4)",
    )
    .bind(&id)
    .bind(owner)
    .bind(agent)
    .bind(serde_json::to_value(spec).map_err(db_error)?)
    .execute(&mut *tx)
    .await
    .map_err(db_error)?;
    tx.commit().await.map_err(db_error)?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({"id":id}))))
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let owner = require_tenant(&state, &headers).await?;
    let rows=sqlx::query("SELECT id,agent_id,spec,enabled,consecutive_failures,active_alert_id,last_result,next_run_at,created_at FROM synthetic_checks WHERE owner_api_key_id=$1 AND deleted_at IS NULL ORDER BY created_at DESC")
        .bind(owner).fetch_all(database(&state)?.pool()).await.map_err(db_error)?;
    Ok(Json(rows.into_iter().map(|row|serde_json::json!({
        "id":row.get::<String,_>("id"),"agent_id":row.get::<String,_>("agent_id"),"spec":row.get::<serde_json::Value,_>("spec"),
        "enabled":row.get::<bool,_>("enabled"),"consecutive_failures":row.get::<i32,_>("consecutive_failures"),
        "active_alert_id":row.get::<Option<String>,_>("active_alert_id"),"last_result":row.get::<Option<serde_json::Value>,_>("last_result"),
        "next_run_at":row.get::<chrono::DateTime<Utc>,_>("next_run_at"),"created_at":row.get::<chrono::DateTime<Utc>,_>("created_at")
    })).collect()))
}

pub async fn set_enabled(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<SetEnabledRequest>,
) -> Result<StatusCode, ApiError> {
    let owner = require_tenant(&state, &headers).await?;
    let result=sqlx::query("UPDATE synthetic_checks SET enabled=$3,lease_token=NULL,leased_until=NULL,next_run_at=now(),consecutive_failures=0 WHERE id=$1 AND owner_api_key_id=$2 AND deleted_at IS NULL")
        .bind(id).bind(owner).bind(payload.enabled).execute(database(&state)?.pool()).await.map_err(db_error)?;
    if result.rows_affected() == 0 {
        return Err(ApiError::Forbidden(
            "Check is not owned by this enterprise key".into(),
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let owner = require_tenant(&state, &headers).await?;
    let db = database(&state)?;
    let mut tx = db.pool().begin().await.map_err(db_error)?;
    let row = sqlx::query(
        "UPDATE synthetic_checks SET deleted_at=now(),enabled=false,lease_token=NULL,leased_until=NULL WHERE id=$1 AND owner_api_key_id=$2 AND deleted_at IS NULL RETURNING agent_id",
    )
    .bind(id)
    .bind(owner)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_error)?
    .ok_or_else(|| ApiError::Forbidden("Check is not owned by this enterprise key".into()))?;
    let agent: String = row.get("agent_id");
    sqlx::query("UPDATE notification_deliveries SET status='failed',attempt_count=max_attempts,error_message='Synthetic monitor deleted before delivery' WHERE alert_id IN (SELECT alert_id FROM alerts WHERE agent_id=$1) AND status IN ('pending','failed') AND attempt_count<max_attempts")
        .bind(&agent).execute(&mut *tx).await.map_err(db_error)?;
    // Keep incident/history references but retire active alerts without claiming recovery.
    sqlx::query("UPDATE alerts SET state='resolved',resolved_at=now(),message='Synthetic monitor deleted by its owner' WHERE agent_id=$1 AND state!='resolved'")
        .bind(&agent).execute(&mut *tx).await.map_err(db_error)?;
    tx.commit().await.map_err(db_error)?;
    for alert in db
        .list_alerts(Some(1000))
        .await
        .map_err(db_error)?
        .into_iter()
        .filter(|alert| alert.agent_id == agent)
    {
        state.alert_manager.cache_persisted_alert(alert).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn history(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<DeliveryQuery>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let owner = require_tenant(&state, &headers).await?;
    let db = database(&state)?;
    let owned: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM synthetic_checks WHERE id=$1 AND owner_api_key_id=$2)",
    )
    .bind(&id)
    .bind(owner)
    .fetch_one(db.pool())
    .await
    .map_err(db_error)?;
    if !owned {
        return Err(ApiError::Forbidden(
            "Check is not owned by this enterprise key".into(),
        ));
    }
    let rows=sqlx::query("SELECT r.id,r.result FROM synthetic_results r JOIN synthetic_checks c ON c.id=r.check_id WHERE r.check_id=$1 AND c.owner_api_key_id=$2 ORDER BY r.id DESC LIMIT $3")
        .bind(id).bind(owner).bind(query.limit.unwrap_or(100).clamp(1,500)).fetch_all(db.pool()).await.map_err(db_error)?;
    Ok(Json(rows.into_iter().map(|row|serde_json::json!({"id":row.get::<i64,_>("id"),"result":row.get::<serde_json::Value,_>("result")})).collect()))
}
