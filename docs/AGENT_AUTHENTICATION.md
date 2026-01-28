# Authentication for Agents

## Overview

Starting now, all agents **MUST** provide a valid API key to send metrics to the server. Agents without valid API keys will be rejected with **401 Unauthorized**.

## Configuration

Add the `api_key` field to your agent configuration:

```toml
[server]
enabled = true
url = "http://monitoring-server:8080"
api_key = "msk_sBrfxVgRgaJQ8KFT_K4056cnjffH3zULDYD2sRJ1kGY"  # Get from admin dashboard
retry_attempts = 3
retry_delay_seconds = 2
```

## Getting an API Key

1. Open the admin dashboard: http://localhost:3000/admin/api-keys
2. Click **"Generate New Key"**
3. Fill in the form:
   - **Name**: Descriptive name (e.g., "production-agents")
   - **Description**: Optional details
   - **Expires In**: Choose expiration period
   - **Max Agents**: Optional limit on how many agents can use this key
4. Click **"Generate Key"**
5. **IMPORTANT**: Copy the key immediately (shown only once)
6. Add the key to your agent's configuration file

## What Happens Without a Key

If an agent tries to send metrics without an API key or with an invalid key:

```bash
# Response: 401 Unauthorized
{
  "error": "Missing or invalid Authorization header. Expected: Authorization: Bearer <api-key>"
}
```

The agent will:
- Log authentication errors
- Retry according to retry configuration
- **NOT send any metrics** until authentication succeeds

## Agent Limit Enforcement

If a key has `max_agents` set (e.g., 5 agents maximum):

1. **First 5 agents** that use the key are allowed
2. **6th agent** is blocked with 401 Unauthorized
3. **Original 5 agents** continue working normally

Error message when limit reached:
```json
{
  "error": "Invalid, expired, or agent limit reached for this API key"
}
```

## Example Configurations

### Single Production Server

```toml
[agent]
name = "production-web-01"

[server]
enabled = true
url = "https://monitoring.example.com"
api_key = "msk_abc123..." # Dedicated key for this agent
```

### Multiple Agents with Shared Key

```toml
# Agent 1
[agent]
name = "staging-web-01"

[server]
enabled = true
url = "http://monitoring-server:8080"
api_key = "msk_shared_staging_key"  # max_agents: 10

---

# Agent 2
[agent]
name = "staging-web-02"

[server]
enabled = true
url = "http://monitoring-server:8080"
api_key = "msk_shared_staging_key"  # Same key, different agent
```

## Troubleshooting

### Error: "Missing or invalid Authorization header"

**Cause**: No API key in configuration

**Solution**: Add `api_key` to `[server]` section in config.toml

### Error: "Invalid or expired API key"

**Causes**:
1. Key was revoked in admin dashboard
2. Key has expired
3. Typo in the key

**Solution**: 
1. Check key status in admin dashboard
2. Generate new key if revoked/expired
3. Verify key is copied correctly (no extra spaces)

### Error: "Agent limit reached"

**Cause**: Too many agents using the same key

**Solutions**:
1. Create a new key with higher `max_agents` limit
2. Create separate keys for different agent groups
3. Remove unused agents to free up slots

### Agent Can't Connect

**Check**:
1. Server logs: `tail -f /tmp/server.log | grep -i auth`
2. Agent logs: Look for "401" or "Unauthorized"
3. Dashboard: Check key status (Active/Expired/Revoked)

## Security Best Practices

### 1. Use Separate Keys per Environment

```toml
# Production agents
api_key = "msk_production_..."

# Staging agents  
api_key = "msk_staging_..."

# Development agents
api_key = "msk_dev_..."
```

### 2. Set Appropriate Expiration

- **Production**: 90-180 days
- **Staging**: 30-90 days
- **Development**: 7-30 days
- **Testing**: 1-7 days

### 3. Rotate Keys Regularly

1. Create new key
2. Update agents gradually
3. Monitor old key usage (last_used_at)
4. Revoke old key once all agents migrated

### 4. Limit Agent Count

- **Single server**: `max_agents: 1`
- **Small cluster**: `max_agents: 5-10`
- **Auto-scaling**: No limit (`max_agents: null`)

### 5. Monitor Key Usage

Check regularly:
- Keys with high agent counts
- Expired keys still being used (check logs)
- Keys with unexpected agent IDs

## Migration from No Auth

If you have agents currently running without authentication:

### Step 1: Create API Keys

Create keys for all your agents (or groups of agents)

### Step 2: Update Agent Configs

Add `api_key` to configuration files

### Step 3: Restart Agents

Restart each agent to load new configuration

### Step 4: Verify

Check server logs for successful authentication:
```bash
grep "Received metrics" /tmp/server.log
```

### Step 5: Enable Enforcement

Update server config:
```toml
[auth]
enabled = true
require_api_key = true  # This blocks unauthenticated agents
```

Restart server.

## API Reference (for custom clients)

### HTTP Request Format

**Important**: For agent limit tracking to work, include `agent_id` as a query parameter:

```http
POST /api/v1/metrics?agent_id=my-agent-01 HTTP/1.1
Host: monitoring-server:8080
Content-Type: application/json
Authorization: Bearer msk_sBrfxVgRgaJQ8KFT_K4056cnjffH3zULDYD2sRJ1kGY

{
  "agent_id": "my-agent-01",
  "timestamp": "2026-01-28T15:20:00Z",
  "metrics": {
    "cpu_usage": 45.0,
    "memory_usage": 67.5
  }
}
```

**Note**: The `agent_id` should be in both the query parameter (for auth) and the JSON body (for metrics). The Rust agent automatically does this.

### Response Codes

- **200 OK**: Metrics accepted
- **401 Unauthorized**: Missing/invalid API key or agent limit reached
- **400 Bad Request**: Malformed request
- **500 Internal Server Error**: Server error

## FAQ

**Q: Can I use the same key for all agents?**

A: Yes, but set an appropriate `max_agents` limit for security.

**Q: What happens to running agents when I revoke a key?**

A: They immediately stop working (next metrics submission fails with 401).

**Q: Can I update an agent's API key without restarting?**

A: No, agents read configuration at startup. You must restart the agent.

**Q: How do I see which agents are using a key?**

A: In the dashboard, each key shows "Agent Limit: X / Y" where X is current usage.

**Q: Can I increase the agent limit for an existing key?**

A: Not yet. Create a new key with higher limit and migrate agents.

**Q: What if I lose an API key?**

A: You cannot retrieve it. Create a new key and update your agents.
