# Monitor Server

Phase 2 central monitoring server that receives and stores metrics from agents.

## Building

```bash
cd server
cargo build --release
```

## Running

Run with default configuration:
```bash
cargo run
```

Run with custom configuration file:
```bash
cargo run -- --config server.toml
```

Run with custom host and port:
```bash
cargo run -- --host 127.0.0.1 --port 9090
```

Run with verbose logging:
```bash
cargo run -- --verbose
```

## Configuration

Edit `server.toml` to configure the server:

```toml
[server]
host = "0.0.0.0"
port = 8080

[storage]
max_points_per_metric = 10000
```

## API Endpoints

### Health Check
```bash
GET /health
```

### Ingest Metrics
```bash
POST /api/v1/metrics
Content-Type: application/json

{
  "agent_id": "web-server-01",
  "timestamp": "2026-01-27T10:30:45Z",
  "metrics": {
    "cpu_percent": 45.2,
    "memory_used_bytes": 4294967296,
    "memory_total_bytes": 8589934592,
    "disk_used_bytes": 107374182400,
    "disk_total_bytes": 536870912000
  }
}
```

### Query Metrics
```bash
GET /api/v1/metrics?agent_id=web-server-01&metric=cpu_percent&limit=100
GET /api/v1/metrics?agent_id=web-server-01&metric=memory_used_bytes&from=2026-01-27T10:00:00Z&to=2026-01-27T11:00:00Z
```

### Get Latest Metrics
```bash
GET /api/v1/metrics/latest?agent_id=web-server-01
```

### List Agents
```bash
GET /api/v1/agents
```

### Get Agent Details
```bash
GET /api/v1/agents/:agent_id?agent_id=web-server-01
```

### Storage Statistics
```bash
GET /api/v1/stats
```

## Testing with curl

**Send a metric**:
```bash
curl -X POST http://localhost:8080/api/v1/metrics \
  -H "Content-Type: application/json" \
  -d '{
    "agent_id": "test-agent",
    "timestamp": "2026-01-27T10:30:45Z",
    "metrics": {
      "cpu_percent": 45.2,
      "memory_used_bytes": 4294967296
    }
  }'
```

**Query metrics**:
```bash
curl "http://localhost:8080/api/v1/metrics?agent_id=test-agent&metric=cpu_percent"
```

**List agents**:
```bash
curl http://localhost:8080/api/v1/agents
```

**Get stats**:
```bash
curl http://localhost:8080/api/v1/stats
```

## Features

- ✅ REST API with Axum web framework
- ✅ In-memory time-series storage
- ✅ Agent registration and tracking
- ✅ Metrics ingestion endpoint
- ✅ Metrics query with time range filtering
- ✅ Latest metrics retrieval
- ✅ Agent status monitoring (Healthy/Degraded/Unreachable)
- ✅ Storage statistics
- ✅ Health check endpoint
- ✅ CORS support for dashboard
- ✅ Request tracing and logging
- ✅ Configurable via TOML file
- ✅ CLI argument overrides

## Architecture

```
POST /api/v1/metrics
         │
         ▼
   Metrics Parser
         │
         ▼
  Agent Registration
         │
         ▼
  Time-Series Storage
  (HashMap in Memory)
         │
         ▼
  GET /api/v1/metrics
```

## Storage Structure

```
HashMap {
  "agent-1" => {
    "cpu_percent" => [
      DataPoint { timestamp, value },
      DataPoint { timestamp, value },
      ...
    ],
    "memory_used_bytes" => [...]
  },
  "agent-2" => {...}
}
```

## Next Steps

Phase 3 will connect the agent to send metrics to this server automatically.
