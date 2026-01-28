# Multi-Agent Docker Testing Guide

Good news! You have all the tools set up. However, Docker networking with `network_mode: host` has some limitations.

## Recommended Approach: Run Multiple Agents Locally

Instead of Docker, here's an easier way to test with multiple "machines":

### Quick Multi-Agent Setup

```bash
cd /home/helazzou/Desktop/DevOps-Monitoring-System/agent

# Create configs for 3 more agents
cat > agent-web.toml << EOF
[agent]
name = "test-web-server"

[collection]
interval_seconds = 15

[metrics]
collect_cpu = true
collect_memory = true
collect_disk = true
collect_network = true

[server]
enabled = true
url = "http://localhost:8080"
retry_attempts = 3
retry_delay_seconds = 2
EOF

cat > agent-api.toml << EOF
[agent]
name = "test-api-server"

[collection]
interval_seconds = 15

[metrics]
collect_cpu = true
collect_memory = true
collect_disk = true
collect_network = true

[server]
enabled = true
url = "http://localhost:8080"
retry_attempts = 3
retry_delay_seconds = 2
EOF

cat > agent-db.toml << EOF
[agent]
name = "test-database-server"

[collection]
interval_seconds = 15

[metrics]
collect_cpu = true
collect_memory = true
collect_disk = true
collect_network = true

[server]
enabled = true
url = "http://localhost:8080"
retry_attempts = 3
retry_delay_seconds = 2
EOF

# Start all agents in background
nohup ./target/release/monitor-agent -c agent-web.toml > /tmp/agent-web.log 2>&1 &
nohup ./target/release/monitor-agent -c agent-api.toml > /tmp/agent-api.log 2>&1 &
nohup ./target/release/monitor-agent -c agent-db.toml > /tmp/agent-db.log 2>&1 &

# Check they're running
sleep 5
curl -s "http://localhost:8080/api/v1/agents" | python3 -c "import sys,json; agents=json.load(sys.stdin); print(f'Total agents: {len(agents)}'); [print(f'  - {a[\"name\"]}: {a[\"status\"]}') for a in agents]"
```

This simulates 4 different servers all reporting to your dashboard!

## View in Dashboard

Open: http://localhost:3000

You'll see all agents displayed as separate server cards. Each one will show different metrics because they're all monitoring the same physical machine from different "perspectives" (processes).

## Docker Solution (For Real Multi-Machine Testing)

For actual Docker multi-machine simulation, you'd need to:

1. **Use bridge network** (not host mode)
2. **Map server port**: Publish 8080 from host
3. **Use host gateway IP**: Find it with `hostname -I`

The Docker setup I provided is ready - you just need to:
1. Stop the host agent temporarily
2. Find your machine's IP: `hostname -I | awk '{print $1}'`
3. Update docker-compose.yml `SERVER_URL` to use your IP (not localhost)
4. Run: `docker-compose up -d`

##Summary

✅ **Easiest**: Run multiple agent processes locally (see commands above)
🐳 **Docker**: Requires IP configuration but works for true isolation
🖥️ **VMs**: Best for production-like testing with actual separate machines

Try the local multi-agent approach first - it's the fastest way to see your multi-server dashboard in action!
