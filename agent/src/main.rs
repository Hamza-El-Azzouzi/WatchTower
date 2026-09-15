mod collector;
mod collectors;
mod config;
mod sender;

use anyhow::Result;
use chrono::Local;
use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info, warn};

use collector::{
    cpu::CpuCollector,
    disk::DiskCollector,
    gpu::{GpuCollector, TemperatureCollector},
    memory::MemoryCollector,
    network::NetworkCollector,
    process::ProcessCollector,
    SystemMetrics,
};
use collectors::database::DatabaseCollector;
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
    gpu: GpuCollector,
    temperature: TemperatureCollector,
    process: ProcessCollector,
}

impl MetricCollectors {
    fn new() -> Self {
        Self {
            cpu: CpuCollector::new(),
            memory: MemoryCollector::new(),
            disk: DiskCollector::new(),
            network: NetworkCollector::new(),
            gpu: GpuCollector::new(),
            temperature: TemperatureCollector::new(),
            process: ProcessCollector::new(),
        }
    }

    fn collect(&mut self, config: &Config) -> SystemMetrics {
        // Collect CPU metrics in one pass (more efficient)
        let (cpu_percent, cpu_per_core) = if config.metrics.collect_cpu {
            self.cpu.collect_all()
        } else {
            (0.0, Vec::new())
        };

        let (memory_used, memory_total, memory_percent) = if config.metrics.collect_memory {
            self.memory.collect()
        } else {
            (0, 0, 0.0)
        };

        let (swap_used, swap_total, swap_percent) = if config.metrics.collect_memory {
            self.memory.collect_swap()
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

        // Collect temperature metrics
        let cpu_temp_celsius = self.temperature.collect_cpu_temp();
        let gpu_temp_celsius = self.temperature.collect_gpu_temp();

        // Collect GPU metrics
        let gpu_usage_percent = self.gpu.collect_usage();
        let (gpu_memory_used, gpu_memory_total) = match self.gpu.collect_memory() {
            Some((used, total)) => (Some(used), Some(total)),
            None => (None, None),
        };

        SystemMetrics {
            cpu_percent,
            cpu_per_core,
            memory_used_bytes: memory_used,
            memory_total_bytes: memory_total,
            memory_percent,
            swap_used_bytes: swap_used,
            swap_total_bytes: swap_total,
            swap_percent,
            disk_used_bytes: disk_used,
            disk_total_bytes: disk_total,
            disk_percent,
            network_rx_bytes: network_rx,
            network_tx_bytes: network_tx,
            cpu_temp_celsius,
            gpu_temp_celsius,
            gpu_usage_percent,
            gpu_memory_used,
            gpu_memory_total,
        }
    }

    fn collect_processes(
        &mut self,
        watched_names: &[String],
    ) -> std::collections::HashMap<String, f64> {
        self.process.collect(watched_names)
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

    // Initialize database collector if enabled
    let db_collector = if let Some(ref db_config) = config.database {
        if db_config.enabled {
            info!(
                "Database monitoring enabled - {} at {}:{}",
                db_config.db_type, db_config.host, db_config.port
            );
            match DatabaseCollector::new(db_config.clone()) {
                Ok(collector) => Some(collector),
                Err(e) => {
                    warn!("Failed to initialize database collector: {}", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    // Initialize log collector if enabled
    let mut log_collector = if let Some(ref logs_config) = config.logs {
        if logs_config.enabled && !logs_config.paths.is_empty() {
            info!(
                "Log collection enabled - watching {} files",
                logs_config.paths.len()
            );
            match collector::LogCollector::new(
                config.agent.name.clone(),
                logs_config.paths.clone(),
                logs_config.batch_size,
            ) {
                Ok(collector) => Some(collector),
                Err(e) => {
                    warn!("Failed to initialize log collector: {}", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    // Initialize metrics sender if server is enabled
    let sender = if config.server.enabled {
        info!(
            "Server integration enabled - sending metrics to {}",
            config.server.url
        );
        if config.server.api_key.is_some() {
            info!("API key authentication enabled");
        } else {
            warn!(
                "No API key configured - server may reject metrics if authentication is required"
            );
        }
        Some(MetricsSender::new(
            config.server.url.clone(),
            config.server.api_key.clone(),
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
            let mut metrics_map = metrics.to_metrics_map();

            if config.process_watch.enabled {
                metrics_map.extend(collectors.collect_processes(&config.process_watch.names));
            }

            // Collect database metrics if database collector is enabled
            if let Some(ref db_collector) = db_collector {
                match db_collector.collect().await {
                    Ok(db_metrics) => {
                        // Add all database metrics to the map
                        for (key, value) in db_metrics.to_hashmap() {
                            metrics_map.insert(key, value);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to collect database metrics: {}", e);
                    }
                }
            }

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

        // Process log file events and send logs if collector is enabled
        if let Some(ref mut log_collector) = log_collector {
            // Process file system events
            if let Err(e) = log_collector.process_events() {
                warn!("Error processing log events: {}", e);
            }

            // Check if we should flush logs (batch size reached or time-based)
            if log_collector.should_flush() {
                if let Some(logs_payload) = log_collector.create_payload() {
                    let log_count = logs_payload.logs.len();

                    if let Some(ref sender) = sender {
                        // Send logs to server
                        if let Err(e) = sender.send_logs(&logs_payload).await {
                            warn!("Failed to send {} logs to server: {}", log_count, e);
                        } else {
                            debug!("Sent {} logs to server", log_count);
                        }
                    }
                }
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

    config.apply_env_overrides();

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
