#![allow(dead_code)]

use sysinfo::{Components, System};

pub struct GpuCollector {
    system: System,
}

impl GpuCollector {
    pub fn new() -> Self {
        let system = System::new();
        Self { system }
    }

    /// Collect GPU usage percentage (if available)
    /// Note: sysinfo doesn't directly provide GPU metrics on all platforms
    /// Returns None if GPU metrics are not available
    pub fn collect_usage(&mut self) -> Option<f32> {
        // GPU metrics are platform-specific and may not be available
        // through sysinfo on all systems. This is a placeholder that
        // can be enhanced with platform-specific GPU libraries like:
        // - nvml-wrapper for NVIDIA GPUs
        // - rocm-smi for AMD GPUs
        // For now, we return None to indicate unavailable
        None
    }

    /// Collect GPU memory usage in bytes (if available)
    pub fn collect_memory(&mut self) -> Option<(u64, u64)> {
        // Returns (used_memory, total_memory) in bytes
        // Platform-specific implementation needed
        None
    }
}

impl Default for GpuCollector {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TemperatureCollector {
    components: Components,
}

impl TemperatureCollector {
    pub fn new() -> Self {
        let mut components = Components::new_with_refreshed_list();
        components.refresh();
        Self { components }
    }

    /// Collect CPU temperature in Celsius
    /// Returns average of all CPU temperature sensors
    pub fn collect_cpu_temp(&mut self) -> Option<f32> {
        self.components.refresh();

        let cpu_temps: Vec<f32> = self
            .components
            .iter()
            .filter(|comp| {
                let label = comp.label().to_lowercase();
                label.contains("cpu") || label.contains("core") || label.contains("processor")
            })
            .map(|comp| comp.temperature())
            .collect();

        if cpu_temps.is_empty() {
            None
        } else {
            Some(cpu_temps.iter().sum::<f32>() / cpu_temps.len() as f32)
        }
    }

    /// Collect GPU temperature in Celsius
    /// Returns average of all GPU temperature sensors
    pub fn collect_gpu_temp(&mut self) -> Option<f32> {
        self.components.refresh();

        let gpu_temps: Vec<f32> = self
            .components
            .iter()
            .filter(|comp| {
                let label = comp.label().to_lowercase();
                label.contains("gpu")
                    || label.contains("nvidia")
                    || label.contains("amd")
                    || label.contains("radeon")
            })
            .map(|comp| comp.temperature())
            .collect();

        if gpu_temps.is_empty() {
            None
        } else {
            Some(gpu_temps.iter().sum::<f32>() / gpu_temps.len() as f32)
        }
    }

    /// Get critical temperature threshold (if available)
    pub fn get_cpu_critical_temp(&self) -> Option<f32> {
        self.components
            .iter()
            .filter(|comp| {
                let label = comp.label().to_lowercase();
                label.contains("cpu") || label.contains("core")
            })
            .filter_map(|comp| comp.critical())
            .next()
    }
}

impl Default for TemperatureCollector {
    fn default() -> Self {
        Self::new()
    }
}
