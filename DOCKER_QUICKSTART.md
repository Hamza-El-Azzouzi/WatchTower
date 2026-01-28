# 🚀 Quick Start with Docker

Get the DevOps Monitoring System running in under 5 minutes!

## Prerequisites

- Docker 20.10+
- Docker Compose 2.0+

## Start the System

```bash
# Clone the repository
git clone <repo-url>
cd DevOps-Monitoring-System

# Start all services (server, dashboard, and example agents)
docker-compose up -d

# View logs
docker-compose logs -f
```

## Access the Dashboard

Open your browser to: **http://localhost:3000**

You should see the monitoring dashboard with metrics from the example agents.

## Enable Authentication (Optional)

### 1. Generate an API Key

Via Dashboard:
1. Go to **Admin > API Keys**
2. Click **Generate New Key**
3. Set a name and agent limit
4. Copy the generated key

Via API:
```bash
curl -X POST http://localhost:8080/api/v1/auth/keys \
  -H "Content-Type: application/json" \
  -d '{"name": "My Agents", "max_agents": 5}'
```

### 2. Update docker-compose.yml

```yaml
services:
  monitor-server:
    environment:
      - AUTH_REQUIRE_API_KEY=true  # Enforce authentication
  
  agent-web:
    environment:
      - API_KEY=msk_your_generated_key_here
```

### 3. Restart Services

```bash
docker-compose down
docker-compose up -d
```

## Add More Agents

### Scale Existing Agents

```bash
docker-compose up -d --scale agent-web=3
```

### Add New Agent

Edit `docker-compose.yml`:

```yaml
services:
  agent-custom:
    build: ./agent
    environment:
      - AGENT_NAME=my-server-01
      - SERVER_URL=http://monitor-server:8080
      - API_KEY=msk_your_key  # If auth is enabled
    networks:
      - monitoring
```

Restart:
```bash
docker-compose up -d
```

## Common Commands

```bash
# View all containers
docker-compose ps

# View logs for specific service
docker-compose logs -f monitor-server

# Stop all services
docker-compose stop

# Stop and remove containers (keeps data)
docker-compose down

# Remove everything including data (⚠️ destructive)
docker-compose down -v

# Rebuild after code changes
docker-compose build
docker-compose up -d
```

## Data Persistence

All data is stored in the `monitor-data` Docker volume:
- SQLite database
- Metrics history
- Alert history
- API keys

### Backup Your Data

```bash
# Create backup
docker run --rm -v devops-monitoring-system_monitor-data:/data \
  -v $(pwd):/backup \
  alpine tar czf /backup/backup-$(date +%Y%m%d).tar.gz /data
```

### Restore Backup

```bash
# Restore from backup
docker run --rm -v devops-monitoring-system_monitor-data:/data \
  -v $(pwd):/backup \
  alpine tar xzf /backup/backup-20260128.tar.gz -C /
```

## Troubleshooting

### Containers Won't Start

```bash
# Check logs
docker-compose logs

# Check specific service
docker-compose logs monitor-server
```

### Port Already in Use

Edit `docker-compose.yml` to use different ports:

```yaml
services:
  monitor-server:
    ports:
      - "8081:8080"  # Change host port
```

### Can't Connect to Server

```bash
# Test from agent container
docker exec monitor-agent-web curl http://monitor-server:8080/health

# Test from host
curl http://localhost:8080/health
```

### Reset Everything

```bash
# Stop and remove everything
docker-compose down -v

# Start fresh
docker-compose up -d
```

## Next Steps

- [Full Docker Deployment Guide](docs/DOCKER_DEPLOYMENT.md)
- [Authentication Setup](docs/AGENT_AUTHENTICATION.md)
- [Configuration Reference](server/config.toml)
- [Phase 7 Documentation](docs/PHASE7.md)

## Production Deployment

For production use:

1. **Enable authentication**: Set `AUTH_REQUIRE_API_KEY=true`
2. **Use HTTPS**: Add reverse proxy (nginx/Traefik)
3. **Set resource limits**: Configure in docker-compose.yml
4. **Regular backups**: Automate backup script
5. **Monitor disk space**: Set up alerts

See [Production Deployment](docs/DOCKER_DEPLOYMENT.md#production-deployment) for details.
