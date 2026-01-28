# Multi-Machine Monitoring Setup Guide

## Overview

This guide explains how to monitor multiple servers/machines with the DevOps Monitoring System. You'll learn how to deploy agents on different machines and view all their metrics in a single centralized dashboard.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Central Dashboard                       │
│                   http://localhost:3000                     │
│                                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ Server 1 │  │ Server 2 │  │ Server 3 │  │ Server N │   │
│  │  Card    │  │  Card    │  │  Card    │  │  Card    │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        │ REST API (Port 8080)
                        ▼
        ┌───────────────────────────────────┐
        │      Central Server              │
        │   http://<SERVER_IP>:8080        │
        │                                   │
        │  - Time-series storage           │
        │  - Agent registry                │
        │  - REST API endpoints            │
        └─────────────┬─────────────────────┘
                      │
                      │ HTTP POST /api/v1/metrics
                      │ (every 10-15 seconds)
                      │
        ┌─────────────┴─────────────────────────┐
        │                                       │
        ▼                                       ▼
┌───────────────┐                     ┌───────────────┐
│  Machine 1    │                     │  Machine 2    │
│               │                     │               │
│  Agent        │                     │  Agent        │
│  web-server-01│                     │  api-server-01│
│               │                     │               │
│  - Collects   │      ...            │  - Collects   │
│  - Sends      │                     │  - Sends      │
└───────────────┘                     └───────────────┘
```

## Prerequisites

- **Central Server Machine**: One machine to run the server and dashboard
- **Agent Machines**: Multiple machines you want to monitor
- Network connectivity between all machines
- Rust installed on all machines (for agent compilation)
- Node.js installed on server machine (for dashboard)

## Step-by-Step Setup

### 1. Set Up the Central Server

Choose one machine as your central server. This will run:
- The Rust backend server (Port 8080)
- The Next.js dashboard (Port 3000)

#### On the Server Machine:

```bash
# Clone the repository
git clone <your-repo-url>
cd DevOps-Monitoring-System

# Build and start the server
cd server
cargo build --release

# Run the server (bind to all interfaces for remote access)
cargo run --release

# Expected output:
# INFO Starting server on 0.0.0.0:8080
# INFO API Endpoints:
#   POST   /api/v1/metrics         - Ingest metrics from agents
#   GET    /api/v1/agents          - List all agents
#   ...
```

**Important**: The server binds to `0.0.0.0:8080`, making it accessible from other machines on the network.

#### Start the Dashboard:

```bash
# In a new terminal, on the same server machine
cd dev-ops-monitoring-dashboard
npm install
npm run dev

# Dashboard available at:
# http://localhost:3000 (local)
# http://<SERVER_IP>:3000 (remote access)
```

### 2. Configure Firewall (If Needed)

On the central server machine, ensure port 8080 is accessible:

```bash
# Ubuntu/Debian
sudo ufw allow 8080/tcp

# CentOS/RHEL
sudo firewall-cmd --permanent --add-port=8080/tcp
sudo firewall-cmd --reload

# Or disable firewall for testing (not recommended for production)
sudo ufw disable
```

### 3. Deploy Agents on Each Machine

#### Option A: Build Agent on Each Machine

On each machine you want to monitor:

```bash
# Clone the repository
git clone <your-repo-url>
cd DevOps-Monitoring-System/agent

# Create a unique configuration file
nano agent.toml
```

**agent.toml configuration:**

```toml
[agent]
name = "web-server-01"  # UNIQUE NAME for each machine

[collection]
interval_seconds = 15    # How often to collect metrics

[metrics]
collect_cpu = true
collect_memory = true
collect_disk = true
collect_network = true

[server]
enabled = true
url = "http://<CENTRAL_SERVER_IP>:8080"  # Replace with your server's IP
retry_attempts = 3
retry_delay_seconds = 2
```

**Important Configuration Notes:**

1. **Unique Agent Names**: Each machine MUST have a unique `name`:
   - Machine 1: `name = "web-server-01"`
   - Machine 2: `name = "api-server-01"`
   - Machine 3: `name = "db-server-01"`
   - Machine 4: `name = "worker-01"`

2. **Server URL**: Replace `<CENTRAL_SERVER_IP>` with the actual IP address of your central server:
   ```toml
   url = "http://192.168.1.100:8080"  # Example
   ```

3. **Find Your Server IP**:
   ```bash
   # On the central server machine
   hostname -I
   # or
   ip addr show
   ```

#### Build and Run the Agent:

```bash
cd DevOps-Monitoring-System/agent
cargo build --release

# Run the agent
cargo run --release -- -c agent.toml

