//! 提权子进程将占用列表写入文件，父进程读取解析。

use super::ProcInfo;
use crate::domain::error::SymmError;
use std::fs;
use std::io::Write;
use std::path::Path;

const FORMAT_MARKER: &str = "symm-lock-snapshot-v1";

pub fn write_snapshot(path: &Path, procs: &[ProcInfo]) -> Result<(), SymmError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| SymmError::IoError {
            message: format!("无法创建占用快照目录：{e}"),
        })?;
    }
    let mut file = fs::File::create(path).map_err(|e| SymmError::IoError {
        message: format!("无法写入占用快照：{e}"),
    })?;
    writeln!(file, "{FORMAT_MARKER}").map_err(io_err)?;
    for proc in procs {
        let display = proc.display.replace(['\t', '\n'], " ");
        writeln!(file, "{}\t{display}", proc.pid).map_err(io_err)?;
    }
    Ok(())
}

pub fn read_snapshot(path: &Path) -> Result<Vec<ProcInfo>, SymmError> {
    let content = fs::read_to_string(path).map_err(|e| SymmError::IoError {
        message: format!("无法读取占用快照：{e}"),
    })?;
    parse_snapshot(&content)
}

pub fn parse_snapshot(content: &str) -> Result<Vec<ProcInfo>, SymmError> {
    let mut lines = content.lines();
    let header = lines.next().unwrap_or_default().trim();
    if header != FORMAT_MARKER {
        return Err(SymmError::IoError {
            message: "占用快照格式无效".to_string(),
        });
    }

    let mut out = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((pid_raw, display)) = line.split_once('\t') else {
            continue;
        };
        let Ok(pid) = pid_raw.trim().parse::<u32>() else {
            continue;
        };
        out.push(ProcInfo {
            pid,
            display: display.to_string(),
        });
    }
    Ok(out)
}

fn io_err(e: std::io::Error) -> SymmError {
    SymmError::IoError {
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_snapshot, write_snapshot};
    use crate::adapters::lock::ProcInfo;
    use tempfile::NamedTempFile;

    #[test]
    fn snapshot_roundtrip() {
        let file = NamedTempFile::new().expect("temp");
        let procs = vec![
            ProcInfo {
                pid: 42,
                display: "PID 42  notepad.exe".to_string(),
            },
            ProcInfo {
                pid: 99,
                display: "PID 99".to_string(),
            },
        ];
        write_snapshot(file.path(), &procs).expect("write");
        let parsed =
            parse_snapshot(&std::fs::read_to_string(file.path()).expect("read")).expect("parse");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].pid, 42);
        assert_eq!(parsed[1].pid, 99);
    }
}
