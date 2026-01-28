mod collector;
mod config;
mod sender;

use anyhow::Result;
use chrono::Local;
use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

use collector::{
    cpu::CpuCollector, disk::DiskCollector, memory::MemoryCollector, network::NetworkCollector,
    SystemMetrics,
};
use config::Config;
use sender::{MetricsPayload, MetricsSender};

#[derive(Parser, Debug)]
#[command(name = "monitor-agent")]
#[command(author = "DevOps Monitoring System")]
#[command(version = "0.1.0")]
#[command(about = "System monitoring agent for collecting metrics", long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Collection interval in seconds (overrides config file)
    #[arg(short, long)]
    interval: Option<u64>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

struct MetricCollectors {
    cpu: CpuCollector,
    memory: MemoryCollector,
    disk: DiskCollector,
    network: NetworkCollector,
}

impl MetricCollectors {
    fn new() -> Self {
        Self {
            cpu: CpuCollector::new(),
            memory: MemoryCollector::new(),
            disk: DiskCollector::new(),
            network: NetworkCollector::new(),
        }
    }

    fn collect(&mut self, config: &Config) -> SystemMetrics {
        let cpu_percent = if config.metrics.collect_cpu {
            self.cpu.collect()
        } else {
            0.0
        };

        let (memory_used, memory_total, memory_percent) = if config.metrics.collect_memory {
            self.memory.collect()
        } else {
            (0, 0, 0.0)
        };

        let (disk_used, disk_total, disk_percent) = if config.metrics.collect_disk {
            self.disk.collect()
        } else {
            (0, 0, 0.0)
        };

        let (network_rx, network_tx) = if config.metrics.collect_network {
            self.network.collect()
        } else {
            (0, 0)
        };

        SystemMetrics {
            cpu_percent,
            memory_used_bytes: memory_used,
            memory_total_bytes: memory_total,
            memory_percent,
            disk_used_bytes: disk_used,
            disk_total_bytes: disk_total,
            disk_percent,
            network_rx_bytes: network_rx,
            network_tx_bytes: network_tx,
        }
    }
}

async fn run_agent(config: Config) -> Result<()> {
    info!("Starting monitoring agent: {}", config.agent.name);
    info!(
        "Collection interval: {} seconds",
        config.collection.interval_seconds
    );
    info!(
        "Metrics enabled - CPU: {}, Memory: {}, Disk: {}, Network: {}",
        config.metrics.collect_cpu,
        config.metrics.collect_memory,
        config.metrics.collect_disk,
        config.metrics.collect_network
    );

    // Initialize metrics sender if server is enabled
    let sender = if config.server.enabled {
        info!(
            "Server integration enabled - sending metrics to {}",
            config.server.url
        );
        Some(MetricsSender::new(
            config.server.url.clone(),
            config.server.retry_attempts,
            config.server.retry_delay_seconds,
        )?)
    } else {
        info!("Server integration disabled - metrics will only be displayed locally");
        None
    };

    let mut collectors = MetricCollectors::new();
    let interval = Duration::from_secs(config.collection.interval_seconds);
    let mut interval_timer = time::interval(interval);

    println!("\n{:-^80}", " Monitoring Agent Started ");
    println!(
        "Agent: {} | Interval: {}s | Server: {}",
        config.agent.name,
        config.collection.interval_seconds,
        if config.server.enabled {
            &config.server.url
        } else {
            "Disabled"
        }
    );
    println!();

    loop {
        interval_timer.tick().await;

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let metrics = collectors.collect(&config);

        // Display metrics locally
        print!("[{}] ", timestamp);
        metrics.display();

        // Send metrics to server if enabled
        if let Some(ref sender) = sender {
            let mut metrics_map = std::collections::HashMap::new();
            metrics_map.insert("cpu_usage".to_string(), metrics.cpu_percent as f64);
            metrics_map.insert("memory_usage".to_string(), metrics.memory_percent as f64);
            metrics_map.insert("disk_usage".to_string(), metrics.disk_percent as f64);
            metrics_map.insert(
                "network_rx_bytes".to_string(),
                metrics.network_rx_bytes as f64,
            );
            metrics_map.insert(
                "network_tx_bytes".to_string(),
                metrics.network_tx_bytes as f64,
            );

            let payload = MetricsPayload {
                agent_id: config.agent.name.clone(),
                timestamp: chrono::Utc::now(),
                metrics: metrics_map,
            };

            if let Err(e) = sender.send_metrics(&payload).await {
                warn!("Failed to send metrics to server: {}", e);
                // Continue despite send failure - we still display metrics locally
            }
        }
    }
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
        .with_file(false)
        .with_line_number(false)
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

    // Override interval if provided via CLI
    if let Some(interval) = args.interval {
        info!("Overriding collection interval to {} seconds", interval);
        config.collection.interval_seconds = interval;
    }

    // Run the agent
    if let Err(e) = run_agent(config).await {
        error!("Agent error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
