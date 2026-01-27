# Phase 1: Basic Monitoring Agent - Implementation Guide

## Overview

Phase 1 focuses on building a foundational monitoring agent that collects system metrics locally and displays them in the console. This phase establishes the core architecture and data collection capabilities that will be extended in later phases.

**Status**: ✅ Complete

**Duration**: Week 1-2

**Effort**: ~15-20 hours

---

## Objectives Achieved

### Primary Goals
- ✅ Collect CPU usage (average across all cores)
- ✅ Collect memory usage (used, total, percentage)
- ✅ Collect disk usage (aggregated across all disks)
- ✅ Collect network traffic (RX/TX bytes)
- ✅ Display metrics in console with timestamps
- ✅ Support configuration via TOML file
- ✅ Provide CLI interface with argument parsing

### Technical Achievements
- Rust project structure established
- Async runtime configured with Tokio
- Modular collector architecture
- Type-safe configuration system
- Human-readable output formatting

---

## Architecture

### Project Structure

```
agent/
├── Cargo.toml              # Project dependencies and metadata
├── agent.toml              # Configuration file
├── README.md               # Quick reference documentation
└── src/
    ├── main.rs             # Entry point, CLI, and main loop
    ├── config.rs           # Configuration parsing and structures
    └── collector/
        ├── mod.rs          # SystemMetrics struct and formatting
        ├── cpu.rs          # CPU usage collection
        ├── memory.rs       # Memory usage collection
        ├── disk.rs         # Disk usage collection
        └── network.rs      # Network traffic collection
```

### Component Diagram

```
┌─────────────────────────────────────────────────────────┐
│                    Main Application                      │
│                      (main.rs)                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │   CLI Args   │  │    Config    │  │  Collection  │ │
│  │   (Clap)     │─▶│   (TOML)     │─▶│    Loop      │ │
│  └──────────────┘  └──────────────┘  └──────┬───────┘ │
│                                              │          │
└──────────────────────────────────────────────┼──────────┘
                                               │
                    ┌──────────────────────────┴─────────────┐
                    │        Metric Collectors               │
                    │         (collector/)                   │
                    │  ┌─────────┐  ┌─────────┐            │
                    │  │   CPU   │  │ Memory  │            │
                    │  └─────────┘  └─────────┘            │
                    │  ┌─────────┐  ┌─────────┐            │
                    │  │  Disk   │  │ Network │            │
                    │  └─────────┘  └─────────┘            │
                    └────────────────────────────────────────┘
                                     │
                                     ▼
                            Console Output (stdout)
```

---

## Implementation Details

### 1. Dependencies (Cargo.toml)

```toml
[dependencies]
tokio = { version = "1.35", features = ["full"] }  # Async runtime
sysinfo = "0.30"                                   # System metrics
serde = { version = "1.0", features = ["derive"] } # Serialization
toml = "0.8"                                       # Config parsing
clap = { version = "4.4", features = ["derive"] }  # CLI parsing
chrono = "0.4"                                     # Timestamps
anyhow = "1.0"                                     # Error handling
tracing = "0.1"                                    # Logging
tracing-subscriber = "0.3"                         # Log output
```

### 2. Configuration System

**File**: `src/config.rs`

Implements a type-safe configuration system using Serde and TOML:

```rust
pub struct Config {
    pub agent: AgentConfig,
    pub collection: CollectionConfig,
    pub metrics: MetricsConfig,
}
```

**Features**:
- Default values for all fields
- Validation on load
- Override via CLI arguments
- Graceful fallback to defaults on error

**Example Configuration** (`agent.toml`):
```toml
[agent]
name = "dev-server-01"

[collection]
interval_seconds = 10

[metrics]
collect_cpu = true
collect_memory = true
collect_disk = true
collect_network = true
```

### 3. Metric Collectors

Each collector is responsible for a specific metric category:

#### CPU Collector (`collector/cpu.rs`)

**Method**: `collect() -> f32`

