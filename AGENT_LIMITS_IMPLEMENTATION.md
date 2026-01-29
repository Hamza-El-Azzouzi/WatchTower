# Agent Limits Implementation

## Overview
Implemented API key-based agent limits to prevent abuse and enforce resource quotas.

## Features Implemented

### 1. Server-Side Validation
**File**: `server/src/api/mod.rs` - `ingest_metrics` handler

- Extracts API key from `X-API-Key` or `Authorization` headers
- Validates API key with agent limit checking using `AuthService::validate_api_key_with_agent()`
- Returns 403 Forbidden if:
  - API key is invalid
  - API key is expired
  - API key is revoked
  - Agent limit is reached (e.g., max_agents=5 and 5 agents already registered)
- Broadcasts metrics via WebSocket for real-time dashboard updates

### 2. Database Limit Checking
**File**: `server/src/auth/api_keys.rs` - `validate_api_key_with_agent` method

```rust
pub async fn validate_api_key_with_agent(
    &self,
    key: &str,
    agent_id: Option<&str>,
) -> Result<Option<i64>>
```

Logic:
1. First validates the API key (expiry, revoked status)
2. If `agent_id` is provided, counts existing agents: `SELECT COUNT(*) FROM agents WHERE api_key_id = $1 AND agent_id != $2`
3. Compares count to `max_agents` limit
4. Returns `Some(api_key_id)` if valid and within limits, `None` otherwise

### 3. Agent Error Handling
**File**: `agent/src/sender.rs` - `send_metrics` method

- Checks HTTP response status after sending metrics
- If status is 403 Forbidden:
  - Logs error message with details
  - Terminates agent with `std::process::exit(1)`
  - Prevents agent from continuously sending rejected requests

## Testing

### Test Scenario 1: Normal Operation
```bash
# Create API key with max_agents=2
curl -X POST http://localhost:3000/api/v1/admin/api-keys \
  -H "Authorization: Bearer <admin_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Key",
    "max_agents": 2,
    "expires_at": null
  }'

# Start 2 agents - both should succeed
./agent --config agent1.toml  # Uses API key, agent_id=agent1
./agent --config agent2.toml  # Uses API key, agent_id=agent2
```

### Test Scenario 2: Limit Reached
```bash
# Start 3rd agent - should fail with 403
./agent --config agent3.toml  # Uses same API key, agent_id=agent3

# Expected output:
# [ERROR] API key is invalid, expired, or agent limit reached: ...
# [ERROR] Terminating agent due to authentication failure
# Process exits with code 1
```

### Test Scenario 3: Invalid API Key
```bash
# Agent with wrong/expired API key
./agent --config bad-key.toml

# Expected: Immediate termination on first metrics send
```

## Configuration

### Server Config
**File**: `server/config.toml`

```toml
[alerts]
check_interval_seconds = 30  # Alert evaluation frequency
```

### Agent Config
**File**: `agent/agent.toml`

```toml
[agent]
name = "agent1"
server_url = "http://localhost:3000"
api_key = "your-api-key-here"  # Required for authentication
interval = 15  # Metric collection interval in seconds
```

### Database Schema
**File**: `server/migrations/20260129000001_initial_schema.sql`

```sql
CREATE TABLE api_keys (
    id BIGSERIAL PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT,
    max_agents INTEGER,  -- NULL = unlimited, number = max concurrent agents
    expires_at TIMESTAMPTZ,
    revoked BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

CREATE TABLE agents (
    agent_id TEXT PRIMARY KEY,
    api_key_id BIGINT REFERENCES api_keys(id),  -- Links agent to API key
    hostname TEXT,
    os TEXT,
    arch TEXT,
    last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## API Endpoints

### Create API Key (Admin)
```bash
POST /api/v1/admin/api-keys
Authorization: Bearer <admin_jwt_token>
Content-Type: application/json

{
  "name": "Production Key",
  "description": "For production agents",
  "max_agents": 10,  // Optional: null = unlimited
  "expires_at": "2024-12-31T23:59:59Z"  // Optional: null = never expires
}

Response 200:
{
  "id": 1,
  "key": "mk_live_abc123...",
  "message": "API key created successfully"
}
```

### List API Keys (Admin)
```bash
GET /api/v1/admin/api-keys
Authorization: Bearer <admin_jwt_token>

Response 200:
[
  {
    "id": 1,
    "key": "mk_live_abc123...",
    "name": "Production Key",
    "description": "For production agents",
    "max_agents": 10,
    "expires_at": "2024-12-31T23:59:59Z",
    "revoked": false,
    "created_at": "2024-01-29T10:00:00Z",
    "last_used_at": "2024-01-29T12:30:00Z"
  }
]
```

### Revoke API Key (Admin)
```bash
DELETE /api/v1/admin/api-keys/:id
Authorization: Bearer <admin_jwt_token>

Response 200:
{
  "message": "API key revoked successfully"
}
```

## Security Features

1. **Immediate Rejection**: Agent limit checks happen before storing any metrics
2. **Fail-Fast Agents**: Agents terminate immediately on 403 to avoid log spam
3. **Graceful Degradation**: Network errors are retried, but auth errors terminate
4. **Real-Time Updates**: WebSocket broadcasts metrics immediately after validation

## Performance Considerations

- **Database Query**: Agent limit check requires 1 additional COUNT query per registration
- **Caching Opportunity**: Consider caching agent counts per API key (invalidate on agent disconnect)
- **Index**: `api_key_id` column in agents table is indexed for fast lookups

## Future Enhancements

1. **Rate Limiting**: Add per-agent or per-API-key rate limits
2. **Soft Limits**: Warn but don't reject when approaching limit (e.g., 80% capacity)
3. **Dashboard Alerts**: Notify admins when API keys reach capacity
4. **Auto-Scaling**: Automatically increase limits based on subscription tier
5. **Agent Heartbeat**: Detect and remove stale agents to free up slots
