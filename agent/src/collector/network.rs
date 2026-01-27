use sysinfo::Networks;

pub struct NetworkCollector {
    networks: Networks,
}

impl NetworkCollector {
    pub fn new() -> Self {
        let networks = Networks::new_with_refreshed_list();
        Self { networks }
    }

    pub fn collect(&mut self) -> (u64, u64) {
        self.networks.refresh_list();
        self.networks.refresh();

        let mut total_rx = 0u64;
        let mut total_tx = 0u64;

        for (_, network) in self.networks.iter() {
            total_rx = total_rx.saturating_add(network.total_received());
            total_tx = total_tx.saturating_add(network.total_transmitted());
        }

        (total_rx, total_tx)
    }

    #[allow(dead_code)]
    pub fn collect_per_interface(&mut self) -> Vec<NetworkInfo> {
        self.networks.refresh_list();
        self.networks.refresh();

        self.networks
            .iter()
            .map(|(name, network)| NetworkInfo {
                interface: name.to_string(),
                rx_bytes: network.total_received(),
                tx_bytes: network.total_transmitted(),
                rx_packets: network.total_packets_received(),
                tx_packets: network.total_packets_transmitted(),
                rx_errors: network.total_errors_on_received(),
                tx_errors: network.total_errors_on_transmitted(),
            })
            .collect()
    }
}

impl Default for NetworkCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct NetworkInfo {
    pub interface: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
}
