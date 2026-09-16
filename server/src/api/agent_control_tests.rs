//! Explicitly enabled tests against a disposable database, never production.
use super::agent_control::*;
use super::*;
use crate::storage::timeseries::DeliveryIdentity;
use sqlx::Row;

#[tokio::test]
#[ignore = "requires WATCHTOWER_TEST_DATABASE_URL pointing to a disposable database"]
async fn reliable_delivery_control_plane() {
    let url =
        std::env::var("WATCHTOWER_TEST_DATABASE_URL").expect("disposable test database required");
    assert!(
        url.contains("watchtower_test"),
        "refusing to run destructive fixtures against a non-test database"
    );
    let db = Arc::new(Database::new(&url).await.unwrap());
    let auth = Arc::new(AuthService::new(db.pool().clone()));
    let store = TimeSeriesStore::new(1000);
    let manager = Arc::new(AlertManager::new(store.clone(), db.clone()).await);
    let state = Arc::new(AppState::new(
        store,
        manager,
        Some(db.clone()),
        Some(auth.clone()),
        Arc::new(WebSocketManager::new()),
        15,
    ));
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let id = format!("delivery-{suffix}");
    let admin_name = format!("admin-{suffix}");
    auth.create_admin(&admin_name, "ci-only-admin-test-password", None, None)
        .await
        .unwrap();
    let admin = auth
        .authenticate_admin(&admin_name, "ci-only-admin-test-password")
        .await
        .unwrap()
        .unwrap();
    let admin_token = AuthService::generate_admin_token(&admin).unwrap();
    let mut admin_headers = HeaderMap::new();
    admin_headers.insert("X-Admin-Token", admin_token.parse().unwrap());
    let Json(token) = create_enrollment(
        State(state.clone()),
        admin_headers.clone(),
        Json(EnrollmentRequest {
            agent_id: id.clone(),
        }),
    )
    .await
    .unwrap();
    let mut token_headers = HeaderMap::new();
    token_headers.insert(
        "X-API-Key",
        token["token"].as_str().unwrap().parse().unwrap(),
    );
    let Json(enrolled) = enroll(
        State(state.clone()),
        token_headers.clone(),
        Json(EnrollmentRequest {
            agent_id: id.clone(),
        }),
    )
    .await
    .unwrap();
    assert!(
        enroll(
            State(state.clone()),
            token_headers,
            Json(EnrollmentRequest {
                agent_id: id.clone()
            })
        )
        .await
        .is_err(),
        "single-use token replay"
    );
    let key = enrolled["api_key"].as_str().unwrap();
    let mut headers = HeaderMap::new();
    headers.insert("X-API-Key", key.parse().unwrap());
    let payload = MetricsPayload {
        agent_id: id.clone(),
        timestamp: Utc::now(),
        metrics: [("cpu_usage".into(), 42.0)].into(),
        delivery: Some(DeliveryIdentity {
            stream_id: "0123456789abcdef0123456789abcdef".into(),
            sequence: 1,
        }),
        processes: vec![],
        mounts: vec![],
        network_interfaces: vec![],
        services: vec![],
        containers: vec![],
    };
    let (first, duplicate) =
        tokio::join!(db.persist_delivery(&payload), db.persist_delivery(&payload));
    assert_ne!(
        first.unwrap(),
        duplicate.unwrap(),
        "concurrent duplicate committed twice"
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM metrics WHERE agent_id=$1")
        .bind(&id)
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(count, 1);
    // Force a database write failure after claiming a sequence: the transaction
    // must roll back the watermark so retrying the same record remains possible.
    sqlx::query("CREATE OR REPLACE FUNCTION watchtower_test_fail_metric() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.metric_name='force_transaction_failure' THEN RAISE EXCEPTION 'test failure'; END IF; RETURN NEW; END $$").execute(db.pool()).await.unwrap();
    sqlx::query("CREATE TRIGGER watchtower_test_metric_failure BEFORE INSERT ON metrics FOR EACH ROW EXECUTE FUNCTION watchtower_test_fail_metric()").execute(db.pool()).await.unwrap();
    let mut failed = payload.clone();
    failed.delivery.as_mut().unwrap().sequence = 2;
    failed.metrics = [("force_transaction_failure".into(), 1.0)].into();
    assert!(db.persist_delivery(&failed).await.is_err());
    let last: i64 =
        sqlx::query_scalar("SELECT last_sequence FROM agent_delivery_streams WHERE agent_id=$1")
            .bind(&id)
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(last, 1);
    failed.metrics = [("cpu_usage".into(), 43.0)].into();
    assert!(db.persist_delivery(&failed).await.unwrap());
    sqlx::query("DROP TRIGGER watchtower_test_metric_failure ON metrics")
        .execute(db.pool())
        .await
        .unwrap();
    sqlx::query("DROP FUNCTION watchtower_test_fail_metric()")
        .execute(db.pool())
        .await
        .unwrap();
    let log_payload = crate::storage::LogsPayload {
        agent_id: id.clone(),
        delivery: Some(DeliveryIdentity {
            stream_id: payload.delivery.as_ref().unwrap().stream_id.clone(),
            sequence: 3,
        }),
        logs: vec![crate::storage::timeseries::LogEntryInput {
            timestamp: Utc::now(),
            level: crate::storage::LogLevel::INFO,
            source: "test".into(),
            message: "durable test".into(),
        }],
    };
    assert!(db.persist_log_delivery(&log_payload).await.unwrap());
    assert!(!db.persist_log_delivery(&log_payload).await.unwrap());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM logs WHERE agent_id=$1")
        .bind(&id)
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(count, 1);
    let report = Heartbeat {
        agent_id: id.clone(),
        version: "0.1.0".into(),
        architecture: "aarch64".into(),
        os: "linux".into(),
        capabilities: vec!["disk_queue".into()],
        queue_records: 1,
        queue_bytes: 128,
        dropped_samples: 0,
        last_successful_upload: Some(Utc::now()),
        config_revision: 0,
    };
    let _ = heartbeat(State(state.clone()), headers.clone(), Json(report))
        .await
        .unwrap();
    let Json(delivery) = status(State(state.clone()), headers.clone(), Path(id.clone()))
        .await
        .unwrap();
    assert_eq!(delivery["report"]["architecture"], "aarch64");
    let other = auth
        .create_api_key(
            CreateApiKeyRequest {
                name: "other".into(),
                description: None,
                expires_in_days: None,
                max_agents: Some(1),
            },
            "test",
        )
        .await
        .unwrap();
    let mut other_headers = HeaderMap::new();
    other_headers.insert("X-API-Key", other.key.parse().unwrap());
    assert!(status(
        State(state.clone()),
        other_headers.clone(),
        Path(id.clone())
    )
    .await
    .is_err());
    assert!(
        rotate(State(state.clone()), other_headers, Path(id.clone()))
            .await
            .is_err()
    );
    let Json(rotation) = rotate(State(state.clone()), headers.clone(), Path(id.clone()))
        .await
        .unwrap();
    assert!(
        auth.validate_api_key(key).await.unwrap().is_some(),
        "old key revoked before local persistence"
    );
    let mut new_headers = HeaderMap::new();
    new_headers.insert(
        "X-API-Key",
        rotation["api_key"].as_str().unwrap().parse().unwrap(),
    );
    let _ = confirm_rotation(State(state.clone()), new_headers.clone(), Path(id.clone()))
        .await
        .unwrap();
    let _ = confirm_rotation(State(state.clone()), new_headers.clone(), Path(id.clone()))
        .await
        .unwrap();
    assert!(auth.validate_api_key(key).await.unwrap().is_none());
    assert!(
        status(State(state.clone()), new_headers.clone(), Path(id.clone()))
            .await
            .is_ok()
    );
    let row = sqlx::query("SELECT last_successful_upload FROM agents WHERE id=$1")
        .bind(&id)
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert!(row
        .get::<Option<DateTime<Utc>>, _>("last_successful_upload")
        .is_some());
    // Configuration signatures are generated server-side with pinned offline keys.
    let temporary = tempfile::tempdir().unwrap();
    let private_path = temporary.path().join("signing.pk8");
    let private =
        ring::signature::Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    std::fs::write(&private_path, private.as_ref()).unwrap();
    std::env::set_var("AGENT_CONFIG_SIGNING_KEY_FILE", &private_path);
    let settings = RemoteSettings {
        interval_seconds: 2,
        collect_cpu: true,
        collect_memory: true,
        collect_disk: true,
        collect_network: true,
    };
    let Json(first) = publish_config(
        State(state.clone()),
        admin_headers.clone(),
        Path(id.clone()),
        Json(settings.clone()),
    )
    .await
    .unwrap();
    let Json(second) = publish_config(
        State(state.clone()),
        admin_headers.clone(),
        Path(id.clone()),
        Json(settings),
    )
    .await
    .unwrap();
    let first_payload: serde_json::Value =
        serde_json::from_str(first["payload"].as_str().unwrap()).unwrap();
    let second_payload: serde_json::Value =
        serde_json::from_str(second["payload"].as_str().unwrap()).unwrap();
    assert_eq!(first_payload["revision"], 1);
    assert_eq!(second_payload["revision"], 2);
    use base64::{engine::general_purpose::STANDARD, Engine};
    let public = STANDARD
        .decode(second["public_key"].as_str().unwrap())
        .unwrap();
    ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, public)
        .verify(
            second["payload"].as_str().unwrap().as_bytes(),
            &STANDARD
                .decode(second["signature"].as_str().unwrap())
                .unwrap(),
        )
        .unwrap();
    let Json(fetched) = fetch_config(State(state.clone()), new_headers, Path(id.clone()))
        .await
        .unwrap();
    assert_eq!(fetched, second);
    std::env::remove_var("AGENT_CONFIG_SIGNING_KEY_FILE");
    let expired_id = format!("expired-{suffix}");
    let Json(expired) = create_enrollment(
        State(state.clone()),
        admin_headers,
        Json(EnrollmentRequest {
            agent_id: expired_id.clone(),
        }),
    )
    .await
    .unwrap();
    sqlx::query(
        "UPDATE agent_enrollment_tokens SET expires_at=now()-interval '1 second' WHERE agent_id=$1",
    )
    .bind(&expired_id)
    .execute(db.pool())
    .await
    .unwrap();
    let mut expired_headers = HeaderMap::new();
    expired_headers.insert(
        "X-API-Key",
        expired["token"].as_str().unwrap().parse().unwrap(),
    );
    assert!(enroll(
        State(state.clone()),
        expired_headers,
        Json(EnrollmentRequest {
            agent_id: expired_id
        })
    )
    .await
    .is_err());
    // Fixtures are confined to the dedicated test database/container.
    sqlx::query("DELETE FROM agents WHERE id=$1")
        .bind(&id)
        .execute(db.pool())
        .await
        .unwrap();
}
