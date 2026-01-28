mod alerts;
mod api;
mod auth;
mod config;
mod db;
mod middleware;
mod storage;

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info};

use api::AppState;
use config::Config;
use storage::TimeSeriesStore;

#[derive(Parser, Debug)]
#[command(name = "monitor-server")]
#[command(author = "DevOps Monitoring System")]
#[command(version = "0.1.0")]
#[command(about = "Central monitoring server for metric aggregation", long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Server host (overrides config file)
    #[arg(long)]
    host: Option<String>,

    /// Server port (overrides config file)
    #[arg(short, long)]
    port: Option<u16>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .with_target(false)
        .with_thread_ids(false)
        .init();

    // Load configuration
    let mut config = if let Some(config_path) = args.config {
        info!("Loading configuration from: {:?}", config_path);
        match Config::from_file(&config_path) {
            Ok(cfg) => cfg,
            Err(e) => {
                error!("Failed to load configuration: {}", e);
                error!("Using default configuration");
                Config::default()
            }
        }
    } else {
        info!("No configuration file specified, using defaults");
        Config::default()
    };

    // Override with CLI arguments
    if let Some(host) = args.host {
        info!("Overriding host to: {}", host);
        config.server.host = host;
    }
    if let Some(port) = args.port {
        info!("Overriding port to: {}", port);
        config.server.port = port;
    }

    // Initialize database if enabled
    let database = if config.database.enabled {
        info!("Initializing database at: {}", config.database.url);
        match db::Database::new(&config.database.url).await {
            Ok(db) => {
                info!("Database initialized successfully");
                Some(Arc::new(db))
            }
            Err(e) => {
                error!("Failed to initialize database: {}", e);
                error!("Continuing with in-memory storage only");
                None
            }
        }
    } else {
        info!("Database disabled, using in-memory storage only");
        None
    };

    // Initialize storage
    let store = TimeSeriesStore::new(config.storage.max_points_per_metric);
    info!(
        "Initialized time-series storage with max {} points per metric",
        config.storage.max_points_per_metric
    );

    // Initialize alert manager
    let alert_manager = Arc::new(alerts::manager::AlertManager::new(store.clone()));
    info!("Initialized alert manager");

    // Start alert evaluation loop
    let alert_manager_clone = alert_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            let triggered = alert_manager_clone.evaluate_alerts().await;
            if !triggered.is_empty() {
                info!("Evaluated alerts: {} triggered", triggered.len());
                // TODO: Send notifications
            }
        }
    });

    // Cleanup old resolved alerts every hour
    let alert_manager_cleanup = alert_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            alert_manager_cleanup.cleanup_old_alerts(24).await;
            info!("Cleaned up old resolved alerts");
        }
    });

    // Start database cleanup task if database is enabled
    if let Some(db) = database.clone() {
        let metrics_retention = config.retention.metrics_hours;
        let alerts_retention = config.retention.alerts_days;
        let cleanup_interval = config.retention.cleanup_interval_hours;

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(cleanup_interval * 3600));
            loop {
                interval.tick().await;
                info!("Starting database cleanup");

                match db.cleanup_old_metrics(metrics_retention).await {
                    Ok(count) => info!("Cleaned up {} old metrics", count),
                    Err(e) => error!("Failed to cleanup metrics: {}", e),
                }

                match db.cleanup_old_alerts(alerts_retention).await {
                    Ok(count) => info!("Cleaned up {} old alerts", count),
                    Err(e) => error!("Failed to cleanup alerts: {}", e),
                }
            }
        });
        info!(
            "Started database cleanup task (metrics: {}h, alerts: {}d, interval: {}h)",
            metrics_retention, alerts_retention, cleanup_interval
        );
    }

    // Initialize authentication service if database is enabled
    let auth_service = if let Some(db) = &database {
        let service = auth::AuthService::new(db.pool().clone());
        info!("Initialized authentication service");
        Some(Arc::new(service))
    } else {
        info!("Authentication disabled (database not enabled)");
        None
    };

    // Clone auth_service for middleware before moving into AppState
    let auth_service_for_middleware = auth_service.clone();

    // Create shared application state
    let state = Arc::new(AppState::new(
        store,
        alert_manager,
        database.clone(),
        auth_service,
    ));

    // Build router with protected routes (only POST metrics - for agents sending data)
    let protected_routes = Router::new().route("/api/v1/metrics", post(api::ingest_metrics));

    // Apply auth middleware only if auth is enabled and required
    let protected_routes = if config.auth.enabled && config.auth.require_api_key {
        if let Some(auth_svc) = auth_service_for_middleware {
            info!("Authentication ENABLED - API keys required for agent metrics submission");
            protected_routes.layer(axum::middleware::from_fn_with_state(
                auth_svc,
                middleware::auth::auth_middleware,
            ))
        } else {
            info!("Authentication DISABLED - no auth service available");
            protected_routes
        }
    } else {
        info!("Authentication DISABLED - metrics submission is open");
        protected_routes
    };

    // Public routes (no authentication required)
    let public_routes = Router::new()
        // Admin authentication endpoints (must be public)
        .route("/api/v1/admin/login", post(api::admin_login))
        .route("/api/v1/admin/validate", get(api::admin_validate))
        // Admin management endpoints (require admin token)
        .route(
            "/api/v1/admin/change-password",
            post(api::admin_change_password),
        )
        .route("/api/v1/admin/users", post(api::admin_create_user))
        .route("/api/v1/admin/users", get(api::admin_list_users))
        .route(
            "/api/v1/admin/users/:username/deactivate",
            post(api::admin_deactivate_user),
        )
        .route(
            "/api/v1/admin/users/:username/activate",
            post(api::admin_activate_user),
        )
        // Health check
        .route("/health", get(api::health_check))
        .route("/api/v1/health", get(api::system_health))
        // API Key management (no auth required for creating the first key)
        .route("/api/v1/auth/keys", post(api::create_api_key))
        .route("/api/v1/auth/keys", get(api::list_api_keys))
        .route(
            "/api/v1/auth/keys/:key_id",
            axum::routing::delete(api::revoke_api_key),
        );

    let app = Router::new()
        .merge(protected_routes)
        .merge(public_routes)
        // Read-only metrics endpoints (unprotected - used by dashboard)
        .route("/api/v1/metrics", get(api::query_metrics))
        .route("/api/v1/metrics/latest", get(api::get_latest_metrics))
        // Agent endpoints (unprotected)
        .route("/api/v1/agents", get(api::list_agents))
        .route("/api/v1/agents/:agent_id", get(api::get_agent))
        // Alert Rule endpoints
        .route("/api/v1/alert-rules", post(api::create_alert_rule))
        .route("/api/v1/alert-rules", get(api::list_alert_rules))
        .route("/api/v1/alert-rules/:rule_id", get(api::get_alert_rule))
        .route(
            "/api/v1/alert-rules/:rule_id",
            axum::routing::put(api::update_alert_rule),
        )
        .route(
            "/api/v1/alert-rules/:rule_id",
            axum::routing::delete(api::delete_alert_rule),
        )
        .route(
            "/api/v1/alert-rules/:rule_id/toggle",
            post(api::toggle_alert_rule),
        )
        // Alert endpoints
        .route("/api/v1/alerts", get(api::list_alerts))
        .route("/api/v1/alerts/:alert_id", get(api::get_alert))
        .route(
            "/api/v1/alerts/:alert_id/acknowledge",
            post(api::acknowledge_alert),
        )
        // Stats endpoints
        .route("/api/v1/stats", get(api::get_stats))
        .route("/api/v1/db/stats", get(api::get_database_stats))
        // Shared state
        .with_state(state)
        // Middleware
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let bind_addr = config.bind_address();
    info!("Starting server on {}", bind_addr);
    info!("");
    info!("API Endpoints:");
    info!("  POST   /api/v1/metrics         - Ingest metrics from agents");
    info!("  GET    /api/v1/metrics         - Query historical metrics");
    info!("  GET    /api/v1/metrics/latest  - Get latest metrics");
    info!("  GET    /api/v1/agents          - List all agents");
    info!("  GET    /api/v1/agents/:id      - Get agent details");
    info!("  GET    /api/v1/stats           - Storage statistics");
    info!("  GET    /health                 - Health check");
    info!("");

    // Start server
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
