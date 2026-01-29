use super::*;
use std::env;

#[test]
fn test_default_config() {
    let config = Config::default();

    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.storage.max_points_per_metric, 10_000);
    assert!(config.database.enabled);
    assert!(config.auth.enabled);
    assert!(!config.auth.require_api_key);
}

#[test]
fn test_bind_address() {
    let config = Config::default();
    assert_eq!(config.bind_address(), "0.0.0.0:8080");

    let mut config = Config::default();
    config.server.host = "127.0.0.1".to_string();
    config.server.port = 3000;
    assert_eq!(config.bind_address(), "127.0.0.1:3000");
}

#[test]
fn test_env_override_server_config() {
    env::set_var("SERVER_HOST", "127.0.0.1");
    env::set_var("SERVER_PORT", "9090");

    let mut config = Config::default();
    config.apply_env_overrides();

    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 9090);

    env::remove_var("SERVER_HOST");
    env::remove_var("SERVER_PORT");
}

#[test]
fn test_env_override_database_config() {
    env::set_var("DATABASE_ENABLED", "false");
    env::set_var("DATABASE_URL", "sqlite:test.db");

    let mut config = Config::default();
    config.apply_env_overrides();

    assert!(!config.database.enabled);
    assert_eq!(config.database.url, "sqlite:test.db");

    env::remove_var("DATABASE_ENABLED");
    env::remove_var("DATABASE_URL");
}

#[test]
fn test_env_override_auth_config() {
    env::set_var("AUTH_ENABLED", "false");
    env::set_var("AUTH_REQUIRE_API_KEY", "true");

    let mut config = Config::default();
    config.apply_env_overrides();

    assert!(!config.auth.enabled);
    assert!(config.auth.require_api_key);

    env::remove_var("AUTH_ENABLED");
    env::remove_var("AUTH_REQUIRE_API_KEY");
}

#[test]
fn test_env_override_retention_config() {
    env::set_var("RETENTION_RAW_METRICS_HOURS", "48");
    env::set_var("RETENTION_ALERTS_DAYS", "7");
    env::set_var("RETENTION_CLEANUP_INTERVAL_HOURS", "6");

    let mut config = Config::default();
    config.apply_env_overrides();

    assert_eq!(config.retention.raw_metrics_hours, 48);
    assert_eq!(config.retention.alerts_days, 7);
    assert_eq!(config.retention.cleanup_interval_hours, 6);

    env::remove_var("RETENTION_RAW_METRICS_HOURS");
    env::remove_var("RETENTION_ALERTS_DAYS");
    env::remove_var("RETENTION_CLEANUP_INTERVAL_HOURS");
}

#[test]
fn test_env_override_storage_config() {
    env::set_var("STORAGE_MAX_POINTS", "5000");

    let mut config = Config::default();
    config.apply_env_overrides();

    assert_eq!(config.storage.max_points_per_metric, 5000);

    env::remove_var("STORAGE_MAX_POINTS");
}

#[test]
fn test_env_override_invalid_values() {
    // Test that invalid values are ignored (use fresh config to avoid test pollution)
    let initial_config = Config::default();
    let initial_port = initial_config.server.port;
    let initial_retention = initial_config.retention.raw_metrics_hours;

    // Set invalid values
    env::set_var("SERVER_PORT", "invalid");
    env::set_var("RETENTION_RAW_METRICS_HOURS", "not_a_number");

    let mut config = Config::default();
    config.apply_env_overrides();

    // Should keep default values when parsing fails
    // Note: We compare with initial defaults, not hardcoded values
    assert_eq!(config.server.port, initial_port);
    assert_eq!(config.retention.raw_metrics_hours, initial_retention);

    env::remove_var("SERVER_PORT");
    env::remove_var("RETENTION_RAW_METRICS_HOURS");
}

#[test]
fn test_default_configs() {
    let db_config = DatabaseConfig::default();
    assert!(db_config.enabled);

    let retention_config = RetentionConfig::default();
    assert_eq!(retention_config.raw_metrics_hours, 24);
    assert_eq!(retention_config.alerts_days, 90);
    assert_eq!(retention_config.cleanup_interval_hours, 1); // Every hour

    let auth_config = AuthConfig::default();
    assert!(auth_config.enabled);
    assert!(!auth_config.require_api_key);
}
