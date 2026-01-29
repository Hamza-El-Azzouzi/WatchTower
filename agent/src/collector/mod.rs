pub mod cpu;
pub mod disk;
pub mod gpu;
pub mod logs;
pub mod memory;
pub mod network;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use logs::{LogCollector, LogsPayload};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_percent: f32,
    pub cpu_per_core: Vec<f32>, // Per-core CPU usage
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub memory_percent: f32,
    pub swap_used_bytes: u64,  // Swap memory used
    pub swap_total_bytes: u64, // Swap memory total
    pub swap_percent: f32,     // Swap usage percentage
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_percent: f32,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub cpu_temp_celsius: Option<f32>,  // CPU temperature
    pub gpu_temp_celsius: Option<f32>,  // GPU temperature
    pub gpu_usage_percent: Option<f32>, // GPU usage
    pub gpu_memory_used: Option<u64>,   // GPU memory used
    pub gpu_memory_total: Option<u64>,  // GPU memory total
}

impl SystemMetrics {
    /// Convert SystemMetrics to a HashMap for sending to the server
    pub fn to_metrics_map(&self) -> HashMap<String, f64> {
        let mut metrics = HashMap::new();

        // Basic metrics
        metrics.insert("cpu_usage".to_string(), self.cpu_percent as f64);
        metrics.insert("memory_usage".to_string(), self.memory_percent as f64);
        metrics.insert(
            "memory_used_bytes".to_string(),
            self.memory_used_bytes as f64,
        );
        metrics.insert(
            "memory_total_bytes".to_string(),
            self.memory_total_bytes as f64,
        );
        metrics.insert("swap_usage".to_string(), self.swap_percent as f64);
        metrics.insert("swap_used_bytes".to_string(), self.swap_used_bytes as f64);
        metrics.insert("swap_total_bytes".to_string(), self.swap_total_bytes as f64);
        metrics.insert("disk_usage".to_string(), self.disk_percent as f64);
        metrics.insert("disk_used_bytes".to_string(), self.disk_used_bytes as f64);
        metrics.insert("disk_total_bytes".to_string(), self.disk_total_bytes as f64);
        metrics.insert("network_rx_bytes".to_string(), self.network_rx_bytes as f64);
        metrics.insert("network_tx_bytes".to_string(), self.network_tx_bytes as f64);

        // Per-core CPU metrics
        for (i, usage) in self.cpu_per_core.iter().enumerate() {
            metrics.insert(format!("cpu_core_{}", i), *usage as f64);
        }

        // Temperature metrics
        if let Some(temp) = self.cpu_temp_celsius {
            metrics.insert("cpu_temp_celsius".to_string(), temp as f64);
        }
        if let Some(temp) = self.gpu_temp_celsius {
            metrics.insert("gpu_temp_celsius".to_string(), temp as f64);
        }

        // GPU metrics
        if let Some(usage) = self.gpu_usage_percent {
            metrics.insert("gpu_usage".to_string(), usage as f64);
        }
        if let Some(used) = self.gpu_memory_used {
            metrics.insert("gpu_memory_used_bytes".to_string(), used as f64);
        }
        if let Some(total) = self.gpu_memory_total {
            metrics.insert("gpu_memory_total_bytes".to_string(), total as f64);
        }

        metrics
    }

    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        const TB: u64 = GB * 1024;

        if bytes >= TB {
            format!("{:.2} TB", bytes as f64 / TB as f64)
        } else if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    pub fn display(&self) {
        println!(
            "CPU: {:.1}%, Memory: {}/{} ({:.1}%), Disk: {}/{} ({:.1}%), Network: RX {} | TX {}",
            self.cpu_percent,
            Self::format_bytes(self.memory_used_bytes),
            Self::format_bytes(self.memory_total_bytes),
            self.memory_percent,
            Self::format_bytes(self.disk_used_bytes),
            Self::format_bytes(self.disk_total_bytes),
            self.disk_percent,
            Self::format_bytes(self.network_rx_bytes),
            Self::format_bytes(self.network_tx_bytes)
        );
    }
}