**Implementation**:
- Uses `sysinfo::System` with CPU refresh kind
- Performs two refreshes with 200ms delay for accurate measurement
- Averages usage across all CPU cores
- Returns percentage (0.0 - 100.0)

**Additional Methods**:
- `collect_per_core() -> Vec<f32>` - Per-core CPU usage (for future phases)

#### Memory Collector (`collector/memory.rs`)

**Method**: `collect() -> (u64, u64, f32)`

**Returns**: (used_bytes, total_bytes, percentage)

**Implementation**:
- Refreshes memory information
- Queries used and total RAM
- Calculates percentage
- Handles division by zero

**Additional Methods**:
- `collect_swap() -> (u64, u64)` - Swap memory stats (for future phases)

#### Disk Collector (`collector/disk.rs`)

**Method**: `collect() -> (u64, u64, f32)`

**Returns**: (used_bytes, total_bytes, percentage)

**Implementation**:
- Refreshes disk list and usage
- Aggregates across all mounted disks
- Calculates total space and used space
- Returns percentage of total capacity

**Additional Methods**:
- `collect_per_disk() -> Vec<DiskInfo>` - Individual disk stats (for future phases)

#### Network Collector (`collector/network.rs`)

**Method**: `collect() -> (u64, u64)`

**Returns**: (rx_bytes, tx_bytes)

**Implementation**:
- Refreshes network interfaces
- Aggregates total received and transmitted bytes
- Returns cumulative totals since system boot

**Additional Methods**:
- `collect_per_interface() -> Vec<NetworkInfo>` - Per-interface stats (for future phases)

### 4. Main Application Loop

**File**: `src/main.rs`

**Flow**:
1. Parse CLI arguments with Clap
2. Initialize logging (info or debug level)
3. Load configuration from file or use defaults
4. Apply CLI overrides (e.g., interval)
5. Initialize all collectors
6. Enter collection loop:
   - Wait for interval timer tick
   - Collect all enabled metrics
   - Format timestamp
   - Display metrics with human-readable formatting
   - Repeat indefinitely

**Key Features**:
- Async/await with Tokio runtime
- Graceful error handling
- Configurable logging levels
- Clean console output formatting

### 5. Output Formatting

**File**: `collector/mod.rs`

**Human-Readable Bytes**:
```rust
format_bytes(bytes: u64) -> String
```

Converts raw byte values to KB, MB, GB, or TB with appropriate precision.

**Display Format**:
```
[YYYY-MM-DD HH:MM:SS] CPU: X.X%, Memory: X.XX GB/X.XX GB (X.X%), Disk: X.XX GB/X.XX GB (X.X%), Network: RX X.XX MB | TX X.XX KB
```

---

## Building and Running

### Prerequisites

- Rust 1.70 or later
- Cargo package manager

### Build Commands

**Development build**:
```bash
cd agent
cargo build
```

**Release build** (optimized):
```bash
cargo build --release
```

### Running the Agent

**With default configuration**:
```bash
cargo run
```

**With custom config file**:
```bash
cargo run -- --config agent.toml
```

**With custom interval** (override config):
```bash
cargo run -- --config agent.toml --interval 5
```

**With verbose logging**:
```bash
cargo run -- --verbose
```

**Run release build**:
```bash
./target/release/monitor-agent --config agent.toml
```

### Expected Output

```
-------------------- Monitoring Agent Started ---------------------
Agent: dev-server-01 | Interval: 10s

[2026-01-27 10:30:45] CPU: 45.2%, Memory: 4.00 GB/8.00 GB (50.0%), Disk: 100.00 GB/500.00 GB (20.0%), Network: RX 1.20 GB | TX 800.50 MB
[2026-01-27 10:30:55] CPU: 47.1%, Memory: 4.10 GB/8.00 GB (51.2%), Disk: 100.00 GB/500.00 GB (20.0%), Network: RX 1.25 GB | TX 850.20 MB
[2026-01-27 10:31:05] CPU: 43.8%, Memory: 4.05 GB/8.00 GB (50.6%), Disk: 100.00 GB/500.00 GB (20.0%), Network: RX 1.30 GB | TX 900.10 MB
```

