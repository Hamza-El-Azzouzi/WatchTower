use sysinfo::Disks;

pub struct DiskCollector {
    disks: Disks,
}

impl DiskCollector {
    pub fn new() -> Self {
        let disks = Disks::new_with_refreshed_list();
        Self { disks }
    }

    pub fn collect(&mut self) -> (u64, u64, f32) {
        self.disks.refresh_list();
        self.disks.refresh();

        let mut total_space = 0u64;
        let mut used_space = 0u64;

        for disk in self.disks.iter() {
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total.saturating_sub(available);

            total_space = total_space.saturating_add(total);
            used_space = used_space.saturating_add(used);
        }

        let percent = if total_space > 0 {
            (used_space as f64 / total_space as f64) * 100.0
        } else {
            0.0
        };

        (used_space, total_space, percent as f32)
    }

    #[allow(dead_code)]
    pub fn collect_per_disk(&mut self) -> Vec<DiskInfo> {
        self.disks.refresh_list();
        self.disks.refresh();

        self.disks
            .iter()
            .map(|disk| {
                let name = disk.name().to_string_lossy().to_string();
                let mount_point = disk.mount_point().to_string_lossy().to_string();
                let total = disk.total_space();
                let available = disk.available_space();
                let used = total.saturating_sub(available);
                let percent = if total > 0 {
                    (used as f64 / total as f64) * 100.0
                } else {
                    0.0
                };

                DiskInfo {
                    name,
                    mount_point,
                    used,
                    total,
                    percent: percent as f32,
                }
            })
            .collect()
    }
}

impl Default for DiskCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub used: u64,
    pub total: u64,
    pub percent: f32,
}
