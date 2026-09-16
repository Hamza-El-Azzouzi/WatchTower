use anyhow::Result;
use chrono::{DateTime, Utc};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Log level for log entries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum LogLevel {
    DEBUG,
    INFO,
    WARN,
    ERROR,
    FATAL,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::DEBUG => write!(f, "DEBUG"),
            LogLevel::INFO => write!(f, "INFO"),
            LogLevel::WARN => write!(f, "WARN"),
            LogLevel::ERROR => write!(f, "ERROR"),
            LogLevel::FATAL => write!(f, "FATAL"),
        }
    }
}

/// A single log entry (without id)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntryInput {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub source: String,
    pub message: String,
}

/// Logs payload to send to server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<crate::delivery::DeliveryIdentity>,
    pub agent_id: String,
    pub logs: Vec<LogEntryInput>,
}

/// File state tracker
struct FileState {
    path: PathBuf,
    position: u64,
    reader: BufReader<File>,
}

impl FileState {
    fn new(path: PathBuf) -> Result<Self> {
        let file = File::open(&path)?;
        let mut reader = BufReader::new(file);

        // Seek to end of file (we only want new entries)
        let position = reader.seek(SeekFrom::End(0))?;

        Ok(Self {
            path,
            position,
            reader,
        })
    }

    fn read_new_lines(&mut self) -> Result<Vec<String>> {
        let mut lines = Vec::new();
        let mut line = String::new();

        while self.reader.read_line(&mut line)? > 0 {
            if !line.trim().is_empty() {
                lines.push(line.trim_end().to_string());
            }
            line.clear();
        }

        // Update position
        self.position = self.reader.stream_position()?;

        Ok(lines)
    }

    fn reopen(&mut self) -> Result<()> {
        // File might have been rotated, reopen it
        let file = File::open(&self.path)?;
        self.reader = BufReader::new(file);
        self.position = 0;
        Ok(())
    }
}

/// Log file collector and parser
pub struct LogCollector {
    agent_id: String,
    file_states: Arc<Mutex<HashMap<PathBuf, FileState>>>,
    buffer: Arc<Mutex<Vec<LogEntryInput>>>,
    batch_size: usize,
    batch_interval: Duration,
    last_flush: Instant,
    _watcher: RecommendedWatcher,
    event_rx: Receiver<Result<Event, notify::Error>>,
    log_patterns: Vec<LogPattern>,
}

/// Pattern for parsing log lines
struct LogPattern {
    regex: Regex,
    level_group: usize,
    message_group: usize,
    timestamp_group: Option<usize>,
}

impl LogPattern {
    fn common_patterns() -> Vec<LogPattern> {
        vec![
            // Pattern 1: [LEVEL] message
            LogPattern {
                regex: Regex::new(r"^\[(\w+)\]\s+(.+)$")
                    .expect("invalid regex pattern 1"),
                level_group: 1,
                message_group: 2,
                timestamp_group: None,
            },
            // Pattern 2: YYYY-MM-DD HH:MM:SS LEVEL message
            LogPattern {
                regex: Regex::new(r"^(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2})\s+(\w+)\s+(.+)$")
                    .expect("invalid regex pattern 2"),
                level_group: 2,
                message_group: 3,
                timestamp_group: Some(1),
            },
            // Pattern 3: LEVEL: message
            LogPattern {
                regex: Regex::new(r"^(\w+):\s+(.+)$")
                    .expect("invalid regex pattern 3"),
                level_group: 1,
                message_group: 2,
                timestamp_group: None,
            },
            // Pattern 4: timestamp [LEVEL] message (ISO format)
            LogPattern {
                regex: Regex::new(
                    r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?)\s+\[(\w+)\]\s+(.+)$",
                )
                .expect("invalid regex pattern 4"),
                level_group: 2,
                message_group: 3,
                timestamp_group: Some(1),
            },
        ]
    }
}