# Expected output:
# INFO Starting monitoring agent: web-server-01
# INFO Server integration enabled - sending metrics to http://192.168.1.100:8080
# 
# Agent: web-server-01 | Interval: 15s | Server: http://192.168.1.100:8080
# [2026-01-28 10:30:00] CPU: 5.2%, Memory: 8.5 GB/16.0 GB (53.1%), ...
```

#### Option B: Copy Compiled Binary

If you don't want to install Rust on every machine:

1. **Build on one machine**:
   ```bash
   cd agent
   cargo build --release
   ```

2. **Copy the binary**:
   ```bash
   # The binary is at: target/release/monitor-agent
   
   # Copy to other machines
   scp target/release/monitor-agent user@machine2:/home/user/
   scp agent.toml user@machine2:/home/user/
   ```

3. **Run on each machine**:
   ```bash
   # On machine 2
   ./monitor-agent -c agent.toml
   ```

### 4. Example Multi-Machine Configuration

Let's say you have 4 machines to monitor:

#### Machine 1 (Web Server) - 192.168.1.101
**agent.toml:**
```toml
[agent]
name = "web-server-01"

[server]
enabled = true
url = "http://192.168.1.100:8080"
```

#### Machine 2 (API Server) - 192.168.1.102
**agent.toml:**
```toml
[agent]
name = "api-server-01"

[server]
enabled = true
url = "http://192.168.1.100:8080"
```

#### Machine 3 (Database Server) - 192.168.1.103
**agent.toml:**
```toml
[agent]
name = "database-server-01"

[server]
enabled = true
url = "http://192.168.1.100:8080"
```

#### Machine 4 (Worker) - 192.168.1.104
**agent.toml:**
```toml
[agent]
name = "worker-01"

[server]
enabled = true
url = "http://192.168.1.100:8080"
```

### 5. Verify All Agents are Connected

From any machine with network access:

```bash
# List all registered agents
curl http://192.168.1.100:8080/api/v1/agents | jq .

# Expected output:
[
  {
    "id": "web-server-01",
    "name": "web-server-01",
    "last_seen": "2026-01-28T10:30:15.123Z",
    "status": "Healthy"
  },
  {
    "id": "api-server-01",
    "name": "api-server-01",
    "last_seen": "2026-01-28T10:30:16.456Z",
    "status": "Healthy"
  },
  {
    "id": "database-server-01",
    "name": "database-server-01",
    "last_seen": "2026-01-28T10:30:17.789Z",
    "status": "Healthy"
  },
  {
    "id": "worker-01",
    "name": "worker-01",
    "last_seen": "2026-01-28T10:30:18.012Z",
    "status": "Healthy"
  }
]
```

### 6. Access the Dashboard

Open your browser and navigate to:

```
http://<CENTRAL_SERVER_IP>:3000
```

You should see:
- **Overview Page**: All 4 servers displayed as cards
- **Stats Cards**: Total Servers: 4, Healthy: 4
- **System Health Overview**: Aggregated metrics from all servers
- **Server Comparison**: Bar chart comparing all 4 servers

Click on any server card to see detailed metrics for that specific machine.

## Running Agents as Background Services

### Linux (systemd)

Create a service file for each agent:

```bash
sudo nano /etc/systemd/system/monitor-agent.service
```

```ini
[Unit]
Description=DevOps Monitoring Agent
After=network.target

[Service]
Type=simple
User=your-user
WorkingDirectory=/path/to/DevOps-Monitoring-System/agent
ExecStart=/path/to/DevOps-Monitoring-System/agent/target/release/monitor-agent -c agent.toml
Restart=on-failure
RestartSec=10

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

### Using Screen (Quick & Dirty)

```bash
# Start agent in background
screen -dmS monitor-agent ./target/release/monitor-agent -c agent.toml

# View agent output
screen -r monitor-agent

# Detach: Ctrl+A, then D
```

## Testing Different Scenarios

### Scenario 1: Agent Goes Down

1. Stop one agent: `Ctrl+C` or `systemctl stop monitor-agent`
2. Refresh dashboard
3. Agent status changes to "Degraded" (after 1 minute) or "Unreachable" (after 5 minutes)
4. Restart agent: `systemctl start monitor-agent`
5. Status returns to "Healthy"

### Scenario 2: High CPU Load

Generate CPU load on one machine:

```bash
# Install stress tool
sudo apt install stress

# Create CPU load
stress --cpu 4 --timeout 60s
```

Watch the dashboard:
- CPU usage increases for that specific server
- Alert threshold chart shows warning/critical zones
- System health overview updates

### Scenario 3: Network Issues

1. Temporarily block port 8080:
   ```bash
   sudo iptables -A OUTPUT -p tcp --dport 8080 -j DROP
   ```

2. Agent will show retry attempts in logs
3. Metrics displayed locally but not sent to server
4. Restore connection:
   ```bash
   sudo iptables -D OUTPUT -p tcp --dport 8080 -j DROP
   ```

## Detailed Server View

### Accessing Individual Server Details

The dashboard already has detailed views for each server:

1. **From Overview Page**: Click any server card
2. **Direct URL**: `http://<DASHBOARD_URL>:3000/server/<agent_id>`

Example:
```
http://192.168.1.100:3000/server/web-server-01
http://192.168.1.100:3000/server/api-server-01
http://192.168.1.100:3000/server/database-server-01
```

### What's Included in Server Detail Page:

