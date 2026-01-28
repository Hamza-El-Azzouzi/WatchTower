# Docker Deployment Guide

Complete guide for deploying the DevOps Monitoring System using Docker and Docker Compose.

## Table of Contents

- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Configuration](#configuration)
- [Building Images](#building-images)
- [Running with Docker Compose](#running-with-docker-compose)
- [Environment Variables](#environment-variables)
- [Volumes and Data Persistence](#volumes-and-data-persistence)
- [Authentication Setup](#authentication-setup)
- [Troubleshooting](#troubleshooting)
- [Production Deployment](#production-deployment)

## Quick Start

### Prerequisites

- Docker 20.10+ and Docker Compose 2.0+
- At least 2GB RAM available
- 10GB disk space

### Start the Full Stack

```bash
# Clone the repository
git clone <repository-url>
cd DevOps-Monitoring-System

# Start all services
docker-compose up -d

# Check service status
docker-compose ps

# View logs
docker-compose logs -f
```

Access the dashboard at: http://localhost:3000

Access the API at: http://localhost:8080

## Architecture

The Docker deployment consists of four main components:

```
┌─────────────────┐
│   Dashboard     │ :3000
│   (Next.js)     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Monitor Server │ :8080
│  (Rust/Axum)    │
└────────┬────────┘
         │
    ┌────┴────┐
    │ SQLite  │
    └─────────┘
         ▲
         │
┌────────┴────────┐
│ Multiple Agents │
│  (System Mon.)  │
└─────────────────┘
```

## Configuration

### Server Configuration

The server can be configured via:
1. **Config file**: `server/config.toml`
2. **Environment variables**: Override config file settings

Example `config.toml`:

```toml
[server]
host = "0.0.0.0"
port = 8080

[storage]
max_points_per_metric = 10000

[retention]
metrics_hours = 168        # 7 days
alerts_days = 30
cleanup_interval_hours = 1

[database]
enabled = true
url = "sqlite:/app/data/monitoring.db"

[auth]
enabled = true
require_api_key = false
```

### Agent Configuration

Agents can be configured via:
1. **Environment variables** (recommended for Docker)
2. **Config file**: Mounted as volume

**Environment Variables:**
- `AGENT_NAME` - Unique identifier for the agent
- `SERVER_URL` - URL of the monitoring server
- `COLLECTION_INTERVAL` - Metrics collection interval in seconds
- `API_KEY` - API key for authentication (if enabled)

**Config File Example:**

```toml
[agent]
name = "production-server-01"

[collection]
interval_seconds = 10

[metrics]
collect_cpu = true
collect_memory = true
collect_disk = true
collect_network = true

[server]
enabled = true
url = "http://monitor-server:8080"
api_key = "msk_your_api_key_here"
retry_attempts = 3
retry_delay_seconds = 2
```

## Building Images

### Build All Images

```bash
docker-compose build
```

### Build Individual Images

**Server:**
```bash
docker build -t devops-monitor-server:latest ./server
```

**Agent:**
```bash
docker build -t devops-monitor-agent:latest ./agent
```

**Dashboard:**
```bash
docker build -t devops-monitor-dashboard:latest ./dev-ops-monitoring-dashboard
```

### Build with Custom Tags

```bash
docker-compose build --build-arg VERSION=1.0.0
```

## Running with Docker Compose

### Start Services

```bash
# Start in detached mode
docker-compose up -d

# Start with specific services
docker-compose up -d monitor-server agent-web

# Start and watch logs
docker-compose up
```

### Stop Services

```bash
# Stop all services
docker-compose stop

# Stop and remove containers
docker-compose down

# Stop and remove volumes (⚠️ deletes data)
docker-compose down -v
```

### Scale Agents

```bash
# Run 5 instances of agent-web
docker-compose up -d --scale agent-web=5
```

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f monitor-server

# Last 100 lines
docker-compose logs --tail=100 monitor-server
```

### Restart Services

```bash
# Restart all
docker-compose restart

# Restart specific service
docker-compose restart monitor-server
```

## Environment Variables

### Server Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SERVER_HOST` | `0.0.0.0` | Bind address |
| `SERVER_PORT` | `8080` | Listen port |
| `DATABASE_ENABLED` | `true` | Enable persistent storage |
| `DATABASE_URL` | `sqlite:/app/data/monitoring.db` | Database connection string |
| `AUTH_ENABLED` | `true` | Enable authentication system |
| `AUTH_REQUIRE_API_KEY` | `false` | Require API keys for agents |
| `RETENTION_METRICS_HOURS` | `168` | Metrics retention (hours) |
| `RETENTION_ALERTS_DAYS` | `30` | Alerts retention (days) |
| `STORAGE_MAX_POINTS` | `10000` | Max data points per metric |

### Agent Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `AGENT_NAME` | Yes | Unique agent identifier |
| `SERVER_URL` | Yes | Monitoring server URL |
| `COLLECTION_INTERVAL` | No | Collection interval (default: 10s) |
| `API_KEY` | No | API key for authentication |

### Dashboard Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `NEXT_PUBLIC_API_URL` | `http://localhost:8080` | Backend API URL |

## Volumes and Data Persistence

### Volume Configuration

The `monitor-data` volume persists:
- SQLite database
- Metrics history
- Alert history
- API keys

### Backup Data

```bash
# Create backup
docker run --rm -v devops-monitoring-system_monitor-data:/data \
  -v $(pwd)/backups:/backup \
  alpine tar czf /backup/monitor-data-$(date +%Y%m%d).tar.gz /data

# Restore backup
docker run --rm -v devops-monitoring-system_monitor-data:/data \
  -v $(pwd)/backups:/backup \
  alpine tar xzf /backup/monitor-data-20260128.tar.gz -C /
```

### View Volume Data

```bash
# List volumes
docker volume ls

# Inspect volume
docker volume inspect devops-monitoring-system_monitor-data

# Access volume data
docker run --rm -it -v devops-monitoring-system_monitor-data:/data alpine sh
# Inside container:
ls -la /data
```

## Authentication Setup

### Enable Authentication

1. **Update `docker-compose.yml`:**

```yaml
services:
  monitor-server:
    environment:
      - AUTH_ENABLED=true
      - AUTH_REQUIRE_API_KEY=true  # Enforce API keys
```

2. **Restart server:**

```bash
docker-compose restart monitor-server
```

3. **Generate API key** via dashboard (Admin > API Keys) or API:

```bash
curl -X POST http://localhost:8080/api/v1/auth/keys \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Production Agents",
    "description": "Key for production environment agents",
    "max_agents": 10,
    "expires_in_days": 365
  }'
```

4. **Update agents with API key:**

```yaml
services:
  agent-web:
    environment:
      - API_KEY=msk_your_generated_key_here
```

5. **Restart agents:**

```bash
docker-compose restart agent-web agent-api agent-db
```

### Agent Limits

Control how many agents can use each API key:

```bash
# Create key with limit of 5 agents
curl -X POST http://localhost:8080/api/v1/auth/keys \
  -H "Content-Type: application/json" \
  -d '{"name": "Team A", "max_agents": 5}'
```

The 6th agent using this key will be rejected with 401.

## Troubleshooting

### Container Won't Start

```bash
# Check logs
docker-compose logs monitor-server

# Check container status
docker-compose ps

# Inspect container
docker inspect devops-monitor-server
```

### Database Issues

```bash
# Check database file exists
docker exec devops-monitor-server ls -la /app/data

# Check database permissions
docker exec devops-monitor-server stat /app/data/monitoring.db

# Reset database (⚠️ deletes all data)
docker-compose down
docker volume rm devops-monitoring-system_monitor-data
docker-compose up -d
```

### Agent Connection Issues

```bash
# Check agent logs
docker-compose logs agent-web

# Test server connectivity from agent
docker exec monitor-agent-web curl http://monitor-server:8080/health

# Check network
docker network inspect devops-monitoring-system_monitoring
```

### Dashboard Can't Connect

```bash
# Check API URL
docker exec devops-monitor-dashboard env | grep API_URL

# Test from dashboard container
docker exec devops-monitor-dashboard curl http://monitor-server:8080/health

# Rebuild dashboard
docker-compose build dashboard
docker-compose up -d dashboard
```

### Authentication Errors (401)

```bash
# Check if auth is enabled
docker-compose logs monitor-server | grep -i "authentication"

# Verify API key is valid
curl http://localhost:8080/api/v1/auth/keys

# Test agent with API key
curl -X POST http://localhost:8080/api/v1/metrics \
  -H "Authorization: Bearer msk_your_key" \
  -H "Content-Type: application/json" \
  -d '{"agent_id": "test", "metrics": []}'
```

### Port Already in Use

```bash
# Check what's using the port
sudo lsof -i :8080
sudo lsof -i :3000

# Change ports in docker-compose.yml
ports:
  - "8081:8080"  # Use port 8081 instead
```

## Production Deployment

### Security Checklist

- [ ] Enable authentication: `AUTH_REQUIRE_API_KEY=true`
- [ ] Use strong API keys (256-bit entropy)
- [ ] Set agent limits on API keys
- [ ] Use HTTPS with reverse proxy (nginx/Traefik)
- [ ] Restrict network access with firewall rules
- [ ] Regular backups of monitor-data volume
- [ ] Monitor disk space for database growth
- [ ] Set up log rotation
- [ ] Use secrets management (Docker secrets/Vault)

### Production docker-compose.yml Example

```yaml
version: '3.8'

services:
  monitor-server:
    image: devops-monitor-server:1.0.0
    restart: always
    environment:
      - AUTH_REQUIRE_API_KEY=true
      - RETENTION_METRICS_HOURS=720  # 30 days
    volumes:
      - monitor-data:/app/data
    networks:
      - monitoring
      - traefik
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.monitor.rule=Host(`monitor.example.com`)"
      - "traefik.http.routers.monitor.tls.certresolver=letsencrypt"
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '1'
          memory: 1G

networks:
  monitoring:
    driver: bridge
  traefik:
    external: true

volumes:
  monitor-data:
    driver: local
```

### Health Monitoring

```bash
# Check service health
docker-compose ps

# Health check endpoint
curl http://localhost:8080/health

# Database stats
curl http://localhost:8080/api/v1/db/stats
```

### Resource Limits

Configure resource limits in `docker-compose.yml`:

```yaml
services:
  monitor-server:
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '1'
          memory: 1G
```

### Log Management

```bash
# Configure logging driver
services:
  monitor-server:
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
```

## Additional Resources

- [Server Configuration Reference](../server/config.toml)
- [Agent Configuration Guide](../agent/README.md)
- [API Documentation](API_REFERENCE.md)
- [Authentication Guide](AGENT_AUTHENTICATION.md)
- [Troubleshooting Guide](TROUBLESHOOTING.md)
