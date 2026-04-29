use crate::adapters::fs::migration_service::MigrationEvent;
use crate::adapters::processes::lock_probe;
use crate::domain::error::SymmError;
use std::io::{IsTerminal, Write};
use std::time::{Duration, Instant};

pub struct MigrationProgressReporter<'a, W: Write> {
    writer: &'a mut W,
    is_terminal: bool,
    last_copy_report_at: Option<Instant>,
    last_copy_snapshot: Option<(u64, u64)>,
}

impl<'a, W: Write> MigrationProgressReporter<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        Self {
            writer,
            is_terminal: std::io::stdout().is_terminal(),
            last_copy_report_at: None,
            last_copy_snapshot: None,
        }
    }

    pub fn write_line(&mut self, message: &str) -> Result<(), SymmError> {
        writeln!(self.writer, "{message}").map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })
    }

    pub fn handle_migration_event(&mut self, event: MigrationEvent) -> Result<(), SymmError> {
        match event {
            MigrationEvent::Scanning { source, target } => {
                self.write_line(&format!("正在扫描迁移内容：{source} -> {target}"))
            }
            MigrationEvent::FastMove { source, target } => {
                self.write_line(&format!("正在快速移动（同盘）：{source} -> {target}"))
            }
            MigrationEvent::Copying {
                copied_bytes,
                total_bytes,
                current_item,
            } => self.write_copy_progress(copied_bytes, total_bytes, current_item.as_deref()),
            MigrationEvent::RemovingSource { source } => {
                self.write_line(&format!("正在删除源路径：{source}"))
            }
            MigrationEvent::CreatingLink { link, target } => {
                self.write_line(&format!("正在创建链接：{link} -> {target}"))
            }
            MigrationEvent::PersistingDb { link } => {
                self.write_line(&format!("正在写入数据库：{link}"))
            }
            MigrationEvent::Done { link } => self.write_line(&format!("迁移完成：{link}")),
        }
    }

    pub fn handle_lock_probe_event(&mut self, event: lock_probe::LockProbeProgress) {
        match event {
            lock_probe::LockProbeProgress::Scanning {
                scanned_files,
                current,
            } => {
                let _ = self.write_line(&format!(
                    "正在扫描占用检测文件：已扫描 {scanned_files} 个（示例：{}）",
                    current.display()
                ));
            }
            lock_probe::LockProbeProgress::Querying {
                batch,
                total_batches,
            } => {
                let _ =
                    self.write_line(&format!("正在查询占用进程：第 {batch}/{total_batches} 批"));
            }
        }
    }

    fn write_copy_progress(
        &mut self,
        copied_bytes: u64,
        total_bytes: u64,
        current_item: Option<&str>,
    ) -> Result<(), SymmError> {
        if !self.should_emit_progress(copied_bytes, total_bytes) {
            return Ok(());
        }
        let mut message = format!(
            "正在复制：{} / {}",
            format_bytes(copied_bytes),
            format_bytes(total_bytes)
        );
        if let Some(item) = current_item.filter(|name| !name.is_empty()) {
            message.push_str("  当前：");
            message.push_str(item);
        }
        self.write_line(&message)
    }

    fn should_emit_progress(&mut self, copied_bytes: u64, total_bytes: u64) -> bool {
        if !self.is_terminal {
            let changed_bucket = self
                .last_copy_snapshot
                .is_none_or(|(_, last_total)| total_bytes != last_total)
                || self.last_copy_snapshot.is_none_or(|(last_copied, _)| {
                    copied_bytes.saturating_sub(last_copied) >= 64 * 1024 * 1024
                });
            if changed_bucket || copied_bytes == total_bytes {
                self.last_copy_snapshot = Some((copied_bytes, total_bytes));
                return true;
            }
            return false;
        }
        let now = Instant::now();
        let should_emit = self
            .last_copy_report_at
            .is_none_or(|last| now.duration_since(last) >= Duration::from_millis(250))
            || copied_bytes == total_bytes;
        if should_emit {
            self.last_copy_report_at = Some(now);
        }
        should_emit
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let bytes_f = bytes as f64;
    if bytes_f >= GB {
        format!("{:.1} GB", bytes_f / GB)
    } else if bytes_f >= MB {
        format!("{:.1} MB", bytes_f / MB)
    } else if bytes_f >= KB {
        format!("{:.1} KB", bytes_f / KB)
    } else {
        format!("{bytes} B")
    }
}