#### 1. Header Section
- Server name
- Status badge (Healthy/Degraded/Unreachable)
- Last seen timestamp
- Back to overview button

#### 2. Current Metrics Section
- Latest CPU, Memory, Disk, Network values
- Color-coded status indicators
- Real-time updates every 10 seconds

#### 3. Alert Thresholds Section (NEW!)
Three threshold charts with colored zones:
- **CPU Usage**: Warning at 70%, Critical at 90%
- **Memory Usage**: Warning at 75%, Critical at 90%
- **Disk Usage**: Warning at 80%, Critical at 95%

Each chart shows:
- Current value with color-coded status icon
- Normal/Warning/Critical zones (Green/Amber/Red)
- Reference lines at threshold boundaries
- Alert messages when thresholds exceeded

#### 4. Historical Charts Section
Four time-series charts:
- **CPU Usage**: Line chart showing trends
- **Memory Usage**: Area chart with gradient
- **Disk Usage**: Area chart showing storage trends
- **Network Traffic**: Dual line chart (RX/TX)

With time range selector:
- 1 Hour (360 data points)
- 6 Hours (2,160 data points)
- 24 Hours (8,640 data points)
- 7 Days (60,480 data points)

## Monitoring Best Practices

### 1. Naming Convention

Use descriptive, hierarchical names:
```
environment-role-number
```

Examples:
- `prod-web-01`, `prod-web-02`
- `staging-api-01`
- `dev-db-01`

### 2. Collection Intervals

- **Production**: 15-30 seconds (balance between accuracy and load)
- **Development**: 10 seconds (more granular)
- **Low-priority**: 60 seconds (reduce overhead)

### 3. Alert Thresholds

Recommended thresholds:
- **CPU**: Warning 70%, Critical 90%
- **Memory**: Warning 75%, Critical 90%
- **Disk**: Warning 80%, Critical 95%

Adjust based on your workload patterns.

### 4. Resource Planning

Server requirements scale with number of agents:
- **1-10 agents**: 1 CPU, 2GB RAM (server)
- **10-50 agents**: 2 CPU, 4GB RAM
- **50-100 agents**: 4 CPU, 8GB RAM
- **100+ agents**: Consider clustering

## Troubleshooting

### Agent Can't Connect to Server

**Symptoms:**
```
WARN Failed to send metrics (attempt 1): Connection refused
```

**Solutions:**
1. Verify server is running: `curl http://<SERVER_IP>:8080/health`
2. Check firewall rules: `sudo ufw status`
3. Ping server: `ping <SERVER_IP>`
4. Test port: `telnet <SERVER_IP> 8080`

### Agent Not Appearing in Dashboard

**Check server received metrics:**
```bash
curl http://<SERVER_IP>:8080/api/v1/agents
```

If agent is listed:
- Dashboard may be cached, hard refresh (Ctrl+Shift+R)
- Check browser console for errors

If agent NOT listed:
- Agent config has wrong server URL
- Agent not sending metrics (check agent logs)
- Server not storing data (check server logs)

### Different Metrics on Different Machines

This is normal! Each machine has different:
- CPU cores and usage patterns
- Memory capacity and allocation
- Disk size and usage
- Network activity

The dashboard compares them side-by-side in the "Server Comparison" chart.

## Security Considerations

### 1. Authentication (TODO - Phase 7)

Currently, the API has no authentication. For production:
- Add API keys for agents
- Implement JWT tokens for dashboard
- Use HTTPS/TLS

### 2. Network Security

- Use VPN or private network for agent-server communication
- Firewall rules to restrict port 8080 access
- Consider using SSH tunnels for remote agents

### 3. Data Privacy

- Metrics don't contain sensitive data by default
- Be careful with custom tags or labels
- Consider data retention policies

## Advanced Configurations

### Custom Collection Intervals Per Machine

High-priority server (10s):
```toml
[collection]
interval_seconds = 10
```

Low-priority server (60s):
```toml
[collection]
interval_seconds = 60
```

### Selective Metric Collection

Disable network monitoring on machines with no network concerns:
```toml
[metrics]
collect_cpu = true
collect_memory = true
collect_disk = true
collect_network = false  # Disabled
```

### Multiple Servers (Load Balancing)

For high availability, run multiple server instances:

```toml
# Agent config with multiple server URLs (requires code modification)
[server]
enabled = true
primary_url = "http://server1:8080"
secondary_url = "http://server2:8080"
```

## Next Steps

Now that you have multi-machine monitoring:

1. **Phase 5**: Add log collection from all machines
2. **Phase 6**: Set up alerting (email/Slack notifications)
3. **Phase 7**: Add authentication and production hardening

## Summary

✅ **Central server** running on one machine
✅ **Multiple agents** deployed across machines
✅ **Unique agent names** for each machine
✅ **Network connectivity** verified
✅ **Dashboard** showing all servers
✅ **Detailed views** for each server
✅ **Real-time updates** every 10 seconds

You now have a complete multi-machine monitoring solution! 🎉
