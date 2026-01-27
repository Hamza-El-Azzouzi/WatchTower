mod api;
mod config;
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

    // Initialize storage
    let store = TimeSeriesStore::new(config.storage.max_points_per_metric);
    info!(
        "Initialized time-series storage with max {} points per metric",
        config.storage.max_points_per_metric
    );

    // Create shared application state
    let state = Arc::new(AppState::new(store));

    // Build router
    let app = Router::new()
        // Metrics endpoints
        .route("/api/v1/metrics", post(api::ingest_metrics))
        .route("/api/v1/metrics", get(api::query_metrics))
        .route("/api/v1/metrics/latest", get(api::get_latest_metrics))
        // Agent endpoints
        .route("/api/v1/agents", get(api::list_agents))
        .route("/api/v1/agents/:agent_id", get(api::get_agent))
        // Stats endpoint
        .route("/api/v1/stats", get(api::get_stats))
        // Health check
        .route("/health", get(api::health_check))
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
