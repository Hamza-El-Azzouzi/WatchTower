# DevOps Monitoring System - Complete Project Specification

## Project Overview
Build a production-grade monitoring system in Rust that tracks system metrics, collects application logs, and provides real-time visibility into server health. This project demonstrates the core skills needed for DevOps/SRE roles: observability, monitoring, alerting, and distributed systems.

**Think of it as:** Building your own Prometheus + Grafana + Fluentd in one unified system.

---

## Why This Project Matters for DevOps

### Skills You'll Demonstrate:
- **Observability**: Understanding what monitoring means in production
- **Systems Programming**: Working with OS-level metrics and processes
- **Distributed Systems**: Multi-server architecture with agents and collectors
- **Data Collection**: Time-series metrics and log aggregation
- **Real-time Processing**: Handling streaming data efficiently
- **Alerting**: Detecting and responding to issues automatically
- **API Design**: Building APIs that other services can consume
- **Dashboard Development**: Visualizing complex data effectively

### Interview Value:
This project answers questions like:
- "How do you monitor production systems?"
- "What metrics matter for system health?"
- "How would you debug a slow application?"
- "Design a monitoring system for microservices"

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Central Server                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │   REST API   │  │  Time-Series │  │   Alert Engine       │  │
│  │   (Axum)     │─▶│   Storage    │─▶│  (Rule Evaluator)    │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
│         │                  │                      │              │
│         │                  │                      ▼              │
│         │                  │           ┌──────────────────────┐ │
│         │                  └──────────▶│   Web Dashboard      │ │
│         │                              │   (HTML/JS/Plotly)   │ │
│         │                              └──────────────────────┘ │
└─────────┼──────────────────────────────────────────────────────┘
          │
          │ HTTP/gRPC (metrics + logs)
          │
    ┌─────┴─────┬──────────┬──────────┐
    │           │          │          │
┌───▼────┐ ┌───▼────┐ ┌───▼────┐ ┌──▼─────┐
│ Agent  │ │ Agent  │ │ Agent  │ │ Agent  │
│Server 1│ │Server 2│ │Server 3│ │Server N│
└────────┘ └────────┘ └────────┘ └────────┘
```

---

## Component Breakdown

### 1. Monitoring Agent (Runs on Each Server)

**Purpose:** Collect metrics and logs from the server and send to central server.

#### Responsibilities:
- **System Metrics Collection**
  - CPU usage (total, per-core, per-process)
  - Memory usage (total, available, swap)
  - Disk usage (space, I/O operations)
  - Network traffic (bytes sent/received, connections)
  
- **Process Monitoring**
  - List running processes
  - Track specific process metrics (CPU, memory per process)
  - Monitor process lifecycle (start, stop, crashes)
  
- **Log Collection**
  - Watch configured log files for new entries
  - Parse log lines (timestamp, level, message)
  - Buffer and batch logs before sending
  
- **Data Transmission**
  - Send metrics via HTTP POST every 10-30 seconds
  - Send logs in real-time or small batches
  - Retry failed transmissions
  - Compress data before sending (optional)

#### Configuration File Example (`agent.toml`):
```toml
[agent]
name = "web-server-01"
server_url = "http://monitoring.example.com:8080"
collection_interval = 15  # seconds

[metrics]
collect_cpu = true
collect_memory = true
collect_disk = true
collect_network = true

[processes]
monitor = ["nginx", "postgres", "redis"]

[logs]
paths = [
    "/var/log/nginx/access.log",
    "/var/log/app/application.log",
    "/var/log/syslog"
]
```

#### Key Rust Crates:
- `sysinfo` - System and process information
- `tokio` - Async runtime
- `reqwest` - HTTP client
- `notify` - File system watcher
- `serde` / `toml` - Configuration parsing
- `chrono` - Timestamp handling

---

### 2. Central Server (Aggregates Data from All Agents)

**Purpose:** Receive, store, and serve metrics and logs from all monitored servers.

#### Responsibilities:

##### A. REST API Endpoints

**Metrics Ingestion:**
```
POST /api/v1/metrics
Content-Type: application/json

{
  "agent_id": "web-server-01",
  "timestamp": "2024-01-27T10:30:45Z",
  "metrics": {
    "cpu_percent": 45.2,
    "memory_used_bytes": 4294967296,
    "memory_total_bytes": 8589934592,
    "disk_used_bytes": 107374182400,
    "disk_total_bytes": 536870912000,
    "network_rx_bytes": 1048576000,
    "network_tx_bytes": 524288000
  }
}
```

**Logs Ingestion:**
```
POST /api/v1/logs
Content-Type: application/json

{
  "agent_id": "web-server-01",
  "logs": [
    {
      "timestamp": "2024-01-27T10:30:45Z",
      "level": "ERROR",
      "source": "/var/log/app/application.log",
      "message": "Database connection timeout after 30s"
    },
    {
      "timestamp": "2024-01-27T10:30:46Z",
      "level": "INFO",
      "source": "/var/log/nginx/access.log",
      "message": "GET /api/users 200 45ms"
    }
  ]
}
```

**Query Endpoints (for Dashboard):**
```
GET /api/v1/metrics?agent_id=web-server-01&metric=cpu_percent&from=2024-01-27T10:00:00Z&to=2024-01-27T11:00:00Z
GET /api/v1/logs?agent_id=web-server-01&level=ERROR&limit=100
GET /api/v1/agents - List all connected agents
GET /api/v1/alerts - List active alerts
```

##### B. Time-Series Storage

**In-Memory Storage (Simple MVP):**
```rust
struct TimeSeriesStore {
    // agent_id -> metric_name -> Vec<(timestamp, value)>
    data: HashMap<String, HashMap<String, Vec<(DateTime<Utc>, f64)>>>
}
```

**Persistent Storage (Phase 2):**
- SQLite for simple deployment
- PostgreSQL with TimescaleDB extension for production
- Or use specialized time-series DB like InfluxDB

**Data Retention:**
- Keep raw data for 24 hours
- Aggregate to 1-minute averages for 7 days
- Aggregate to 1-hour averages for 30 days
- Delete data older than 30 days

##### C. Alert Engine

**Alert Rules Configuration:**
```toml
[[alert]]
name = "High CPU Usage"
condition = "cpu_percent > 80"
duration = "5m"  # Alert only if sustained for 5 minutes
severity = "warning"
action = "log"  # Future: email, slack, webhook

[[alert]]
name = "Disk Space Critical"
condition = "disk_used_percent > 90"
duration = "1m"
severity = "critical"
action = "log"

