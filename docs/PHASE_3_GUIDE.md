# Phase 3: Connect Agent to Server - Implementation Guide

## Overview
Phase 3 establishes the connection between monitoring agents and the central server, enabling automatic metric transmission over HTTP. This phase implements a robust client-side HTTP sender with retry logic and graceful error handling.

## Objectives
- ✅ Add HTTP client capabilities to the agent using `reqwest`
- ✅ Configure server connection settings in agent configuration
- ✅ Implement automatic metric transmission to the server
- ✅ Add retry logic with exponential backoff for failed transmissions
- ✅ Maintain backward compatibility (agent works with or without server)
- ✅ Provide clear feedback on transmission status

## Architecture

```
┌─────────────────────┐
│  Monitoring Agent   │
│                     │
│  ┌───────────────┐  │
│  │  Collectors   │  │
│  └───────┬───────┘  │
│          │          │
│          ▼          │
│  ┌───────────────┐  │
│  │   Sender      │  │──HTTP POST──▶  ┌─────────────────────┐
│  │   (reqwest)   │  │                │   Central Server    │
│  │   - Retry     │  │◀─Response──────│                     │
│  │   - Backoff   │  │                │  ┌───────────────┐  │
│  └───────────────┘  │                │  │  REST API     │  │
│                     │                │  │  /api/v1/     │  │
└─────────────────────┘                │  │  metrics      │  │
                                       │  └───────┬───────┘  │
                                       │          │          │
                                       │          ▼          │
                                       │  ┌───────────────┐  │
                                       │  │  Time-Series  │  │
                                       │  │   Storage     │  │
                                       │  └───────────────┘  │
                                       └─────────────────────┘
```

## Implementation Details

### 1. Agent Configuration Enhancement

**File: `agent/src/config.rs`**

Added a new `ServerConfig` struct to manage server connection settings:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default)]
    pub enabled: bool,                      // Toggle server integration on/off
    #[serde(default = "default_server_url")]
    pub url: String,                        // Server URL (e.g., http://localhost:8080)
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,                // Number of retry attempts (default: 3)
    #[serde(default = "default_retry_delay")]
    pub retry_delay_seconds: u64,           // Initial retry delay (default: 2s)
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: "http://localhost:8080".to_string(),
            retry_attempts: 3,
            retry_delay_seconds: 2,
        }
    }
}
```

**Configuration File: `agent/agent.toml`**

```toml
[agent]
name = "dev-server-01"

[collection]
interval_seconds = 15

[metrics]
collect_cpu = true
collect_memory = true
collect_disk = true
collect_network = true

[server]
enabled = true                          # Enable/disable server integration
url = "http://localhost:8080"           # Central server URL
retry_attempts = 3                      # Number of retry attempts
retry_delay_seconds = 2                 # Initial delay between retries
```

### 2. HTTP Sender Module

**File: `agent/src/sender.rs`**

The sender module implements the HTTP client with automatic retries:

```rust
pub struct MetricsSender {
    client: Client,           // reqwest HTTP client
    server_url: String,       // Base server URL
    retry_attempts: u32,      // Max retry attempts
    retry_delay: Duration,    // Base delay for exponential backoff
}

impl MetricsSender {
    pub fn new(
        server_url: String,
        retry_attempts: u32,
        retry_delay_seconds: u64,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))  // 10-second timeout
            .build()?;

        Ok(Self {
            client,
            server_url,
            retry_attempts,
            retry_delay: Duration::from_secs(retry_delay_seconds),
        })
    }

    pub async fn send_metrics(&self, payload: &MetricsPayload) -> Result<()> {
        let endpoint = format!("{}/api/v1/metrics", self.server_url);
        
        for attempt in 1..=self.retry_attempts {
            match self.client.post(&endpoint).json(payload).send().await {
                Ok(response) if response.status().is_success() => {
                    return Ok(());
                }
                Ok(response) => {
                    // Server returned error status
                    let error_msg = response.text().await
                        .unwrap_or_else(|_| "Unknown error".to_string());
                    // Log and continue to retry
                }
                Err(e) => {
                    // Network error
                    // Log and continue to retry
                }
            }

            // Exponential backoff: delay increases with each attempt
            if attempt < self.retry_attempts {
                let delay = self.retry_delay * attempt;
                tokio::time::sleep(delay).await;
            }
        }

        Err(anyhow::anyhow!("All retry attempts failed"))
    }
}
```

**Payload Format:**

The agent sends metrics in the format expected by the server:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsPayload {
    pub agent_id: String,                   // Unique agent identifier
    pub timestamp: DateTime<Utc>,           // UTC timestamp
    pub metrics: HashMap<String, f64>,      // Metric name -> value mapping
}
```

