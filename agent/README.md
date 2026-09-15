# 🤖 WatchTower Agent

<p align="center">
  <strong>Lightweight, cross-platform system metrics collector for WatchTower monitoring platform</strong>
</p>

---

## Overview

The Monitoring Agent is a high-performance, low-overhead system metrics collector that runs on your servers and sends real-time performance data to the central monitoring server. Built with Rust for maximum efficiency and reliability.

### Key Features

- ✅ **Low Resource Usage**: < 2% CPU, < 50MB RAM
- ✅ **Cross-Platform**: Linux, macOS, Windows
- ✅ **Comprehensive Metrics**: CPU, Memory, Disk, Network, Swap, GPU, Database
- ✅ **Per-Core CPU Tracking**: Individual CPU core monitoring
- ✅ **GPU Monitoring**: NVIDIA/AMD GPU usage & temperature
- ✅ **Database Monitoring**: PostgreSQL/MySQL metrics (connections, QPS, cache)
- ✅ **Temperature Sensors**: CPU/GPU temperature (when available)
- ✅ **Reliable Transmission**: Retry logic with exponential backoff
- ✅ **Secure**: API key authentication
- ✅ **Configurable**: TOML-based configuration
- ✅ **Production-Ready**: Auto-reconnect, error handling

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│            MONITORING AGENT                     │
│                                                 │
│  ┌─────────────────────────────────────────┐    │
│  │         Metric Collectors               │    │
│  ├─────────────────────────────────────────┤    │
│  │  • CPU Collector (sysinfo)              │    │
│  │    - Total CPU usage                    │    │
│  │    - Per-core usage                     │    │
│  │    - CPU temperature                    │    │
│  │                                         │    │
│  │  • Memory Collector                     │    │
│  │    - Used/Total memory                  │    │
│  │    - Swap usage                         │    │
│  │                                         │    │
│  │  • Disk Collector                       │    │
│  │    - Used/Total disk space              │    │
│  │    - Multiple mount points              │    │
│  │                                         │    │
│  │  • Network Collector                    │    │
│  │    - RX/TX bytes                        │    │
│  │    - All network interfaces             │    │
│  │  • GPU Collector                        │    │
│  │    - GPU usage & temperature            │    │
│  │    - GPU memory (NVIDIA/AMD)            │    │
│  │                                         │    │
│  │  • Database Collector                   │    │
│  │    - PostgreSQL/MySQL metrics           │    │
│  │    - Connections, QPS, cache hit ratio  │    │
│  └─────────────────────────────────────────┘    │
│                     │                           │
│                     ▼                           │
│  ┌─────────────────────────────────────────┐    │
│  │        Metrics Aggregator               │    │
│  │   (Combines all metrics with timestamp) │    │
│  └─────────────────────────────────────────┘    │
│                     │                           │
│                     ▼                           │
│  ┌──────────────────────────────────────────┐   │
│  │          HTTP Sender                     │   │
│  │  • Batch metrics                         │   │
│  │  • Add API key authentication            │   │
│  │  • Retry on failure (exponential backoff)│   │
│  │  • POST to /api/v1/metrics               │   │
│  └──────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
                     │
                     │ HTTPS
                     ▼
        ┌─────────────────────────┐
        │   Central Server        │
        │   (port 8080)           │
        └─────────────────────────┘
```

---

## Installation

### Build from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/devops-monitoring-system.git
cd devops-monitoring-system/agent

# Build release binary
cargo build --release

# Binary will be at: target/release/monitor-agent
./target/release/monitor-agent -c agent.toml
```


---

## Configuration

Create an `agent.toml` configuration file:

```toml
# Agent Configuration

[agent]
# Unique agent identifier and display name
name = "web-server-01"

[server]
enabled = true
# Central monitoring server URL
url = "http://monitoring.example.com:8080"

# API key for authentication (get from admin dashboard)
api_key = "msk_prod_abc123xyz789..."

[collection]
# How often to collect and send metrics (in seconds)
interval_seconds = 15

[metrics]
# Which metrics to collect (all enabled by default)
collect_cpu = true
collect_memory = true
collect_disk = true
collect_network = true

# Optional process monitoring. Executable names use an exact,
# case-insensitive match. Command lines and environments are not sent.
[process_watch]
enabled = true
names = ["postgres", "nginx"]

# Optional: Database monitoring
[database]
enabled = true
interval_seconds = 15
db_type = "postgres"  # or "mysql"
host = "localhost"
port = 5432
database = "myapp"
username = "monitor_user"
password = "secure_password"
```

