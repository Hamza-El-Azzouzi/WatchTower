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
    let rule_id =
        verify_enterprise_alerting(&state, &headers, &other_headers, &admin_headers, &payload)
            .await;
    verify_synthetic_monitoring(&state, &headers, &other_headers).await;
    assert!(status(
        State(state.clone()),
        other_headers.clone(),
        Path(id.clone())
    )
    .await
    .is_err());
    assert!(rotate(
        State(state.clone()),
        other_headers.clone(),
        Path(id.clone())
    )
    .await
    .is_err());
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
    let Json(rotated_rules) = list_alert_rules(State(state.clone()), new_headers.clone())
        .await
        .unwrap();
    assert!(
        rotated_rules.rules.iter().any(|rule| rule.id == rule_id),
        "rotation lost enterprise rule ownership"
    );
    let Json(rotated_channels) =
        list_notification_channels(State(state.clone()), new_headers.clone())
            .await
            .unwrap();
    assert_eq!(
        rotated_channels.len(),
        1,
        "rotation lost notification channels"
    );
    let Json(rotated_deliveries) = list_notification_deliveries(
        State(state.clone()),
        new_headers.clone(),
        Query(DeliveryQuery { limit: Some(100) }),
    )
    .await
    .unwrap();
    assert!(
        !rotated_deliveries.is_empty(),
        "rotation lost delivery history"
    );
    let persisted = db.get_alert_rule(&rule_id).await.unwrap().unwrap();
    assert_eq!(
        persisted.owner_api_key_id,
        rotated_rules
            .rules
            .iter()
            .find(|rule| rule.id == rule_id)
            .unwrap()
            .owner_api_key_id
    );
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

async fn verify_synthetic_monitoring(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    other: &HeaderMap,
) {
    let Json(channels) = list_notification_channels(State(state.clone()), headers.clone())
        .await
        .unwrap();
    let spec=serde_json::from_value::<crate::synthetic::CheckSpec>(serde_json::json!({"name":"API health","kind":"http","target":"https://example.com/health","channels":[channels[0].id]})).unwrap();
    let _ = super::synthetic::create(State(state.clone()), headers.clone(), Json(spec))
        .await
        .unwrap();
    let Json(checks) = super::synthetic::list(State(state.clone()), headers.clone())
        .await
        .unwrap();
    let id = checks[0]["id"].as_str().unwrap();
    let agent = checks[0]["agent_id"].as_str().unwrap();
    assert!(super::synthetic::history(
        State(state.clone()),
        other.clone(),
        Path(id.into()),
        Query(DeliveryQuery { limit: None })
    )
    .await
    .is_err());
    assert!(super::synthetic::set_enabled(
        State(state.clone()),
        other.clone(),
        Path(id.into()),
        Json(SetEnabledRequest { enabled: false })
    )
    .await
    .is_err());
    assert!(
        super::synthetic::delete(State(state.clone()), other.clone(), Path(id.into()))
            .await
            .is_err()
    );
    let db = database(state).unwrap();
    let mut result = crate::synthetic::ProbeResult {
        checked_at: Utc::now(),
        success: false,
        response_time_ms: 12.0,
        status_code: Some(503),
        content_matched: None,
        resolved_addresses: vec!["93.184.215.14".into()],
        tls_expires_at: None,
        tls_days_remaining: None,
        error: Some("HTTP returned 503; expected 200".into()),
    };
    for attempt in 1..=4 {
        let lease = format!("test-{attempt}");
        sqlx::query("UPDATE synthetic_checks SET lease_token=$2 WHERE id=$1")
            .bind(id)
            .bind(&lease)
            .execute(db.pool())
            .await
            .unwrap();
        crate::synthetic::finish(state, id, &lease, &result)
            .await
            .unwrap();
        crate::synthetic::finish(state, id, &lease, &result)
            .await
            .unwrap();
    }
    let Json(history) = super::synthetic::history(
        State(state.clone()),
        headers.clone(),
        Path(id.into()),
        Query(DeliveryQuery { limit: None }),
    )
    .await
    .unwrap();
    assert_eq!(history.len(), 4, "stale leases duplicated history");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM alerts WHERE agent_id=$1")
        .bind(agent)
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(
        count, 1,
        "consecutive failures produced duplicate incidents"
    );
    let Json(alerts) = list_alerts(State(state.clone()), headers.clone())
        .await
        .unwrap();
    let alert = alerts
        .active_alerts
        .iter()
        .find(|alert| alert.agent_id == agent)
        .unwrap();
    let alert_id = alert.id.clone();
    let Json(other_alerts) = list_alerts(State(state.clone()), other.clone())
        .await
        .unwrap();
    assert!(!other_alerts
        .active_alerts
        .iter()
        .any(|alert| alert.id == alert_id));
    result.success = true;
    result.error = None;
    result.status_code = Some(200);
    sqlx::query("UPDATE synthetic_checks SET lease_token='recovery' WHERE id=$1")
        .bind(id)
        .execute(db.pool())
        .await
        .unwrap();
    crate::synthetic::finish(state, id, "recovery", &result)
        .await
        .unwrap();
    let recovered = db.get_alert(&alert_id).await.unwrap().unwrap();
    assert_eq!(recovered.state, crate::alerts::AlertState::Resolved);
    let delivery_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM notification_deliveries WHERE alert_id=$1")
            .bind(&alert_id)
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(
        delivery_count, 2,
        "firing/recovery outbox was not atomic and exactly once"
    );
    let Json(timeline) = list_incident_timeline(
        State(state.clone()),
        headers.clone(),
        Query(IncidentQuery {
            agent_id: Some(agent.into()),
            limit: None,
        }),
    )
    .await
    .unwrap();
    assert!(timeline.iter().any(|event| event.event_type == "recovery"));
    let _ = super::synthetic::delete(State(state.clone()), headers.clone(), Path(id.into()))
        .await
        .unwrap();
    let Json(history) = super::synthetic::history(
        State(state.clone()),
        headers.clone(),
        Path(id.into()),
        Query(DeliveryQuery { limit: None }),
    )
    .await
    .unwrap();
    assert_eq!(
        history.len(),
        5,
        "deletion discarded history instead of retaining it"
    );
}

