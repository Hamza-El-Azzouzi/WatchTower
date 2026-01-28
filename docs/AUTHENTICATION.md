# API Key Authentication Guide

## Overview

The DevOps Monitoring System now supports API key authentication to secure access to the monitoring server. This ensures that only authorized agents and users can submit metrics and manage alert rules.

## Features

- **Secure API Key Generation**: Cryptographically secure random keys with `msk_` prefix
- **Expiration Support**: Keys can have optional expiration dates
- **Key Revocation**: Instantly revoke compromised or unused keys
- **Last Used Tracking**: Monitor when keys were last used
- **Optional Authentication**: Can be disabled for development/testing

## Quick Start

### 1. Create an API Key

```bash
curl -X POST http://localhost:8080/api/v1/auth/keys \
  -H "Content-Type: application/json" \
  -d '{
    "name": "production-agent-1",
    "description": "API key for production monitoring agent",
    "expires_in_days": 90
  }'
```

Response:
```json
{
  "key": "msk_7-tjswXzILHmqQ8kASN-d_tsSXtdKO2S7BNwiEbgoAY",
  "name": "production-agent-1",
  "expires_at": "2026-04-28T14:52:03.869Z"
}
```

**⚠️ Important:** Save the `key` value immediately - it won't be shown again!

### 2. Use the API Key

Send the API key in the `Authorization` header with the `Bearer` prefix:

```bash
curl -X POST http://localhost:8080/api/v1/metrics \
  -H "Authorization: Bearer msk_7-tjswXzILHmqQ8kASN-d_tsSXtdKO2S7BNwiEbgoAY" \
  -H "Content-Type: application/json" \
  -d '{ ... }'
```

### 3. Configure Agent

Update your agent configuration file:

```toml
[server]
enabled = true
url = "http://localhost:8080"
api_key = "msk_7-tjswXzILHmqQ8kASN-d_tsSXtdKO2S7BNwiEbgoAY"
```

## API Endpoints

### Create API Key
**POST** `/api/v1/auth/keys`

Request body:
```json
{
  "name": "key-name",
  "description": "Optional description",
  "expires_in_days": 90
}
```

- `name` (required): Human-readable name for the key
- `description` (optional): Additional context about the key
- `expires_in_days` (optional): Days until expiration (omit for no expiration)

### List API Keys
**GET** `/api/v1/auth/keys`

Returns all API keys (key values are masked):

```json
[
  {
    "id": 1,
    "key": "msk_7-tjswXz...",
    "name": "production-agent-1",
    "description": "API key for production monitoring agent",
    "created_at": "2026-01-28T14:52:03Z",
    "expires_at": "2026-04-28T14:52:03Z",
    "last_used_at": "2026-01-28T15:00:00Z",
    "revoked": false,
    "revoked_at": null,
    "created_by": "admin"
  }
]
```

### Revoke API Key
**DELETE** `/api/v1/auth/keys/:key_id`

Immediately revokes an API key. Returns:

```json
{
  "success": true,
  "message": "API key revoked successfully"
}
```

## Configuration

### Server Configuration

Edit `config.toml`:

```toml
[database]
enabled = true  # Required for authentication
url = "sqlite:monitoring.db"

[auth]
enabled = true  # Enable authentication features
require_api_key = false  # Make API keys optional (for gradual rollout)
```

**Authentication Modes:**

1. **Development Mode** (`require_api_key = false`):
   - API keys can be created and managed
   - Requests without API keys are allowed
   - Useful for testing and gradual rollout

2. **Production Mode** (`require_api_key = true`):
   - All protected endpoints require valid API key
   - Requests without valid keys are rejected with 401 Unauthorized
   - Recommended for production deployments

### Agent Configuration

Add to `agent/config.toml`:

```toml
[server]
enabled = true
url = "http://monitoring-server:8080"
api_key = "msk_YOUR_API_KEY_HERE"
retry_attempts = 3
```

## Security Best Practices

### 1. Key Management
- **Rotate keys regularly** (every 90 days recommended)
- **Use descriptive names** to track key purposes
- **Revoke unused keys** immediately
- **Never commit keys to version control**

### 2. Storage
- Store keys in environment variables or secure vaults
- Use different keys for different agents/environments
- Implement key rotation automation

### 3. Monitoring
- Check `last_used_at` to detect unused keys
- Monitor for expired keys
- Set up alerts for authentication failures

### 4. Access Control
- Limit who can create/revoke API keys
- Keep audit logs of key creation/revocation
- Use separate keys for production and development

## Migration Guide

### Enabling Authentication on Existing Installation

1. **Create API keys for all agents**:
```bash
for agent in agent-1 agent-2 agent-3; do
  curl -X POST http://localhost:8080/api/v1/auth/keys \
    -H "Content-Type: application/json" \
    -d "{\"name\": \"$agent\", \"expires_in_days\": 90}"
done
```

2. **Update agent configurations** with new API keys

3. **Test agents** are still reporting metrics

4. **Enable required authentication**:
```toml
[auth]
enabled = true
require_api_key = true  # Now enforce authentication
```

5. **Restart server** to apply changes

## Troubleshooting

### 401 Unauthorized Error

**Problem**: Agent requests return 401 Unauthorized

**Solutions**:
1. Check API key is included in Authorization header
2. Verify key format: `Authorization: Bearer msk_...`
3. Confirm key hasn't expired
4. Check key isn't revoked using `/api/v1/auth/keys`

### Authentication Disabled

**Problem**: Cannot create API keys

**Error**: `Authentication is not enabled`

**Solution**: 
1. Enable database in `config.toml`:
   ```toml
   [database]
   enabled = true
   ```
2. Restart server

### Key Not Working

**Checklist**:
- [ ] Key copied correctly (no whitespace)
- [ ] `Bearer ` prefix included in Authorization header
- [ ] Key not expired (check `expires_at`)
- [ ] Key not revoked (check `revoked` field)
- [ ] Database accessible

## API Key Format

Format: `msk_<base64-encoded-random-bytes>`

- **Prefix**: `msk_` (Monitoring System Key)
- **Length**: 48 characters (32 random bytes encoded)
- **Encoding**: Base64 URL-safe without padding
- **Entropy**: 256 bits of randomness

Example: `msk_7-tjswXzILHmqQ8kASN-d_tsSXtdKO2S7BNwiEbgoAY`

## Limitations

- API keys are stored in the database (requires database to be enabled)
- Currently single-tier authentication (no role-based access control)
- No automatic key rotation (manual process required)
- Key validation happens on every request (minimal performance impact)

## Future Enhancements

- Role-based access control (RBAC)
- Automatic key rotation
- OAuth 2.0 support for dashboard access
- Audit logging for all authentication events
- IP-based restrictions
- Rate limiting per API key

## Support

For issues or questions:
1. Check server logs: `/tmp/server.log`
2. Verify configuration: `config.toml`
3. Test with curl examples above
4. Review database entries: `SELECT * FROM api_keys;`
