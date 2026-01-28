#!/bin/bash
set -e

# Generate agent.toml from environment variables
cat > /tmp/agent.toml << EOF
[agent]
name = "${AGENT_NAME}"

[collection]
interval_seconds = ${COLLECTION_INTERVAL}

[metrics]
collect_cpu = true
collect_memory = true
collect_disk = true
collect_network = true

[server]
enabled = true
url = "${SERVER_URL}"
retry_attempts = 3
retry_delay_seconds = 2
EOF

# Add API key if provided
if [ ! -z "${API_KEY}" ]; then
    echo "api_key = \"${API_KEY}\"" >> /tmp/agent.toml
    echo "API key authentication enabled"
fi

echo "Starting agent: ${AGENT_NAME}"
echo "Server URL: ${SERVER_URL}"
echo "Collection interval: ${COLLECTION_INTERVAL}s"

# Run the agent
exec monitor-agent -c /tmp/agent.toml

