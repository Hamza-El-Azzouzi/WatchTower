mod collector;
mod collectors;
mod config;
mod control;
mod delivery;
mod sender;

use anyhow::Result;
use chrono::Local;
use clap::{Parser, Subcommand};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::time;
use tracing::{debug, error, info, warn};

use collector::{
    cpu::CpuCollector,
    disk::{DiskCollector, MountSnapshot},
    docker::{ContainerSnapshot, DockerCollector},
    gpu::{GpuCollector, TemperatureCollector},
    host::HostCollector,
    memory::MemoryCollector,
    network::{NetworkCollector, NetworkInterfaceSnapshot},
    process::ProcessCollector,
    service::{ServiceCollector, ServiceSnapshot},
    SystemMetrics,
};
use collectors::database::DatabaseCollector;
use config::Config;
use sender::{MetricsPayload, MetricsSender};

#[derive(Parser, Debug)]
#[command(name = "monitor-agent")]
#[command(author = "DevOps Monitoring System")]
#[command(version)]
#[command(about = "System monitoring agent for collecting metrics", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

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

#[derive(Subcommand, Debug)]
enum Command {
    /// Enroll using the short-lived AGENT_ENROLLMENT_TOKEN environment variable.
    Enroll,
    /// Rotate a dedicated agent key; stop the agent service before running.
    RotateKey,
    /// Write a sanitized Docker telemetry snapshot for the unprivileged agent.
    DockerSnapshot {
        #[arg(long, default_value = "/var/run/docker.sock")]
        socket: PathBuf,
        #[arg(long, default_value = "/run/watchtower/docker-telemetry.json")]
        output: PathBuf,
    },
}

struct MetricCollectors {
    cpu: CpuCollector,
    memory: MemoryCollector,
    disk: DiskCollector,
    network: NetworkCollector,
    gpu: GpuCollector,
    temperature: TemperatureCollector,
    process: ProcessCollector,
    host: HostCollector,
    service: ServiceCollector,
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
            host: HostCollector::new(),
            service: ServiceCollector,
        }
    }

    fn collect(
        &mut self,
        config: &Config,
    ) -> (
        SystemMetrics,
        Vec<MountSnapshot>,
        Vec<NetworkInterfaceSnapshot>,
    ) {
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

        let ((disk_used, disk_total, disk_percent), mounts) = if config.metrics.collect_disk {
            self.disk.collect_all()
        } else {
            ((0, 0, 0.0), Vec::new())
        };

        let ((network_rx, network_tx), network_interfaces) = if config.metrics.collect_network {
            self.network.collect_all()
        } else {
            ((0, 0), Vec::new())
        };

        let host = self.host.collect();

        // Collect temperature metrics
        let cpu_temp_celsius = self.temperature.collect_cpu_temp();
        let gpu_temp_celsius = self.temperature.collect_gpu_temp();

        // Collect GPU metrics
        let gpu_usage_percent = self.gpu.collect_usage();
        let (gpu_memory_used, gpu_memory_total) = match self.gpu.collect_memory() {
            Some((used, total)) => (Some(used), Some(total)),
            None => (None, None),
        };

        let metrics = SystemMetrics {
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
            host,
        };

        (metrics, mounts, network_interfaces)
    }

    fn collect_processes(
        &mut self,
        watched_names: &[String],
    ) -> (
        std::collections::HashMap<String, f64>,
        Vec<collector::process::ProcessSnapshot>,
    ) {
        self.process.collect(watched_names, 256)
    }

    fn collect_services(&self, names: &[String], uptime_seconds: u64) -> Vec<ServiceSnapshot> {
        self.service.collect(names, uptime_seconds)
    }
}