impl LogCollector {
    pub fn new(
        agent_id: String,
        log_paths: Vec<String>,
        batch_size: usize,
        batch_interval_seconds: u64,
    ) -> Result<Self> {
        info!("Initializing log collector for {} files", log_paths.len());

        let (tx, rx) = channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

        let file_states = Arc::new(Mutex::new(HashMap::new()));
        let buffer = Arc::new(Mutex::new(Vec::new()));

        // Initialize file states and start watching
        for path_str in &log_paths {
            let path = PathBuf::from(path_str);

            if !path.exists() {
                warn!("Log file does not exist: {}", path.display());
                continue;
            }

            match FileState::new(path.clone()) {
                Ok(state) => {
                    file_states
                        .lock()
                        .expect("file_states lock poisoned")
                        .insert(path.clone(), state);

                    // Watch the file for changes
                    if let Err(e) = watcher.watch(&path, RecursiveMode::NonRecursive) {
                        error!("Failed to watch file {}: {}", path.display(), e);
                    } else {
                        info!("Watching log file: {}", path.display());
                    }
                }
                Err(e) => {
                    error!("Failed to open log file {}: {}", path.display(), e);
                }
            }
        }

        Ok(Self {
            agent_id,
            file_states,
            buffer,
            batch_size,
            batch_interval: Duration::from_secs(batch_interval_seconds.max(1)),
            last_flush: Instant::now(),
            _watcher: watcher,
            event_rx: rx,
            log_patterns: LogPattern::common_patterns(),
        })
    }

    /// Parse a log line into a LogEntryInput
    fn parse_log_line(&self, line: &str, source: &str) -> LogEntryInput {
        // Try each pattern
        for pattern in &self.log_patterns {
            if let Some(captures) = pattern.regex.captures(line) {
                let level_str = captures.get(pattern.level_group).map(|m| m.as_str());
                let message = captures.get(pattern.message_group).map(|m| m.as_str());

                if let (Some(level_str), Some(message)) = (level_str, message) {
                    let level = match level_str.to_uppercase().as_str() {
                        "DEBUG" | "DBG" => LogLevel::DEBUG,
                        "INFO" | "INF" => LogLevel::INFO,
                        "WARN" | "WARNING" | "WRN" => LogLevel::WARN,
                        "ERROR" | "ERR" => LogLevel::ERROR,
                        "FATAL" | "FTL" | "CRITICAL" | "CRIT" => LogLevel::FATAL,
                        _ => LogLevel::INFO,
                    };

                    let timestamp = if let Some(ts_group) = pattern.timestamp_group {
                        captures
                            .get(ts_group)
                            .and_then(|m| {
                                // Try parsing ISO format first
                                DateTime::parse_from_rfc3339(m.as_str())
                                    .ok()
                                    .map(|dt| dt.with_timezone(&Utc))
                                    .or_else(|| {
                                        // Try other formats
                                        chrono::NaiveDateTime::parse_from_str(
                                            m.as_str(),
                                            "%Y-%m-%d %H:%M:%S",
                                        )
                                        .ok()
                                        .map(|ndt| {
                                            DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc)
                                        })
                                    })
                            })
                            .unwrap_or_else(Utc::now)
                    } else {
                        Utc::now()
                    };

                    return LogEntryInput {
                        timestamp,
                        level,
                        source: source.to_string(),
                        message: message.to_string(),
                    };
                }
            }
        }

