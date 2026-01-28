# API Key Agent Limits

## Overview

The monitoring system now supports limiting how many agents can use a single API key. This provides better security and resource control by preventing unauthorized key sharing.

## Features

### Agent Limit Enforcement
- **Optional Limit**: Set a maximum number of agents that can use each API key
- **Unlimited by Default**: If no limit is specified, unlimited agents can use the key
- **Automatic Tracking**: System automatically tracks which agent IDs have used each key
- **Real-time Validation**: When an agent attempts to use a key, the system checks if the limit is reached

### How It Works

1. **Key Creation**: Admin creates an API key with an optional `max_agents` parameter
2. **First Use**: When an agent first uses the key, its `agent_id` is recorded in the `used_by_agents` list
3. **Limit Check**: For each subsequent use by a new agent:
   - System checks if agent is already in the list → allow access
   - System checks if limit is reached → reject if at capacity
   - System adds new agent to list if under limit → allow access
4. **Agent Blocking**: If the limit is reached, new agents are blocked with 401 Unauthorized

## Database Schema

### New Fields in `api_keys` Table

```sql
-- Maximum number of agents that can use this key (NULL = unlimited)
max_agents INTEGER DEFAULT NULL

-- JSON array of agent IDs that have used this key
used_by_agents TEXT DEFAULT '[]'
```

### Migration

Migration `003_api_key_agent_limit.sql` adds these fields to existing installations:

```sql
ALTER TABLE api_keys ADD COLUMN max_agents INTEGER DEFAULT NULL;
ALTER TABLE api_keys ADD COLUMN used_by_agents TEXT DEFAULT '[]';
CREATE INDEX IF NOT EXISTS idx_api_keys_max_agents ON api_keys(max_agents) WHERE max_agents IS NOT NULL;
```

## API Reference

### Create API Key with Agent Limit

**Endpoint**: `POST /api/v1/auth/keys`

**Request Body**:
```json
{
  "name": "production-agent-key",
  "description": "Key for production agents",
  "expires_in_days": 90,
  "max_agents": 5
}
```

**Response**:
```json
{
  "key": "msk_jJhYQAEZ0YS5Bf3hG4Gx_Q1yR6Gt2aAMLW3UxCpVdoM",
  "name": "production-agent-key",
  "expires_at": "2026-04-28T15:12:44Z"
}
```

### List API Keys with Agent Usage

**Endpoint**: `GET /api/v1/auth/keys`

**Response**:
```json
[
  {
    "id": 1,
    "key": "msk_jJhYQAEZ...",
    "name": "production-agent-key",
    "description": "Key for production agents",
    "created_at": "2026-01-28T15:12:44Z",
    "expires_at": "2026-04-28T15:12:44Z",
    "last_used_at": "2026-01-28T16:30:00Z",
    "revoked": false,
    "revoked_at": null,
    "created_by": "admin",
    "max_agents": 5,
    "used_by_agents": ["agent-1", "agent-2", "agent-3"]
  }
]
```

## Admin Dashboard

### Creating Keys with Agent Limits

1. Navigate to **Admin → API Keys** in the sidebar
2. Click **Generate New Key**
3. Fill in the form:
   - **Key Name**: Descriptive name for the key
   - **Description**: Optional details about the key's purpose
   - **Expires In**: Key expiration period
   - **Maximum Agents**: Number of agents that can use this key (leave empty for unlimited)
4. Click **Generate Key**
5. Copy the key (shown only once)

### Viewing Agent Usage

In the API Keys list, each key displays:

```
Agent Limit: 3 / 5
```

This means:
- 3 agents have used this key
- 5 agents maximum are allowed
- 2 more agents can use this key

For unlimited keys:
```
Agent Limit: 3 / Unlimited
```

### Visual Indicators

- **Green Badge**: Active key (not expired, not revoked)
- **Orange Badge**: Expired key (agents cannot use)
- **Red Badge**: Revoked key (agents cannot use)
- **Agent Count**: Shows current usage vs. limit

## Agent Configuration

Agents don't need any special configuration for agent limits. The enforcement happens server-side during authentication.

### Example Agent Config

```toml
[server]
enabled = true
url = "http://monitoring-server:8080"
api_key = "msk_jJhYQAEZ0YS5Bf3hG4Gx_Q1yR6Gt2aAMLW3UxCpVdoM"
```

