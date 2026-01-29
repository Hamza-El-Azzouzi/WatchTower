# Production Upgrade Implementation Plan

## Overview
This document outlines the complete production upgrade from SQLite to PostgreSQL with WebSocket support, data aggregation, and enhanced authentication.

## ✅ Completed

### 1. Docker Compose with PostgreSQL
- Updated `docker-compose.yml` with PostgreSQL 16
- Configured health checks and dependencies
- Updated agent intervals to 15 seconds

### 2. Database Migrations
- Created `server/migrations/20260129000001_initial_schema.sql`
- Includes tables for:
  - admin_users (username/password authentication)
  - api_keys (with max_agents limit)
  - agents (with api_key_id foreign key)
  - metrics (raw data, 24h retention)
  - metrics_1min (1-minute aggregates, 7 days retention)
  - metrics_1hour (1-hour aggregates, 30 days retention)
  - logs
  - alerts
  - alert_rules

### 3. Server Configuration
- Updated `server/config.toml` with:
  - PostgreSQL connection string
  - Aggregation intervals (hourly for 1-min, daily for 1-hour)
  - Alert checking interval (30 seconds)
  - WebSocket configuration

### 4. Dependencies
- Updated `server/Cargo.toml`:
  - Changed from `sqlite` to `postgres` feature in SQLx
  - Added `axum-extra` for WebSocket support
  - Added `futures` and `tokio-stream` for async streams

### 5. PostgreSQL Database Module
- Created `server/src/db/postgres.rs` with:
  - Connection pooling (50 max, 5 min connections)
  - Automatic migration runner
  - Data retention cleanup methods
  - Aggregation methods (raw → 1min → 1hour)
  - API key limit checking
  - API key update methods

## 🔄 In Progress

### Remaining Implementation Tasks

#### 1. Server Core Updates

**File: `server/src/db/mod.rs`**
- Replace SQLite imports with PostgreSQL
- Update Database struct to use PgPool
- Import postgres module

**File: `server/src/models/*.rs`**
- Update all SQL queries from SQLite syntax to PostgreSQL
- Change datetime functions (datetime() → CURRENT_TIMESTAMP)
- Update AUTOINCREMENT to SERIAL

**File: `server/src/handlers/*.rs`**
- Add API key limit enforcement in agent registration
- Return error 403 with message when limit reached
- Update all database queries for PostgreSQL

#### 2. Agent Limit Enforcement

**File: `server/src/handlers/agents.rs`**
```rust
// When agent registers:
1. Extract API key from header
2. Call db.check_api_key_agent_limit(api_key)
3. If limit reached, return 403 with JSON:
   {
     "error": "Agent limit reached",
     "message": "This API key has reached its maximum agent limit (X/Y agents)",
     "max_agents": Y,
     "current_count": X
   }
4. If OK, proceed with registration and link api_key_id
```

**File: `agent/src/main.rs`**
```rust
// Update error handling:
- Check for 403 status code
- Parse error message
- Log error and exit with code 1
- Display: "Failed to register: API key limit reached (X/Y agents)"
```

#### 3. Admin Authentication

**File: `server/src/auth/admin.rs`** (new)
```rust
- Login endpoint: POST /api/v1/admin/login
- Input: { "username": "admin", "password": "admin123" }
- Verify password with bcrypt
- Generate JWT session token (30 min expiry)
- Return: { "token": "jwt_token", "user": {...} }
```

**File: `server/src/middleware/admin_auth.rs`** (new)
```rust
- Middleware to check JWT token
- Extract from Authorization: Bearer <token>
- Verify token signature and expiry
- Attach AdminUser to request extensions
```

**Files to update:**
- `server/src/handlers/admin/*.rs` - Add admin auth middleware
- `dashboard/app/admin-login/page.tsx` - Create admin login page
- `dashboard/lib/admin-auth.ts` - JWT handling

#### 4. WebSocket Implementation

**File: `server/src/websocket/mod.rs`** (new)
```rust
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use tokio::sync::broadcast;

// WebSocket connection manager
pub struct WebSocketManager {
    metrics_tx: broadcast::Sender<MetricUpdate>,
    logs_tx: broadcast::Sender<LogEntry>,
    alerts_tx: broadcast::Sender<Alert>,
}

// Endpoints:
// - GET /api/v1/ws/metrics - Real-time metrics stream
// - GET /api/v1/ws/logs - Real-time logs stream
// - GET /api/v1/ws/alerts - Real-time alerts stream
```

