# Phase 5: Database Monitoring

## Overview
Phase 5 adds comprehensive database monitoring capabilities to track database performance, connections, queries, and health metrics.

## Supported Databases
- **PostgreSQL** (via `postgres` feature)
- **MySQL** (via `mysql` feature)

## Metrics Collected

### Connection Metrics
- `db_connections_active` - Number of active database connections
- `db_connections_idle` - Number of idle connections
- `db_connections_max` - Maximum allowed connections

### Query Metrics
- `db_queries_per_second` - Query throughput
- `db_slow_queries` - Number of slow queries (>1 second)

### Performance Metrics
- `db_cache_hit_ratio` - Buffer cache hit ratio (%)
- `db_transactions_committed` - Total committed transactions
- `db_transactions_rolled_back` - Total rolled back transactions

### Storage Metrics
- `db_database_size_bytes` - Total database size in bytes
- `db_locks_waiting` - Number of locks waiting to be acquired

## Configuration

Add a `[database]` section to your `agent.toml`:

```toml
[database]
enabled = true
db_type = "postgres"  # or "mysql"
host = "localhost"
port = 5432
database = "mydb"
username = "dbuser"
password = "dbpass"
```

## Building with Database Support

### PostgreSQL Only
```bash
cargo build --release --features postgres
```

### MySQL Only
```bash
cargo build --release --features mysql
```

### All Databases
```bash
cargo build --release --features all-databases
```

## Testing

### Start Test Databases
```bash
cd docs/database-testing
docker-compose up -d
```

### Run Agent with Database Monitoring
```bash
cd agent
cargo run --release --features postgres -- -c examples/agent-with-database.toml
```

### Verify Metrics
```bash
# The agent will now send database metrics along with system metrics
curl -s "http://localhost:8080/api/v1/metrics/latest?agent_id=db-monitor-01" | jq '.metrics[] | select(.name | startswith("db_"))'
```

## PostgreSQL-Specific Queries

The agent queries the following PostgreSQL system views:
- `pg_stat_activity` - Active connections and queries
- `pg_stat_database` - Database-level statistics
- `pg_settings` - Configuration parameters
- `pg_locks` - Lock information

## MySQL-Specific Queries

The agent queries:
- `SHOW STATUS` - Server status variables
- `SHOW VARIABLES` - Configuration variables
- `information_schema.TABLES` - Table sizes

## Dashboard Integration

Database metrics will appear in the dashboard with the `db_` prefix and can be visualized in dedicated database monitoring charts.

## Security Notes

- Store database credentials securely
- Use read-only database users when possible
- Consider using connection strings with SSL/TLS
- Rotate credentials regularly

## Performance Impact

Database metric collection typically adds:
- **PostgreSQL**: ~10-20ms per collection cycle
- **MySQL**: ~15-30ms per collection cycle

The agent runs queries that only read system tables and should not impact database performance significantly.
