#!/bin/bash

echo "🔥 PostgreSQL Stress Test Script"
echo "=================================="
echo ""
echo "Make sure PostgreSQL is running: docker ps | grep test-postgres"
echo ""

# Connection details
export PGHOST=localhost
export PGPORT=5432
export PGDATABASE=testdb
export PGUSER=postgres
export PGPASSWORD=postgres

echo "1️⃣  Creating test data..."
psql -c "
CREATE TABLE IF NOT EXISTS stress_test (
    id SERIAL PRIMARY KEY,
    data TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_stress_created ON stress_test(created_at);
" 2>/dev/null

echo "2️⃣  Inserting 10,000 rows to increase database size..."
psql -c "
INSERT INTO stress_test (data)
SELECT md5(random()::text)
FROM generate_series(1, 10000);
" 2>/dev/null

echo "3️⃣  Creating multiple connections (will show in active connections)..."
for i in {1..10}; do
    psql -c "SELECT pg_sleep(5), 'Connection $i' as conn;" &
done

echo "4️⃣  Running slow queries (will show in slow_queries metric)..."
for i in {1..5}; do
    psql -c "SELECT pg_sleep(2), COUNT(*) FROM stress_test WHERE data LIKE '%a%';" &
done

echo "5️⃣  Creating transaction load..."
for i in {1..20}; do
    psql -c "BEGIN; INSERT INTO stress_test (data) VALUES ('tx_$i'); COMMIT;" &
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
echo "Waiting for all queries to complete..."
wait
echo "✅ Stress test completed!"

# Show current stats
echo ""
echo "📈 Final Database Stats:"
psql -c "
SELECT 
    (SELECT count(*) FROM pg_stat_activity WHERE state = 'active') as active_conns,
    (SELECT count(*) FROM stress_test) as total_rows,
    (SELECT pg_size_pretty(pg_database_size(current_database()))) as db_size;
"
