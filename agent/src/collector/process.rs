use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind, Users};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub user: String,
    pub state: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub virtual_memory_bytes: u64,
    pub disk_read_bytes: u64,
    pub disk_written_bytes: u64,
    pub run_time_seconds: u64,
    /// Executable name only. Command-line arguments are intentionally excluded.
    pub command: String,
}

/// Collects aggregate metrics for an explicit allow-list of executable names.
/// Restricting collection avoids leaking arbitrary command lines or environment data.
pub struct ProcessCollector {
    system: System,
    users: Users,
}

impl ProcessCollector {
    pub fn new() -> Self {
        Self {
            system: System::new_with_specifics(
                RefreshKind::new().with_processes(
                    ProcessRefreshKind::new()
                        .with_cpu()
                        .with_memory()
                        .with_disk_usage()
                        .with_user(UpdateKind::OnlyIfNotSet),
                ),
            ),
            users: Users::new_with_refreshed_list(),
        }
    }

    pub fn collect(
        &mut self,
        watched_names: &[String],
        process_limit: usize,
    ) -> (HashMap<String, f64>, Vec<ProcessSnapshot>) {
        self.system.refresh_processes();

        let mut metrics = HashMap::new();
        metrics.insert(
            "process_total".to_string(),
            self.system.processes().len() as f64,
        );

        for watched_name in watched_names {
            let trimmed = watched_name.trim();
            if trimmed.is_empty() {
                continue;
            }

            let key = metric_key(trimmed);
            if key.is_empty() {
                continue;
            }

            let matching: Vec<_> = self
                .system
                .processes()
                .values()
                .filter(|process| process.name().eq_ignore_ascii_case(trimmed))
                .collect();

            let instances = matching.len();
            let cpu_usage: f64 = matching
                .iter()
                .map(|process| process.cpu_usage() as f64)
                .sum();
            let memory_bytes: u64 = matching.iter().map(|process| process.memory()).sum();

            metrics.insert(
                format!("process_{key}_running"),
                (instances > 0) as u8 as f64,
            );
            metrics.insert(format!("process_{key}_instances"), instances as f64);
            metrics.insert(format!("process_{key}_cpu_usage"), cpu_usage);
            metrics.insert(format!("process_{key}_memory_bytes"), memory_bytes as f64);
        }

        let mut processes: Vec<_> = self
            .system
            .processes()
            .iter()
            .map(|(pid, process)| {
                let disk = process.disk_usage();
                ProcessSnapshot {
                    pid: pid.as_u32(),
                    parent_pid: process.parent().map(|pid| pid.as_u32()),
                    user: process
                        .user_id()
                        .and_then(|user_id| self.users.get_user_by_id(user_id))
                        .map(|user| user.name().to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    state: format!("{:?}", process.status()).to_lowercase(),
                    cpu_percent: process.cpu_usage(),
                    memory_bytes: process.memory(),
                    virtual_memory_bytes: process.virtual_memory(),
                    disk_read_bytes: disk.read_bytes,
                    disk_written_bytes: disk.written_bytes,
                    run_time_seconds: process.run_time(),
                    command: process.name().chars().take(96).collect(),
                }
            })
            .collect();

        processes.sort_by(|left, right| {
            right
                .cpu_percent
                .total_cmp(&left.cpu_percent)
                .then_with(|| right.memory_bytes.cmp(&left.memory_bytes))
                .then_with(|| left.pid.cmp(&right.pid))
        });
        processes.truncate(process_limit.min(256));

        (metrics, processes)
    }
}

fn metric_key(name: &str) -> String {
    name.chars()
        .take(48)
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::metric_key;

    #[test]
    fn creates_bounded_safe_metric_keys() {
        assert_eq!(metric_key("Postgres Worker"), "postgres_worker");
        assert!(metric_key(&"a".repeat(100)).len() <= 48);
        assert_eq!(metric_key("../"), "");
    }
}
