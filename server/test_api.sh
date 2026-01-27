#!/bin/bash

# Test script for Phase 2 server API endpoints

SERVER_URL="${SERVER_URL:-http://localhost:8080}"

echo "=================================="
echo "Testing Monitor Server API"
echo "Server: $SERVER_URL"
echo "=================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test 1: Health check
echo -e "${BLUE}[Test 1]${NC} Health Check"
curl -s "$SERVER_URL/health" | jq .
echo ""
echo ""

# Test 2: Send metrics
echo -e "${BLUE}[Test 2]${NC} Sending metrics for test-agent-1"
curl -s -X POST "$SERVER_URL/api/v1/metrics" \
  -H "Content-Type: application/json" \
  -d '{
    "agent_id": "test-agent-1",
    "timestamp": "2026-01-27T10:30:45Z",
    "metrics": {
      "cpu_percent": 45.2,
      "memory_used_bytes": 4294967296,
      "memory_total_bytes": 8589934592,
      "disk_used_bytes": 107374182400,
      "disk_total_bytes": 536870912000,
      "network_rx_bytes": 1048576000,
      "network_tx_bytes": 524288000
    }
  }' | jq .
echo ""
echo ""

# Test 3: Send more metrics (different timestamp)
echo -e "${BLUE}[Test 3]${NC} Sending more metrics for test-agent-1"
curl -s -X POST "$SERVER_URL/api/v1/metrics" \
  -H "Content-Type: application/json" \
  -d '{
    "agent_id": "test-agent-1",
    "timestamp": "2026-01-27T10:30:55Z",
    "metrics": {
      "cpu_percent": 52.8,
      "memory_used_bytes": 4404019200,
      "memory_total_bytes": 8589934592,
      "disk_used_bytes": 107374182400,
      "disk_total_bytes": 536870912000,
      "network_rx_bytes": 1073741824,
      "network_tx_bytes": 536870912
    }
  }' | jq .
echo ""
echo ""

# Test 4: Send metrics for another agent
echo -e "${BLUE}[Test 4]${NC} Sending metrics for test-agent-2"
curl -s -X POST "$SERVER_URL/api/v1/metrics" \
  -H "Content-Type: application/json" \
  -d '{
    "agent_id": "test-agent-2",
    "timestamp": "2026-01-27T10:31:00Z",
    "metrics": {
      "cpu_percent": 23.5,
      "memory_used_bytes": 2147483648,
      "memory_total_bytes": 4294967296
    }
  }' | jq .
echo ""
echo ""

# Test 5: List all agents
echo -e "${BLUE}[Test 5]${NC} Listing all agents"
curl -s "$SERVER_URL/api/v1/agents" | jq .
echo ""
echo ""

# Test 6: Query specific metric
echo -e "${BLUE}[Test 6]${NC} Querying cpu_percent for test-agent-1"
curl -s "$SERVER_URL/api/v1/metrics?agent_id=test-agent-1&metric=cpu_percent" | jq .
echo ""
echo ""

# Test 7: Get latest metrics
echo -e "${BLUE}[Test 7]${NC} Getting latest metrics for test-agent-1"
curl -s "$SERVER_URL/api/v1/metrics/latest?agent_id=test-agent-1" | jq .
echo ""
echo ""

# Test 8: Storage statistics
echo -e "${BLUE}[Test 8]${NC} Storage statistics"
curl -s "$SERVER_URL/api/v1/stats" | jq .
echo ""
echo ""

echo -e "${GREEN}=================================="
echo "All tests completed!"
echo "==================================${NC}"