**File: `server/src/handlers/metrics.rs`**
```rust
// When new metrics arrive:
1. Store in database
2. Broadcast to WebSocket clients via metrics_tx.send()
```

**Dashboard Files:**
- `dashboard/lib/websocket.ts` - WebSocket client manager
- `dashboard/hooks/useWebSocket.ts` - React hook for WS
- `dashboard/components/*.tsx` - Replace useEffect polling with useWebSocket

#### 5. Data Aggregation Jobs

**File: `server/src/aggregation/mod.rs`** (new)
```rust
// Background jobs using tokio::spawn

// Job 1: Every hour, aggregate raw → 1-minute
pub async fn run_minute_aggregation_job(db: Database) {
    loop {
        tokio::time::sleep(Duration::from_secs(3600)).await;
        db.aggregate_metrics_to_1min(1).await;
    }
}

// Job 2: Every day, aggregate 1-minute → 1-hour
pub async fn run_hour_aggregation_job(db: Database) {
    loop {
        tokio::time::sleep(Duration::from_secs(86400)).await;
        db.aggregate_metrics_to_1hour(1).await;
    }
}

// Job 3: Every hour, cleanup old data
pub async fn run_cleanup_job(db: Database, config: Config) {
    loop {
        tokio::time::sleep(Duration::from_secs(3600)).await;
        db.cleanup_old_raw_metrics(config.retention.raw_metrics_hours).await;
        db.cleanup_old_minute_aggregates(config.retention.minute_aggregates_days).await;
        db.cleanup_old_hour_aggregates(config.retention.hour_aggregates_days).await;
        db.cleanup_old_alerts(config.retention.alerts_days).await;
    }
}
```

**File: `server/src/main.rs`**
```rust
// Spawn background jobs:
tokio::spawn(aggregation::run_minute_aggregation_job(db.clone()));
tokio::spawn(aggregation::run_hour_aggregation_job(db.clone()));
tokio::spawn(aggregation::run_cleanup_job(db.clone(), config.clone()));
```

#### 6. Smart Data Fetching

**File: `server/src/handlers/metrics.rs`**
```rust
// Update GET /api/v1/metrics endpoint:
pub async fn get_historical_metrics(
    Query(params): Query<MetricsQuery>
) -> Result<Json<Vec<MetricPoint>>> {
    let time_range = params.to - params.from;
    
    // Smart selection:
    // - If range <= 1 hour: use raw metrics
    // - If range <= 7 days: use 1-minute aggregates
    // - If range > 7 days: use 1-hour aggregates
    
    let table = if time_range <= Duration::hours(1) {
        "metrics"
    } else if time_range <= Duration::days(7) {
        "metrics_1min"
    } else {
        "metrics_1hour"
    };
    
    // Query from appropriate table
    // If aggregated, use avg_value as value
}
```

#### 7. Rich Admin Dashboard

**Dashboard Files to Create:**
- `dashboard/app/admin/overview/page.tsx`
  - Total agents per API key (bar chart)
  - API key usage percentages
  - Agent status distribution
  - Recent agent registrations

- `dashboard/app/admin/api-keys/page.tsx` (enhance)
  - Add "Current Agents" column showing X/Y (or X/Unlimited)
  - Add "Update" button to edit name, description, max_agents, expires_at
  - Add modal for editing key properties
  - Show agent list for each key

- `dashboard/components/AdminStats.tsx`
  - Cards showing:
    - Total API keys
    - Total agents
    - Keys near limit (e.g., 90%+ of max_agents)
    - Expired keys
    - Agents per key average

#### 8. Alert Engine Update

**File: `server/src/alerts/engine.rs`**
```rust
// Update to run every 30 seconds instead of 60
pub async fn run_alert_engine(db: Database, rules: AlertRules) {
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;
        // Check all rules
        // Broadcast new alerts via WebSocket
    }
}
```

## Testing Plan

### 1. Database Migration Testing
```bash
# Start PostgreSQL
docker-compose up -d postgres

# Run server (should auto-migrate)
cd server && cargo run

# Verify tables created
psql -h localhost -U monitoring_user -d monitoring -c "\dt"
```

