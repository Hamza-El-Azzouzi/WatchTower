# Enhanced Metrics Implementation

This document describes the new advanced metrics added to the DevOps Monitoring System.

## Overview

The monitoring system now collects and displays:
- **Per-Core CPU Usage**: Individual CPU core metrics (cpu_core_0, cpu_core_1, etc.)
- **Swap Memory**: Swap usage percentage and bytes (swap_usage, swap_used_bytes, swap_total_bytes)
- **CPU Temperature**: Temperature monitoring in Celsius (cpu_temp_celsius)
- **GPU Temperature**: GPU temperature monitoring (gpu_temp_celsius)
- **GPU Usage**: GPU utilization percentage (gpu_usage)
- **GPU Memory**: GPU memory usage in bytes (gpu_memory_used_bytes, gpu_memory_total_bytes)

## Agent-Side Implementation

### 1. New Collectors

#### GPU Collector (`agent/src/collector/gpu.rs`)
```rust
pub struct GpuCollector {
    system: System,
}

pub struct TemperatureCollector {
    components: Components,
}
```

**Features:**
- GPU usage percentage (platform-specific, may not be available on all systems)
- GPU memory usage and total
- CPU temperature from system sensors
- GPU temperature from system sensors

**Note:** GPU metrics availability depends on platform and hardware. The collector returns `Option<T>` types that will be `None` if metrics are not available.

#### Temperature Monitoring
Uses sysinfo's `Components` API to read temperature sensors:
- Filters sensors by label (cpu, core, processor, gpu, nvidia, amd, radeon)
- Returns average temperature across all matching sensors
- Provides critical temperature thresholds (if available)

### 2. Enhanced Collectors

#### CPU Collector Updates
```rust
pub fn collect_per_core(&mut self) -> Vec<f32>
```
Returns individual CPU core usage percentages as a vector.

#### Memory Collector Updates
```rust
pub fn collect_swap(&mut self) -> (u64, u64, f32)
```
Returns (used_bytes, total_bytes, percentage) for swap memory.

### 3. SystemMetrics Structure

Updated structure in `agent/src/collector/mod.rs`:

```rust
pub struct SystemMetrics {
    // Original metrics
    pub cpu_percent: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub memory_percent: f32,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_percent: f32,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    
    // NEW: Per-core CPU
    pub cpu_per_core: Vec<f32>,
    
    // NEW: Swap memory
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_percent: f32,
    
    // NEW: Temperature
    pub cpu_temp_celsius: Option<f32>,
    pub gpu_temp_celsius: Option<f32>,
    
    // NEW: GPU metrics
    pub gpu_usage_percent: Option<f32>,
    pub gpu_memory_used: Option<u64>,
    pub gpu_memory_total: Option<u64>,
}
```

### 4. Metrics Payload Conversion

The `to_metrics_map()` method automatically converts all metrics to a HashMap:

```rust
pub fn to_metrics_map(&self) -> HashMap<String, f64> {
    // Basic metrics
    metrics.insert("cpu_usage", self.cpu_percent as f64);
    metrics.insert("memory_usage", self.memory_percent as f64);
    metrics.insert("swap_usage", self.swap_percent as f64);
    
    // Per-core metrics
    for (i, usage) in self.cpu_per_core.iter().enumerate() {
        metrics.insert(format!("cpu_core_{}", i), *usage as f64);
    }
    
    // Optional metrics (only if available)
    if let Some(temp) = self.cpu_temp_celsius {
        metrics.insert("cpu_temp_celsius", temp as f64);
    }
    // ... GPU metrics
}
```

## Server-Side Implementation

The server's existing time-series storage automatically handles all metric types:
- Stores metrics in `HashMap<String, Vec<DataPoint>>`
- No schema changes needed
- Query API supports filtering by any metric name

## Dashboard Implementation

### 1. Enhanced MetricsSection Component

Located in `components/MetricsSection.tsx`, now displays:

**First Row - Core Metrics:**
- CPU Usage (with core count)
- Memory Usage
- Disk Usage
- Network Traffic

**Second Row - Extended Metrics:**
- Swap Memory (usage % and bytes)
- CPU Temperature (with color-coded status)
- GPU Temperature (with color-coded status)
- GPU Usage (with memory if available)

**Third Row - Per-Core Breakdown:**
- Grid showing individual CPU core usage
- Color-coded progress bars per core
- Automatically scales to number of cores detected

### 2. Temperature Color Coding

