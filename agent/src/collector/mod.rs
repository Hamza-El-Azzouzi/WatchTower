pub mod cpu;
pub mod disk;
pub mod logs;
pub mod memory;
pub mod network;

use serde::{Deserialize, Serialize};

pub use logs::{LogCollector, LogEntryInput, LogLevel, LogsPayload};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_percent: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub memory_percent: f32,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_percent: f32,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

impl SystemMetrics {
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
