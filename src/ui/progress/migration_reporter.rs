use crate::adapters::lock::LockProbeProgress;
use crate::adapters::migrate::MigrationEvent;
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
                self.write_line(&format!("正在扫描：{source} → {target}"))
            }
            MigrationEvent::FastMove { source, target } => {
                self.write_line(&format!("正在同盘移动：{source} → {target}"))
            }
            MigrationEvent::Copying {
                copied_bytes,
                files_copied,
                current_item,
            } => self.write_copy_progress(copied_bytes, files_copied, current_item.as_deref()),
            MigrationEvent::RemovingSource { source } => {
                self.write_line(&format!("正在删除源：{source}"))
            }
            MigrationEvent::CreatingLink { link, target } => {
                self.write_line(&format!("正在创建软链：{link} → {target}"))
            }
            MigrationEvent::PersistingDb { link } => {
                self.write_line(&format!("正在保存记录：{link}"))
            }
            MigrationEvent::Done { link } => self.write_line(&format!("完成：{link}")),
        }
    }

    pub fn handle_lock_probe_event(&mut self, event: LockProbeProgress) {
        match event {
            LockProbeProgress::Scanning {
                scanned_files,
                current,
            } => {
                let _ = self.write_line(&format!(
                    "正在扫描占用：已检查 {scanned_files} 个文件（当前 {}）",
                    current.display()
                ));
            }
            LockProbeProgress::Querying {
                batch,
                total_batches,
            } => {
                let _ = self.write_line(&format!("正在查询占用进程：{batch}/{total_batches}"));
            }
        }
    }

    fn write_copy_progress(
        &mut self,
        copied_bytes: u64,
        files_copied: u64,
        current_item: Option<&str>,
    ) -> Result<(), SymmError> {
        if !self.should_emit_progress(copied_bytes, files_copied) {
            return Ok(());
        }
        let mut message = format!(
            "正在复制 {}，已处理 {} 个文件",
            format_bytes(copied_bytes),
            files_copied
        );
        if let Some(item) = current_item.filter(|name| !name.is_empty()) {
            message.push_str("，当前：");
            message.push_str(item);
        }
        self.write_line(&message)
    }

    fn should_emit_progress(&mut self, copied_bytes: u64, files_copied: u64) -> bool {
        if !self.is_terminal {
            let changed_bucket = self
                .last_copy_snapshot
                .is_none_or(|(last_copied, last_files)| {
                    copied_bytes.saturating_sub(last_copied) >= 64 * 1024 * 1024
                        || files_copied.saturating_sub(last_files) >= 100
                });
            if changed_bucket {
                self.last_copy_snapshot = Some((copied_bytes, files_copied));
                return true;
            }
            return false;
        }
        let now = Instant::now();
        let should_emit = self
            .last_copy_report_at
            .is_none_or(|last| now.duration_since(last) >= Duration::from_millis(250));
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
