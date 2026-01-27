use sysinfo::System;

pub struct MemoryCollector {
    system: System,
}

impl MemoryCollector {
    pub fn new() -> Self {
        let system = System::new();
        Self { system }
    }

    pub fn collect(&mut self) -> (u64, u64, f32) {
        self.system.refresh_memory();

        let total = self.system.total_memory();
        let used = self.system.used_memory();
        let percent = if total > 0 {
            (used as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        (used, total, percent)
    }

    #[allow(dead_code)]
    pub fn collect_swap(&mut self) -> (u64, u64) {
        self.system.refresh_memory();
        (self.system.used_swap(), self.system.total_swap())
    }
}

impl Default for MemoryCollector {
    fn default() -> Self {
        Self::new()
    }
}