[[alert]]
name = "Application Errors"
condition = "log_level == 'ERROR' AND source contains 'application.log'"
count = 10  # Alert if 10 errors in time window
window = "5m"
severity = "warning"
action = "log"
```

**Alert State Management:**
```rust
struct Alert {
    id: String,
    rule_name: String,
    agent_id: String,
    severity: AlertSeverity,
    message: String,
    triggered_at: DateTime<Utc>,
    resolved_at: Option<DateTime<Utc>>,
    state: AlertState,  // Firing, Resolved
}
```

#### Key Rust Crates:
- `axum` or `actix-web` - Web framework
- `tokio` - Async runtime
- `serde` / `serde_json` - JSON handling
- `sqlx` or `rusqlite` - Database (optional)
- `chrono` - Time handling
- `tracing` - Logging

---

### 3. Web Dashboard (Visualize Everything)

**Purpose:** Provide real-time visibility into all monitored systems.

#### Pages:

##### A. Overview Dashboard
- **Server List**: All connected agents with status (healthy, warning, critical)
- **Quick Stats**: Total servers, total alerts, average CPU/memory across fleet
- **Recent Alerts**: Last 10 alerts with severity indicators

##### B. Server Detail View
- **Time-Series Graphs** (last 1 hour, 24 hours, 7 days):
  - CPU usage (line chart)
  - Memory usage (line chart)
  - Disk usage (bar chart)
  - Network traffic (area chart)
  
- **Current Metrics Table**:
  ```
  Metric          Current    Average    Peak
  CPU             45.2%      38.5%      87.3%
  Memory          4.0 GB     3.8 GB     6.2 GB
  Disk Used       100 GB     95 GB      110 GB
  Network RX      1.2 MB/s   0.8 MB/s   5.4 MB/s
  ```

- **Running Processes**:
  ```
  PID    Name        CPU%    Memory     Status
  1234   nginx       5.2%    128 MB     Running
  5678   postgres    12.3%   512 MB     Running
  9012   app         25.1%   256 MB     Running
  ```

##### C. Logs Viewer
- **Real-time Log Stream**: Auto-updating list of recent logs
- **Filters**:
  - By agent (dropdown)
  - By log level (ERROR, WARN, INFO, DEBUG)
  - By time range
  - By keyword search
  
- **Log Entry Display**:
  ```
  [2024-01-27 10:30:45] [ERROR] web-server-01 | /var/log/app/application.log
  Database connection timeout after 30s
  ```

##### D. Alerts Page
- **Active Alerts**: Currently firing alerts with details
- **Alert History**: Resolved alerts with resolution time
- **Alert Configuration**: View/edit alert rules (advanced)

#### Technology Stack:
- **Backend API**: Served by Central Server (Axum)
- **Frontend**:
  - NextJs
  - **Plotly.js** for interactive charts
  - **WebSocket** or **Server-Sent Events** for real-time updates
---

## Data Models

### Metric Data Point
```rust
#[derive(Serialize, Deserialize, Clone)]
struct MetricPoint {
    agent_id: String,
    timestamp: DateTime<Utc>,
    metric_name: String,
    value: f64,
    unit: String,  // "percent", "bytes", "bytes/sec"
}
```

### Log Entry
```rust
#[derive(Serialize, Deserialize, Clone)]
struct LogEntry {
    id: u64,
    agent_id: String,
    timestamp: DateTime<Utc>,
    level: LogLevel,
    source: String,  // File path
    message: String,
}

#[derive(Serialize, Deserialize, Clone)]
enum LogLevel {
    DEBUG,
    INFO,
    WARN,
    ERROR,
    FATAL,
}
```

### Agent Registration
```rust
#[derive(Serialize, Deserialize, Clone)]
struct Agent {
    id: String,
    name: String,
    hostname: String,
    ip_address: String,
    os: String,
    last_seen: DateTime<Utc>,
    status: AgentStatus,
}

#[derive(Serialize, Deserialize, Clone)]
enum AgentStatus {
    Healthy,      // Last seen < 1 minute ago
    Degraded,     // Last seen 1-5 minutes ago
    Unreachable,  // Last seen > 5 minutes ago
}
```

---

## Implementation Phases

### Phase 1: Basic Agent (Week 1-2)
**Goal:** Agent can collect and display system metrics locally.

**Tasks:**
1. Set up Rust project structure
2. Use `sysinfo` crate to collect CPU, memory, disk metrics
3. Print metrics to console every 10 seconds
4. Add configuration file parsing (TOML)
5. Package as CLI tool with `clap`

**Deliverable:** Agent binary that prints system stats continuously.

**Testing:**
```bash
$ cargo run
[2024-01-27 10:30:45] CPU: 45.2%, Memory: 4.0/8.0 GB, Disk: 100/500 GB
[2024-01-27 10:30:55] CPU: 47.1%, Memory: 4.1/8.0 GB, Disk: 100/500 GB
```

---

### Phase 2: Central Server - Metrics API (Week 2-3)
**Goal:** Server can receive and store metrics from agents.

**Tasks:**
1. Set up Axum web server project
2. Create POST `/api/v1/metrics` endpoint
3. Implement in-memory time-series storage
4. Create GET endpoint to query metrics
5. Add agent registration mechanism
6. Test with curl/Postman

**Deliverable:** Running server that accepts and returns metrics.

**Testing:**
```bash
# Send metric
$ curl -X POST http://localhost:8080/api/v1/metrics \
  -H "Content-Type: application/json" \
  -d '{"agent_id":"test","timestamp":"2024-01-27T10:30:45Z","metrics":{"cpu_percent":45.2}}'

