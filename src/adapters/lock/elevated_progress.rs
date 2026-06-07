//! 提权子进程进度：子进程写入 JSONL，父进程轮询并转发到主终端。

use crate::adapters::platform::process::LockProbeProgress;
use crate::domain::error::SymmError;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::Duration;

const PROGRESS_MARKER: &str = "symm-lock-progress-v1";
const FLUSH_EVERY_EVENTS: usize = 32;

#[cfg(test)]
fn append_progress(path: &Path, event: &LockProbeProgress) -> Result<(), SymmError> {
    let mut appender = ProgressAppender::new(path)?;
    appender.append(event)?;
    appender.finish()
}

pub struct ProgressAppender {
    writer: BufWriter<File>,
    pending: usize,
}

impl ProgressAppender {
    pub fn new(path: &Path) -> Result<Self, SymmError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| SymmError::IoError {
                message: format!("无法创建进度目录：{e}"),
            })?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| SymmError::IoError {
                message: format!("无法写入占用检测进度：{e}"),
            })?;
        let is_empty = file.metadata().map(|m| m.len()).unwrap_or(0) == 0;
        let mut writer = BufWriter::new(file);
        if is_empty {
            writeln!(writer, "{PROGRESS_MARKER}").map_err(io_err)?;
            writer.flush().map_err(io_err)?;
        }
        Ok(Self { writer, pending: 0 })
    }

    pub fn append(&mut self, event: &LockProbeProgress) -> Result<(), SymmError> {
        let line = serde_json::to_string(event).map_err(|e| SymmError::IoError {
            message: format!("无法序列化占用检测进度：{e}"),
        })?;
        writeln!(self.writer, "{line}").map_err(io_err)?;
        self.pending += 1;
        if self.pending == 1 || self.pending >= FLUSH_EVERY_EVENTS {
            self.flush()?;
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<(), SymmError> {
        self.flush()
    }

    fn flush(&mut self) -> Result<(), SymmError> {
        self.pending = 0;
        self.writer.flush().map_err(io_err)
    }
}

pub fn spawn_progress_relay(
    path: PathBuf,
    tx: Sender<LockProbeProgress>,
    stop: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut offset = 0u64;
        let mut header_seen = false;
        let mut pending = String::new();
        while !stop.load(Ordering::Relaxed) {
            relay_once(&path, &mut offset, &mut pending, &mut header_seen, &tx);
            thread::sleep(Duration::from_millis(80));
        }
        relay_once(&path, &mut offset, &mut pending, &mut header_seen, &tx);
    })
}

fn relay_once(
    path: &Path,
    offset: &mut u64,
    pending: &mut String,
    header_seen: &mut bool,
    tx: &Sender<LockProbeProgress>,
) {
    let Ok(meta) = fs::metadata(path) else {
        return;
    };
    let len = meta.len();
    if len < *offset {
        *offset = 0;
        pending.clear();
        *header_seen = false;
    }
    if len <= *offset {
        return;
    }
    let Ok(mut file) = File::open(path) else {
        return;
    };
    if file.seek(SeekFrom::Start(*offset)).is_err() {
        return;
    }
    let mut text = String::new();
    if file.read_to_string(&mut text).is_err() {
        return;
    };
    *offset = len;
    pending.push_str(&text);
    let Some(consume_to) = pending.rfind('\n').map(|pos| pos + 1) else {
        return;
    };
    let tail = pending[consume_to..].to_string();
    let complete = &pending[..consume_to];
    for line in complete.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !*header_seen {
            if line == PROGRESS_MARKER {
                *header_seen = true;
            }
            continue;
        }
        if let Ok(event) = serde_json::from_str::<LockProbeProgress>(line) {
            let _ = tx.send(event);
        }
    }
    *pending = tail;
}

fn io_err(e: std::io::Error) -> SymmError {
    SymmError::IoError {
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn progress_roundtrip_line() {
        let file = NamedTempFile::new().expect("temp");
        let event = LockProbeProgress::Querying {
            batch: 2,
            total_batches: Some(5),
        };
        append_progress(file.path(), &event).expect("write");
        let content = fs::read_to_string(file.path()).expect("read");
        let line = content.lines().nth(1).expect("line");
        let parsed: LockProbeProgress = serde_json::from_str(line).expect("parse");
        assert!(matches!(
            parsed,
            LockProbeProgress::Querying {
                batch: 2,
                total_batches: Some(5)
            }
        ));
    }
}
