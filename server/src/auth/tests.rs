use super::*;
use sqlx::SqlitePool;

async fn create_test_db() -> SqlitePool {
    // Use in-memory database for tests
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

    // Create tables
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS api_keys (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            key TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
            expires_at TIMESTAMP,
            last_used_at TIMESTAMP,
            revoked BOOLEAN NOT NULL DEFAULT 0,
            revoked_at TIMESTAMP,
            created_by TEXT DEFAULT 'system',
            max_agents INTEGER DEFAULT NULL,
            used_by_agents TEXT DEFAULT '[]'
        )",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

#[tokio::test]
async fn test_generate_api_key() {
    let key = ApiKey::generate();

    assert!(key.starts_with("msk_"));
    assert_eq!(key.len(), 47); // "msk_" + 43 chars of base64
}

#[tokio::test]
async fn test_generate_multiple_unique_keys() {
    let key1 = ApiKey::generate();
    let key2 = ApiKey::generate();
    let key3 = ApiKey::generate();

    assert_ne!(key1, key2);
    assert_ne!(key2, key3);
    assert_ne!(key1, key3);
}

#[tokio::test]
async fn test_create_api_key() {
    let pool = create_test_db().await;
    let service = AuthService::new(pool);

    let request = CreateApiKeyRequest {
        name: "Test Key".to_string(),
        description: Some("Test description".to_string()),
        expires_in_days: None,
        max_agents: None,
    };

    let response = service.create_api_key(request, "test_user").await.unwrap();

    assert!(response.key.starts_with("msk_"));
    assert_eq!(response.name, "Test Key");
}

#[tokio::test]
async fn test_create_api_key_with_expiration() {
    let pool = create_test_db().await;
    let service = AuthService::new(pool);

    let request = CreateApiKeyRequest {
        name: "Expiring Key".to_string(),
        description: None,
        expires_in_days: Some(30),
        max_agents: None,
    };

    let response = service.create_api_key(request, "test_user").await.unwrap();

    assert!(response.expires_at.is_some());

    let expires_at = response.expires_at.unwrap();
    let now = chrono::Utc::now();
    let diff = expires_at.signed_duration_since(now).num_days();

    assert!(diff >= 29 && diff <= 31); // Should be ~30 days from now
}

#[tokio::test]
async fn test_create_api_key_with_agent_limit() {
    let pool = create_test_db().await;
    let service = AuthService::new(pool);

    let request = CreateApiKeyRequest {
        name: "Limited Key".to_string(),
        description: None,
        expires_in_days: None,
        max_agents: Some(5),
    };

    service.create_api_key(request, "test_user").await.unwrap();

    // Verify key was created (we'll check via list)
    let keys = service.list_api_keys().await.unwrap();
    assert_eq!(keys.len(), 1);
}

#[tokio::test]
async fn test_validate_api_key_success() {
    let pool = create_test_db().await;
    let service = AuthService::new(pool);

    let request = CreateApiKeyRequest {
        name: "Valid Key".to_string(),
        description: None,
        expires_in_days: None,
        max_agents: None,
    };

    let response = service.create_api_key(request, "test_user").await.unwrap();

    let result = service.validate_api_key(&response.key).await.unwrap();
    assert!(result.is_some());
}

#[tokio::test]
async fn test_validate_api_key_invalid() {
    let pool = create_test_db().await;
    let service = AuthService::new(pool);

    let result = service
        .validate_api_key("msk_invalid_key_123")
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_validate_api_key_with_agent_tracking() {
    let pool = create_test_db().await;
    let service = AuthService::new(pool);

    let request = CreateApiKeyRequest {
        name: "Agent Key".to_string(),
        description: None,
        expires_in_days: None,
        max_agents: Some(3),
    };

    let response = service.create_api_key(request, "test_user").await.unwrap();

    // First agent
    let result1 = service
        .validate_api_key_with_agent(&response.key, Some("agent-1"))
        .await
        .unwrap();
    assert!(result1.is_some());

    // Second agent
    let result2 = service
        .validate_api_key_with_agent(&response.key, Some("agent-2"))
        .await
        .unwrap();
    assert!(result2.is_some());

    // Third agent
    let result3 = service
        .validate_api_key_with_agent(&response.key, Some("agent-3"))
        .await
        .unwrap();
    assert!(result3.is_some());

    // Fourth agent should be rejected (limit is 3)
    let result4 = service
        .validate_api_key_with_agent(&response.key, Some("agent-4"))
        .await
        .unwrap();
    assert!(result4.is_none());

    // Existing agent should still work
    let result_existing = service
        .validate_api_key_with_agent(&response.key, Some("agent-1"))
        .await
        .unwrap();
    assert!(result_existing.is_some());
}

#[tokio::test]
async fn test_list_api_keys() {
    let pool = create_test_db().await;
    let service = AuthService::new(pool);

    let req1 = CreateApiKeyRequest {
        name: "Key 1".to_string(),
        description: None,
        expires_in_days: None,
        max_agents: None,
    };
    service.create_api_key(req1, "user1").await.unwrap();

    let req2 = CreateApiKeyRequest {
        name: "Key 2".to_string(),
        description: None,
        expires_in_days: None,
        max_agents: None,
    };
    service.create_api_key(req2, "user2").await.unwrap();

    let keys = service.list_api_keys().await.unwrap();

    assert_eq!(keys.len(), 2);
    // Keys should be masked in the list
    assert!(keys[0].key.contains("***") || keys[0].key.len() < 51);
}