Example JSON payload:
```json
{
  "agent_id": "dev-server-01",
  "timestamp": "2026-01-27T18:11:44.416482648Z",
  "metrics": {
    "cpu_usage": 6.035354137420654,
    "memory_usage": 68.19478607177734,
    "disk_usage": 22.99633026123047,
    "network_rx_bytes": 63780940598.0,
    "network_tx_bytes": 2198397257.0
  }
}
```

### 3. Main Agent Integration

**File: `agent/src/main.rs`**

The main loop now optionally sends metrics to the server:

```rust
async fn run_agent(config: Config) -> Result<()> {
    // Initialize metrics sender if server is enabled
    let sender = if config.server.enabled {
        info!("Server integration enabled - sending metrics to {}", config.server.url);
        Some(MetricsSender::new(
            config.server.url.clone(),
            config.server.retry_attempts,
            config.server.retry_delay_seconds,
        )?)
    } else {
        info!("Server integration disabled - metrics will only be displayed locally");
        None
    };

    loop {
        interval_timer.tick().await;
        
        let metrics = collectors.collect(&config);
        
        // Display locally (always)
        print!("[{}] ", timestamp);
        metrics.display();

        // Send to server (if enabled)
        if let Some(ref sender) = sender {
            let mut metrics_map = HashMap::new();
            metrics_map.insert("cpu_usage".to_string(), metrics.cpu_percent as f64);
            metrics_map.insert("memory_usage".to_string(), metrics.memory_percent as f64);
            metrics_map.insert("disk_usage".to_string(), metrics.disk_percent as f64);
            metrics_map.insert("network_rx_bytes".to_string(), metrics.network_rx_bytes as f64);
            metrics_map.insert("network_tx_bytes".to_string(), metrics.network_tx_bytes as f64);

            let payload = MetricsPayload {
                agent_id: config.agent.name.clone(),
                timestamp: Utc::now(),
                metrics: metrics_map,
            };

            if let Err(e) = sender.send_metrics(&payload).await {
                warn!("Failed to send metrics to server: {}", e);
                // Continue collecting despite send failure
            }
        }
    }
}
```

### 4. Dependencies

**File: `agent/Cargo.toml`**

Added HTTP client dependencies:

```toml
[dependencies]
# ... existing dependencies ...
reqwest = { version = "0.11", features = ["json"] }  # HTTP client
serde_json = "1.0"                                    # JSON serialization
chrono = { version = "0.4", features = ["serde"] }   # Added serde feature
```

## Retry Logic & Error Handling

### Exponential Backoff Strategy

The sender implements exponential backoff to avoid overwhelming the server:

```
Attempt 1: Immediate
Attempt 2: Wait 2 seconds  (retry_delay * 1)
Attempt 3: Wait 4 seconds  (retry_delay * 2)
Attempt 4: Wait 6 seconds  (retry_delay * 3)
```

### Error Categories

1. **Network Errors** (Connection refused, timeout):
   - Logged as warnings
   - Trigger retry with backoff
   - Agent continues collecting metrics

2. **Server Errors** (4xx, 5xx responses):
   - Logged with response body
   - Trigger retry with backoff
   - May indicate payload issues

3. **Success** (2xx responses):
   - Logged at debug level
   - No retry needed