        // Fallback: treat as INFO level with full line as message
        LogEntryInput {
            timestamp: Utc::now(),
            level: LogLevel::INFO,
            source: source.to_string(),
            message: line.to_string(),
        }
    }

    /// Process file system events and read new log lines
    pub fn process_events(&mut self) -> Result<()> {
        // Process all pending events
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                Ok(ev) => {
                    if matches!(ev.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        for path in &ev.paths {
                            self.read_file_updates(path)?;
                        }
                    }
                }
                Err(e) => {
                    error!("File watcher error: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Read updates from a specific file
    fn read_file_updates(&mut self, path: &Path) -> Result<()> {
        let mut states = self.file_states.lock().expect("file_states lock poisoned");

        if let Some(state) = states.get_mut(path) {
            match state.read_new_lines() {
                Ok(lines) => {
                    let source = path.to_string_lossy().to_string();
                    let mut buffer = self.buffer.lock().expect("buffer lock poisoned");

                    for line in lines {
                        let log_entry = self.parse_log_line(&line, &source);
                        debug!("Parsed log: {:?}", log_entry);
                        buffer.push(log_entry);
                    }
                }
                Err(e) => {
                    warn!(
                        "Error reading file {}: {}, attempting to reopen",
                        path.display(),
                        e
                    );
                    // Try to reopen the file (might have been rotated)
                    if let Err(e) = state.reopen() {
                        error!("Failed to reopen file {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if a non-empty buffer reached its size or time threshold.
    pub fn should_flush(&self) -> bool {
        let buffer = self.buffer.lock().expect("buffer lock poisoned");
        !buffer.is_empty()
            && (buffer.len() >= self.batch_size || self.last_flush.elapsed() >= self.batch_interval)
    }

    /// Build a payload without deleting buffered entries. They are acknowledged only
    /// after the server accepts the batch so transient failures do not lose logs.
    pub fn create_payload(&self) -> Option<LogsPayload> {
        let logs = self
            .buffer
            .lock()
            .expect("buffer lock poisoned")
            .iter()
            .take(self.batch_size)
            .scan(0usize, |bytes, log| {
                let size = serde_json::to_vec(log).ok()?.len() + 1;
                if bytes.saturating_add(size) > 480 * 1024 {
                    return None;
                }
                *bytes += size;
                Some(log)
            })
            .cloned()
            .collect::<Vec<_>>();

        if logs.is_empty() {
            None
        } else {
            Some(LogsPayload {
                delivery: None,
                agent_id: self.agent_id.clone(),
                logs,
            })
        }
    }

    pub fn mark_sent(&mut self, count: usize) {
        let mut buffer = self.buffer.lock().expect("buffer lock poisoned");
        let acknowledged = count.min(buffer.len());
        buffer.drain(..acknowledged);
        self.last_flush = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to parse log lines for testing (mirrors LogCollector::parse_log_line)
    fn parse_test_log_line(line: &str, source: &str, patterns: &[LogPattern]) -> LogEntryInput {
        for pattern in patterns {
            if let Some(captures) = pattern.regex.captures(line) {
                let level_str = captures.get(pattern.level_group).map(|m| m.as_str());
                let message = captures
                    .get(pattern.message_group)
                    .map(|m| m.as_str())
                    .unwrap_or(line);

                let level = match level_str.unwrap_or("INFO").to_uppercase().as_str() {
                    "DEBUG" | "DBG" | "TRACE" => LogLevel::DEBUG,
                    "INFO" | "INF" => LogLevel::INFO,
                    "WARN" | "WRN" | "WARNING" => LogLevel::WARN,
                    "ERROR" | "ERR" => LogLevel::ERROR,
                    "FATAL" | "FTL" | "CRITICAL" | "CRIT" => LogLevel::FATAL,
                    _ => LogLevel::INFO,
                };

                return LogEntryInput {
                    timestamp: Utc::now(),
                    level,
                    source: source.to_string(),
                    message: message.to_string(),
                };
            }
        }

        // Fallback: treat as INFO level with full line as message
        LogEntryInput {
            timestamp: Utc::now(),
            level: LogLevel::INFO,
            source: source.to_string(),
            message: line.to_string(),
        }
    }

    #[test]
    fn test_log_parsing() {
        let patterns = LogPattern::common_patterns();

        // Test pattern 1: [LEVEL] message
        let entry = parse_test_log_line(
            "[ERROR] Database connection failed",
            "/var/log/app.log",
            &patterns,
        );
        assert_eq!(entry.level, LogLevel::ERROR);
        assert_eq!(entry.message, "Database connection failed");

        // Test pattern 3: LEVEL: message
        let entry = parse_test_log_line("WARN: Low disk space", "/var/log/system.log", &patterns);
        assert_eq!(entry.level, LogLevel::WARN);
        assert_eq!(entry.message, "Low disk space");

        // Test fallback (no pattern matches)
        let entry = parse_test_log_line("Some random log line", "/var/log/app.log", &patterns);
        assert_eq!(entry.level, LogLevel::INFO);
        assert_eq!(entry.message, "Some random log line");
    }
}
