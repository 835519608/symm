//! `rm`：删库后删除 link，或将 target 迁回 link 路径。
use crate::adapters::db::repository;
use crate::adapters::fs::link_remover;
use crate::adapters::fs::migration_service as migration;
use crate::domain::error::SymmError;
use crate::ui::interaction::choice;
use crate::ui::progress::migration_reporter::MigrationProgressReporter;
use crate::workflows::perf;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

pub fn run<W: Write>(
    conn: &rusqlite::Connection,
    selector: &str,
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    let record = repository::get_by_selector(conn, selector)?;
    let link = Path::new(&record.link_path);
    let target = Path::new(&record.target_path);
    let action = select_rm_action()?;
    if action == RmAction::RestoreTargetToLink {
        restore_target_to_link(writer, link, target)?;
    }
    repository::delete_by_selector(conn, selector)?;
    if action == RmAction::DeleteLinkOnly {
        link_remover::remove_link(link)?;
    }
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
    link_remover::remove_link(link)?;
    let mut reporter = MigrationProgressReporter::new(writer);
    migration::migrate_path(target, link, &mut |event| {
        reporter.handle_migration_event(event)
    })
    .map_err(|e| SymmError::IoError {
        message: format!("恢复 target 到 link 失败：{e}"),
    })
}