# Query metric
$ curl "http://localhost:8080/api/v1/metrics?agent_id=test&metric=cpu_percent"
```

---

### Phase 3: Connect Agent to Server (Week 3)
**Goal:** Agent sends metrics to server automatically.

**Tasks:**
1. Add HTTP client to agent using `reqwest`
2. Send metrics every 15 seconds
3. Handle connection failures gracefully
4. Add retry logic with exponential backoff
5. Test with agent and server running

**Deliverable:** Agent and server communicating successfully.

**Testing:**
- Start server
- Start agent
- Verify metrics arriving in server logs
- Query metrics via API

---

### Phase 4: Basic Dashboard (Week 3-4)
**Goal:** Web UI showing real-time metrics.

**Tasks:**
1. Add static file serving to server
2. Create HTML page with Plotly.js charts
3. Fetch metrics from API every 10 seconds
4. Display CPU and memory graphs
5. Add server list with status indicators

**Deliverable:** Working dashboard accessible in browser.

**Testing:**
- Open http://localhost:8080 in browser
- See real-time updating charts
- Verify data matches agent metrics

---

### Phase 5: Log Collection (Week 4-5)
**Goal:** Agent collects logs and sends to server.

**Tasks:**
1. Add file watcher to agent using `notify`
2. Parse log entries (timestamp, level, message)
3. Send logs to server via POST `/api/v1/logs`
4. Store logs in server (in-memory or SQLite)
5. Add logs query endpoint
6. Add logs viewer to dashboard

**Deliverable:** Complete log aggregation pipeline.

**Testing:**
- Agent watches `/var/log/test.log`
- Append line to log file
- See log appear in dashboard within seconds

---

### Phase 6: Alert Engine (Week 5-6)
**Goal:** System detects and alerts on issues.

**Tasks:**
1. Define alert rule format
2. Implement rule evaluation engine
3. Check rules against incoming metrics/logs
4. Store alert state (firing, resolved)
5. Add alerts endpoint to API
6. Display active alerts on dashboard

**Deliverable:** Working alert system.

**Testing:**
- Define rule: "CPU > 80%"
- Simulate high CPU load
- See alert trigger on dashboard
- Verify alert resolves when CPU drops

---

### Phase 7: Polish & Production-Ready (Week 6+) ✅ 87.5% COMPLETE
**Goal:** Make it deployment-ready.

**Tasks:**
1. ✅ Add persistent storage (SQLite with automatic migrations)
2. ✅ Implement data retention policies (configurable cleanup)
3. ✅ Add authentication (API keys with agent limits & admin UI)
4. ✅ Add configuration management (TOML + environment variables)
5. ✅ Create Docker deployment (full stack with Compose)
6. ✅ Write deployment documentation (Docker guide + authentication)
7. ✅ Add unit and integration tests (18 tests passing + CI/CD)
8. ⏳ Performance optimization implementation

**Deliverable:** Production-grade monitoring system ready for deployment.

**Status:** Testing suite complete with CI/CD pipeline, comprehensive documentation available. Only performance optimization implementation remains.

**Test Results:**
```bash
running 18 tests
test result: ok. 18 passed; 0 failed; 0 ignored
```

**See:** 
- [Phase 7 Documentation](docs/PHASE7.md) for detailed progress
- [Testing Guide](TESTING.md) for test documentation
- [Testing Completion Summary](PHASE7_TESTING_COMPLETE.md)

**Quick Start:**
```bash
# Start full stack with Docker
docker-compose up -d

# Access dashboard
open http://localhost:3000

# Run tests
cd server && cargo test

# Run benchmarks
cargo bench

# Generate API key (via dashboard or API)
curl -X POST http://localhost:8080/api/v1/auth/keys \
  -H "Content-Type: application/json" \
  -d '{"name": "Production Agents", "max_agents": 10}'
```

---

## Technical Deep Dives

### How to Collect System Metrics

#### CPU Usage:
```rust
use sysinfo::{System, SystemExt, ProcessorExt};

fn get_cpu_usage() -> f32 {
    let mut system = System::new_all();
    system.refresh_cpu();
    std::thread::sleep(std::time::Duration::from_millis(200));
    system.refresh_cpu();
    
    let usage: f32 = system.processors()
        .iter()
        .map(|p| p.cpu_usage())
        .sum::<f32>() / system.processors().len() as f32;
    
    usage
}
```

#### Memory Usage:
```rust
fn get_memory_usage() -> (u64, u64) {
    let mut system = System::new_all();
    system.refresh_memory();
    
    let total = system.total_memory();
    let used = system.used_memory();
    
    (used, total)
}
```

#### Disk Usage:
```rust
use sysinfo::{System, SystemExt, DiskExt};

fn get_disk_usage() -> Vec<(String, u64, u64)> {
    let mut system = System::new_all();
    system.refresh_disks_list();
    
    system.disks()
        .iter()
        .map(|disk| {
            let name = disk.mount_point().to_string_lossy().to_string();
            let total = disk.total_space();
            let available = disk.available_space();
            (name, total - available, total)
        })
        .collect()
}
```

#### Network Traffic:
```rust
use sysinfo::{System, SystemExt, NetworkExt};

fn get_network_stats() -> (u64, u64) {
    let mut system = System::new_all();
    system.refresh_networks();
    
    let (mut rx, mut tx) = (0, 0);
    for (_, data) in system.networks() {
        rx += data.received();
        tx += data.transmitted();
    }
    
    (rx, tx)
}
```

### How to Watch Log Files

```rust
use notify::{Watcher, RecursiveMode, Result};
use std::sync::mpsc::channel;

fn watch_log_file(path: &str) -> Result<()> {
    let (tx, rx) = channel();
    let mut watcher = notify::watcher(tx, Duration::from_secs(1))?;
    
    watcher.watch(path, RecursiveMode::NonRecursive)?;
    
    loop {
        match rx.recv() {
            Ok(event) => {
                // Read new lines from file
                let new_lines = read_new_lines(path);
                for line in new_lines {
                    let log_entry = parse_log_line(&line);
                    send_to_server(log_entry);
                }
            }
            Err(e) => println!("Watch error: {:?}", e),
        }
    }
}
```

### How to Create Real-Time Dashboard Updates

**Option 1: Polling (Simple)**
```javascript
// Dashboard polls API every 10 seconds
setInterval(async () => {
    const response = await fetch('/api/v1/metrics/latest');
    const data = await response.json();
    updateCharts(data);
}, 10000);
```

**Option 2: Server-Sent Events (Better)**
```rust
// Server endpoint
async fn metrics_stream() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = tokio_stream::wrappers::BroadcastStream::new(rx)
        .map(|msg| {
            Ok(Event::default()
                .data(serde_json::to_string(&msg).unwrap()))
        });
    
    Sse::new(stream)
}
```

```javascript
// Dashboard client
const eventSource = new EventSource('/api/v1/metrics/stream');
eventSource.onmessage = (event) => {
    const metric = JSON.parse(event.data);
    updateChart(metric);
};
```

### How to Implement Time-Series Storage

**Simple In-Memory Store:**
```rust
use std::collections::HashMap;
use chrono::{DateTime, Utc};

struct TimeSeriesStore {
    data: HashMap<String, Vec<DataPoint>>,
}

struct DataPoint {
    timestamp: DateTime<Utc>,
    value: f64,
}

impl TimeSeriesStore {
    fn insert(&mut self, series: String, timestamp: DateTime<Utc>, value: f64) {
        self.data
            .entry(series)
            .or_insert_with(Vec::new)
            .push(DataPoint { timestamp, value });
    }
    