### What Happens When Limit is Reached

When an agent tries to use a key that has reached its agent limit:

1. **HTTP 401 Unauthorized** response
2. **Agent logs error**: "Authentication failed: Unauthorized"
3. **Agent retries** according to retry configuration
4. **Metrics not sent** until authentication succeeds

## Use Cases

### Single-Agent Keys
```json
{
  "name": "dedicated-db-agent",
  "max_agents": 1
}
```
Ensures only one specific agent can use this key. Good for critical systems.

### Team Keys
```json
{
  "name": "dev-team-agents",
  "max_agents": 10
}
```
Allow a team to deploy multiple test agents with a shared key.

### Unlimited Keys
```json
{
  "name": "dynamic-scaling-key",
  "max_agents": null
}
```
For auto-scaling environments where agent count varies.

## Security Benefits

1. **Prevent Key Sharing**: Limit unauthorized key distribution
2. **Resource Control**: Prevent one key from being used by too many agents
3. **Compliance**: Track which agents have accessed the system
4. **Audit Trail**: `used_by_agents` provides usage history
5. **Capacity Planning**: See which keys are heavily used

## Monitoring Recommendations

### Alert on High Usage

Create alerts when keys approach their agent limit:

```
if (used_by_agents.length >= max_agents * 0.8) {
  alert("Key approaching agent limit");
}
```

### Regular Audits

Periodically review:
- Keys with max_agents reached
- Keys with unexpected agent IDs in used_by_agents
- Unused keys (used_by_agents is empty)

### Rotation Policy

For high-security environments:
1. Create new key with same agent limit
2. Update agents to use new key
3. Monitor old key's last_used_at
4. Revoke old key when no longer in use

## Troubleshooting

### Agent Can't Authenticate

**Symptom**: Agent logs "401 Unauthorized"

**Check**:
1. Is the key revoked? → Create new key
2. Is the key expired? → Create new key or update expiration
3. Is agent limit reached? → Increase max_agents or create new key

### Too Many Agents Using Key

**Symptom**: More agents in `used_by_agents` than expected

**Action**:
1. Review list of agent IDs
2. Identify unauthorized agents
3. Revoke key immediately
4. Create new key with stricter limit
5. Update only authorized agents

### Can't Increase Agent Limit

**Current Limitation**: Agent limits cannot be modified after key creation

**Workaround**:
1. Create new key with higher limit
2. Update agents to use new key
3. Revoke old key

**Future Enhancement**: Add endpoint to update `max_agents` for existing keys

## Implementation Details

### AuthService Methods

```rust
// Validate key and enforce agent limits
pub async fn validate_api_key_with_agent(
    &self, 
    key: &str, 
    agent_id: Option<&str>
) -> Result<Option<ApiKey>>
```

### Validation Logic

1. Check if key exists and is valid (not revoked, not expired)
2. If agent_id is provided and key has max_agents:
   - If agent already in list → allow
   - If agent not in list and limit reached → deny
   - If agent not in list and under limit → add to list and allow
3. Update last_used_at timestamp
4. Return validation result

### Data Format

Agent IDs are stored as JSON array in SQLite:

```sql
-- Empty list
used_by_agents = '[]'

-- With agents
used_by_agents = '["agent-1", "agent-2", "agent-3"]'
```

## Future Enhancements

### Planned Features

1. **Update Agent Limit**: Allow modifying max_agents for existing keys
2. **Remove Agent from List**: Manually remove agent IDs to free up slots
3. **Agent Metadata**: Store more info (hostname, IP, first_seen, last_seen)
4. **Usage Analytics**: Dashboard graphs showing agent usage over time
5. **Bulk Operations**: Update multiple keys' limits at once
6. **Export/Import**: Backup and restore agent lists

### API Endpoint Ideas

```
PATCH /api/v1/auth/keys/:id/max-agents
  - Update the agent limit

DELETE /api/v1/auth/keys/:id/agents/:agent_id
  - Remove specific agent from allowed list

GET /api/v1/auth/keys/:id/agents
  - Get detailed agent usage information
```

## Conclusion

Agent limits provide fine-grained control over API key usage, improving security and resource management. The feature is backward compatible (existing keys without limits continue to work) and easy to use through both API and dashboard.

For questions or issues, see the main documentation or check server logs for authentication errors.