async fn verify_enterprise_alerting(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    other_headers: &HeaderMap,
    admin_headers: &HeaderMap,
    payload: &MetricsPayload,
) -> String {
    let owner = require_tenant(state, headers).await.unwrap();
    let other_owner = require_tenant(state, other_headers).await.unwrap();
    let other_agent = format!("other-{}", payload.agent_id);
    let db = database(state).unwrap();
    db.register_agent(&other_agent, Some(other_owner), None, None, None)
        .await
        .unwrap();
    state.store.insert_metrics(payload.clone());
    let mut other_payload = payload.clone();
    other_payload.agent_id = other_agent.clone();
    state.store.insert_metrics(other_payload);
    let rule_request = || {
        serde_json::from_value::<crate::alerts::CreateAlertRuleRequest>(serde_json::json!({
            "name": "Enterprise CPU", "metric": "cpu_usage", "condition": "greater_than",
            "threshold": 10, "duration_seconds": 0, "severity": "warning", "channels": [],
            "cooldown_seconds": 300
        }))
        .unwrap()
    };
    assert!(create_alert_rule(
        State(state.clone()),
        admin_headers.clone(),
        Json(rule_request())
    )
    .await
    .is_err());
    let Json(rule) = create_alert_rule(State(state.clone()), headers.clone(), Json(rule_request()))
        .await
        .unwrap();
    assert_eq!(rule.owner_api_key_id, Some(owner));
    let mut request = rule_request();
    request.agent_filter = Some(other_agent.clone());
    assert!(
        create_alert_rule(State(state.clone()), headers.clone(), Json(request))
            .await
            .is_err()
    );
    assert!(get_alert_rule(
        State(state.clone()),
        other_headers.clone(),
        Path(rule.id.clone())
    )
    .await
    .is_err());
    let update = || {
        serde_json::from_value::<crate::alerts::UpdateAlertRuleRequest>(
            serde_json::json!({"threshold": 20}),
        )
        .unwrap()
    };
    let Json(updated) = update_alert_rule(
        State(state.clone()),
        headers.clone(),
        Path(rule.id.clone()),
        Json(update()),
    )
    .await
    .unwrap();
    assert_eq!(updated.threshold, 20.0);
    assert_eq!(
        db.get_alert_rule(&rule.id)
            .await
            .unwrap()
            .unwrap()
            .threshold,
        20.0
    );
    let Json(toggled) =
        toggle_alert_rule(State(state.clone()), headers.clone(), Path(rule.id.clone()))
            .await
            .unwrap();
    assert!(!toggled.enabled);
    assert!(!db.get_alert_rule(&rule.id).await.unwrap().unwrap().enabled);
    let _ = toggle_alert_rule(State(state.clone()), headers.clone(), Path(rule.id.clone()))
        .await
        .unwrap();
    assert!(update_alert_rule(
        State(state.clone()),
        other_headers.clone(),
        Path(rule.id.clone()),
        Json(update())
    )
    .await
    .is_err());
    assert!(toggle_alert_rule(
        State(state.clone()),
        other_headers.clone(),
        Path(rule.id.clone())
    )
    .await
    .is_err());
    assert!(delete_alert_rule(
        State(state.clone()),
        other_headers.clone(),
        Path(rule.id.clone())
    )
    .await
    .is_err());
    let Json(other_rules) = list_alert_rules(State(state.clone()), other_headers.clone())
        .await
        .unwrap();
    assert!(other_rules.rules.is_empty());
    let Json(effective) = list_effective_alert_rules(
        State(state.clone()),
        other_headers.clone(),
        Query(EffectiveRulesQuery {
            agent_id: other_agent.clone(),
        }),
    )
    .await
    .unwrap();
    assert!(
        effective.rules.is_empty(),
        "tenant-wide rule leaked into another enterprise's chart thresholds"
    );
    let transitions = state.alert_manager.evaluate_alerts(300).await;
    let transition = transitions
        .iter()
        .find(|transition| transition.alert.rule_id == rule.id)
        .unwrap();
    assert_eq!(transition.alert.agent_id, payload.agent_id);
    assert!(!transitions
        .iter()
        .any(|transition| transition.alert.rule_id == rule.id
            && transition.alert.agent_id == other_agent));

    let channel_request = || {
        serde_json::from_value::<crate::alerts::CreateNotificationChannelRequest>(serde_json::json!({
        "name": "Enterprise webhook", "channel_type": "generic_webhook", "webhook_url": "https://example.com/hook"
    })).unwrap()
    };
    let Json(channel) = create_notification_channel(
        State(state.clone()),
        headers.clone(),
        Json(channel_request()),
    )
    .await
    .unwrap();
    let Json(other_channel) = create_notification_channel(
        State(state.clone()),
        other_headers.clone(),
        Json(channel_request()),
    )
    .await
    .unwrap();
    assert!(test_notification_channel(
        State(state.clone()),
        other_headers.clone(),
        Path(channel.id.clone())
    )
    .await
    .is_err());
    assert!(delete_notification_channel(
        State(state.clone()),
        other_headers.clone(),
        Path(channel.id.clone())
    )
    .await
    .is_err());
    let mut request = rule_request();
    request.channels = vec![other_channel.id.clone()];
    assert!(
        create_alert_rule(State(state.clone()), headers.clone(), Json(request))
            .await
            .is_err()
    );
    let Json(channels) = list_notification_channels(State(state.clone()), headers.clone())
        .await
        .unwrap();
    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].id, channel.id);
    let dispatcher = crate::alerts::notifications::NotificationDispatcher::new(db.clone()).unwrap();
    let mut offline = transition.clone();
    offline.alert.rule_id = "system-agent-offline".into();
    offline.channel_ids.clear();
    assert_eq!(
        dispatcher.enqueue_transition(&offline).await.unwrap(),
        1,
        "offline alert routed across tenant boundaries"
    );
    let Json(deliveries) = list_notification_deliveries(
        State(state.clone()),
        headers.clone(),
        Query(DeliveryQuery { limit: Some(100) }),
    )
    .await
    .unwrap();
    assert!(deliveries
        .iter()
        .any(|delivery| delivery.channel_id.as_deref() == Some(&channel.id)));
    let Json(other_deliveries) = list_notification_deliveries(
        State(state.clone()),
        other_headers.clone(),
        Query(DeliveryQuery { limit: Some(100) }),
    )
    .await
    .unwrap();
    assert!(other_deliveries.is_empty());
    let silence_request = serde_json::from_value::<crate::alerts::CreateAlertSilenceRequest>(serde_json::json!({
        "name": "Tenant-wide maintenance", "starts_at": Utc::now() - chrono::Duration::seconds(1),
        "ends_at": Utc::now() + chrono::Duration::hours(1), "created_by": "forged-admin"
    })).unwrap();
    let Json(silence) =
        create_alert_silence(State(state.clone()), headers.clone(), Json(silence_request))
            .await
            .unwrap();
    assert_eq!(silence.created_by, format!("enterprise-key-{owner}"));
    assert!(db
        .is_alert_silenced("system-agent-offline", &payload.agent_id)
        .await
        .unwrap());
    assert!(!db
        .is_alert_silenced("system-agent-offline", &other_agent)
        .await
        .unwrap());
    assert!(delete_alert_silence(
        State(state.clone()),
        other_headers.clone(),
        Path(silence.id.clone())
    )
    .await
    .is_err());
    delete_alert_silence(State(state.clone()), headers.clone(), Path(silence.id))
        .await
        .unwrap();
    rule.id
}
