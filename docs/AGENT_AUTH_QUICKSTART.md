# Quick Start: Agent with Authentication

## Problem: Getting 401 Unauthorized

If your agent is getting **401 Unauthorized** errors, it means the server requires API key authentication.

## Solution: Add API Key to Agent Config

### Step 1: Get an API Key

Ask the server admin to generate an API key for you from the dashboard at:
```
http://server-address:3000/admin/api-keys
```

Or if you have access, generate it yourself:
1. Open dashboard → Admin → API Keys
2. Click "Generate New Key"
3. Copy the key (it's shown only once!)

### Step 2: Update Agent Configuration

Edit your `agent.toml` file and add the `api_key` line:

```toml
[agent]
name = "my-agent"

[collection]
interval_seconds = 10

[metrics]
collect_cpu = true
collect_memory = true
collect_disk = true
collect_network = true

[server]
enabled = true
url = "http://server-address:8080"
api_key = "msk_sBrfxVgRgaJQ8KFT_K4056cnjffH3zULDYD2sRJ1kGY"  # ADD THIS LINE
retry_attempts = 3
retry_delay_seconds = 2
```

### Step 3: Rebuild and Restart Agent

```bash
# Build the new version
cd agent
cargo build --release

# Stop old agent
pkill monitor-agent

# Start with new config
./target/release/monitor-agent --config agent.toml
```

## Verification

You should see in the agent logs:
```
INFO Server integration enabled - sending metrics to http://...
INFO API key authentication enabled
INFO Metrics sent successfully
```

**No more 401 errors!**

## Troubleshooting

### Still getting 401?

**Check:**
1. ✅ API key is correct (no extra spaces, copy-paste carefully)
2. ✅ API key hasn't expired (check dashboard)
3. ✅ API key hasn't been revoked (check dashboard)
4. ✅ Agent limit not reached (if key has `max_agents` limit)

**Example working config:**
```toml
[server]
enabled = true
url = "http://10.1.5.1:8080"
api_key = "msk_sBrfxVgRgaJQ8KFT_K4056cnjffH3zULDYD2sRJ1kGY"
retry_attempts = 3
retry_delay_seconds = 2
```

### Agent says "No API key configured"

This warning means you forgot to add the `api_key` line. Add it under `[server]` section.

### Server logs show "Invalid API key"

The key might be:
- Typed incorrectly
- Expired
- Revoked
- For a different server

Get a fresh key from the admin dashboard.

## Need Help?

Contact your server administrator to:
- Generate a new API key
- Check if your current key is valid
- Increase agent limit if needed
