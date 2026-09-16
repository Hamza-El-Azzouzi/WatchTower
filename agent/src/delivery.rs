//! FIFO spool: persist before upload, delete only after the server's durable ACK.
use crate::sender::MetricsPayload;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeliveryIdentity {
    pub stream_id: String,
    pub sequence: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload")]
pub enum QueuedPayload {
    Metrics(MetricsPayload),
    Logs(crate::collector::LogsPayload),
}
impl From<MetricsPayload> for QueuedPayload {
    fn from(payload: MetricsPayload) -> Self {
        Self::Metrics(payload)
    }
}
impl From<crate::collector::LogsPayload> for QueuedPayload {
    fn from(payload: crate::collector::LogsPayload) -> Self {
        Self::Logs(payload)
    }
}
impl QueuedPayload {
    fn set_identity(&mut self, identity: DeliveryIdentity) {
        match self {
            Self::Metrics(payload) => payload.delivery = Some(identity),
            Self::Logs(payload) => payload.delivery = Some(identity),
        }
    }
    #[cfg(test)]
    fn identity(&self) -> &DeliveryIdentity {
        match self {
            Self::Metrics(payload) => payload.delivery.as_ref().unwrap(),
            Self::Logs(payload) => payload.delivery.as_ref().unwrap(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct State {
    stream_id: String,
    next_sequence: u64,
    last_successful_upload: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct DiskQueue {
    directory: PathBuf,
    state: State,
    max_bytes: u64,
    max_records: usize,
    pub dropped_samples: u64,
    // Held for the lifetime of the spool: two agents must never share a sequence space.
    _lock: fs::File,
    records: std::collections::BTreeMap<u64, (PathBuf, u64)>,
    used_bytes: u64,
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = path.with_extension("tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    fs::rename(&temporary, path)?;
    fs::File::open(path.parent().context("missing parent")?)?.sync_all()?;
    Ok(())
}

impl DiskQueue {
    pub fn open(directory: PathBuf, max_bytes: u64, max_records: usize) -> Result<Self> {
        if max_bytes == 0 || max_records == 0 {
            bail!("delivery storage limits must be positive");
        }
        fs::create_dir_all(&directory)?;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(directory.join("lock"))?;
        use std::os::fd::AsRawFd;
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            bail!("delivery spool is already in use");
        }
        // Interrupted atomic writes are never acknowledged records.
        for entry in fs::read_dir(&directory)? {
            let path = entry?.path();
            if path.extension().is_some_and(|extension| extension == "tmp") {
                fs::remove_file(path)?;
            }
        }
        let state_path = directory.join("state.json");
        let mut state: State = if state_path.exists() {
            serde_json::from_slice(&fs::read(&state_path)?)
                .context("invalid delivery state; refusing to reset sequence")?
        } else {
            let mut random = [0u8; 16];
            fs::File::open("/dev/urandom")?.read_exact(&mut random)?;
            State {
                stream_id: random.iter().map(|byte| format!("{byte:02x}")).collect(),
                next_sequence: 1,
                last_successful_upload: None,
            }
        };
        // Recover a record committed just before its sequence checkpoint.
        let mut records = std::collections::BTreeMap::new();
        let mut used_bytes = 0u64;
        for entry in fs::read_dir(&directory)? {
            let path = entry?.path();
            if path.extension().is_none_or(|extension| extension != "json") {
                continue;
            }
            if let Some(sequence) = path
                .file_stem()
                .and_then(|name| name.to_str())
                .and_then(|name| name.parse::<u64>().ok())
            {
                let bytes = fs::metadata(&path)?.len();
                records.insert(sequence, (path, bytes));
                used_bytes = used_bytes.saturating_add(bytes);
                state.next_sequence = state
                    .next_sequence
                    .max(sequence.checked_add(1).context("sequence exhausted")?);
            }
        }
        atomic_write(&state_path, &serde_json::to_vec(&state)?)?;
        Ok(Self {
            directory,
            state,
            max_bytes,
            max_records,
            dropped_samples: 0,
            _lock: lock,
            records,
            used_bytes,
        })
    }

    pub fn status(&self) -> Result<(usize, u64, Option<chrono::DateTime<chrono::Utc>>)> {
        Ok((
            self.records.len(),
            self.used_bytes,
            self.state.last_successful_upload,
        ))
    }
    pub fn enqueue(&mut self, payload: impl Into<QueuedPayload>) -> Result<bool> {
        let mut payload = payload.into();
        payload.set_identity(DeliveryIdentity {
            stream_id: self.state.stream_id.clone(),
            sequence: self.state.next_sequence,
        });
        let bytes = serde_json::to_vec(&payload)?;
        let (records, used, _) = self.status()?;
        if bytes.len() > 500 * 1024
            || records >= self.max_records
            || used.saturating_add(bytes.len() as u64) > self.max_bytes
        {
            self.dropped_samples += 1;
            return Ok(false); // Explicit reject-newest policy: never overwrite unacknowledged records.
        }
        let path = self
            .directory
            .join(format!("{:020}.json", self.state.next_sequence));
        atomic_write(&path, &bytes)?;
        self.records
            .insert(self.state.next_sequence, (path, bytes.len() as u64));
        self.used_bytes += bytes.len() as u64;
        self.state.next_sequence = self
            .state
            .next_sequence
            .checked_add(1)
            .context("sequence exhausted")?;
        atomic_write(
            &self.directory.join("state.json"),
            &serde_json::to_vec(&self.state)?,
        )?;
        Ok(true)
    }
    pub fn peek(&self) -> Result<Option<(PathBuf, QueuedPayload)>> {
        self.records
            .first_key_value()
            .map(|(_, (path, _))| Ok((path.clone(), serde_json::from_slice(&fs::read(path)?)?)))
            .transpose()
    }
    pub fn acknowledge(&mut self, path: &Path) -> Result<()> {
        let sequence = path
            .file_stem()
            .and_then(|name| name.to_str())
            .and_then(|name| name.parse::<u64>().ok())
            .context("invalid spool record")?;
        if self
            .records
            .first_key_value()
            .is_none_or(|(first, (record, _))| *first != sequence || record != path)
        {
            bail!("only the oldest queue record can be acknowledged");
        }
        fs::remove_file(path)?;
        fs::File::open(&self.directory)?.sync_all()?;
        if let Some((_, bytes)) = self.records.remove(&sequence) {
            self.used_bytes = self.used_bytes.saturating_sub(bytes);
        }
        self.state.last_successful_upload = Some(chrono::Utc::now());
        atomic_write(
            &self.directory.join("state.json"),
            &serde_json::to_vec(&self.state)?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn payload() -> MetricsPayload {
        MetricsPayload {
            agent_id: "test".into(),
            timestamp: chrono::Utc::now(),
            metrics: [("cpu_usage".into(), 1.0)].into(),
            processes: vec![],
            mounts: vec![],
            network_interfaces: vec![],
            services: vec![],
            containers: vec![],
            delivery: None,
        }
    }
    #[test]
    fn survives_restart_and_rejects_newest_at_capacity() {
        let path =
            std::env::temp_dir().join(format!("watchtower-spool-test-{}", std::process::id()));
        let mut queue = DiskQueue::open(path.clone(), 100_000, 1).unwrap();
        assert!(DiskQueue::open(path.clone(), 100_000, 1).is_err());
        assert!(queue.enqueue(payload()).unwrap());
        assert!(!queue.enqueue(payload()).unwrap());
        let identity = queue.peek().unwrap().unwrap().1.identity().clone();
        drop(queue);
        let mut queue = DiskQueue::open(path.clone(), 100_000, 1).unwrap();
        let (record, replay) = queue.peek().unwrap().unwrap();
        assert_eq!(replay.identity().stream_id, identity.stream_id);
        queue.acknowledge(&record).unwrap();
        assert!(queue.enqueue(payload()).unwrap());
        assert_eq!(queue.peek().unwrap().unwrap().1.identity().sequence, 2);
        drop(queue);
        fs::remove_dir_all(path).unwrap();
    }
    #[test]
    fn enforces_byte_limit_and_fifo_acknowledgements() {
        let path =
            std::env::temp_dir().join(format!("watchtower-spool-bounds-{}", std::process::id()));
        let mut queue = DiskQueue::open(path.clone(), 1, 10).unwrap();
        assert!(!queue.enqueue(payload()).unwrap());
        assert_eq!(queue.status().unwrap().0, 0);
        drop(queue);
        let mut queue = DiskQueue::open(path.clone(), 100_000, 10).unwrap();
        assert!(queue.enqueue(payload()).unwrap());
        assert!(queue
            .enqueue(crate::collector::LogsPayload {
                delivery: None,
                agent_id: "test".into(),
                logs: vec![crate::collector::logs::LogEntryInput {
                    timestamp: chrono::Utc::now(),
                    level: crate::collector::logs::LogLevel::INFO,
                    source: "test".into(),
                    message: "hello".into()
                }]
            })
            .unwrap());
        let newest = queue.records.last_key_value().unwrap().1 .0.clone();
        assert!(queue.acknowledge(&newest).is_err());
        let (oldest, sample) = queue.peek().unwrap().unwrap();
        assert_eq!(sample.identity().sequence, 1);
        queue.acknowledge(&oldest).unwrap();
        assert!(matches!(
            queue.peek().unwrap().unwrap().1,
            QueuedPayload::Logs(_)
        ));
        drop(queue);
        // Simulate a crash between committing sequence 2 and checkpointing it.
        let state_path = path.join("state.json");
        let mut state: State = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
        state.next_sequence = 2;
        atomic_write(&state_path, &serde_json::to_vec(&state).unwrap()).unwrap();
        let mut queue = DiskQueue::open(path.clone(), 100_000, 10).unwrap();
        assert!(queue.enqueue(payload()).unwrap());
        assert_eq!(*queue.records.last_key_value().unwrap().0, 3);
        drop(queue);
        fs::remove_dir_all(path).unwrap();
    }
}
