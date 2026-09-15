use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSnapshot {
    pub name: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub main_pid: u32,
    pub restart_count: u64,
    pub active_for_seconds: u64,
}

#[derive(Default)]
pub struct ServiceCollector;

impl ServiceCollector {
    pub fn collect(
        &self,
        configured_names: &[String],
        uptime_seconds: u64,
    ) -> Vec<ServiceSnapshot> {
        let units: Vec<String> = configured_names
            .iter()
            .filter_map(|name| normalize_unit(name))
            .take(32)
            .collect();
        if units.is_empty() {
            return Vec::new();
        }

        let output = Command::new("systemctl")
            .arg("show")
            .arg("--no-pager")
            .arg("--property=Id,LoadState,ActiveState,SubState,MainPID,NRestarts,ActiveEnterTimestampMonotonic")
            .args(&units)
            .output();
        let Ok(output) = output else {
            return Vec::new();
        };
        // systemctl returns non-zero when any requested unit is missing, but it
        // still prints valid properties for every unit. Preserve that useful
        // partial snapshot so one removed service cannot hide the others.
        parse_systemctl(&String::from_utf8_lossy(&output.stdout), uptime_seconds)
    }
}

fn normalize_unit(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 128
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '@' | '_' | '.' | ':' | '-')
        })
    {
        return None;
    }
    Some(if value.ends_with(".service") {
        value.to_string()
    } else {
        format!("{value}.service")
    })
}

fn parse_systemctl(output: &str, uptime_seconds: u64) -> Vec<ServiceSnapshot> {
    let mut snapshots = Vec::new();
    let mut current = ServiceSnapshot {
        name: String::new(),
        load_state: String::new(),
        active_state: String::new(),
        sub_state: String::new(),
        main_pid: 0,
        restart_count: 0,
        active_for_seconds: 0,
    };
    let finish = |current: &mut ServiceSnapshot, snapshots: &mut Vec<ServiceSnapshot>| {
        if !current.name.is_empty() {
            snapshots.push(current.clone());
            current.name.clear();
        }
    };
    for line in output.lines().chain(std::iter::once("")) {
        if line.is_empty() {
            finish(&mut current, &mut snapshots);
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "Id" => current.name = value.to_string(),
            "LoadState" => current.load_state = value.to_string(),
            "ActiveState" => current.active_state = value.to_string(),
            "SubState" => current.sub_state = value.to_string(),
            "MainPID" => current.main_pid = value.parse().unwrap_or(0),
            "NRestarts" => current.restart_count = value.parse().unwrap_or(0),
            "ActiveEnterTimestampMonotonic" => {
                let active_at_us = value.parse::<u64>().unwrap_or(0);
                current.active_for_seconds =
                    uptime_seconds.saturating_sub(active_at_us / 1_000_000);
            }
            _ => {}
        }
    }
    snapshots
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_service_properties() {
        let output = "Id=nginx.service\nLoadState=loaded\nActiveState=active\nSubState=running\nMainPID=42\nNRestarts=3\nActiveEnterTimestampMonotonic=5000000\n\n";
        let result = parse_systemctl(output, 15);
        assert_eq!(result[0].restart_count, 3);
        assert_eq!(result[0].active_for_seconds, 10);
    }
    #[test]
    fn rejects_unsafe_unit_names() {
        assert!(normalize_unit("nginx;shutdown").is_none());
    }
}
