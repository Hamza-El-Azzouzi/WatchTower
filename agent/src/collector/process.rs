use std::collections::HashMap;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

/// Collects aggregate metrics for an explicit allow-list of executable names.
/// Restricting collection avoids leaking arbitrary command lines or environment data.
pub struct ProcessCollector {
    system: System,
}

impl ProcessCollector {
    pub fn new() -> Self {
        Self {
            system: System::new_with_specifics(
                RefreshKind::new()
                    .with_processes(ProcessRefreshKind::new().with_cpu().with_memory()),
            ),
        }
    }

    pub fn collect(&mut self, watched_names: &[String]) -> HashMap<String, f64> {
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

        metrics
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
