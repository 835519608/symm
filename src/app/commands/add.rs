use crate::domain::error::SymmError;
use crate::domain::model::{LinkKind, LinkStatus};
use crate::infra::db::repository;
use crate::infra::fs::link_ops;
use crate::infra::fs::migration::{self, MigrationEvent};
use crate::infra::paths::runtime_paths;
use crate::infra::processes::locker;
use crate::interface::interaction::choice;
use crate::interface::progress::migration_reporter::MigrationProgressReporter;
use crate::usecases::add::adopt;
use crate::usecases::add::ports::PathMigrator;
use inquire::Text;
use std::fs;
use std::io::Write;
use std::path::Path;

struct FsPathMigrator;

impl PathMigrator for FsPathMigrator {
    fn migrate_path<F>(&self, src: &Path, dst: &Path, reporter: &mut F) -> Result<(), SymmError>
    where
        F: FnMut(MigrationEvent) -> Result<(), SymmError>,
    {
        migration::migrate_path(src, dst, reporter)
    }

    fn move_path_without_progress(&self, src: &Path, dst: &Path) -> Result<(), SymmError> {
        migration::move_path_without_progress(src, dst)
    }
}

pub fn run<W: Write>(
    conn: &rusqlite::Connection,
    link: &Path,
    target: &Path,
    writer: &mut W,
) -> Result<(), SymmError> {
    let migrator = FsPathMigrator;
    let link_norm = runtime_paths::normalize_link(link);
    let existing = repository::get_by_link_path(conn, &link_norm)?;
    let mut reporter = MigrationProgressReporter::new(writer);
    ensure_link_not_locked(Path::new(&link_norm), &mut reporter)?;
    let prep =
        adopt::resolve_add_conflict(&migrator, Path::new(&link_norm), target, &mut |event| {
            reporter.handle_migration_event(event)
        })?;

    let target_norm = runtime_paths::normalize_target(target)?;
    let link_exists_after_prep = fs::symlink_metadata(Path::new(&link_norm)).is_ok();
    let link_kind = if link_exists_after_prep {
        existing
            .as_ref()
            .map(|r| r.link_kind)
            .unwrap_or(LinkKind::Symlink)
    } else {
        reporter.handle_migration_event(MigrationEvent::CreatingLink {
            link: link_norm.clone(),
            target: target_norm.clone(),
        })?;
        match link_ops::create_link(Path::new(&target_norm), Path::new(&link_norm)) {
            Ok(kind) => kind,
            Err(e) => {
                let _ = prep.rollback(&migrator, Path::new(&link_norm), Path::new(&target_norm));
                return Err(e);
            }
        }
    };

    let default_name = existing.as_ref().map(|r| r.name.as_str()).unwrap_or("");
    let name = resolve_add_name(default_name)?;
    reporter.handle_migration_event(MigrationEvent::PersistingDb {
        link: link_norm.clone(),
    })?;
    if let Err(e) = repository::insert_link(conn, &name, &link_norm, &target_norm, link_kind) {
        let _ = link_ops::remove_link(Path::new(&link_norm));
        let _ = prep.rollback(&migrator, Path::new(&link_norm), Path::new(&target_norm));
        return Err(e);
    }
    prep.commit()?;
    reporter.handle_migration_event(MigrationEvent::Done {
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

pub fn status_to_model(arg: crate::interface::cli::StatusArg) -> LinkStatus {
    match arg {
        crate::interface::cli::StatusArg::Ok => LinkStatus::Ok,
        crate::interface::cli::StatusArg::Broken => LinkStatus::Broken,
        crate::interface::cli::StatusArg::Missing => LinkStatus::Missing,
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

fn ensure_link_not_locked<W: Write>(
    link: &Path,
    reporter: &mut MigrationProgressReporter<'_, W>,
) -> Result<(), SymmError> {
    reporter.write_line(&format!("正在检查 link 占用：{}", link.display()))?;
    let procs = locker::list_locking_processes_with_progress(link, |event| {
        reporter.handle_lock_probe_event(event)
    })?;
    if procs.is_empty() {
        return Ok(());
    }
    reporter.write_line("检测到占用进程，等待用户选择“解除占用/取消”")?;
    let action = select_lock_resolution_action(&procs)?;
    if action == LockResolutionAction::Cancel {
        return Err(SymmError::InvalidArgument {
            message: format!(
                "link 路径当前被占用，已取消解除占用，未执行 add：{}",
                link.display()
            ),
        });
    }
    reporter.write_line("正在结束全部占用进程")?;
    let pids = procs.iter().map(|proc| proc.pid).collect::<Vec<_>>();
    locker::kill_processes(&pids)?;
    reporter.write_line("正在重新确认占用状态")?;
    let remaining = locker::list_locking_processes_with_progress(link, |_event| {})?;
    if remaining.is_empty() {
        return Ok(());
    }
    Err(SymmError::IoError {
        message: format!(
            "link 路径仍被占用，未执行 add：{}（剩余 {} 个进程，示例：{}）",
            link.display(),
            remaining.len(),
            remaining[0]
        ),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LockResolutionAction {
    UnlockAll,
    Cancel,
}

fn select_lock_resolution_action(
    procs: &[locker::ProcInfo],
) -> Result<LockResolutionAction, SymmError> {
    let occupied = procs
        .iter()
        .map(|proc| format!("  - {}", proc))
        .collect::<Vec<_>>()
        .join("\n");
    let prompt = format!("检测到 link 被以下进程占用：\n{occupied}\n请选择后续操作：");
    choice::choose_with_env(
        "SYMM_ADD_LOCK_CHOICE",
        parse_lock_resolution_action,
        &prompt,
        "↑↓ 移动  Enter 确认  Esc 取消",
        vec![
            (
                format!("解除占用并继续（结束 {} 个进程）", procs.len()),
                LockResolutionAction::UnlockAll,
            ),
            ("取消".to_string(), LockResolutionAction::Cancel),
        ],
    )
}

fn parse_lock_resolution_action(raw: &str) -> Result<LockResolutionAction, SymmError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "unlock" | "unlock_all" | "kill" | "continue" => Ok(LockResolutionAction::UnlockAll),
        "cancel" | "abort" => Ok(LockResolutionAction::Cancel),
        _ => Err(SymmError::InvalidArgument {
            message: format!("环境变量 SYMM_ADD_LOCK_CHOICE 值无效：{raw}（可选：unlock/cancel）"),
        }),
    }
}
