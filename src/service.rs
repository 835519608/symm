use crate::adopt;
use crate::cli::{Commands, StatusArg};
use crate::db;
use crate::error::SymmError;
use crate::link_ops;
use crate::migration::MigrationEvent;
use crate::model::LinkStatus;
use crate::output;
use crate::paths;
use inquire::Text;
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::Path;
use std::time::{Duration, Instant};

pub fn execute<W: Write>(command: Commands, writer: &mut W) -> Result<(), SymmError> {
    let conn = db::open_db()?;
    match command {
        Commands::Add { link, target } => {
            let link_norm = paths::normalize_link(&link);
            let existing = db::get_by_link_path(&conn, &link_norm)?;
            let mut reporter = AddProgressReporter::new(writer);
            let prep = adopt::resolve_add_conflict(Path::new(&link_norm), &target, &mut |event| {
                reporter.handle(event)
            })?;

            let target_norm = paths::normalize_target(&target)?;
            let link_exists_after_prep = fs::symlink_metadata(Path::new(&link_norm)).is_ok();
            let link_kind = if link_exists_after_prep {
                existing
                    .as_ref()
                    .map(|r| r.link_kind)
                    .unwrap_or(crate::model::LinkKind::Symlink)
            } else {
                reporter.handle(MigrationEvent::CreatingLink {
                    link: link_norm.clone(),
                    target: target_norm.clone(),
                })?;
                match link_ops::create_link(Path::new(&target_norm), Path::new(&link_norm)) {
                    Ok(kind) => kind,
                    Err(e) => {
                        let _ = prep.rollback(Path::new(&link_norm), Path::new(&target_norm));
                        return Err(e);
                    }
                }
            };

            let default_name = existing.as_ref().map(|r| r.name.as_str()).unwrap_or("");
            let name = resolve_add_name(default_name)?;
            reporter.handle(MigrationEvent::PersistingDb {
                link: link_norm.clone(),
            })?;
            if let Err(e) = db::insert_link(&conn, &name, &link_norm, &target_norm, link_kind) {
                let _ = link_ops::remove_link(Path::new(&link_norm));
                let _ = prep.rollback(Path::new(&link_norm), Path::new(&target_norm));
                return Err(e);
            }
            prep.commit()?;
            reporter.handle(MigrationEvent::Done {
                link: link_norm.clone(),
            })?;
            let display_name = if name.is_empty() {
                "(空)"
            } else {
                name.as_str()
            };
            reporter.write_line(&format!("创建成功：{link_norm}（name: {display_name}）"))?;
            Ok(())
        }
        Commands::Rm { selector } => {
            let record = db::get_by_selector(&conn, &selector)?;
            link_ops::remove_link(Path::new(&record.link_path))?;
            db::delete_by_selector(&conn, &selector)?;
            writeln!(writer, "删除成功：{}", record.name).map_err(|e| SymmError::IoError {
                message: e.to_string(),
            })?;
            Ok(())
        }
        Commands::Ls { json, status } => {
            let wanted = status.map(status_to_model);
            if json {
                stream_ls_json(&conn, wanted, writer)
            } else {
                stream_ls_table(&conn, wanted, writer)
            }
        }
        Commands::Show { selector, json } => {
            let view = link_ops::as_view(db::get_by_selector(&conn, &selector)?);
            if json {
                let text = output::render_json(&view)?;
                writeln!(writer, "{text}").map_err(|e| SymmError::IoError {
                    message: e.to_string(),
                })?;
                Ok(())
            } else {
                writer
                    .write_all(output::render_show_table(&view).as_bytes())
                    .map_err(|e| SymmError::IoError {
                        message: e.to_string(),
                    })?;
                Ok(())
            }
        }
    }
}

fn stream_ls_table<W: Write>(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
    writer: &mut W,
) -> Result<(), SymmError> {
    let records = db::list_links(conn)?;
    output::write_list_header(writer)?;
    for record in records {
        let view = link_ops::as_view(record);
        if wanted.is_none_or(|s| view.status == s) {
            output::write_list_row(writer, &view)?;
        }
    }
    Ok(())
}

fn stream_ls_json<W: Write>(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
    writer: &mut W,
) -> Result<(), SymmError> {
    let records = db::list_links(conn)?;
    output::write_json_array_start(writer)?;
    let mut first = true;
    for record in records {
        let view = link_ops::as_view(record);
        if wanted.is_none_or(|s| view.status == s) {
            output::write_json_item(writer, &view, first)?;
            first = false;
        }
    }
    output::write_json_array_end(writer)
}

fn status_to_model(arg: StatusArg) -> LinkStatus {
    match arg {
        StatusArg::Ok => LinkStatus::Ok,
        StatusArg::Broken => LinkStatus::Broken,
        StatusArg::Missing => LinkStatus::Missing,
    }
}

fn resolve_add_name(default_name: &str) -> Result<String, SymmError> {
    if let Ok(v) = std::env::var("SYMM_ADD_NAME") {
        return Ok(v.trim().to_string());
    }

    Text::new("可选填写 name（回车保持默认值）:")
        .with_default(default_name)
        .prompt()
        .map(|s| s.trim().to_string())
        .map_err(|e| SymmError::InvalidArgument {
            message: format!("已取消：{e}"),
        })
}

struct AddProgressReporter<'a, W: Write> {
    writer: &'a mut W,
    is_terminal: bool,
    last_copy_report_at: Option<Instant>,
    last_copy_snapshot: Option<(u64, u64)>,
}

impl<'a, W: Write> AddProgressReporter<'a, W> {
    fn new(writer: &'a mut W) -> Self {
        Self {
            writer,
            is_terminal: std::io::stdout().is_terminal(),
            last_copy_report_at: None,
            last_copy_snapshot: None,
        }
    }

    fn handle(&mut self, event: MigrationEvent) -> Result<(), SymmError> {
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
                ..
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

    fn write_line(&mut self, message: &str) -> Result<(), SymmError> {
        writeln!(self.writer, "{message}").map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })
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