```typescript
const getTempColor = (temp: number | null): string => {
    if (!temp) return 'gray';
    if (temp >= 80) return 'red';      // Hot
    if (temp >= 70) return 'amber';    // Warm
    if (temp >= 60) return 'yellow';   // Elevated
    return 'emerald';                  // Normal
};
```

### 3. Per-Core CPU Display

```tsx
<div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 xl:grid-cols-8 gap-3">
  {cpuCores.map((usage, index) => (
    <div key={index}>
      <div>Core {index}</div>
      <div>{usage.toFixed(1)}%</div>
      <progress-bar width={usage} />
    </div>
  ))}
</div>
```

## Metric Availability

### Always Available
- Per-core CPU usage
- Swap memory (may be 0 if no swap configured)

### Platform/Hardware Dependent
- CPU temperature (requires temperature sensors)
- GPU metrics (requires dedicated GPU and platform support)

### Handling Unavailable Metrics

The dashboard gracefully handles missing metrics:
- Displays "N/A" for unavailable values
- Shows "Not available" as secondary status
- Hides sections if no data is present

## Usage Example

### Agent Configuration

No configuration changes needed. New metrics are automatically collected.

### API Query Examples

```bash
# Get all metrics for an agent
curl http://localhost:8000/api/v1/metrics/agent-1?limit=10

# Query specific CPU core
curl http://localhost:8000/api/v1/metrics/agent-1?metric=cpu_core_0&limit=100

# Query temperatures
curl http://localhost:8000/api/v1/metrics/agent-1?metric=cpu_temp_celsius&limit=50
curl http://localhost:8000/api/v1/metrics/agent-1?metric=gpu_temp_celsius&limit=50

# Query GPU metrics
curl http://localhost:8000/api/v1/metrics/agent-1?metric=gpu_usage&limit=100
curl http://localhost:8000/api/v1/metrics/agent-1?metric=gpu_memory_used_bytes&limit=100

# Query swap
curl http://localhost:8000/api/v1/metrics/agent-1?metric=swap_usage&limit=100
```

## Performance Impact

### Agent
- Per-core CPU: Minimal overhead (same sysinfo refresh)
- Temperature: ~1ms per refresh (reads /sys/class/thermal on Linux)
- GPU: Negligible (currently returns None, platform-specific libs would add ~5-10ms)

### Server
- Storage: Scales linearly with number of cores (typically 4-32 additional metrics per agent)
- Memory: ~48 bytes per data point

### Dashboard
- Rendering: Per-core grid uses CSS Grid for efficient layout
- Updates: Same 10-second refresh interval

## Future Enhancements

### GPU Support
To enable actual GPU metrics, integrate platform-specific libraries:

**NVIDIA:**
```toml
[dependencies]
nvml-wrapper = "0.9"
```

**AMD:**
```toml
[dependencies]
rocm-smi = "0.3"
```

### Alert Thresholds
Add temperature-based alerts:
- CPU temperature > 80°C
- GPU temperature > 85°C
- Individual core usage > 95%

### Trends
- Temperature trends over time
- Per-core usage distribution histograms
- GPU utilization patterns

## Testing

### Verify Per-Core CPU
```bash
# Start agent
./target/release/monitor-agent -c agent.toml

# Check metrics sent
# You should see cpu_core_0, cpu_core_1, etc. in the metrics payload
```

### Verify Temperature
```bash
# On Linux, check available sensors
sensors

# Start agent and verify temperatures appear in dashboard
```

### Verify Dashboard
```bash
cd dev-ops-monitoring-dashboard
npm run dev

# Navigate to server detail page
# You should see:
# - 8 metric cards (4 + 4)
# - Per-core CPU grid
# - Temperature readings (if available)
```

## Troubleshooting

### No Temperature Data
- Check if sensors are available: `sensors` (Linux) or `sysctl -a | grep temperature` (macOS)
- Some systems require additional kernel modules
- Virtual machines may not expose temperature sensors

### No GPU Data
- Currently returns None on all platforms
- Requires platform-specific GPU library integration
- Check hardware: `lspci | grep -i vga` (Linux)

### High Memory Usage
- Each CPU core adds one metric per collection interval
- For 32-core systems collecting every 5 seconds: ~6000 data points/hour
- Adjust retention policy if needed

## Summary

The enhanced metrics implementation provides comprehensive system monitoring with:
✅ Per-core CPU visibility
✅ Swap memory tracking
✅ Temperature monitoring (when available)
✅ GPU metrics framework (ready for platform-specific implementation)
✅ Backward compatibility (existing metrics unchanged)
✅ Graceful degradation (missing metrics shown as N/A)
