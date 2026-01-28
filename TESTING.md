# Testing Guide

This document describes the testing infrastructure and procedures for the DevOps Monitoring System.

## Test Organization

The project includes three types of tests:

### 1. Unit Tests

Unit tests are located in `tests.rs` modules within each source file:

- **Configuration Tests** (`server/src/config/tests.rs`)
  - Default configuration values
  - Environment variable overrides
  - Invalid value handling
  
- **Authentication Tests** (`server/src/auth/tests.rs`)
  - API key generation and validation
  - Agent tracking and limits
  - Key expiration
  - Key listing and revocation

### 2. Integration Tests

Integration tests verify the system works correctly when all components are combined:

- **API Tests** (`server/tests/api_tests.rs`)
  - HTTP endpoint functionality
  - Requires server binary to be built
  - Marked with `#[ignore]` - run manually when needed

### 3. Benchmarks

Performance benchmarks measure system throughput and response times:

- **Performance Benchmarks** (`server/benches/performance.rs`)
  - Single metric ingestion
  - Batch metric ingestion
  - Metric retrieval
  - Database queries
  - Concurrent operations
  - Agent listing

## Running Tests

### Run All Unit Tests

```bash
cd server
cargo test
```

Expected output:
```
running 18 tests
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Run Specific Test Module

```bash
# Config tests only
cargo test config::tests

# Auth tests only
cargo test auth::tests

# Specific test
cargo test test_create_api_key
```

### Run Integration Tests

Integration tests are marked as `#[ignore]` because they require the full server binary:

```bash
# Build the server first
cargo build

# Run ignored tests
cargo test --test api_tests -- --ignored
```

### Run Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench metric_insert_single
```

Benchmark reports are generated in `target/criterion/` with HTML visualizations.

## Test Coverage

### Configuration Module (10 tests)
- ✅ Default configuration
- ✅ Bind address parsing
- ✅ Environment variable overrides (database, server, alerting, logging)
- ✅ Invalid value handling

### Authentication Module (8 tests)
- ✅ API key generation (format and uniqueness)
- ✅ Key creation with various options
- ✅ Key validation (success and failure cases)
- ✅ Agent tracking and limits
- ✅ Key expiration
- ✅ Key listing

### Integration Tests (2 tests)
- ⚠️ Health check endpoint (requires server binary)
- ⚠️ Metrics ingestion (requires server binary)

## Continuous Integration

Tests run automatically on every push via GitHub Actions (`.github/workflows/ci.yml`):

### CI Pipeline Jobs

1. **test-server** - Runs all server unit tests
2. **test-agent** - Runs agent tests (when available)
3. **test-dashboard** - Runs dashboard tests (when available)
4. **build-docker** - Builds Docker images
5. **integration-test** - Runs docker-compose integration tests
6. **benchmark** - Runs performance benchmarks on main branch

### Running CI Locally

You can run the docker-compose integration tests locally:

```bash
# Build containers
docker-compose build

# Run full stack
docker-compose up -d

# Wait for services to start
sleep 10

# Test API endpoints
curl http://localhost:8080/health
curl -H "X-API-Key: your-key" http://localhost:8080/api/agents

# View logs
docker-compose logs -f server

# Clean up
docker-compose down
```

## Writing New Tests

### Unit Test Template

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_feature_name() {
        // Arrange
        let input = setup_test_data();
        
        // Act
        let result = function_under_test(input).await;
        
        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_value);
    }
}
```

### Database Test Setup

For tests requiring a database:

```rust
use sqlx::SqlitePool;

async fn create_test_db() -> SqlitePool {
    // Use in-memory database for isolation
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    
    // Run migrations
    sqlx::query(/* CREATE TABLE ... */)
        .execute(&pool)
        .await
        .unwrap();
    
    pool
}

#[tokio::test]
async fn test_with_database() {
    let pool = create_test_db().await;
    // ... test code
}
```

### Benchmark Template

```rust
use criterion::{criterion_group, criterion_main, Criterion, black_box};

fn benchmark_function(c: &mut Criterion) {
    c.bench_function("operation_name", |b| {
        b.iter(|| {
            // Operation to benchmark
            black_box(expensive_operation())
        });
    });
}

criterion_group!(benches, benchmark_function);
criterion_main!(benches);
```

## Performance Metrics

Benchmark results are tracked to monitor performance regressions:

- **Metric Ingestion**: ~X ms per metric
- **Batch Insert**: ~Y ms per batch of 100 metrics
- **Metric Retrieval**: ~Z ms per query
- **Concurrent Operations**: N requests/second

_(Actual numbers will be available after first benchmark run)_

## Test Databases

Tests use in-memory SQLite databases (`sqlite::memory:`) for:
- Fast execution
- Isolation between tests
- No cleanup required
- Parallel test execution

## Troubleshooting

### Tests fail with "unable to open database file"

The test is using a file-based database instead of in-memory. Update the test to use:
```rust
SqlitePool::connect("sqlite::memory:").await.unwrap()
```

### Benchmarks show high variance

Benchmarks are sensitive to system load. For consistent results:
- Close other applications
- Run benchmarks multiple times
- Use `--sample-size` to increase measurement count

### Integration tests timeout

Increase server startup wait time or check if the server is actually starting:
```bash
# Check if server can start
cargo run --bin monitor-server &
sleep 2
curl http://localhost:8080/health
```

## Next Steps

- [ ] Add agent module tests when implemented
- [ ] Add dashboard component tests
- [ ] Increase test coverage to 80%+
- [ ] Add property-based tests with proptest
- [ ] Add mutation testing
- [ ] Set up code coverage reporting

## Resources

- [Rust Testing Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Tokio Testing Guide](https://tokio.rs/tokio/topics/testing)
- [Criterion Benchmarking](https://bheisler.github.io/criterion.rs/book/)
- [SQLx Testing](https://github.com/launchbadge/sqlx#testing)
