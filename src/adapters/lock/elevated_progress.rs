//! 提权子进程进度：子进程写入 JSONL，父进程轮询并转发到主终端。

use crate::adapters::platform::process::LockProbeProgress;
use crate::domain::error::SymmError;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::Duration;

const PROGRESS_MARKER: &str = "symm-lock-progress-v1";

pub fn append_progress(path: &Path, event: &LockProbeProgress) -> Result<(), SymmError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| SymmError::IoError {
            message: format!("无法创建进度目录：{e}"),
        })?;
    }
    let line = serde_json::to_string(event).map_err(|e| SymmError::IoError {
        message: format!("无法序列化占用检测进度：{e}"),
    })?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| SymmError::IoError {
            message: format!("无法写入占用检测进度：{e}"),
        })?;
    if file.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
        writeln!(file, "{PROGRESS_MARKER}").map_err(io_err)?;
    }
    writeln!(file, "{line}").map_err(io_err)?;
    file.flush().map_err(io_err)
}

pub fn spawn_progress_relay(
    path: PathBuf,
    tx: Sender<LockProbeProgress>,
    stop: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut offset = 0u64;
        let mut header_seen = false;
        while !stop.load(Ordering::Relaxed) {
            relay_once(&path, &mut offset, &mut header_seen, &tx);
            thread::sleep(Duration::from_millis(80));
        }
        relay_once(&path, &mut offset, &mut header_seen, &tx);
    })
}

fn relay_once(
    path: &Path,
    offset: &mut u64,
    header_seen: &mut bool,
    tx: &Sender<LockProbeProgress>,
) {
    let Ok(meta) = fs::metadata(path) else {
        return;
    };
    let len = meta.len();
    if len <= *offset {
        return;
    }
    let Ok(bytes) = fs::read(path) else {
        return;
    };
    let slice = &bytes[*offset as usize..];
    *offset = len;
    let text = String::from_utf8_lossy(slice);
    for line in text.lines() {
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
            total_batches: 5,
        };
        append_progress(file.path(), &event).expect("write");
        let content = fs::read_to_string(file.path()).expect("read");
        let line = content.lines().nth(1).expect("line");
        let parsed: LockProbeProgress = serde_json::from_str(line).expect("parse");
        assert!(matches!(
            parsed,
            LockProbeProgress::Querying {
                batch: 2,
                total_batches: 5
            }
        ));
    }
}
