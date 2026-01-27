# Monitor Agent

Phase 1 basic monitoring agent that collects system metrics.

## Building

```bash
cd agent
cargo build --release
```

## Running

Run with default configuration:
```bash
cargo run
```

Run with custom configuration file:
```bash
cargo run -- --config agent.toml
```

Run with custom interval (overrides config file):
```bash
cargo run -- --config agent.toml --interval 5
```

Run with verbose logging:
```bash
cargo run -- --config agent.toml --verbose
```

## Configuration

Edit `agent.toml` to configure the agent:

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

## Expected Output

```
-------------------- Monitoring Agent Started ---------------------
Agent: dev-server-01 | Interval: 10s

[2024-01-27 10:30:45] CPU: 45.2%, Memory: 4.00 GB/8.00 GB (50.0%), Disk: 100.00 GB/500.00 GB (20.0%), Network: RX 1.20 MB | TX 800.50 KB
[2024-01-27 10:30:55] CPU: 47.1%, Memory: 4.10 GB/8.00 GB (51.2%), Disk: 100.00 GB/500.00 GB (20.0%), Network: RX 1.25 MB | TX 850.20 KB
```

## Features

- ✅ CPU usage monitoring (average across all cores)
- ✅ Memory usage monitoring (used/total with percentage)
- ✅ Disk usage monitoring (aggregated across all disks)
- ✅ Network traffic monitoring (total RX/TX bytes)
- ✅ Configurable collection interval
- ✅ TOML configuration file support
- ✅ CLI arguments for runtime overrides
- ✅ Human-readable output with timestamps
- ✅ Formatted byte sizes (KB, MB, GB, TB)