Service and container deployments can override settings with `AGENT_NAME`,
`SERVER_URL`, `API_KEY`, `COLLECTION_INTERVAL`, `PROCESS_WATCH_ENABLED`, and a
comma-separated `PROCESS_WATCH_NAMES` value. Optional database monitoring uses
`DB_MONITOR_ENABLED`, `DB_MONITOR_TYPE`, `DB_MONITOR_HOST`, `DB_MONITOR_PORT`,
`DB_MONITOR_DATABASE`, `DB_MONITOR_USERNAME`, `DB_MONITOR_PASSWORD`, and
`DB_MONITOR_INTERVAL_SECONDS`.
Optional file-log collection uses `LOG_COLLECTION_ENABLED`, comma-separated
`LOG_PATHS`, `LOG_BATCH_SIZE`, and `LOG_BATCH_INTERVAL_SECONDS`. Keep secrets in
a root-owned environment file with mode `0600`; do not commit them.

---

## Usage

### Basic Usage

```bash
# Run with configuration file
./monitor-agent -c agent.toml

# Run with custom config path
./monitor-agent --config /etc/monitor/agent.toml

# Show version
./monitor-agent --version

```

### Running as a Service

#### Linux (systemd)

Create `/etc/systemd/system/monitor-agent.service`:

```ini
[Unit]
Description=Monitoring Agent
After=network.target

[Service]
Type=simple
User=monitor
Group=monitor
WorkingDirectory=/opt/monitor-agent
ExecStart=/opt/monitor-agent/monitor-agent -c /opt/monitor-agent/agent.toml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable monitor-agent
sudo systemctl start monitor-agent
sudo systemctl status monitor-agent
```

---

## Collected Metrics

### CPU Metrics

| Metric Name | Type | Description |
|-------------|------|-------------|
| `cpu_usage` | Percentage | Overall CPU usage (0-100%) |
| `cpu_core_0` | Percentage | CPU core 0 usage |
| `cpu_core_N` | Percentage | CPU core N usage |
| `cpu_temp_celsius` | Temperature | CPU temperature |

### Memory Metrics

| Metric Name | Type | Description |
|-------------|------|-------------|
| `memory_usage` | Percentage | Memory usage (0-100%) |
| `memory_used_bytes` | Bytes | Used memory |
| `memory_total_bytes` | Bytes | Total memory |

### Swap Metrics

| Metric Name | Type | Description |
|-------------|------|-------------|
| `swap_usage` | Percentage | Swap usage (0-100%) |
| `swap_used_bytes` | Bytes | Used swap |
| `swap_total_bytes` | Bytes | Total swap |

### Disk Metrics

| Metric Name | Type | Description |
|-------------|------|-------------|
| `disk_usage` | Percentage | Disk usage (0-100%) |
| `disk_used_bytes` | Bytes | Used disk space |
| `disk_total_bytes` | Bytes | Total disk space |

### Network Metrics

| Metric Name | Type | Description |
|-------------|------|-------------|
| `network_rx_bytes` | Bytes | Total bytes received |
| `network_tx_bytes` | Bytes | Total bytes transmitted |

### GPU Metrics

| Metric Name | Type | Description |
|-------------|------|-------------|
| `gpu_usage` | Percentage | GPU utilization (0-100%) |
| `gpu_memory_used` | Bytes | GPU memory used |
| `gpu_memory_total` | Bytes | Total GPU memory |
| `gpu_temp_celsius` | Temperature | GPU temperature |

**Note**: GPU metrics require NVIDIA (via nvml-wrapper) or AMD GPUs. Auto-detected when hardware is available.

### Database Metrics

| Metric Name | Type | Description |
|-------------|------|-------------|
| `db_connections_active` | Count | Active database connections |
| `db_connections_idle` | Count | Idle database connections |
| `db_connections_max` | Count | Maximum allowed connections |
| `db_queries_per_second` | Rate | Query throughput (QPS) |
| `db_slow_queries` | Count | Slow queries (>1 second) |
| `db_cache_hit_ratio` | Percentage | Database cache hit ratio |
| `db_transactions_committed` | Count | Committed transactions |
| `db_transactions_rolled_back` | Count | Rolled back transactions |
| `db_database_size_bytes` | Bytes | Database size |
| `db_locks_waiting` | Count | Waiting locks count |

**Supported databases**: PostgreSQL, MySQL

---

## Performance

| Metric | Value |
|--------|-------|
| CPU Usage | < 2% |
| Memory Usage | < 50 MB |
| Network | ~1-2 KB/s |
| Collection Time | < 100ms |

---

## License

MIT License - See [LICENSE](../LICENSE) for details.

---

<p align="center">
  Built with ❤️ using Rust
</p>