### 2. API Key Limit Testing
```bash
# Create API key with max_agents=2
curl -X POST http://localhost:8080/api/v1/auth/keys \
  -H "Content-Type: application/json" \
  -d '{"name": "test-key", "max_agents": 2}'

# Start 3 agents with same key
# - Agent 1: Should register ✓
# - Agent 2: Should register ✓
# - Agent 3: Should fail with 403 ✗
```

### 3. WebSocket Testing
```javascript
// Browser console
const ws = new WebSocket('ws://localhost:8080/api/v1/ws/metrics');
ws.onmessage = (event) => console.log('Metric:', JSON.parse(event.data));
// Should see real-time metrics
```

### 4. Aggregation Testing
```bash
# Generate metrics for 2 hours
# Wait for hourly aggregation job
# Check metrics_1min table has data
psql -c "SELECT COUNT(*) FROM metrics_1min;"
```

### 5. Data Retention Testing
```bash
# Insert old metrics
# Run cleanup job
# Verify old data deleted
```

## Deployment Checklist

- [ ] Update `.env` with PostgreSQL credentials
- [ ] Change default admin password
- [ ] Set up PostgreSQL backups
- [ ] Configure firewall rules (port 8080, 5432)
- [ ] Set up SSL/TLS certificates
- [ ] Configure monitoring alerts
- [ ] Test WebSocket behind reverse proxy (nginx)
- [ ] Load test with multiple agents
- [ ] Document API key generation process
- [ ] Create admin user guide

## Performance Considerations

### Database Indexing
All critical indexes are included in migrations:
- `idx_metrics_agent_timestamp` - Fast metric queries
- `idx_api_keys_key` - Fast auth lookups
- `idx_agents_api_key_id` - Fast agent counting

### Connection Pooling
- Min: 5 connections (always ready)
- Max: 50 connections (handle load spikes)
- Timeout: 30 seconds

### WebSocket Scaling
- Max 1000 concurrent connections per server instance
- Use Redis pub/sub for multi-instance setup
- Heartbeat every 30 seconds to detect dead connections

### Aggregation Performance
- Runs during low-traffic periods (hourly/daily)
- Uses `ON CONFLICT DO NOTHING` to avoid duplicates
- Indexes on timestamp columns for fast grouping

## Security Hardening

1. **API Keys**
   - Generate with cryptographically secure RNG
   - Hash before storage (currently stored plain)
   - Rotate regularly (add rotation reminder)

2. **Admin Authentication**
   - Use strong password requirements
   - Implement rate limiting on login
   - Add 2FA (future enhancement)
   - Log all admin actions

3. **Database**
   - Use least-privilege database user
   - Enable SSL for PostgreSQL connections
   - Regular backup and recovery testing

4. **WebSocket**
   - Require authentication for WS connections
   - Implement rate limiting per connection
   - Validate all incoming messages

## Monitoring the Monitor

Add self-monitoring:
- Server health endpoint
- Database connection pool metrics
- WebSocket connection count
- Aggregation job status
- Alert rule execution time
- API request latency

## Documentation Updates

Files to update:
- `README.md` - Add PostgreSQL setup instructions
- `docs/DEPLOYMENT.md` - Docker Compose guide
- `docs/API.md` - WebSocket endpoints
- `docs/ADMIN_GUIDE.md` - API key management
- `docs/ARCHITECTURE.md` - Data flow diagrams

## Estimated Timeline

- Database layer: 4-6 hours
- Agent limit enforcement: 2 hours
- Admin authentication: 3-4 hours
- WebSocket implementation: 6-8 hours
- Data aggregation: 4-5 hours
- Admin dashboard enhancements: 4-6 hours
- Testing and debugging: 6-8 hours
- Documentation: 3-4 hours

**Total: 32-43 hours of development**

## Next Immediate Steps

1. Update `server/src/db/mod.rs` to use PostgreSQL module
2. Fix all SQL queries in models/ and handlers/
3. Implement agent limit check in agent registration handler
4. Test database migration and basic functionality
5. Implement WebSocket manager
6. Create aggregation background jobs
7. Build admin login UI
8. Enhance admin dashboard with rich statistics

Would you like me to continue with the implementation, or would you prefer to review this plan first?
