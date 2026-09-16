//! Narrow control plane. Configuration never includes commands, credentials or paths.
use super::*;
use crate::auth::ApiKey;
use base64::{engine::general_purpose::STANDARD, Engine};
use ring::signature::{Ed25519KeyPair, KeyPair};
use sqlx::Row;

fn db_error(error: impl std::fmt::Display) -> ApiError {
    error!("Agent control operation failed: {error}");
    ApiError::InternalError("agent control operation failed".into())
}
fn valid_id(id: &str) -> Result<(), ApiError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:".contains(&byte))
    {
        return Err(ApiError::BadRequest("invalid agent id".into()));
    }
    Ok(())
}
async fn key_owner(state: &AppState, headers: &HeaderMap, id: &str) -> Result<i64, ApiError> {
    match request_principal(state, headers).await? {
        RequestPrincipal::ApiKey(key_id) => {
            ensure_agent_access(state, headers, id).await?;
            Ok(key_id)
        }
        _ => Err(ApiError::Forbidden("an agent API key is required".into())),
    }
}
#[derive(Deserialize, Serialize)]
pub struct Heartbeat {
    pub agent_id: String,
    pub version: String,
    pub architecture: String,
    pub os: String,
    pub capabilities: Vec<String>,
    pub queue_records: u64,
    pub queue_bytes: u64,
    pub dropped_samples: u64,
    pub last_successful_upload: Option<DateTime<Utc>>,
    pub config_revision: u64,
}
pub async fn heartbeat(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(report): Json<Heartbeat>,
) -> Result<Json<serde_json::Value>, ApiError> {
    valid_id(&report.agent_id)?;
    if [&report.version, &report.architecture, &report.os]
        .iter()
        .any(|text| text.is_empty() || text.len() > 96 || text.chars().any(char::is_control))
        || report.capabilities.len() > 32
        || report
            .capabilities
            .iter()
            .any(|text| text.len() > 96 || text.chars().any(char::is_control))
    {
        return Err(ApiError::BadRequest("invalid heartbeat metadata".into()));
    }
    let key_id = key_owner(&state, &headers, &report.agent_id).await?;
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| db_error("database unavailable"))?;
    db.register_agent(
        &report.agent_id,
        Some(key_id),
        None,
        Some(&report.os),
        Some(&report.architecture),
    )
    .await
    .map_err(db_error)?;
    sqlx::query("UPDATE agents SET delivery_status=$2,last_heartbeat_at=now() WHERE id=$1")
        .bind(&report.agent_id)
        .bind(serde_json::to_value(&report).map_err(db_error)?)
        .execute(db.pool())
        .await
        .map_err(db_error)?;
    state
        .store
        .register_agent(&report.agent_id, &report.agent_id);
    Ok(Json(serde_json::json!({"status":"accepted"})))
}
pub async fn status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    ensure_agent_access(&state, &headers, &id).await?;
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| db_error("database unavailable"))?;
    let row = sqlx::query(
        "SELECT delivery_status,last_seen,last_successful_upload,last_heartbeat_at FROM agents WHERE id=$1",
    )
    .bind(id)
    .fetch_one(db.pool())
    .await
    .map_err(db_error)?;
    Ok(Json(
        serde_json::json!({"report":row.get::<serde_json::Value,_>("delivery_status"),"last_seen":row.get::<DateTime<Utc>,_>("last_seen"),"last_successful_upload":row.get::<Option<DateTime<Utc>>,_>("last_successful_upload"),"last_heartbeat_at":row.get::<Option<DateTime<Utc>>,_>("last_heartbeat_at")}),
    ))
}
#[derive(Deserialize)]
pub struct EnrollmentRequest {
    pub agent_id: String,
}
pub async fn create_enrollment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<EnrollmentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin = require_admin(&state, &headers).await?;
    valid_id(&request.agent_id)?;
    let token = ApiKey::generate();
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| db_error("database unavailable"))?;
    let existing: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM agents WHERE id=$1)")
        .bind(&request.agent_id)
        .fetch_one(db.pool())
        .await
        .map_err(db_error)?;
    if existing {
        return Err(ApiError::BadRequest(
            "agent already enrolled; rotate its key instead".into(),
        ));
    }
    sqlx::query("INSERT INTO agent_enrollment_tokens(token_hash,agent_id,expires_at,created_by) VALUES($1,$2,now()+interval '15 minutes',$3)").bind(ApiKey::hash(&token)).bind(&request.agent_id).bind(admin.username).execute(db.pool()).await.map_err(db_error)?;
    Ok(Json(
        serde_json::json!({"token":token,"agent_id":request.agent_id,"expires_in_seconds":900}),
    ))
}
async fn insert_key(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    key: &str,
    name: &str,
    pending: bool,
) -> Result<i64, ApiError> {
    sqlx::query_scalar("INSERT INTO api_keys(key_hash,key_prefix,name,created_by,max_agents,expires_at) VALUES($1,$2,$3,'agent-control',1,CASE WHEN $4 THEN now()+interval '15 minutes' ELSE NULL END) RETURNING id::BIGINT")
        .bind(ApiKey::hash(key)).bind(key.chars().take(12).collect::<String>()).bind(name).bind(pending).fetch_one(&mut **transaction).await.map_err(db_error)
}
pub async fn enroll(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<EnrollmentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    valid_id(&request.agent_id)?;
    let token = extract_api_key(&headers)
        .ok_or_else(|| ApiError::Unauthorized("enrollment token required".into()))?;
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| db_error("database unavailable"))?;
    let mut tx = db.pool().begin().await.map_err(db_error)?;
    let used = sqlx::query("UPDATE agent_enrollment_tokens SET consumed_at=now() WHERE token_hash=$1 AND agent_id=$2 AND consumed_at IS NULL AND expires_at>now()")
        .bind(ApiKey::hash(token)).bind(&request.agent_id).execute(&mut *tx).await.map_err(db_error)?;
    if used.rows_affected() != 1 {
        return Err(ApiError::Unauthorized(
            "invalid, used or expired enrollment token".into(),
        ));
    }
    let key = ApiKey::generate();
    let key_id = insert_key(&mut tx, &key, &request.agent_id, false).await?;
    // Unique agent identity and token consumption are committed together.
    sqlx::query("INSERT INTO agents(id,name,agent_type,api_key_id,last_seen) VALUES($1,$1,'server',$2,now())").bind(&request.agent_id).bind(key_id).execute(&mut *tx).await.map_err(db_error)?;
    tx.commit().await.map_err(db_error)?;
    Ok(Json(serde_json::json!({"api_key":key})))
}
pub async fn rotate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let old_id = key_owner(&state, &headers, &id).await?;
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| db_error("database unavailable"))?;
    let mut tx = db.pool().begin().await.map_err(db_error)?;
    sqlx::query("SELECT id FROM agents WHERE id=$1 FOR UPDATE")
        .bind(&id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_error)?;
    let agent_count: i64 = sqlx::query_scalar("SELECT count(*) FROM agents WHERE api_key_id=$1")
        .bind(old_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_error)?;
    if agent_count != 1 {
        return Err(ApiError::BadRequest(
            "rotation requires a dedicated single-agent key".into(),
        ));
    }
    sqlx::query("UPDATE api_keys SET revoked=true,revoked_at=now() WHERE id IN (SELECT new_key_id FROM agent_key_rotations WHERE agent_id=$1)").bind(&id).execute(&mut *tx).await.map_err(db_error)?;
    let key = ApiKey::generate();
    let new_id = insert_key(&mut tx, &key, &id, true).await?;
    sqlx::query("INSERT INTO agent_key_rotations(agent_id,old_key_id,new_key_id,expires_at) VALUES($1,$2,$3,now()+interval '15 minutes') ON CONFLICT(agent_id) DO UPDATE SET old_key_id=$2,new_key_id=$3,expires_at=now()+interval '15 minutes'").bind(&id).bind(old_id).bind(new_id).execute(&mut *tx).await.map_err(db_error)?;
    tx.commit().await.map_err(db_error)?;
    Ok(Json(serde_json::json!({"api_key":key})))
}
pub async fn confirm_rotation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let auth = state
        .auth_service
        .as_ref()
        .ok_or_else(|| db_error("auth unavailable"))?;
    let key = extract_api_key(&headers)
        .ok_or_else(|| ApiError::Unauthorized("new agent key required".into()))?;
    let key_id = auth
        .validate_api_key(key)
        .await
        .map_err(db_error)?
        .ok_or_else(|| ApiError::Unauthorized("invalid key".into()))?;
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| db_error("database unavailable"))?;
    let mut tx = db.pool().begin().await.map_err(db_error)?;
    sqlx::query("SELECT id FROM agents WHERE id=$1 FOR UPDATE")
        .bind(&id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_error)?;
    let rotation = sqlx::query("DELETE FROM agent_key_rotations WHERE agent_id=$1 AND new_key_id=$2 AND expires_at>now() RETURNING old_key_id").bind(&id).bind(key_id).fetch_optional(&mut *tx).await.map_err(db_error)?;
    if let Some(rotation) = rotation {
        sqlx::query("UPDATE agents SET api_key_id=$2 WHERE id=$1")
            .bind(&id)
            .bind(key_id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        sqlx::query("UPDATE api_keys SET expires_at=NULL WHERE id=$1")
            .bind(key_id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        sqlx::query("UPDATE api_keys SET revoked=true,revoked_at=now() WHERE id=$1")
            .bind(rotation.get::<i64, _>("old_key_id"))
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
    } else {
        let owned: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM agents WHERE id=$1 AND api_key_id=$2)")
                .bind(&id)
                .bind(key_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db_error)?;
        if !owned {
            return Err(ApiError::Forbidden(
                "rotation is not pending for this key".into(),
            ));
        }
    }
    tx.commit().await.map_err(db_error)?;
    Ok(Json(serde_json::json!({"status":"confirmed"})))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteSettings {
    pub interval_seconds: u64,
    pub collect_cpu: bool,
    pub collect_memory: bool,
    pub collect_disk: bool,
    pub collect_network: bool,
}
#[derive(Serialize)]
struct SignedPayload {
    agent_id: String,
    revision: i64,
    expires_at: DateTime<Utc>,
    settings: RemoteSettings,
}
pub async fn publish_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(settings): Json<RemoteSettings>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let admin = require_admin(&state, &headers).await?;
    if !(1..=300).contains(&settings.interval_seconds) {
        return Err(ApiError::BadRequest(
            "interval must be 1-300 seconds".into(),
        ));
    }
    let path = std::env::var("AGENT_CONFIG_SIGNING_KEY_FILE")
        .map_err(|_| ApiError::BadRequest("configuration signing is not configured".into()))?;
    let bytes = std::fs::read(path).map_err(db_error)?;
    let key = Ed25519KeyPair::from_pkcs8(&bytes).map_err(db_error)?;
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| db_error("database unavailable"))?;
    let mut tx = db.pool().begin().await.map_err(db_error)?;
    sqlx::query("SELECT id FROM agents WHERE id=$1 FOR UPDATE")
        .bind(&id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_error)?;
    let previous: Option<i64> =
        sqlx::query_scalar("SELECT revision FROM agent_signed_configs WHERE agent_id=$1")
            .bind(&id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_error)?;
    let payload = serde_json::to_string(&SignedPayload {
        agent_id: id.clone(),
        revision: previous.unwrap_or(0) + 1,
        expires_at: Utc::now() + chrono::Duration::days(7),
        settings,
    })
    .map_err(db_error)?;
    let envelope = serde_json::json!({"payload":payload,"signature":STANDARD.encode(key.sign(payload.as_bytes()).as_ref()),"public_key":STANDARD.encode(key.public_key().as_ref())});
    sqlx::query("INSERT INTO agent_signed_configs(agent_id,revision,envelope,updated_by) VALUES($1,$2,$3,$4) ON CONFLICT(agent_id) DO UPDATE SET revision=$2,envelope=$3,updated_by=$4,updated_at=now()")
        .bind(&id).bind(previous.unwrap_or(0)+1).bind(&envelope).bind(admin.username).execute(&mut *tx).await.map_err(db_error)?;
    tx.commit().await.map_err(db_error)?;
    Ok(Json(envelope))
}
pub async fn fetch_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    key_owner(&state, &headers, &id).await?;
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| db_error("database unavailable"))?;
    let envelope: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT envelope FROM agent_signed_configs WHERE agent_id=$1")
            .bind(id)
            .fetch_optional(db.pool())
            .await
            .map_err(db_error)?;
    Ok(Json(envelope.unwrap_or(serde_json::Value::Null)))
}