    fn query(&self, series: &str, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<DataPoint> {
        self.data
            .get(series)
            .map(|points| {
                points.iter()
                    .filter(|p| p.timestamp >= from && p.timestamp <= to)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
}
```

---

## Configuration Files

### Agent Configuration (`agent.toml`)
```toml
[agent]
id = "web-server-01"
name = "Production Web Server"
server_url = "http://monitoring.example.com:8080"
api_key = "secret-key-here"

[collection]
interval_seconds = 15
batch_size = 10

[metrics]
cpu = true
memory = true
disk = true
network = true
processes = ["nginx", "postgres", "redis"]

[logs]
enabled = true
paths = [
    "/var/log/nginx/access.log",
    "/var/log/nginx/error.log",
    "/var/log/app/application.log"
]

# Log parsing patterns
[[logs.patterns]]
name = "nginx-access"
regex = '''(?P<ip>\S+) - - \[(?P<timestamp>[^\]]+)\] "(?P<method>\S+) (?P<path>\S+) (?P<protocol>\S+)" (?P<status>\d+) (?P<bytes>\d+)'''

[[logs.patterns]]
name = "application"
regex = '''(?P<timestamp>\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) \[(?P<level>\w+)\] (?P<message>.+)'''
```

### Server Configuration (`server.toml`)
```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4

[storage]
type = "sqlite"  # or "memory", "postgres"
path = "./data/monitoring.db"

[retention]
raw_data_hours = 24
minute_aggregates_days = 7
hour_aggregates_days = 30

[alerts]
enabled = true
check_interval_seconds = 30
config_file = "./alerts.toml"

[dashboard]
enabled = true
static_dir = "./dashboard"
```

### Alert Rules (`alerts.toml`)
```toml
[[rule]]
name = "high-cpu"
description = "CPU usage above 80% for 5 minutes"
condition = "avg(cpu_percent) > 80"
duration = "5m"
severity = "warning"
labels = { team = "platform", service = "infrastructure" }

[[rule]]
name = "low-disk-space"
description = "Disk usage above 90%"
condition = "disk_used_percent > 90"
duration = "1m"
severity = "critical"
labels = { team = "platform", service = "infrastructure" }

[[rule]]
name = "high-error-rate"
description = "More than 10 errors per minute"
condition = "count(log_level == 'ERROR') > 10"
window = "1m"
severity = "warning"
labels = { team = "backend", service = "application" }

[[rule]]
name = "process-down"
description = "Critical process is not running"
condition = "process_running('nginx') == false"
duration = "30s"
severity = "critical"
labels = { team = "platform", service = "web" }
```

---

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metric_collection() {
        let cpu = get_cpu_usage();
        assert!(cpu >= 0.0 && cpu <= 100.0);
    }
    
    #[test]
    fn test_log_parsing() {
        let line = "2024-01-27 10:30:45 [ERROR] Connection timeout";
        let entry = parse_log_line(line);
        assert_eq!(entry.level, LogLevel::ERROR);
        assert!(entry.message.contains("timeout"));
    }
    
    #[test]
    fn test_alert_evaluation() {
        let mut engine = AlertEngine::new();
        engine.add_rule("cpu > 80", Duration::from_secs(300));
        
        // Simulate high CPU
        for _ in 0..10 {
            engine.evaluate("cpu", 85.0);
        }
        
        assert!(engine.has_active_alert("cpu > 80"));
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_agent_to_server_flow() {
    // Start test server
    let server = spawn_test_server().await;
    
    // Create test agent
    let agent = Agent::new("test-agent", server.url());
    
    // Send metric
    agent.send_metric("cpu_percent", 45.2).await.unwrap();
    
    // Query metric from server
    let metrics = server.query_metrics("test-agent").await.unwrap();
    
    assert_eq!(metrics.len(), 1);
    assert_eq!(metrics[0].value, 45.2);
}
```

### Load Testing
```bash
# Use Apache Bench to test server
ab -n 10000 -c 100 http://localhost:8080/api/v1/metrics

# Or use vegeta
echo "POST http://localhost:8080/api/v1/metrics" | \
  vegeta attack -duration=30s -rate=100 | \
  vegeta report
```

---

## Deployment

### Using Docker

**Agent Dockerfile:**
```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/monitor-agent /usr/local/bin/
COPY agent.toml /etc/monitor/agent.toml
CMD ["monitor-agent", "--config", "/etc/monitor/agent.toml"]
```

**Server Dockerfile:**
```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/monitor-server /usr/local/bin/
COPY server.toml /etc/monitor/server.toml
COPY dashboard /var/www/dashboard
EXPOSE 8080
CMD ["monitor-server", "--config", "/etc/monitor/server.toml"]
```

**Docker Compose:**
```yaml
version: '3.8'

services:
  server:
    build: ./server
    ports:
      - "8080:8080"
    volumes:
      - ./data:/data
    environment:
      - RUST_LOG=info
  
  agent:
    build: ./agent
    depends_on:
      - server
    environment:
      - SERVER_URL=http://server:8080
```

### Manual Deployment

```bash
# Build binaries
cargo build --release

# Copy to server
scp target/release/monitor-server user@server:/usr/local/bin/
scp target/release/monitor-agent user@agent:/usr/local/bin/

# Create systemd service
cat > /etc/systemd/system/monitor-agent.service <<EOF
[Unit]
Description=Monitoring Agent
After=network.target

[Service]
ExecStart=/usr/local/bin/monitor-agent --config /etc/monitor/agent.toml
Restart=always
User=monitor

[Install]
WantedBy=multi-user.target
EOF

systemctl enable monitor-agent
systemctl start monitor-agent
```

---

## Performance Considerations

### Agent Optimization
- **Batch metrics**: Send 10-20 data points per request instead of one at a time
- **Compression**: Gzip payloads before sending (reduces bandwidth by 70-80%)
- **Buffering**: Queue metrics locally if server is unreachable
- **Sampling**: For high-frequency metrics, sample at lower rate

### Server Optimization
- **Connection pooling**: Reuse database connections
- **Caching**: Cache frequently queried metrics in memory
- **Aggregation**: Pre-compute aggregates (avg, min, max) instead of calculating on-the-fly
- **Indexing**: Index by agent_id and timestamp for fast queries
- **Batch inserts**: Insert multiple metrics in single transaction

### Dashboard Optimization
- **Data downsampling**: For long time ranges, fetch aggregated data instead of raw points
- **Lazy loading**: Load charts on-demand as user scrolls
- **WebSocket instead of polling**: Reduces HTTP overhead for real-time updates
- **Client-side caching**: Cache metric data for recently viewed time ranges
- **Progressive rendering**: Show partial data while loading rest

### Memory Management
```rust
// Limit stored data points per metric
const MAX_POINTS_PER_METRIC: usize = 10_000;

impl TimeSeriesStore {
    fn insert(&mut self, series: String, point: DataPoint) {
        let points = self.data.entry(series).or_insert_with(Vec::new);
        points.push(point);
        
        // Keep only recent points
        if points.len() > MAX_POINTS_PER_METRIC {
            points.drain(0..points.len() - MAX_POINTS_PER_METRIC);
        }
    }
}
```

---

---

## Authentication & Dashboard Access Control

### Overview
The monitoring system implements a comprehensive API key-based authentication system that ensures secure access to both the agent metrics ingestion and the web dashboard. This provides:

- **Token-Based Authentication**: All API requests require a valid API key
- **Dashboard Access Control**: Users must authenticate before accessing the dashboard
- **Multi-Tenant Isolation**: Each API key can see only their associated agents and data
- **Admin Dashboard**: Centralized management of API keys, agents, and usage statistics

### Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                     Authentication Flow                         │
└────────────────────────────────────────────────────────────────┘

1. User Access Flow:
   ┌─────────┐      ┌──────────┐      ┌───────────┐
   │ Browser │─────▶│  Login   │─────▶│ Dashboard │
   │         │      │   Page   │      │  (Auth'd) │
   └─────────┘      └──────────┘      └───────────┘
                          │
                          │ Enter API Key
                          ▼
                    ┌──────────┐
                    │ Validate │
                    │   Key    │
                    └──────────┘

2. Agent Authentication:
   ┌─────────┐      ┌──────────────┐      ┌────────────┐
   │  Agent  │─────▶│ HTTP Request │─────▶│   Server   │
   │         │      │ X-API-Key:   │      │  Validates │
   └─────────┘      │ msk_xxx...   │      └────────────┘
                    └──────────────┘
```

### Dashboard Authentication

#### Login Page (`/login`)
When accessing the dashboard, users are presented with a login page where they must enter their API key:

**Features:**
- Secure API key input (password field)
- Validation against the server
- Error handling for invalid keys
- Automatic redirection after successful login
- API key stored in browser's localStorage

**Implementation:**
```typescript
// app/login/page.tsx
const handleLogin = async (e: React.FormEvent) => {
  e.preventDefault()
  setLoading(true)
  setError('')

  try {
    // Test the API key by making a request
    const response = await fetch(`${API_BASE_URL}/api/v1/agents`, {
      headers: { 'X-API-Key': apiKey }
    })

    if (response.ok) {
      // Store API key in localStorage
      localStorage.setItem('api_key', apiKey)
      router.push('/')
    } else {
      setError('Invalid API key')
    }
  } catch (err) {
    setError('Connection error')
  } finally {
    setLoading(false)
  }
}
```

#### Route Protection
All dashboard routes are protected by the `AuthProvider` component:

```typescript
// components/AuthProvider.tsx
export function AuthProvider({ children }: { children: React.ReactNode }) {
  const router = useRouter()
  const pathname = usePathname()

  useEffect(() => {
    const apiKey = localStorage.getItem('api_key')
    
    // Redirect to login if no API key and not already on login page
    if (!apiKey && pathname !== '/login') {
      router.push('/login')
    }
  }, [pathname, router])

  return <>{children}</>
}
```

#### API Request Authentication
All API requests include the API key in the `X-API-Key` header:

```typescript
// lib/auth-utils.ts
export function getAuthHeaders(): HeadersInit {
  const apiKey = localStorage.getItem('api_key')
  return apiKey ? { 'X-API-Key': apiKey } : {}
}

// lib/api.ts
export async function getAgents() {
  const response = await fetch(`${API_BASE_URL}/api/v1/agents`, {
    headers: getAuthHeaders()
  })
  return response.json()
}
```

### Admin Dashboard

#### Admin Overview (`/admin`)
Centralized dashboard showing system-wide statistics:

**Metrics Displayed:**
- Total active API keys
- Total agents (servers + databases)
- Server agent count
- Database agent count
- Active alerts count

**Features:**
- Real-time statistics
- Quick links to detailed pages
- Visual distribution charts
- Agent usage by API key
- Key status monitoring

#### API Key Management (`/admin/api-keys`)
Comprehensive interface for managing authentication tokens:

**Features:**
1. **Generate New Keys**
   - Custom key names
   - Optional descriptions
   - Expiration dates (7, 30, 90, 180, 365 days, or never)
   - Agent limits (restrict how many agents can use the key)

2. **View Key Details**
   - Creation date and creator
   - Last used timestamp
   - Expiration status
   - Active/Revoked/Expired status
   - List of agents using the key

3. **Key Actions**
   - Copy key to clipboard
   - Revoke keys (with confirmation)
   - View usage statistics

**Example: Creating an API Key**
```typescript
const createKey = async () => {
  const response = await createApiKey({
    name: 'production-web-servers',
    description: 'Keys for all production web server agents',
    expires_in_days: 90,
    max_agents: 10
  })
  
  // Response contains the generated key (only shown once)
  console.log(response.key) // msk_prod_abc123...
}
```

### Server-Side Authentication

#### API Key Validation
The server validates API keys on every request:

```rust
// server/src/middleware/auth.rs
pub async fn auth_middleware(
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req.headers()
        .get("Authorization")
        .or_else(|| req.headers().get("X-API-Key"))
        .and_then(|h| h.to_str().ok());

    let api_key = match auth_header {
        Some(key) if key.starts_with("Bearer ") => &key[7..],
        Some(key) => key,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    // Validate key against database
    let auth_manager = req.extensions().get::<AuthManager>().unwrap();
    match auth_manager.validate_api_key(api_key).await {
        Ok(_) => Ok(next.run(req).await),
        Err(_) => Err(StatusCode::FORBIDDEN),
    }
}
```

#### Data Isolation
Each API key only has access to its associated agents and metrics:

```rust
// server/src/handlers/metrics.rs
pub async fn get_agents(
    Extension(auth): Extension<AuthContext>,
    Extension(db): Extension<Database>,
) -> Result<Json<Vec<Agent>>> {
    // Only return agents associated with this API key
    let agents = db.get_agents_by_api_key(&auth.api_key).await?;
    Ok(Json(agents))
}
```

### Database Schema

```sql
-- API Keys table
CREATE TABLE api_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP,
    last_used_at TIMESTAMP,
    revoked BOOLEAN DEFAULT FALSE,
    revoked_at TIMESTAMP,
    created_by TEXT NOT NULL,
    max_agents INTEGER
);

-- Agent tracking with API keys
CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    agent_type TEXT NOT NULL,
    api_key_id INTEGER,
    last_seen TIMESTAMP,
    FOREIGN KEY (api_key_id) REFERENCES api_keys(id)
);
```

### Usage Examples

#### 1. Setting Up a New API Key
```bash
# Create a new API key via the admin dashboard
# Navigate to /admin/api-keys
# Click "Generate New Key"
# Enter:
#   - Name: "production-servers"
#   - Description: "Keys for production server agents"
#   - Expires In: 90 days
#   - Max Agents: 5
# Copy the generated key: msk_prod_abc123xyz...
```

#### 2. Configuring an Agent
```toml
# agent.toml
[agent]
id = "web-server-01"
name = "Production Web Server 1"
agent_type = "server"

[server]
url = "https://monitoring.example.com"
api_key = "msk_prod_abc123xyz..."

[collection]
interval_seconds = 15
```

#### 3. Accessing the Dashboard
```bash
# 1. Navigate to https://monitoring.example.com
# 2. You'll be redirected to /login
# 3. Enter your API key
# 4. After validation, you'll see only your agents and metrics
```

#### 4. Logout
```bash
# Click the "Logout" button in the sidebar footer
# This clears the API key from localStorage
# You'll be redirected back to the login page
```

### Security Best Practices

1. **Key Format**: API keys use the format `msk_<environment>_<random>`
   - `msk_`: Prefix for "monitoring system key"
   - Easy to identify in logs and code
   - Prevents accidental commits (can be detected by secret scanners)

2. **Key Storage**:
   - **Server**: Keys are hashed using bcrypt before storage
   - **Client**: Keys stored in localStorage (browser-only, not accessible to other domains)
   - **Agent**: Keys stored in config file with restricted permissions (600)

3. **Key Rotation**:
   - Set expiration dates on all keys
   - Regular rotation (90 days recommended)
   - Revoke old keys after agent updates

4. **Monitoring**:
   - Track last_used_at for each key
   - Alert on unused keys (potential security risk)
   - Monitor for suspicious usage patterns

### Logout Implementation

Users can logout from the dashboard:

```typescript
// components/Sidebar.tsx
<button 
  onClick={() => {
    localStorage.removeItem('api_key')
    router.push('/login')
  }}
  className="logout-button"
>
  <LogOut className="w-5 h-5" />
  <span>Logout</span>
</button>
```

### Multi-Tenant Support

The authentication system enables multi-tenant deployments:

1. **Isolated Dashboards**: Each team/user sees only their agents
2. **Usage Limits**: Control agent count per API key
3. **Usage Tracking**: Monitor API key usage for billing or quotas
4. **Admin Access**: Separate admin keys can view all agents

---

## Security Considerations

### Agent Authentication
```rust
// Server validates agent API key
async fn validate_agent(
    headers: HeaderMap,
    db: Extension<Database>,
) -> Result<Agent, StatusCode> {
    let api_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    db.get_agent_by_key(api_key)
        .await
        .ok_or(StatusCode::FORBIDDEN)
}
```

### Rate Limiting
```rust
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

let governor_conf = Box::new(
    GovernorConfigBuilder::default()
        .per_second(10)  // Max 10 requests per second per IP
        .burst_size(20)
        .finish()
        .unwrap(),
);

let app = Router::new()
    .route("/api/v1/metrics", post(ingest_metrics))
    .layer(GovernorLayer { config: governor_conf });
```

### HTTPS/TLS
```rust
// Use rustls for TLS
use axum_server::tls_rustls::RustlsConfig;

let config = RustlsConfig::from_pem_file(
    "/path/to/cert.pem",
    "/path/to/key.pem",
).await?;

axum_server::bind_rustls("0.0.0.0:8443".parse()?, config)
    .serve(app.into_make_service())
    .await?;
```

### Data Sanitization
```rust
fn sanitize_log_message(msg: &str) -> String {
    // Remove potential injection attacks
    msg.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || ".,!?-_:".contains(*c))
        .take(1000)  // Limit length
        .collect()
}
```

---

## Advanced Features (Phase 8+)

### 1. Distributed Tracing Integration
Add trace context to logs for request correlation:
```rust
struct LogEntry {
    // ... existing fields
    trace_id: Option<String>,
    span_id: Option<String>,
    parent_span_id: Option<String>,
}
```

### 2. Anomaly Detection
Use statistical methods to detect unusual patterns:
```rust
struct AnomalyDetector {
    baseline: HashMap<String, Statistics>,
}

impl AnomalyDetector {
    fn is_anomaly(&self, metric: &str, value: f64) -> bool {
        if let Some(stats) = self.baseline.get(metric) {
            // Value is anomalous if > 3 standard deviations from mean
            let z_score = (value - stats.mean) / stats.stddev;
            z_score.abs() > 3.0
        } else {
            false
        }
    }
}
```

### 3. Predictive Alerts
Predict when metrics will breach thresholds:
```rust
fn predict_disk_full(current_usage: f64, growth_rate: f64) -> Option<DateTime<Utc>> {
    let remaining = 100.0 - current_usage;
    if growth_rate <= 0.0 {
        return None;
    }
    
    let hours_until_full = remaining / growth_rate;
    Some(Utc::now() + Duration::hours(hours_until_full as i64))
}
```

### 4. Custom Metrics
Allow agents to send application-specific metrics:
```rust
// Agent code
monitor.record_custom_metric("api.response_time", 45.2, "milliseconds");
monitor.record_custom_metric("db.query_count", 1523.0, "count");
monitor.record_custom_metric("cache.hit_rate", 87.5, "percent");
```

### 5. Correlation Engine
Find relationships between metrics:
```rust
// When CPU spikes, also check:
// - Memory usage
// - Disk I/O
// - Network traffic
// - Application error logs
// Show correlated metrics on dashboard
```

### 6. Historical Comparison
Compare current metrics to historical data:
```rust
// "CPU is 20% higher than same time yesterday"
// "Memory usage is at weekly peak"
// "Error rate is 5x normal for this hour"
```

### 7. Multi-Tenancy
Support multiple teams/projects on same server:
```rust
struct Agent {
    id: String,
    tenant_id: String,  // Isolate data by tenant
    // ...
}

// Dashboard shows only tenant's agents
```

### 8. Synthetic Monitoring
Periodically test endpoints are responding:
```rust
async fn synthetic_check(url: &str) -> HealthCheck {
    let start = Instant::now();
    let response = reqwest::get(url).await;
    let duration = start.elapsed();
    
    HealthCheck {
        url: url.to_string(),
        status: response.map(|r| r.status().as_u16()).ok(),
        response_time_ms: duration.as_millis() as u64,
        timestamp: Utc::now(),
    }
}
```

### 9. Incident Management
Link alerts to incidents:
```rust
struct Incident {
    id: String,
    title: String,
    severity: Severity,
    status: IncidentStatus,  // Open, Acknowledged, Resolved
    alerts: Vec<String>,     // Related alert IDs
    created_at: DateTime<Utc>,
    resolved_at: Option<DateTime<Utc>>,
    responders: Vec<String>,
}
```

### 10. Integrations
Connect to external services:
- **Slack**: Send alert notifications
- **PagerDuty**: Create incidents
- **Jira**: Auto-create tickets for critical alerts
- **Webhook**: POST to any URL on alert

```rust
async fn send_slack_alert(alert: &Alert, webhook_url: &str) {
    let payload = json!({
        "text": format!("🚨 Alert: {}", alert.message),
        "attachments": [{
            "color": match alert.severity {
                Severity::Critical => "danger",
                Severity::Warning => "warning",
                _ => "good"
            },
            "fields": [
                { "title": "Agent", "value": alert.agent_id, "short": true },
                { "title": "Severity", "value": format!("{:?}", alert.severity), "short": true }
            ]
        }]
    });
    
    reqwest::Client::new()
        .post(webhook_url)
        .json(&payload)
        .send()
        .await
        .ok();
}
```

---

## Monitoring Best Practices

### What Metrics to Monitor (The Four Golden Signals)

1. **Latency**: How long operations take
   - API response times
   - Database query duration
   - Cache hit/miss latency

2. **Traffic**: How much demand on the system
   - Requests per second
   - Concurrent connections
   - Bandwidth usage

3. **Errors**: Rate of failed operations
   - HTTP 5xx errors
   - Exception count
   - Failed database queries

4. **Saturation**: How "full" the system is
   - CPU usage
   - Memory usage
   - Disk space
   - Connection pool utilization

### Alert Fatigue Prevention
- **Use duration thresholds**: Alert only if condition persists (e.g., CPU > 80% for 5 minutes)
- **Group related alerts**: Don't send 10 alerts for same root cause
- **Implement alert routing**: Send critical alerts to on-call, warnings to Slack
- **Add runbooks**: Every alert should link to resolution steps
- **Regular review**: Disable/adjust noisy alerts

### Dashboard Design Principles
- **Most important info first**: Critical alerts and system health at top
- **Use color meaningfully**: Red = critical, Yellow = warning, Green = healthy
- **Show trends**: Not just current value but direction (↑ increasing, ↓ decreasing)
- **Context matters**: Show "CPU: 80%" AND "Average: 45%, Peak: 95%"
- **Make it scannable**: Large, clear numbers for key metrics

---

## Project Structure

```
monitoring-system/
├── agent/
│   ├── src/
│   │   ├── main.rs
│   │   ├── collector/
│   │   │   ├── mod.rs
│   │   │   ├── cpu.rs
│   │   │   ├── memory.rs
│   │   │   ├── disk.rs
│   │   │   └── network.rs
│   │   ├── logs/
│   │   │   ├── mod.rs
│   │   │   ├── watcher.rs
│   │   │   └── parser.rs
│   │   ├── sender.rs
│   │   └── config.rs
│   ├── Cargo.toml
│   └── agent.toml
├── server/
│   ├── src/
│   │   ├── main.rs
│   │   ├── api/
│   │   │   ├── mod.rs
│   │   │   ├── metrics.rs
│   │   │   ├── logs.rs
│   │   │   └── agents.rs
│   │   ├── storage/
│   │   │   ├── mod.rs
│   │   │   ├── timeseries.rs
│   │   │   └── logs.rs
│   │   ├── alerts/
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs
│   │   │   └── rules.rs
│   │   └── config.rs
│   ├── Cargo.toml
│   └── server.toml
├── dashboard/
│   ├── index.html
│   ├── css/
│   │   └── style.css
│   ├── js/
│   │   ├── main.js
│   │   ├── charts.js
│   │   └── logs.js
│   └── assets/
├── docs/
│   ├── README.md
│   ├── ARCHITECTURE.md
│   ├── API.md
│   ├── DEPLOYMENT.md
│   └── DEVELOPMENT.md
├── tests/
│   ├── integration/
│   └── load/
├── docker/
│   ├── agent.Dockerfile
│   ├── server.Dockerfile
│   └── docker-compose.yml
└── README.md
```

---

## Documentation Requirements

### README.md
```markdown
# DevOps Monitoring System

Real-time infrastructure monitoring and log aggregation built with Rust.

## Features
- System metrics collection (CPU, memory, disk, network)
- Log aggregation from multiple sources
- Real-time alerting
- Web dashboard with live updates
- Multi-server support

## Quick Start
\`\`\`bash
# Start server
docker-compose up -d server

# Run agent
docker run -v /var/log:/var/log monitoring-agent
\`\`\`

## Documentation
- [Architecture](docs/ARCHITECTURE.md)
- [API Reference](docs/API.md)
- [Deployment Guide](docs/DEPLOYMENT.md)

## Screenshots
[Dashboard showing real-time metrics]
```

### ARCHITECTURE.md
Detailed explanation of:
- System design decisions
- Component interactions
- Data flow diagrams
- Scalability considerations

### API.md
Complete API documentation:
- All endpoints with examples
- Request/response schemas
- Authentication requirements
- Rate limiting details

### DEPLOYMENT.md
Step-by-step deployment guide:
- Prerequisites
- Installation steps
- Configuration options
- Troubleshooting

---

## Common Issues and Solutions

### Issue: Agent can't reach server
**Symptoms:** Agent logs show connection errors
**Solutions:**
- Check network connectivity: `ping server-hostname`
- Verify server is running: `curl http://server:8080/health`
- Check firewall rules
- Verify API key is correct

### Issue: Metrics not appearing in dashboard
**Symptoms:** Dashboard shows no data
**Debug steps:**
1. Check agent is sending: Look at agent logs
2. Check server receiving: Look at server logs
3. Verify API endpoint: `curl http://server:8080/api/v1/metrics`
4. Check browser console for JavaScript errors

### Issue: High memory usage on server
**Symptoms:** Server using too much RAM
**Solutions:**
- Reduce data retention period
- Enable data aggregation
- Move to persistent storage (database)
- Increase cleanup frequency

### Issue: Dashboard is slow
**Symptoms:** Charts take long to load
**Solutions:**
- Reduce time range being queried
- Use aggregated data for long ranges
- Add data point limits per chart
- Enable browser caching

---

## Performance Benchmarks

### Expected Performance (Single Server)

**Agent:**
- CPU overhead: < 2%
- Memory usage: < 50 MB
- Disk I/O: Negligible
- Network: ~1 KB/s per metric

**Server:**
- Handle 100 agents simultaneously
- Store 1M metric points in memory
- Process 1000 log entries per second
- Serve dashboard to 10 concurrent users

**Dashboard:**
- Load time: < 2 seconds
- Chart render: < 500ms
- Real-time update latency: < 1 second

### Scaling Limits

**Without optimization:**
- Max agents: ~100
- Max metrics/sec: ~1000
- Max log entries/sec: ~500

**With optimization (Phase 7+):**
- Max agents: ~1000+
- Max metrics/sec: ~10,000+
- Max log entries/sec: ~5000+
- Use PostgreSQL + Redis for larger scale

---

## Interview Preparation

### Questions You Should Be Able to Answer

**Architecture:**
- "Walk me through your system architecture"
- "Why did you choose this tech stack?"
- "How does the agent communicate with the server?"
- "How do you handle network failures?"

**Monitoring:**
- "What metrics are most important to monitor?"
- "How do you prevent alert fatigue?"
- "What's the difference between metrics and logs?"
- "How would you monitor a distributed system?"

**Performance:**
- "How do you optimize for high data volumes?"
- "What's your data retention strategy?"
- "How do you handle time-series data efficiently?"

**Reliability:**
- "What happens if the server goes down?"
- "How do you ensure agents don't lose data?"
- "What's your disaster recovery plan?"

**Scaling:**
- "How would you scale this to 1000 servers?"
- "What would you change for global deployment?"
- "How would you add high availability?"

### Demo Script for Interviews

1. **Show the architecture diagram** (2 min)
   - Explain agent, server, dashboard
   
2. **Live demo** (5 min)
   - Open dashboard showing multiple servers
   - Point out real-time metrics updating
   - Show logs streaming in
   - Trigger an alert (simulate high CPU)
   - Show alert appearing on dashboard
   
3. **Code walkthrough** (5 min)
   - Show metric collection code
   - Explain API endpoints
   - Show alert rule evaluation
   
4. **Challenges and solutions** (3 min)
   - "Initially struggled with X, solved by Y"
   - "Learned about time-series databases"
   - "Had to optimize for memory usage"

---

## Resources and Learning

### Rust Resources
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial) - Async runtime
- [Axum Examples](https://github.com/tokio-rs/axum/tree/main/examples) - Web framework
- [The Rust Book](https://doc.rust-lang.org/book/) - Language fundamentals

### Monitoring Concepts
- [Google SRE Book](https://sre.google/books/) - Monitoring best practices
- [Prometheus Documentation](https://prometheus.io/docs/) - Metrics patterns
- [The Four Golden Signals](https://sre.google/sre-book/monitoring-distributed-systems/) - What to monitor

### Similar Projects (Study These)
- [Netdata](https://github.com/netdata/netdata) - Real-time monitoring
- [Vector](https://github.com/vectordotdev/vector) - Log collection
- [Telegraf](https://github.com/influxdata/telegraf) - Metrics agent

### DevOps Tools to Know
- **Prometheus**: Time-series metrics database
- **Grafana**: Visualization and dashboards
- **ELK Stack**: Elasticsearch, Logstash, Kibana for logs
- **Datadog/New Relic**: Commercial monitoring platforms

---

## Success Criteria

### MVP (Minimum Viable Product) Checklist
- [ ] Agent collects CPU, memory, disk metrics
- [ ] Agent sends metrics to server via HTTP
- [ ] Server stores metrics (in-memory is fine)
- [ ] API endpoints for querying metrics
- [ ] Dashboard shows real-time charts
- [ ] Basic log collection and viewing
- [ ] At least one working alert rule
- [ ] Documentation (README + API docs)
- [ ] Can demo end-to-end flow

### Production-Ready Checklist
- [ ] Persistent storage (SQLite/PostgreSQL)
- [ ] Authentication and API keys
- [ ] Multiple servers being monitored
- [ ] Alert engine with multiple rules
- [ ] Data retention and cleanup
- [ ] Docker deployment
- [ ] Comprehensive documentation
- [ ] Unit and integration tests
- [ ] Performance benchmarks documented

### Portfolio-Ready Checklist
- [ ] GitHub repo with good README
- [ ] Architecture diagrams
- [ ] Screenshots of dashboard
- [ ] Video demo (2-3 minutes)
- [ ] Blog post explaining the project
- [ ] Code is clean and well-commented
- [ ] Deployment instructions tested

---

## Timeline Summary

| Week | Phase | Deliverable | Effort |
|------|-------|-------------|--------|
| 1-2 | Agent Basics | Agent collecting metrics locally | 15-20h |
| 2-3 | Server API | Server receiving and storing metrics | 15-20h |
| 3 | Integration | Agent sending to server | 10h |
| 3-4 | Dashboard | Basic web UI with charts | 15-20h |
| 4-5 | Logs | Log collection and viewing | 15-20h |
| 5-6 | Alerts | Alert engine and rules | 10-15h |
| 6+ | Polish | Documentation, testing, deployment | 10-15h |

**Total: 90-130 hours** over 6-8 weeks

**Realistic schedule:**
- 10-15 hours/week = 6-8 weeks
- 20 hours/week = 4-5 weeks
- Full-time (40h/week) = 2-3 weeks

---

## Final Thoughts

This project is **ambitious but absolutely doable**. You'll learn:
- Real systems programming in Rust
- API design and web development
- How production monitoring actually works
- Performance optimization
- DevOps best practices

**Most importantly:** You'll have a project that makes interviewers say "Wow, you actually built this?"

The key is to **build incrementally**:
1. Make it work (MVP)
2. Make it right (clean code, tests)
3. Make it fast (optimization)
4. Make it production-ready (deployment, docs)

Don't try to do everything at once. Each phase should be working and testable before moving to the next.

**You've got this!** 💪

---

## Getting Help

When you get stuck:
1. **Read the error message carefully** - Rust errors are very helpful
2. **Check the documentation** - Most crates have great examples
3. **Search GitHub issues** - Someone probably hit the same problem
4. **Ask specific questions** - "How do I X?" vs "Nothing works"
5. **Share code snippets** - Makes debugging much easier

**I'm here to help you through every phase.** Just ask! 🚀