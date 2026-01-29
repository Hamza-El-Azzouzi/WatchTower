use crate::db::Database;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info};

// Background job to aggregate raw metrics to 1-minute aggregates
pub async fn run_minute_aggregation_job(db: Arc<Database>, interval_hours: u64) {
    let mut ticker = interval(Duration::from_secs(interval_hours * 3600));

    info!(
        "Starting 1-minute aggregation job (runs every {} hours)",
        interval_hours
    );

    loop {
        ticker.tick().await;

        info!("Running 1-minute aggregation");
        match db.aggregate_metrics_to_1min(1).await {
            Ok(count) => info!("Aggregated {} metric points to 1-minute intervals", count),
            Err(e) => error!("Failed to run 1-minute aggregation: {}", e),
        }
    }
}

// Background job to aggregate 1-minute to 1-hour aggregates
pub async fn run_hour_aggregation_job(db: Arc<Database>, interval_hours: u64) {
    let mut ticker = interval(Duration::from_secs(interval_hours * 3600));

    info!(
        "Starting 1-hour aggregation job (runs every {} hours)",
        interval_hours
    );

    loop {
        ticker.tick().await;

        info!("Running 1-hour aggregation");
        match db.aggregate_metrics_to_1hour(1).await {
            Ok(count) => info!("Aggregated {} metric points to 1-hour intervals", count),
            Err(e) => error!("Failed to run 1-hour aggregation: {}", e),
        }
    }
}

// Background job to clean up old data
pub async fn run_cleanup_job(
    db: Arc<Database>,
    interval_hours: u64,
    raw_retention_hours: i64,
    minute_retention_days: i64,
    hour_retention_days: i64,
    alerts_retention_days: i64,
) {
    let mut ticker = interval(Duration::from_secs(interval_hours * 3600));

    info!("Starting cleanup job (runs every {} hours)", interval_hours);
    info!(
        "Retention policy: raw={}h, 1min={}d, 1hour={}d, alerts={}d",
        raw_retention_hours, minute_retention_days, hour_retention_days, alerts_retention_days
    );

    loop {
        ticker.tick().await;

        info!("Running data cleanup");

        // Clean up old raw metrics
        match db.cleanup_old_raw_metrics(raw_retention_hours).await {
            Ok(count) => info!("Deleted {} old raw metrics", count),
            Err(e) => error!("Failed to clean up raw metrics: {}", e),
        }

        // Clean up old 1-minute aggregates
        match db
            .cleanup_old_minute_aggregates(minute_retention_days)
            .await
        {
            Ok(count) => info!("Deleted {} old 1-minute aggregates", count),
            Err(e) => error!("Failed to clean up 1-minute aggregates: {}", e),
        }

        // Clean up old 1-hour aggregates
        match db.cleanup_old_hour_aggregates(hour_retention_days).await {
            Ok(count) => info!("Deleted {} old 1-hour aggregates", count),
            Err(e) => error!("Failed to clean up 1-hour aggregates: {}", e),
        }

        // Clean up old resolved alerts
        match db.cleanup_old_alerts(alerts_retention_days).await {
            Ok(count) => info!("Deleted {} old resolved alerts", count),
            Err(e) => error!("Failed to clean up alerts: {}", e),
        }

        info!("Data cleanup completed");
    }
}

// Start all background aggregation and cleanup jobs
#[allow(clippy::too_many_arguments)]
pub fn start_background_jobs(
    db: Arc<Database>,
    minute_interval_hours: u64,
    hour_interval_hours: u64,
    cleanup_interval_hours: u64,
    raw_retention_hours: i64,
    minute_retention_days: i64,
    hour_retention_days: i64,
    alerts_retention_days: i64,
) {
    // Spawn 1-minute aggregation job
    let db_clone = db.clone();
    tokio::spawn(async move {
        run_minute_aggregation_job(db_clone, minute_interval_hours).await;
    });

    // Spawn 1-hour aggregation job
    let db_clone = db.clone();
    tokio::spawn(async move {
        run_hour_aggregation_job(db_clone, hour_interval_hours).await;
    });

    // Spawn cleanup job
    tokio::spawn(async move {
        run_cleanup_job(
            db,
            cleanup_interval_hours,
            raw_retention_hours,
            minute_retention_days,
            hour_retention_days,
            alerts_retention_days,
        )
        .await;
    });

    info!("All background jobs started successfully");
}