---

## Testing

### Manual Testing

1. **Verify CPU collection**:
   - Run agent
   - Open CPU-intensive application
   - Observe CPU percentage increase

2. **Verify memory collection**:
   - Run agent
   - Note initial memory usage
   - Open memory-intensive application
   - Observe memory usage increase

3. **Verify disk collection**:
   - Check disk usage with `df -h`
   - Compare with agent output
   - Values should match

4. **Verify network collection**:
   - Note initial network bytes
   - Download a large file
   - Observe RX bytes increase

### Configuration Testing

**Test default configuration**:
```bash
cargo run
# Should use defaults: 10s interval, all metrics enabled
```

**Test custom interval**:
```bash
cargo run -- --interval 5
# Should collect every 5 seconds
```

**Test selective metrics**:
Edit `agent.toml`:
```toml
[metrics]
collect_cpu = true
collect_memory = false
collect_disk = false
collect_network = false
```
Run and verify only CPU is shown.

### Performance Testing

**CPU overhead**:
```bash
# Terminal 1: Run agent
cargo run --release

# Terminal 2: Monitor agent CPU usage
top -p $(pgrep monitor-agent)
# Should use < 2% CPU
```

**Memory overhead**:
```bash
ps aux | grep monitor-agent
# Should use < 50 MB RAM
```

---

## Technical Decisions

### Why Rust?

- **Performance**: Low overhead, critical for monitoring
- **Safety**: Memory safety without garbage collection
- **Concurrency**: Excellent async/await support with Tokio
- **System access**: Direct OS API access via sysinfo crate

### Why Tokio?

- Industry-standard async runtime
- Excellent timer support for collection intervals
- Prepares for future HTTP client/server needs
- Non-blocking I/O for network operations

### Why sysinfo?

- Cross-platform (Linux, Windows, macOS)
- Well-maintained and actively developed
- Comprehensive system metrics
- Safe abstractions over OS APIs

### Design Patterns Used

1. **Builder Pattern**: Configuration with defaults
2. **Module Pattern**: Separate collectors by concern
3. **Trait Implementation**: Default trait for collectors
4. **Error Handling**: Result types with anyhow for context

---

## Challenges and Solutions

### Challenge 1: Accurate CPU Measurement

**Problem**: Single CPU refresh returns 0% or inaccurate values.

**Solution**: Perform two refreshes with 200ms delay to measure actual usage over time.

```rust
self.system.refresh_cpu();
std::thread::sleep(Duration::from_millis(200));
self.system.refresh_cpu();
```

### Challenge 2: Network Bytes are Cumulative

**Problem**: Network bytes are cumulative since boot, not per-interval.

**Solution**: Display total bytes. In Phase 2, we'll calculate deltas for rate.

**Future Enhancement**:
```rust
let rate_rx = (current_rx - previous_rx) / interval_seconds;
```

### Challenge 3: API Compatibility

**Problem**: sysinfo 0.30 changed API for System initialization.

**Solution**: Use `System::new()` instead of `System::new_with_specifics()`.

### Challenge 4: Unused Code Warnings

**Problem**: Per-core and per-disk methods not used in Phase 1.

**Solution**: Add `#[allow(dead_code)]` annotation. These will be used in future phases for detailed metrics.

---

## Code Quality

### Formatting

```bash
cargo fmt
```

### Linting

```bash
cargo clippy
```

### Documentation

```bash
cargo doc --open
```

---

## Future Enhancements (Phase 2+)

### Immediate Next Steps (Phase 2)

- [ ] HTTP client to send metrics to central server
- [ ] JSON serialization of metrics
- [ ] Retry logic with exponential backoff
- [ ] Buffering for offline resilience

### Potential Improvements

