#![allow(dead_code)]

use std::{fs, path::Path};
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
        self.components.refresh_list();
        self.components.refresh();

        let cpu_temps: Vec<f32> = self
            .components
            .iter()
            .filter(|comp| is_cpu_sensor(comp.label()))
            .map(|comp| comp.temperature())
            .filter(|temp| valid_temperature(*temp))
            .collect();

        if cpu_temps.is_empty() {
            thermal_cpu_temperature(Path::new("/sys/class/thermal"))
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
                    || label.contains("amdgpu")
                    || label.contains("radeon")
            })
            .map(|comp| comp.temperature())
            .filter(|temp| valid_temperature(*temp))
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
            .filter(|comp| is_cpu_sensor(comp.label()))
            .filter_map(|comp| comp.critical())
            .next()
    }
}

fn is_cpu_sensor(label: &str) -> bool {
    let label = label.to_lowercase();
    !label.contains("gpu")
        && [
            "cpu",
            "core",
            "processor",
            "k10temp",
            "k8temp",
            "zenpower",
            "tctl",
            "tdie",
            "x86_pkg_temp",
        ]
        .iter()
        .any(|name| label.contains(name))
}

fn valid_temperature(value: f32) -> bool {
    value.is_finite() && (-40.0..=150.0).contains(&value)
}

fn thermal_cpu_temperature(root: &Path) -> Option<f32> {
    let temperatures: Vec<f32> = fs::read_dir(root)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            if !entry
                .file_name()
                .to_string_lossy()
                .starts_with("thermal_zone")
            {
                return None;
            }
            let kind = fs::read_to_string(entry.path().join("type")).ok()?;
            if !is_cpu_sensor(kind.trim()) {
                return None;
            }
            let value = fs::read_to_string(entry.path().join("temp"))
                .ok()?
                .trim()
                .parse::<f32>()
                .ok()?
                / 1000.0;
            valid_temperature(value).then_some(value)
        })
        .collect();
    (!temperatures.is_empty()).then(|| temperatures.iter().sum::<f32>() / temperatures.len() as f32)
}

impl Default for TemperatureCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod temperature_tests {
    use super::*;

    #[test]
    fn recognizes_cpu_drivers_without_misclassifying_gpu_or_storage() {
        for label in [
            "coretemp Package id 0",
            "k10temp Tctl",
            "cpu-thermal",
            "x86_pkg_temp",
        ] {
            assert!(is_cpu_sensor(label));
        }
        for label in ["amdgpu", "nvme Composite", "acpitz", "gpu-core"] {
            assert!(!is_cpu_sensor(label));
        }
        assert!(valid_temperature(0.0));
        assert!(!valid_temperature(f32::NAN));
        assert!(!valid_temperature(1000.0));
    }
}
