use sysinfo::{CpuRefreshKind, RefreshKind, System};

pub struct CpuCollector {
    system: System,
}

impl CpuCollector {
    pub fn new() -> Self {
        let system =
            System::new_with_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::everything()));
        Self { system }
    }

    pub fn collect(&mut self) -> f32 {
        // First refresh to get initial values
        self.system.refresh_cpu();

        // Sleep briefly to get accurate CPU usage
        std::thread::sleep(std::time::Duration::from_millis(200));

        // Second refresh to calculate usage
        self.system.refresh_cpu();

        // Calculate average CPU usage across all cores
        let total_usage: f32 = self.system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum();
        let cpu_count = self.system.cpus().len() as f32;

        if cpu_count > 0.0 {
            total_usage / cpu_count
        } else {
            0.0
        }
    }

    #[allow(dead_code)]
    pub fn collect_per_core(&mut self) -> Vec<f32> {
        self.system.refresh_cpu();
        std::thread::sleep(std::time::Duration::from_millis(200));
        self.system.refresh_cpu();

        self.system
            .cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .collect()
    }
}

impl Default for CpuCollector {
    fn default() -> Self {
        Self::new()
    }
}
