#![allow(dead_code)]

use sysinfo::{CpuRefreshKind, RefreshKind, System};

pub struct CpuCollector {
    system: System,
    last_refresh: std::time::Instant,
}

impl CpuCollector {
    pub fn new() -> Self {
        let mut system =
            System::new_with_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::everything()));

        // Initial refresh to prime the CPU data
        system.refresh_cpu();

        Self {
            system,
            last_refresh: std::time::Instant::now(),
        }
    }

    pub fn collect(&mut self) -> f32 {
        // Refresh CPU data
        self.system.refresh_cpu();
        self.last_refresh = std::time::Instant::now();

        // Calculate average CPU usage across all cores
        let total_usage: f32 = self.system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum();
        let cpu_count = self.system.cpus().len() as f32;

        if cpu_count > 0.0 {
            total_usage / cpu_count
        } else {
            0.0
        }
    }

    /// Collect per-core CPU usage percentages
    /// Uses the already-refreshed CPU data from the last collect() call
    pub fn collect_per_core(&self) -> Vec<f32> {
        self.system
            .cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .collect()
    }

    /// Collect both average and per-core CPU in one operation (more efficient and accurate)
    pub fn collect_all(&mut self) -> (f32, Vec<f32>) {
        // Single refresh for accuracy - sysinfo internally tracks changes from last refresh
        self.system.refresh_cpu();
        self.last_refresh = std::time::Instant::now();

        let per_core: Vec<f32> = self
            .system
            .cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .collect();

        let total_usage: f32 = per_core.iter().sum();
        let cpu_count = per_core.len() as f32;

        let average = if cpu_count > 0.0 {
            total_usage / cpu_count
        } else {
            0.0
        };

        (average, per_core)
    }
}

impl Default for CpuCollector {
    fn default() -> Self {
        Self::new()
    }
}
