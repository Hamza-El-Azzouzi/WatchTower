mod aggregation;
mod alerts;
mod api;
mod auth;
mod config;
mod db;
mod middleware;
mod storage;
mod websocket;

use anyhow::Result;
use axum::{
    extract::DefaultBodyLimit,
    http::{header, HeaderName, HeaderValue, Method},
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

    if config.auth.enabled {
        auth::admin::validate_configuration()?;
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

    // Initialize alert manager (requires database for persistence)
    let alert_manager = if let Some(ref db) = database {
        Arc::new(alerts::manager::AlertManager::new(store.clone(), db.clone()).await)
    } else {
        error!("Database is required for AlertManager. Please configure database connection.");
        std::process::exit(1);
    };
    info!("Initialized alert manager with database persistence");

    // Initialize WebSocket manager
    let ws_manager = Arc::new(websocket::WebSocketManager::new());
    info!("Initialized WebSocket manager");

    // Start alert evaluation loop
    let alert_manager_clone = alert_manager.clone();
    let alert_check_interval = config.alerts.check_interval_seconds;
    let ws_manager_clone = ws_manager.clone();
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(alert_check_interval));
        loop {
            interval.tick().await;
            let triggered = alert_manager_clone.evaluate_alerts().await;
            if !triggered.is_empty() {
                info!("Evaluated alerts: {} triggered", triggered.len());
                // Broadcast alerts via WebSocket
                for alert in &triggered {
                    ws_manager_clone.broadcast_alert(websocket::WsMessage::Alert {
                        alert_id: alert.id.to_string(),
                        agent_id: alert.agent_id.clone(),
                        severity: format!("{:?}", alert.severity).to_lowercase(),
                        message: alert.message.clone(),
                        state: format!("{:?}", alert.state).to_lowercase(),
                    });
                }
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

    // Start background jobs if database is enabled
    if let Some(db) = database.clone() {
        // Start aggregation and cleanup jobs
        aggregation::start_background_jobs(
            db,
            config.aggregation.minute_interval_hours,
            config.aggregation.hour_interval_hours,
            config.retention.cleanup_interval_hours,
            config.retention.raw_metrics_hours,
            config.retention.minute_aggregates_days,
            config.retention.hour_aggregates_days,
            config.retention.alerts_days,
        );
        info!("Started all background aggregation and cleanup jobs");
    }

    // Initialize authentication service if database is enabled
    let auth_service = if let Some(db) = &database {
        let service = auth::AuthService::new(db.pool().clone());
        if config.auth.enabled {
            service.ensure_bootstrap_admin().await?;
        }
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
        ws_manager.clone(),
    ));

    // Build router with protected routes (metrics/logs ingestion + dashboard read endpoints)
    let protected_routes = Router::new()
        // Agent write endpoints (metrics/logs ingestion)
        .route("/api/v1/metrics", post(api::ingest_metrics))
        .route("/api/v1/logs", post(api::ingest_logs))
        // Dashboard read endpoints (require API key)
        .route("/api/v1/metrics", get(api::query_metrics))
        .route("/api/v1/metrics/latest", get(api::get_latest_metrics))
        .route("/api/v1/logs", get(api::query_logs))
        .route("/api/v1/agents", get(api::list_agents))
        .route("/api/v1/agents/:agent_id", get(api::get_agent))
        .route("/api/v1/stats", get(api::get_stats))
        // Alert endpoints (require API key)
        .route("/api/v1/alerts", get(api::list_alerts))
        .route(
            "/api/v1/alerts/:alert_id/acknowledge",
            post(api::acknowledge_alert),
        )
        // Alert Rule endpoints (require API key)
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
        );

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
        // Health check (public)
        .route("/health", get(api::health_check))
        .route("/api/v1/health", get(api::system_health))
        // API Key validation endpoint (public - used by login)
        .route("/api/v1/auth/validate", get(api::validate_api_key))
        // API Key management (admin-only - checked in handlers)
        .route("/api/v1/auth/keys", post(api::create_api_key))
        .route("/api/v1/auth/keys", get(api::list_api_keys))
        .route(
            "/api/v1/auth/keys/:key_id",
            axum::routing::delete(api::revoke_api_key),
        )
        // Agent request endpoints (public submit, admin list/update)
        .route("/api/v1/requests/agents", post(api::submit_agent_request))
        .route("/api/v1/requests/agents", get(api::list_agent_requests))
        .route(
            "/api/v1/requests/agents/:request_id/status",
            axum::routing::put(api::update_agent_request_status),
        );

    let allowed_origin = std::env::var("CORS_ALLOWED_ORIGIN")
        .unwrap_or_else(|_| "http://localhost:3000".to_string())
        .parse::<HeaderValue>()?;
    let cors = CorsLayer::new()
        .allow_origin(allowed_origin)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            HeaderName::from_static("x-api-key"),
            HeaderName::from_static("x-admin-token"),
        ]);

    let app = Router::new()
        .merge(protected_routes)
        .merge(public_routes)
        // WebSocket endpoints (real-time updates - require auth)
        .route("/api/v1/ws/metrics", get(websocket::ws_metrics_handler))
        .route("/api/v1/ws/logs", get(websocket::ws_logs_handler))
        .route("/api/v1/ws/alerts", get(websocket::ws_alerts_handler))
        // Shared state
        .with_state(state)
        // Middleware
        .layer(cors)
        // Bound request buffering even when authentication is disabled.
        .layer(DefaultBodyLimit::max(512 * 1024))
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