- [ ] Per-core CPU metrics display
- [ ] Per-disk usage breakdown
- [ ] Per-interface network stats
- [ ] Process monitoring (top N by CPU/memory)
- [ ] Temperature sensors
- [ ] Battery status (for laptops)
- [ ] GPU metrics

---

## Troubleshooting

### Issue: Agent won't compile

**Error**: `error: no bin target named monitor-agent`

**Solution**: Ensure you're in the `agent` directory:
```bash
cd agent
cargo build
```

### Issue: Permission denied reading system info

**Error**: Some metrics return 0 or error

**Solution**: Run with appropriate permissions:
```bash
sudo cargo run --release
```

### Issue: High CPU usage

**Symptom**: Agent uses > 5% CPU

**Cause**: Collection interval too short or CPU refresh delay too long

**Solution**: Increase interval or reduce refresh delay:
```toml
[collection]
interval_seconds = 15  # Increase from 10
```

### Issue: Network bytes don't update

**Symptom**: RX/TX bytes stay constant

**Explanation**: Network bytes are cumulative totals since boot, not per-interval rates. If no network activity occurs, values remain the same. Transfer data to see changes.

---

## Learning Outcomes

By completing Phase 1, you learned:

### Rust Skills
- Project structure and module organization
- Working with external crates
- Async/await programming with Tokio
- Error handling with Result and anyhow
- Trait implementations (Default)
- Derive macros (Serialize, Deserialize, Parser)

### System Programming
- Reading OS-level metrics
- Working with system APIs via abstractions
- Understanding CPU measurement techniques
- Memory management concepts
- Disk I/O monitoring
- Network interface statistics

### DevOps Concepts
- What metrics matter for system health
- Collection intervals and their tradeoffs
- Monitoring agent architecture
- Configuration management
- Observability foundations

### Software Engineering
- Modular design and separation of concerns
- Configuration-driven applications
- CLI tool development
- Human-readable output formatting
- Code organization for maintainability

---

## Metrics Reference

### CPU Usage
- **Unit**: Percentage (0.0 - 100.0)
- **Meaning**: Average CPU utilization across all cores
- **Interpretation**:
  - 0-30%: Low usage
  - 30-70%: Moderate usage
  - 70-90%: High usage
  - 90-100%: Saturated

### Memory Usage
- **Unit**: Bytes (displayed as GB)
- **Meaning**: RAM currently in use vs. total available
- **Interpretation**:
  - < 80%: Healthy
  - 80-90%: Monitor closely
  - > 90%: Memory pressure

### Disk Usage
- **Unit**: Bytes (displayed as GB)
- **Meaning**: Total disk space used across all mounted drives
- **Interpretation**:
  - < 70%: Healthy
  - 70-85%: Monitor closely
  - 85-95%: Plan cleanup
  - > 95%: Critical

### Network Traffic
- **Unit**: Bytes (displayed as MB/GB)
- **Meaning**: Cumulative bytes received/transmitted since boot
- **Note**: In Phase 2, we'll calculate rates (MB/s)

---

## Resources

### Documentation
- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [sysinfo Documentation](https://docs.rs/sysinfo/)
- [Clap Documentation](https://docs.rs/clap/)

### Related Reading
- [Google SRE Book - Monitoring](https://sre.google/sre-book/monitoring-distributed-systems/)
- [The Four Golden Signals](https://sre.google/sre-book/monitoring-distributed-systems/#xref_monitoring_golden-signals)

---

## Summary

Phase 1 successfully established the foundation for our monitoring system:

✅ **Core functionality**: System metrics collection working reliably
✅ **Architecture**: Modular, extensible design ready for future phases
✅ **Configuration**: Flexible TOML-based configuration with CLI overrides
✅ **User experience**: Clean, readable console output
✅ **Code quality**: Well-structured, documented, and maintainable

**Next Phase**: [Phase 2 - Central Server Metrics API](PHASE_2_GUIDE.md)

---

**Last Updated**: January 27, 2026
**Phase Status**: Complete ✅
**Next Milestone**: Phase 2 - Server Integration
