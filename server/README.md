# ⚙️ WatchTower Server

<p align="center">
  <strong>High-performance central monitoring server for WatchTower platform - built with Rust, Axum, and Tokio</strong>
</p>

---

## Overview

The Monitoring Server is the heart of the system - a production-grade backend that receives metrics from agents, stores them efficiently, broadcasts real-time updates via WebSocket, evaluates alert rules, and serves data to the dashboard.

### Key Features

- ✅ **High Throughput**: 10,000+ metrics/second
- ✅ **Real-time WebSocket**: Instant metric broadcasting
- ✅ **Persistent Storage**: PostgreSQL with in-memory caching
- ✅ **Alert Engine**: Rule-based alerting system
- ✅ **API Key Authentication**: Secure multi-tenant access
- ✅ **RESTful API**: Comprehensive HTTP endpoints
- ✅ **Auto-Migrations**: Database schema management
- ✅ **Production-Ready**: Error handling, logging, monitoring

---

## Architecture

```
┌────────────────────────────────────────────────────────────┐
│                   MONITORING SERVER                        │
│                  (Axum + Tokio Runtime)                    │
│                                                            │
│  ┌────────────────────────────────────────────────────┐    │
│  │             HTTP Layer (Axum Router)               │    │
│  ├────────────────────────────────────────────────────┤    │
│  │  POST  /api/v1/metrics      - Ingest metrics       │    │
│  │  POST  /api/v1/logs         - Ingest logs          │    │
│  │  GET   /api/v1/agents       - List agents          │    │
│  │  GET   /api/v1/metrics      - Query metrics        │    │
│  │  GET   /api/v1/logs         - Query logs           │    │
│  │  GET   /api/v1/alerts       - Get alerts           │    │
│  │  POST  /api/v1/auth/keys    - Create API key       │    │
│  │  WS    /api/v1/ws/metrics   - WebSocket stream     │    │
│  │  WS    /api/v1/ws/logs      - Logs WebSocket       │    │
│  │  WS    /api/v1/ws/alerts    - Alerts WebSocket     │    │
│  └────────────────────────────────────────────────────┘    │
│                         │                                  │
│                         ▼                                  │
│  ┌────────────────────────────────────────────────────┐    │
│  │           Authentication Middleware                │    │
│  │  - API key validation                              │    │
│  │  - Agent limit enforcement                         │    │
│  │  - Multi-tenancy isolation                         │    │
│  └────────────────────────────────────────────────────┘    │
│                         │                                  │
│        ┌────────────────┼────────────────┐                 │
│        │                │                │                 │
│        ▼                ▼                ▼                 │
│  ┌─────────┐    ┌──────────────┐  ┌──────────────┐         │
│  │  Auth   │    │   Storage    │  │   WebSocket  │         │
│  │ Service │    │    Layer     │  │   Manager    │         │
│  └─────────┘    └──────────────┘  └──────────────┘         │
│        │                │                  │               │
│        │                │                  │               │
│  ┌─────▼────────────────▼──────────────────▼─────────┐     │
│  │           Database & Cache Layer                  │     │
│  │  ┌─────────────┐          ┌──────────────────┐    │     │
│  │  │ PostgreSQL  │◄────────▶│  In-Memory Cache │    │     │
│  │  │  (Persist)  │          │    (HashMap)     │    │     │
│  │  └─────────────┘          └──────────────────┘    │     │
│  └───────────────────────────────────────────────────┘     │
│                                                            │
│  ┌────────────────────────────────────────────────────┐    │
│  │              Alert Engine                          │    │
│  │  - Rule evaluation (every 30s)                     │    │
│  │  - Threshold detection                             │    │
│  │  - Alert state management                          │    │
│  │  - WebSocket alert broadcasting                    │    │
│  └────────────────────────────────────────────────────┘    │
└────────────────────────────────────────────────────────────┘
```

---

## Installation

### Build from Source

```bash
# Clone repository
git clone https://github.com/yourusername/devops-monitoring-system.git
cd devops-monitoring-system/server

# Build release binary
cargo build --release

# Binary at: target/release/monitor-server
./target/release/monitor-server --config config.toml
```


---

## Configuration

Create `config.toml`:

```toml
# Server Configuration

[server]
# Bind address
host = "0.0.0.0"

# Port to listen on
port = 8080

# Number of worker threads
workers = 4

[database]
# Database connection URL
url = "postgresql://monitoring_user:password@localhost:5432/monitoring"

# Connection pool settings
max_connections = 10
min_connections = 2
connection_timeout_seconds = 30

[storage]
# In-memory cache settings
max_points_per_metric = 10000

# Data retention
retention_days = 30

[auth]
# Enable API key authentication
enabled = true

# Require API key for all requests
require_api_key = true

# Default key expiration (days)
default_expiration_days = 90

[alerts]
# Enable alert engine
enabled = true

# Alert evaluation interval (seconds)
check_interval_seconds = 30

# Alert history retention (days)
history_retention_days = 90

[websocket]
# Heartbeat interval (seconds)
heartbeat_interval_seconds = 30

# Max concurrent connections per client
max_connections_per_client = 5

[logging]
# Log level: trace, debug, info, warn, error
level = "info"

# Log format: json, pretty
format = "pretty"
```

### Environment Variables

