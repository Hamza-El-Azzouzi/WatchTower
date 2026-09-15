use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, time::Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceSnapshot {
    pub interface: String,
    pub rx_bytes_per_sec: f64,
    pub tx_bytes_per_sec: f64,
    pub rx_packets_per_sec: f64,
    pub tx_packets_per_sec: f64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_dropped: u64,
    pub tx_dropped: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct Counters {
    rx_bytes: u64,
    rx_packets: u64,
    rx_errors: u64,
    rx_dropped: u64,
    tx_bytes: u64,
    tx_packets: u64,
    tx_errors: u64,
    tx_dropped: u64,
}

pub struct NetworkCollector {
    previous: HashMap<String, Counters>,
    previous_at: Instant,
}

impl NetworkCollector {
    pub fn new() -> Self {
        Self {
            previous: read_counters(),
            previous_at: Instant::now(),
        }
    }

    pub fn collect_all(&mut self) -> ((u64, u64), Vec<NetworkInterfaceSnapshot>) {
        let now = Instant::now();
        let elapsed = now
            .duration_since(self.previous_at)
            .as_secs_f64()
            .max(0.001);
        let current = read_counters();
        let mut total_rx = 0.0;
        let mut total_tx = 0.0;
        let mut interfaces = Vec::new();
        for (name, counters) in &current {
            let previous = self.previous.get(name).copied().unwrap_or(*counters);
            let rx_rate = counters.rx_bytes.saturating_sub(previous.rx_bytes) as f64 / elapsed;
            let tx_rate = counters.tx_bytes.saturating_sub(previous.tx_bytes) as f64 / elapsed;
            if name != "lo" {
                total_rx += rx_rate;
                total_tx += tx_rate;
            }
            interfaces.push(NetworkInterfaceSnapshot {
                interface: name.clone(),
                rx_bytes_per_sec: rx_rate,
                tx_bytes_per_sec: tx_rate,
                rx_packets_per_sec: counters.rx_packets.saturating_sub(previous.rx_packets) as f64
                    / elapsed,
                tx_packets_per_sec: counters.tx_packets.saturating_sub(previous.tx_packets) as f64
                    / elapsed,
                rx_errors: counters.rx_errors,
                tx_errors: counters.tx_errors,
                rx_dropped: counters.rx_dropped,
                tx_dropped: counters.tx_dropped,
            });
        }
        interfaces.sort_by(|left, right| left.interface.cmp(&right.interface));
        self.previous = current;
        self.previous_at = now;
        (
            (total_rx.max(0.0) as u64, total_tx.max(0.0) as u64),
            interfaces,
        )
    }
}

impl Default for NetworkCollector {
    fn default() -> Self {
        Self::new()
    }
}

fn read_counters() -> HashMap<String, Counters> {
    fs::read_to_string("/proc/net/dev")
        .unwrap_or_default()
        .lines()
        .skip(2)
        .filter_map(|line| {
            let (name, values) = line.split_once(':')?;
            let values: Vec<u64> = values
                .split_whitespace()
                .filter_map(|value| value.parse().ok())
                .collect();
            if values.len() < 16 {
                return None;
            }
            Some((
                name.trim().to_string(),
                Counters {
                    rx_bytes: values[0],
                    rx_packets: values[1],
                    rx_errors: values[2],
                    rx_dropped: values[3],
                    tx_bytes: values[8],
                    tx_packets: values[9],
                    tx_errors: values[10],
                    tx_dropped: values[11],
                },
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rates_remain_finite() {
        let (_, interfaces) = NetworkCollector::new().collect_all();
        assert!(interfaces
            .iter()
            .all(|interface| interface.rx_bytes_per_sec.is_finite()));
    }
}
