use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    ffi::CString,
    fs,
    os::unix::ffi::OsStrExt,
    path::Path,
    time::Instant,
};
use sysinfo::Disks;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountSnapshot {
    pub device: String,
    pub mount_point: String,
    pub filesystem: String,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub total_bytes: u64,
    pub usage_percent: f64,
    pub inodes_used: u64,
    pub inodes_total: u64,
    pub inode_usage_percent: f64,
    pub read_bytes_per_sec: f64,
    pub write_bytes_per_sec: f64,
    pub read_iops: f64,
    pub write_iops: f64,
    pub average_latency_ms: f64,
}

#[derive(Debug, Clone, Copy, Default)]
struct DiskIoCounters {
    reads: u64,
    sectors_read: u64,
    read_ms: u64,
    writes: u64,
    sectors_written: u64,
    write_ms: u64,
}

pub struct DiskCollector {
    disks: Disks,
    previous_io: HashMap<String, DiskIoCounters>,
    previous_at: Instant,
}

impl DiskCollector {
    pub fn new() -> Self {
        Self {
            disks: Disks::new_with_refreshed_list(),
            previous_io: read_diskstats(),
            previous_at: Instant::now(),
        }
    }

    pub fn collect_all(&mut self) -> ((u64, u64, f32), Vec<MountSnapshot>) {
        self.disks.refresh_list();
        self.disks.refresh();
        let now = Instant::now();
        let elapsed = now
            .duration_since(self.previous_at)
            .as_secs_f64()
            .max(0.001);
        let current_io = read_diskstats();
        let mut total_space = 0u64;
        let mut used_space = 0u64;
        let mut seen_mounts = HashSet::new();
        let mut mounts = Vec::new();

        for disk in self.disks.iter() {
            let mount_point = disk.mount_point().to_string_lossy().to_string();
            if !seen_mounts.insert(mount_point.clone()) {
                continue;
            }
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total.saturating_sub(available);
            total_space = total_space.saturating_add(total);
            used_space = used_space.saturating_add(used);

            let device = disk.name().to_string_lossy().to_string();
            let resolved_device =
                fs::canonicalize(&device).unwrap_or_else(|_| Path::new(&device).to_path_buf());
            let device_key = resolved_device
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| device.clone());
            let current = current_io.get(&device_key).copied().unwrap_or_default();
            let previous = self
                .previous_io
                .get(&device_key)
                .copied()
                .unwrap_or(current);
            let reads = current.reads.saturating_sub(previous.reads);
            let writes = current.writes.saturating_sub(previous.writes);
            let operations = reads.saturating_add(writes);
            let io_time = current
                .read_ms
                .saturating_sub(previous.read_ms)
                .saturating_add(current.write_ms.saturating_sub(previous.write_ms));
            let (inodes_total, inodes_available) = inode_counts(disk.mount_point());
            let inodes_used = inodes_total.saturating_sub(inodes_available);

            mounts.push(MountSnapshot {
                device,
                mount_point,
                filesystem: disk.file_system().to_string_lossy().to_string(),
                used_bytes: used,
                available_bytes: available,
                total_bytes: total,
                usage_percent: ratio(used, total),
                inodes_used,
                inodes_total,
                inode_usage_percent: ratio(inodes_used, inodes_total),
                read_bytes_per_sec: current.sectors_read.saturating_sub(previous.sectors_read)
                    as f64
                    * 512.0
                    / elapsed,
                write_bytes_per_sec: current
                    .sectors_written
                    .saturating_sub(previous.sectors_written)
                    as f64
                    * 512.0
                    / elapsed,
                read_iops: reads as f64 / elapsed,
                write_iops: writes as f64 / elapsed,
                average_latency_ms: if operations > 0 {
                    io_time as f64 / operations as f64
                } else {
                    0.0
                },
            });
        }

        self.previous_io = current_io;
        self.previous_at = now;
        (
            (
                used_space,
                total_space,
                ratio(used_space, total_space) as f32,
            ),
            mounts,
        )
    }
}

impl Default for DiskCollector {
    fn default() -> Self {
        Self::new()
    }
}

fn ratio(used: u64, total: u64) -> f64 {
    if total > 0 {
        used as f64 * 100.0 / total as f64
    } else {
        0.0
    }
}

fn read_diskstats() -> HashMap<String, DiskIoCounters> {
    fs::read_to_string("/proc/diskstats")
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 14 {
                return None;
            }
            Some((
                fields[2].to_string(),
                DiskIoCounters {
                    reads: fields[3].parse().ok()?,
                    sectors_read: fields[5].parse().ok()?,
                    read_ms: fields[6].parse().ok()?,
                    writes: fields[7].parse().ok()?,
                    sectors_written: fields[9].parse().ok()?,
                    write_ms: fields[10].parse().ok()?,
                },
            ))
        })
        .collect()
}

fn inode_counts(path: &Path) -> (u64, u64) {
    let Ok(path) = CString::new(path.as_os_str().as_bytes()) else {
        return (0, 0);
    };
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: path is NUL-terminated and stats points to writable memory.
    let result = unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) };
    if result != 0 {
        return (0, 0);
    }
    // SAFETY: statvfs returned success and initialized the structure.
    let stats = unsafe { stats.assume_init() };
    (stats.f_files, stats.f_favail)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collects_bounded_mount_data() {
        let (_, mounts) = DiskCollector::new().collect_all();
        assert!(mounts.iter().all(|mount| mount.usage_percent.is_finite()));
    }
}