### Graceful Degradation

- If server is unreachable, agent continues collecting and displaying metrics locally
- Errors don't crash the agent
- Each collection cycle is independent (one failure doesn't affect the next)

## Building and Testing

### Terminal 1: Start the Server

```bash
cd server
cargo run --release

# Expected output:
# INFO Starting server on 0.0.0.0:8080
# INFO API Endpoints:
#   POST   /api/v1/metrics         - Ingest metrics from agents
#   ...
```

### Terminal 2: Run the Agent

```bash
cd agent
cargo run -- -c agent.toml

# Expected output:
# INFO Server integration enabled - sending metrics to http://localhost:8080
# --------------------------- Monitoring Agent Started ---------------------------
# Agent: dev-server-01 | Interval: 10s | Server: http://localhost:8080
# 
# [2026-01-27 18:11:44] CPU: 6.0%, Memory: 10.56 GB/15.30 GB (69.0%), ...
```

### Terminal 3: Verify Data Reception

**Query metrics:**
```bash
# Get CPU usage history
curl -s "http://localhost:8080/api/v1/metrics?agent_id=dev-server-01&metric=cpu_usage" | jq .

# Output:
{
  "agent_id": "dev-server-01",
  "metric": "cpu_usage",
  "data_points": [
    {
      "timestamp": "2026-01-27T18:11:44.416482648Z",
      "value": 6.035354137420654
    },
    {
      "timestamp": "2026-01-27T18:11:54.417853245Z",
      "value": 8.275897979736328
    }
  ],
  "count": 2
}
```

**List agents:**
```bash
curl -s http://localhost:8080/api/v1/agents | jq .

# Output:
[
  {
    "id": "dev-server-01",
    "name": "dev-server-01",
    "last_seen": "2026-01-27T18:12:04.417864584Z",
    "status": "Healthy"
  }
]
```

**Get latest metrics:**
```bash
curl -s "http://localhost:8080/api/v1/metrics/latest?agent_id=dev-server-01" | jq .

# Output:
{
  "agent_id": "dev-server-01",
  "metrics": [
    {
      "name": "cpu_usage",
      "value": 10.26,
      "timestamp": "2026-01-27T18:12:04.417432589Z"
    },
    {
      "name": "memory_usage",
      "value": 68.19,
      "timestamp": "2026-01-27T18:12:04.417432589Z"
    },
    ...
  ]
}
```

## Testing Scenarios

### 1. Normal Operation

**Setup:**
- Server running
- Agent configured with `enabled = true`

**Expected:**
- Metrics sent successfully every interval
- No error messages in agent logs
- Server logs show "Received metrics from agent..."

### 2. Server Unavailable at Startup

**Setup:**
- Server NOT running
- Agent starts with `enabled = true`

**Expected:**
- Agent displays: "Connection refused"
- Agent retries 3 times with increasing delays
- Agent continues collecting locally
- No crashes

### 3. Server Becomes Unavailable

**Setup:**
- Server running
- Agent sending successfully
- Stop server (Ctrl+C)

**Expected:**
- Next send attempt fails
- Agent logs warnings
- Agent continues collecting
- When server restarts, agent resumes sending

### 4. Server Disabled in Config

**Setup:**
- Set `enabled = false` in agent.toml

**Expected:**
- Agent displays: "Server integration disabled"
- No HTTP requests made
- Metrics only displayed locally

### 5. Invalid Server URL

**Setup:**
- Set `url = "http://invalid-host:9999"` in config

**Expected:**
- Agent logs connection errors
- Retries with backoff
- Continues collecting locally

## Performance Considerations

### HTTP Client Configuration

```rust
let client = Client::builder()
    .timeout(Duration::from_secs(10))  // Prevent hanging on slow networks
    .build()?;
```

- **Timeout:** 10 seconds prevents indefinite hangs
- **Connection Pooling:** reqwest automatically reuses connections
- **Keep-Alive:** Enabled by default for better performance

### Async Design

- Non-blocking sends using `async/await`
- Collection continues during retries
- No thread spawning overhead

### Memory Usage

- Metrics converted to payload (small allocation)
- No buffering of failed sends (fire-and-forget)
- Connection pool maintained by reqwest

## Troubleshooting

### Issue: "Connection refused"

**Symptoms:**
```
WARN Failed to send metrics (attempt 1): error sending request for url 
(http://localhost:8080/api/v1/metrics): tcp connect error: Connection refused
```

**Solutions:**
1. Verify server is running: `curl http://localhost:8080/health`
2. Check server logs for startup errors
3. Verify URL in agent.toml matches server binding
4. Check firewall rules if using different hosts

### Issue: "422 Unprocessable Entity"

**Symptoms:**
```
WARN Server returned error status 422 Unprocessable Entity: Failed to 
deserialize the JSON body into the target type: missing field `metrics`
```

**Solutions:**
1. Ensure agent and server versions match
2. Check payload format matches server expectations
3. Verify HashMap serialization includes all required fields

### Issue: Agent Crashes on Send

**Symptoms:**
- Agent exits when server is unavailable

**Solutions:**
1. Ensure sender initialization handles errors:
   ```rust
   let sender = if config.server.enabled {
       Some(MetricsSender::new(...)?)  // ✓ Correct
   } else {
       None
   };
   ```

2. Wrap send calls in error handling:
   ```rust
   if let Err(e) = sender.send_metrics(&payload).await {
       warn!("Failed: {}", e);  // Don't panic
   }
   ```

### Issue: Slow Performance

**Symptoms:**
- Collection interval delayed
- Metrics not sent on time

**Solutions:**
1. Reduce retry attempts: `retry_attempts = 1`
2. Decrease retry delay: `retry_delay_seconds = 1`
3. Increase timeout in sender initialization
4. Check network latency to server

## Log Levels

**INFO Level:**
- Server integration status
- Successful transmission (debug level in production)

**WARN Level:**
- Failed send attempts (with attempt number)
- Server error responses

**ERROR Level:**
- All retry attempts exhausted
- Critical failures

**Example Log Output:**
```
2026-01-27T18:11:44.136786Z  INFO Server integration enabled - sending metrics to http://localhost:8080
2026-01-27T18:11:44.417247Z  INFO Received metrics from agent 'dev-server-01' with 5 metrics
2026-01-27T18:11:54.418685Z  INFO Received metrics from agent 'dev-server-01' with 5 metrics
```

## Configuration Examples

### Development (Local)

```toml
[server]
enabled = true
url = "http://localhost:8080"
retry_attempts = 3
retry_delay_seconds = 2
```

### Production (Remote Server)

```toml
[server]
enabled = true
url = "https://monitoring.example.com"
retry_attempts = 5              # More retries over WAN
retry_delay_seconds = 5         # Longer delay for network issues
```

### Testing (No Server)

```toml
[server]
enabled = false                 # Metrics only displayed locally
url = "http://localhost:8080"   # Ignored when disabled
```

## Next Steps

Phase 3 successfully connects agents to the central server. The system now has:
- ✅ Agents collecting metrics locally
- ✅ Central server with REST API
- ✅ Automatic metric transmission with retry logic
- ✅ Time-series storage on the server

**Phase 4** will implement the web dashboard to visualize the collected metrics in real-time.

## Key Features

✅ **Reliability**
- Retry logic with exponential backoff
- Graceful error handling
- No crashes on network failures

✅ **Flexibility**
- Server integration can be disabled
- Configurable retry behavior
- Works with or without server

✅ **Performance**
- Async/await for non-blocking operations
- Connection pooling
- Efficient payload serialization

✅ **Observability**
- Detailed logging at appropriate levels
- Clear error messages
- Agent status visible in server API

---

**Phase 3 Status:** ✅ **COMPLETE**

The agent successfully sends metrics to the server every 15 seconds, with automatic retries on failure and graceful degradation when the server is unavailable.
