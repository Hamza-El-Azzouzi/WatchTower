#!/bin/bash

echo "🔥 PostgreSQL Stress Test (Docker Version)"
echo "==========================================="
echo ""

CONTAINER="test-postgres"

# Check if container is running
if ! docker ps | grep -q $CONTAINER; then
    echo "❌ PostgreSQL container not running!"
    echo "Start it with: docker-compose up -d postgres"
    exit 1
fi

echo "✅ PostgreSQL container is running"
echo ""

echo "1️⃣  Creating test data..."
docker exec $CONTAINER psql -U postgres -d testdb -c "
CREATE TABLE IF NOT EXISTS stress_test (
    id SERIAL PRIMARY KEY,
    data TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_stress_created ON stress_test(created_at);
" > /dev/null 2>&1

echo "2️⃣  Inserting 10,000 rows to increase database size..."
docker exec $CONTAINER psql -U postgres -d testdb -c "
INSERT INTO stress_test (data)
SELECT md5(random()::text)
FROM generate_series(1, 10000);
" > /dev/null 2>&1

echo "3️⃣  Creating multiple connections (will show in active connections)..."
for i in {1..10}; do
    docker exec $CONTAINER psql -U postgres -d testdb -c "SELECT pg_sleep(5), 'Connection $i' as conn;" > /dev/null 2>&1 &
done

echo "4️⃣  Running slow queries (will show in slow_queries metric)..."
for i in {1..5}; do
    docker exec $CONTAINER psql -U postgres -d testdb -c "SELECT pg_sleep(2), COUNT(*) FROM stress_test WHERE data LIKE '%a%';" > /dev/null 2>&1 &
done

echo "5️⃣  Creating transaction load..."
for i in {1..20}; do
    docker exec $CONTAINER psql -U postgres -d testdb -c "BEGIN; INSERT INTO stress_test (data) VALUES ('tx_$i'); COMMIT;" > /dev/null 2>&1 &
done

echo ""
echo "✅ Stress test running!"
echo "📊 Watch the dashboard at http://localhost:3000/databases"
echo ""
echo "Metrics to watch:"
echo "  - Active Connections: Should spike to ~15-20"
echo "  - Slow Queries: Should show ~5"
echo "  - Database Size: Should increase"
echo "  - Cache Hit Ratio: May fluctuate"
echo "  - Transactions Committed: Should increase rapidly"
echo ""
echo "⏳ Waiting for all queries to complete (about 5-8 seconds)..."
wait

echo "✅ Stress test completed!"
echo ""
echo "📈 Final Database Stats:"
docker exec $CONTAINER psql -U postgres -d testdb -c "
SELECT 
    (SELECT count(*) FROM pg_stat_activity WHERE state = 'active') as active_conns,
    (SELECT count(*) FROM stress_test) as total_rows,
    (SELECT pg_size_pretty(pg_database_size(current_database()))) as db_size;
"

echo ""
echo "🔍 Current metrics from agent:"
echo "Run: curl -s http://localhost:8080/api/v1/metrics/latest?agent_id=db-monitor-01 | jq '.metrics[] | select(.name | startswith(\"db_\"))'"