```bash
# Override configuration
export SERVER_HOST="0.0.0.0"
export SERVER_PORT="8080"
export DATABASE_URL="postgresql://user:pass@localhost:5432/monitoring"
export AUTH_ENABLED="true"
export AUTH_REQUIRE_API_KEY="true"
export RUST_LOG="info"

./monitor-server
```

---

## API Endpoints

### Metrics Ingestion

**POST** `/api/v1/metrics`

Ingest metrics from agents.

**Headers:**
- `Content-Type: application/json`
- `X-API-Key: <api-key>`

**Request Body:**
```json
{
  "agent_id": "web-server-01",
  "timestamp": "2026-02-01T12:34:56Z",
  "metrics": {
    "cpu_usage": 45.2,
    "memory_usage": 67.5,
    "disk_usage": 58.9
  }
}
```

**Response:**
```json
{
  "status": "accepted"
}
```

---

### Query Metrics

**GET** `/api/v1/metrics/latest?agent_id=<agent-id>`

Get latest metrics for an agent.

**Response:**
```json
{
  "agent_id": "web-server-01",
  "metrics": [
    {
      "name": "cpu_usage",
      "value": 45.2,
      "timestamp": "2026-02-01T12:34:56Z"
    }
  ]
}
```

---

### List Agents

**GET** `/api/v1/agents`

Get all registered agents.

**Response:**
```json
[
  {
    "id": "web-server-01",
    "name": "web-server-01",
    "status": "Healthy",
    "last_seen": "2026-02-01T12:34:56Z"
  }
]
```

---

### Create API Key

**POST** `/api/v1/auth/keys`

Generate a new API key.

**Request:**
```json
{
  "name": "production-agents",
  "description": "Keys for production servers",
  "max_agents": 10,
  "expires_in_days": 90
}
```

**Response:**
```json
{
  "id": 1,
  "key": "msk_prod_abc123xyz...",
  "name": "production-agents",
  "created_at": "2026-02-01T12:00:00Z"
}
```

⚠️ **Note**: The API key is only shown once!

---

### WebSocket Endpoints

#### Metrics Stream

**WS** `/api/v1/ws/metrics`

Real-time metric updates.

**Messages:**

Initial state on connect:
```json
{
  "type": "initial_state",
  "agents": [...],
  "metrics": [...]
}
```

Real-time updates:
```json
{
  "type": "metric",
  "agent_id": "web-server-01",
  "metric_name": "cpu_usage",
  "value": 45.2,
  "timestamp": "2026-02-01T12:34:56Z"
}
```

---

## Database Schema

### Tables

```sql
-- API Keys
CREATE TABLE api_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP,
    last_used_at TIMESTAMP,
    revoked BOOLEAN DEFAULT FALSE,
    max_agents INTEGER
);

-- Agents
CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    agent_type TEXT NOT NULL,
    api_key_id INTEGER,
    last_seen TIMESTAMP,
    FOREIGN KEY (api_key_id) REFERENCES api_keys(id)
);

-- Metrics
CREATE TABLE metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    metric_name TEXT NOT NULL,
    value REAL NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    FOREIGN KEY (agent_id) REFERENCES agents(id)
);

-- Alerts
CREATE TABLE alerts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    rule_id INTEGER NOT NULL,
    rule_name TEXT NOT NULL,
    severity TEXT NOT NULL,
    message TEXT NOT NULL,
    triggered_at TIMESTAMP NOT NULL,
    resolved_at TIMESTAMP,
    status TEXT NOT NULL,
    FOREIGN KEY (agent_id) REFERENCES agents(id)
);
```

---

## Running

### Development

```bash
# Run with live reloading
cargo watch -x run

# Run with debug logging
RUST_LOG=debug cargo run

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Production

```bash
# Build optimized binary
cargo build --release

# Run with systemd
sudo systemctl start monitor-server

# Check status
sudo systemctl status monitor-server

# View logs
journalctl -u monitor-server -f
```

---

## Performance

### Benchmarks

| Metric | Value |
|--------|-------|
| Throughput | 10,000+ metrics/sec |
| Latency | < 5ms per request |
| Memory | ~200MB for 1M metrics |
| WebSocket | < 1s delivery |

### Optimization

- In-memory cache for fast queries
- Connection pooling (10 connections)
- Batch database inserts
- Async I/O with Tokio
- Zero-copy serialization

---

## Monitoring

### Health Check

**GET** `/health`

```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime": "3h 45m"
}
```

### Metrics

The server exposes Prometheus-compatible metrics at `/metrics`:

- `http_requests_total`
- `http_request_duration_seconds`
- `websocket_connections_active`
- `metrics_ingested_total`
- `database_connections_active`

---

## Testing

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test '*'

# With coverage
cargo tarpaulin

# Load testing
ab -n 10000 -c 100 http://localhost:8080/api/v1/agents
```

---

## Troubleshooting

### Database connection errors

```bash
# Test PostgreSQL connection
psql -h localhost -U monitoring_user -d monitoring

# Check connection pool
curl http://localhost:8080/health
```

### High memory usage

- Reduce `max_points_per_metric` in config
- Enable data cleanup: `retention_days = 7`
- Increase database flush frequency

### WebSocket disconnections

- Check firewall rules
- Increase `heartbeat_interval_seconds`
- Check nginx/proxy timeout settings

---

## License

MIT License - See [LICENSE](../LICENSE) for details.

---

<p align="center">
  Built with ❤️ using Rust, Axum, and Tokio
</p>