async fn run_agent(mut config: Config) -> Result<()> {
    // Lock and recover interrupted writes before loading a rotated credential.
    let startup_queue = if config.server.enabled {
        Some(delivery::DiskQueue::open(
            config.delivery.directory.clone(),
            config.delivery.max_bytes,
            config.delivery.max_records,
        )?)
    } else {
        None
    };
    control::load_credentials(&mut config).await?;
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
                "Database monitoring enabled - {} at {}:{} every {} seconds",
                db_config.db_type, db_config.host, db_config.port, db_config.interval_seconds
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
                logs_config.batch_interval_seconds,
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
        Some(std::sync::Arc::new(MetricsSender::new(
            config.server.url.clone(),
            config.server.api_key.clone(),
            config.server.retry_attempts,
            config.server.retry_delay_seconds,
        )?))
    } else {
        info!("Server integration disabled - metrics will only be displayed locally");
        None
    };

    let remote_settings =
        std::sync::Arc::new(std::sync::Mutex::new(None::<control::RemotePayload>));
    let spool = if let Some(sender) = sender.clone() {
        let queue = std::sync::Arc::new(std::sync::Mutex::new(
            startup_queue.expect("enabled delivery queue"),
        ));
        let worker_queue = queue.clone();
        let replay_interval =
            Duration::from_millis(config.delivery.replay_interval_ms.clamp(20, 5000));
        control::start(config.clone(), queue.clone(), remote_settings.clone())?;
        tokio::spawn(async move {
            let mut failures = 0u32;
            loop {
                let record = worker_queue.lock().expect("spool lock poisoned").peek();
                match record {
                    Ok(Some((path, payload))) => match sender.send_queued(&payload).await {
                        Ok(()) => {
                            if let Err(error) = worker_queue
                                .lock()
                                .expect("spool lock poisoned")
                                .acknowledge(&path)
                            {
                                error!("Cannot acknowledge spool record: {error}");
                            }
                            failures = 0;
                            time::sleep(replay_interval).await;
                        }
                        Err(error) => {
                            failures = failures.saturating_add(1);
                            warn!("Queued upload failed; preserving record: {error}");
                            let jitter = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .subsec_millis() as u64;
                            time::sleep(Duration::from_millis(
                                (2u64.pow(failures.min(5)) * 1000).min(30000) + jitter,
                            ))
                            .await;
                        }
                    },
                    Ok(None) => time::sleep(Duration::from_millis(250)).await,
                    Err(error) => {
                        error!("Spool cannot be read; preserving data: {error}");
                        time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });
        Some(queue)
    } else {
        None
    };

    let mut collectors = MetricCollectors::new();
    let docker_collector = if config.docker_monitor.enabled {
        match DockerCollector::new(config.docker_monitor.endpoint.clone()) {
            Ok(collector) => {
                info!(
                    "Read-only Docker telemetry enabled through {}",
                    config.docker_monitor.endpoint
                );
                Some(collector)
            }
            Err(error) => {
                warn!("Docker telemetry disabled: {}", error);
                None
            }
        }
    } else {
        None
    };
    let interval = Duration::from_secs(config.collection.interval_seconds);
    let mut interval_timer = time::interval(interval);
    interval_timer.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
    let mut last_database_collection: Option<Instant> = None;
    let mut last_service_collection: Option<Instant> = None;
    let mut last_docker_collection: Option<Instant> = None;
    let mut service_snapshots: Vec<ServiceSnapshot> = Vec::new();
    let mut container_snapshots: Vec<ContainerSnapshot> = Vec::new();

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
        let update = remote_settings
            .lock()
            .expect("config lock poisoned")
            .clone();
        if let Some(update) = update {
            if config.collection.interval_seconds != update.settings.interval_seconds {
                config.collection.interval_seconds = update.settings.interval_seconds;
                interval_timer =
                    time::interval(Duration::from_secs(update.settings.interval_seconds));
                interval_timer.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
            }
            config.metrics.collect_cpu = update.settings.collect_cpu;
            config.metrics.collect_memory = update.settings.collect_memory;
            config.metrics.collect_disk = update.settings.collect_disk;
            config.metrics.collect_network = update.settings.collect_network;
        }

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let (metrics, mounts, network_interfaces) = collectors.collect(&config);

        // Display metrics locally
        print!("[{}] ", timestamp);
        metrics.display();

        // Send metrics to server if enabled
        if let Some(ref spool) = spool {
            let mut metrics_map = metrics.to_metrics_map();
            {
                let queue = spool.lock().expect("spool lock poisoned");
                let (records, bytes, last_success) = queue.status()?;
                metrics_map.insert("agent_queue_records".into(), records as f64);
                metrics_map.insert("agent_queue_bytes".into(), bytes as f64);
                metrics_map.insert("agent_dropped_samples".into(), queue.dropped_samples as f64);
                metrics_map.insert(
                    "agent_last_successful_upload_timestamp".into(),
                    last_success.map_or(0.0, |timestamp| timestamp.timestamp() as f64),
                );
            }
            let mut process_snapshot = Vec::new();

            if config.process_watch.enabled {
                let (process_metrics, processes) =
                    collectors.collect_processes(&config.process_watch.names);
                metrics_map.extend(process_metrics);
                process_snapshot = processes;
            }

            let service_due = config.service_watch.enabled
                && last_service_collection.is_none_or(|last| {
                    last.elapsed()
                        >= Duration::from_secs(config.service_watch.interval_seconds.max(5))
                });
            if service_due {
                service_snapshots = collectors
                    .collect_services(&config.service_watch.names, metrics.host.uptime_seconds);
                last_service_collection = Some(Instant::now());
            }

            let docker_due = docker_collector.is_some()
                && last_docker_collection.is_none_or(|last| {
                    last.elapsed()
                        >= Duration::from_secs(config.docker_monitor.interval_seconds.max(5))
                });
            if docker_due {
                if let Some(collector) = docker_collector.as_ref() {
                    match collector.collect().await {
                        Ok(containers) => container_snapshots = containers,
                        Err(error) => warn!("Failed to collect Docker telemetry: {}", error),
                    }
                }
                last_docker_collection = Some(Instant::now());
            }

            add_deep_telemetry_metrics(
                &mut metrics_map,
                &mounts,
                &network_interfaces,
                &service_snapshots,
                &container_snapshots,
            );

            // Collect database metrics if database collector is enabled
            let database_due = config.database.as_ref().is_some_and(|database| {
                last_database_collection.is_none_or(|last| {
                    last.elapsed() >= Duration::from_secs(database.interval_seconds)
                })
            });
            if database_due {
                last_database_collection = Some(Instant::now());
            }
            if let (true, Some(db_collector)) = (database_due, db_collector.as_ref()) {
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
                delivery: None,
                agent_id: config.agent.name.clone(),
                timestamp: chrono::Utc::now(),
                metrics: metrics_map,
                processes: process_snapshot,
                mounts,
                network_interfaces,
                services: service_snapshots.clone(),
                containers: container_snapshots.clone(),
            };

            match spool.lock().expect("spool lock poisoned").enqueue(payload) {
                Ok(true)=>{},
                Ok(false)=>warn!("Delivery storage full: rejecting newest sample; existing queued data preserved"),
                Err(error)=>error!("Cannot persist sample; collection continues, existing spool preserved: {error}"),
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

                    if let Some(ref spool) = spool {
                        match spool
                            .lock()
                            .expect("spool lock poisoned")
                            .enqueue(logs_payload)
                        {
                            Ok(true) => {
                                log_collector.mark_sent(log_count);
                                debug!("Persisted {} logs in delivery spool", log_count);
                            }
                            Ok(false) => warn!("Delivery storage full; logs remain buffered"),
                            Err(error) => error!("Cannot persist logs; buffer retained: {error}"),
                        }
                    }
                }
            }
        }
    }
}

