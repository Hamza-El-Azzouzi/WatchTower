use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, time::Instant};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HostMetrics {
    pub load_1: f64,
    pub load_5: f64,
    pub load_15: f64,
    pub uptime_seconds: u64,
    pub cpu_user_percent: f64,
    pub cpu_system_percent: f64,
    pub cpu_iowait_percent: f64,
    pub cpu_steal_percent: f64,
    pub memory_available_bytes: u64,
    pub memory_cached_bytes: u64,
    pub swap_in_bytes_per_sec: f64,
    pub swap_out_bytes_per_sec: f64,
    pub oom_kills_total: u64,
    pub oom_kills_delta: u64,
    pub tcp_total: u64,
    pub tcp_established: u64,
    pub tcp_listen: u64,
    pub tcp_time_wait: u64,
    pub tcp_close_wait: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct CpuTimes {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

impl CpuTimes {
    fn total(self) -> u64 {
        self.user
            .saturating_add(self.nice)
            .saturating_add(self.system)
            .saturating_add(self.idle)
            .saturating_add(self.iowait)
            .saturating_add(self.irq)
            .saturating_add(self.softirq)
            .saturating_add(self.steal)
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct VmCounters {
    swap_in_pages: u64,
    swap_out_pages: u64,
    oom_kills: u64,
}

pub struct HostCollector {
    previous_cpu: Option<CpuTimes>,
    previous_vm: Option<VmCounters>,
    previous_at: Instant,
    page_size: u64,
}

impl HostCollector {
    pub fn new() -> Self {
        // SAFETY: sysconf is thread-safe and has no pointer arguments.
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        Self {
            previous_cpu: None,
            previous_vm: None,
            previous_at: Instant::now(),
            page_size: if page_size > 0 {
                page_size as u64
            } else {
                4096
            },
        }
    }

    pub fn collect(&mut self) -> HostMetrics {
        let now = Instant::now();
        let elapsed = now
            .duration_since(self.previous_at)
            .as_secs_f64()
            .max(0.001);
        let current_cpu = read_cpu_times();
        let current_vm = read_vm_counters();
        let (load_1, load_5, load_15) = read_load_average();
        let memory = read_meminfo();
        let tcp = read_tcp_counts();
        let mut metrics = HostMetrics {
            load_1,
            load_5,
            load_15,
            uptime_seconds: read_uptime(),
            memory_available_bytes: memory.get("MemAvailable").copied().unwrap_or(0),
            memory_cached_bytes: memory
                .get("Cached")
                .copied()
                .unwrap_or(0)
                .saturating_add(memory.get("SReclaimable").copied().unwrap_or(0)),
            oom_kills_total: current_vm.oom_kills,
            tcp_total: tcp.total,
            tcp_established: tcp.established,
            tcp_listen: tcp.listen,
            tcp_time_wait: tcp.time_wait,
            tcp_close_wait: tcp.close_wait,
            ..HostMetrics::default()
        };
        if let Some(previous) = self.previous_cpu {
            let total = current_cpu.total().saturating_sub(previous.total());
            if total > 0 {
                let percent = |value: u64| value as f64 * 100.0 / total as f64;
                metrics.cpu_user_percent = percent(
                    current_cpu
                        .user
                        .saturating_sub(previous.user)
                        .saturating_add(current_cpu.nice.saturating_sub(previous.nice)),
                );
                metrics.cpu_system_percent = percent(
                    current_cpu
                        .system
                        .saturating_sub(previous.system)
                        .saturating_add(current_cpu.irq.saturating_sub(previous.irq))
                        .saturating_add(current_cpu.softirq.saturating_sub(previous.softirq)),
                );
                metrics.cpu_iowait_percent =
                    percent(current_cpu.iowait.saturating_sub(previous.iowait));
                metrics.cpu_steal_percent =
                    percent(current_cpu.steal.saturating_sub(previous.steal));
            }
        }
        if let Some(previous) = self.previous_vm {
            metrics.swap_in_bytes_per_sec = current_vm
                .swap_in_pages
                .saturating_sub(previous.swap_in_pages)
                as f64
                * self.page_size as f64
                / elapsed;
            metrics.swap_out_bytes_per_sec = current_vm
                .swap_out_pages
                .saturating_sub(previous.swap_out_pages)
                as f64
                * self.page_size as f64
                / elapsed;
            metrics.oom_kills_delta = current_vm.oom_kills.saturating_sub(previous.oom_kills);
        }
        self.previous_cpu = Some(current_cpu);
        self.previous_vm = Some(current_vm);
        self.previous_at = now;
        metrics
    }
}

impl Default for HostCollector {
    fn default() -> Self {
        Self::new()
    }
}

fn read_cpu_times() -> CpuTimes {
    let content = fs::read_to_string("/proc/stat").unwrap_or_default();
    let values: Vec<u64> = content
        .lines()
        .find(|line| line.starts_with("cpu "))
        .unwrap_or_default()
        .split_whitespace()
        .skip(1)
        .filter_map(|value| value.parse().ok())
        .collect();
    CpuTimes {
        user: values.first().copied().unwrap_or(0),
        nice: values.get(1).copied().unwrap_or(0),
        system: values.get(2).copied().unwrap_or(0),
        idle: values.get(3).copied().unwrap_or(0),
        iowait: values.get(4).copied().unwrap_or(0),
        irq: values.get(5).copied().unwrap_or(0),
        softirq: values.get(6).copied().unwrap_or(0),
        steal: values.get(7).copied().unwrap_or(0),
    }
}

fn read_load_average() -> (f64, f64, f64) {
    let content = fs::read_to_string("/proc/loadavg").unwrap_or_default();
    let mut values = content
        .split_whitespace()
        .take(3)
        .filter_map(|value| value.parse::<f64>().ok());
    (
        values.next().unwrap_or(0.0),
        values.next().unwrap_or(0.0),
        values.next().unwrap_or(0.0),
    )
}

fn read_uptime() -> u64 {
    fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|content| content.split_whitespace().next()?.parse::<f64>().ok())
        .map(|value| value.max(0.0) as u64)
        .unwrap_or(0)
}

fn read_meminfo() -> HashMap<String, u64> {
    fs::read_to_string("/proc/meminfo")
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            let kib = value.split_whitespace().next()?.parse::<u64>().ok()?;
            Some((key.to_string(), kib.saturating_mul(1024)))
        })
        .collect()
}

fn read_vm_counters() -> VmCounters {
    let content = fs::read_to_string("/proc/vmstat").unwrap_or_default();
    let values: HashMap<&str, u64> = content
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            Some((parts.next()?, parts.next()?.parse().ok()?))
        })
        .collect();
    VmCounters {
        swap_in_pages: values.get("pswpin").copied().unwrap_or(0),
        swap_out_pages: values.get("pswpout").copied().unwrap_or(0),
        oom_kills: values.get("oom_kill").copied().unwrap_or(0),
    }
}

#[derive(Default)]
struct TcpCounts {
    total: u64,
    established: u64,
    listen: u64,
    time_wait: u64,
    close_wait: u64,
}

fn read_tcp_counts() -> TcpCounts {
    let mut counts = TcpCounts::default();
    for path in ["/proc/net/tcp", "/proc/net/tcp6"] {
        for line in fs::read_to_string(path).unwrap_or_default().lines().skip(1) {
            let state = line.split_whitespace().nth(3).unwrap_or_default();
            counts.total += 1;
            match state {
                "01" => counts.established += 1,
                "0A" => counts.listen += 1,
                "06" => counts.time_wait += 1,
                "08" => counts.close_wait += 1,
                _ => {}
            }
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collectors_are_safe_when_proc_is_available() {
        let metrics = HostCollector::new().collect();
        assert!(metrics.load_1.is_finite());
        assert!(metrics.uptime_seconds > 0);
    }
}
