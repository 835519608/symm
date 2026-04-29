use crate::adapters::db::repository;
use crate::adapters::fs::migration_service as migration;
use crate::adapters::fs::migration_service::{rollback_staged_path, stage_existing_path};
use crate::adapters::fs::path_ops;
use crate::domain::error::SymmError;
use crate::ui::interaction::choice;
use crate::ui::progress::migration_reporter::MigrationProgressReporter;
use crate::workflows::lifecycle::operation_tracker::OperationTracker;
use crate::workflows::perf;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub fn run<W: Write>(
    conn: &rusqlite::Connection,
    selector: &str,
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    let record = repository::get_by_selector(conn, selector)?;
    let payload = serde_json::json!({
        "selector": selector,
        "link_path": record.link_path.clone(),
        "target_path": record.target_path.clone(),
    })
    .to_string();
    let tracker = OperationTracker::begin(conn, "rm", &payload)?;
    let action = select_rm_action()?;
    tracker.pending(repository::OperationStep::Staging, "rm 预处理与暂存");
    let mut prep = RmPreparation::prepare(
        action,
        writer,
        Path::new(&record.link_path),
        Path::new(&record.target_path),
    )?;
    tracker.run_pending(
        repository::OperationStep::DbWrite,
        "删除 links 记录",
        "删除记录失败",
        || {
            repository::delete_by_selector(conn, selector).inspect_err(|_e| {
                let _ = prep.rollback();
            })
        },
    )?;
    prep.commit()?;
    tracker.done();
    let success_message = match action {
        RmAction::DeleteLinkOnly => format!("删除成功：{}", record.name),
        RmAction::RestoreTargetToLink => {
            format!("删除成功并已恢复 target 到 link：{}", record.name)
        }
    };
    writeln!(writer, "{success_message}").map_err(|e| SymmError::IoError {
        message: e.to_string(),
    })?;
    perf::log_perf(
        "rm",
        started.elapsed(),
        &[
            ("selector", selector.to_string()),
            ("action", format!("{action:?}")),
        ],
    );
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RmAction {
    DeleteLinkOnly,
    RestoreTargetToLink,
}

struct RmPreparation {
    action: RmAction,
    link: PathBuf,
    target: PathBuf,
    staged_link: Option<PathBuf>,
    target_moved_to_link: bool,
}

impl RmPreparation {
    fn prepare<W: Write>(
        action: RmAction,
        writer: &mut W,
        link: &Path,
        target: &Path,
    ) -> Result<Self, SymmError> {
        let staged_link = stage_existing_path(link, ".__symm_rm_staging__", "link")?;
        let mut prep = Self {
            action,
            link: link.to_path_buf(),
            target: target.to_path_buf(),
            staged_link,
            target_moved_to_link: false,
        };

        if action == RmAction::RestoreTargetToLink {
            if let Err(e) = restore_target_to_link(writer, link, target) {
                prep.rollback()?;
                return Err(e);
            }
            prep.target_moved_to_link = true;
        }
        Ok(prep)
    }

    fn commit(&self) -> Result<(), SymmError> {
        if let Some(path) = &self.staged_link {
            path_ops::remove_path_any(path)?;
        }
        Ok(())
    }

    fn rollback(&mut self) -> Result<(), SymmError> {
        if self.action == RmAction::RestoreTargetToLink && self.target_moved_to_link {
            if self.target.exists() {
                path_ops::remove_path_any(&self.target)?;
            }
            if self.link.exists() {
                migration::move_path_without_progress(&self.link, &self.target).map_err(|e| {
                    SymmError::IoError {
                        message: format!("回滚失败：无法恢复 target：{e}"),
                    }
                })?;
            }
        }
        rollback_staged_path(&self.link, self.staged_link.as_deref(), "link")
    }
}

fn select_rm_action() -> Result<RmAction, SymmError> {
    choice::choose_with_env(
        "SYMM_RM_ACTION",
        parse_rm_action,
        "是否将 target 恢复到原 link 位置？",
        "↑↓ 移动  Enter 确认  Esc 取消",
        vec![
            (
                "否：仅删除软链接并删除数据库记录".to_string(),
                RmAction::DeleteLinkOnly,
            ),
            (
                "是：删除软链接并将 target 恢复到 link 位置".to_string(),
                RmAction::RestoreTargetToLink,
            ),
        ],
    )
}

fn parse_rm_action(raw: &str) -> Result<RmAction, SymmError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "no" | "n" | "delete" | "delete_only" => Ok(RmAction::DeleteLinkOnly),
        "yes" | "y" | "restore" | "restore_target" => Ok(RmAction::RestoreTargetToLink),
        _ => Err(SymmError::InvalidArgument {
            message: format!(
                "环境变量 SYMM_RM_ACTION 值无效：{raw}（可选：no/yes 或 delete/restore）"
            ),
        }),
    }
}

fn restore_target_to_link<W: Write>(
    writer: &mut W,
    link: &Path,
    target: &Path,
) -> Result<(), SymmError> {
    let mut reporter = MigrationProgressReporter::new(writer);
    if let Err(e) = migration::migrate_path(target, link, &mut |event| {
        reporter.handle_migration_event(event)
    }) {
        return Err(SymmError::IoError {
            message: format!("恢复 target 到 link 失败：{e}"),
        });
    }
    Ok(())
}