fn add_deep_telemetry_metrics(
    metrics: &mut std::collections::HashMap<String, f64>,
    mounts: &[MountSnapshot],
    interfaces: &[NetworkInterfaceSnapshot],
    services: &[ServiceSnapshot],
    containers: &[ContainerSnapshot],
) {
    metrics.insert(
        "disk_read_bytes_per_sec".to_string(),
        mounts.iter().map(|mount| mount.read_bytes_per_sec).sum(),
    );
    metrics.insert(
        "disk_write_bytes_per_sec".to_string(),
        mounts.iter().map(|mount| mount.write_bytes_per_sec).sum(),
    );
    metrics.insert(
        "disk_read_iops".to_string(),
        mounts.iter().map(|mount| mount.read_iops).sum(),
    );
    metrics.insert(
        "disk_write_iops".to_string(),
        mounts.iter().map(|mount| mount.write_iops).sum(),
    );
    metrics.insert(
        "disk_max_latency_ms".to_string(),
        mounts
            .iter()
            .map(|mount| mount.average_latency_ms)
            .fold(0.0, f64::max),
    );
    metrics.insert(
        "inode_max_usage".to_string(),
        mounts
            .iter()
            .map(|mount| mount.inode_usage_percent)
            .fold(0.0, f64::max),
    );
    metrics.insert(
        "network_rx_errors_total".to_string(),
        interfaces
            .iter()
            .map(|interface| interface.rx_errors)
            .sum::<u64>() as f64,
    );
    metrics.insert(
        "network_tx_errors_total".to_string(),
        interfaces
            .iter()
            .map(|interface| interface.tx_errors)
            .sum::<u64>() as f64,
    );
    metrics.insert(
        "network_dropped_total".to_string(),
        interfaces
            .iter()
            .map(|interface| interface.rx_dropped + interface.tx_dropped)
            .sum::<u64>() as f64,
    );
    metrics.insert("services_total".to_string(), services.len() as f64);
    metrics.insert(
        "services_unhealthy".to_string(),
        services
            .iter()
            .filter(|service| service.active_state != "active")
            .count() as f64,
    );
    metrics.insert(
        "service_restarts_total".to_string(),
        services
            .iter()
            .map(|service| service.restart_count)
            .sum::<u64>() as f64,
    );
    metrics.insert("containers_total".to_string(), containers.len() as f64);
    metrics.insert(
        "containers_unhealthy".to_string(),
        containers
            .iter()
            .filter(|container| {
                container.state != "running"
                    || matches!(container.health.as_str(), "unhealthy" | "starting")
            })
            .count() as f64,
    );
    metrics.insert(
        "container_restarts_total".to_string(),
        containers
            .iter()
            .map(|container| container.restart_count)
            .sum::<u64>() as f64,
    );
    metrics.insert(
        "container_cpu_percent".to_string(),
        containers
            .iter()
            .map(|container| container.cpu_percent)
            .sum(),
    );
    metrics.insert(
        "container_memory_bytes".to_string(),
        containers
            .iter()
            .map(|container| container.memory_bytes)
            .sum::<u64>() as f64,
    );
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

    if let Some(Command::DockerSnapshot { socket, output }) = &args.command {
        write_docker_snapshot(socket.clone(), output.clone()).await?;
        return Ok(());
    }

    // Load configuration
    let mut config = if let Some(config_path) = args.config {
        info!("Loading configuration from: {:?}", config_path);
        Config::from_file(&config_path)?
    } else {
        info!("No configuration file specified, using defaults");
        Config::default()
    };

    config.apply_env_overrides();
    if matches!(args.command, Some(Command::Enroll)) {
        return control::enroll(&config).await;
    }
    if matches!(args.command, Some(Command::RotateKey)) {
        return control::rotate(&config).await;
    }

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

async fn write_docker_snapshot(socket: PathBuf, output: PathBuf) -> Result<()> {
    let snapshots = DockerCollector::from_unix_socket(socket).collect().await?;
    let temporary = output.with_extension("json.tmp");
    let contents = serde_json::to_vec(&snapshots)?;
    std::fs::write(&temporary, contents)?;
    std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o640))?;
    std::fs::rename(&temporary, &output)?;
    info!(
        "Wrote {} sanitized Docker snapshots to {}",
        snapshots.len(),
        output.display()
    );
    Ok(())
}
