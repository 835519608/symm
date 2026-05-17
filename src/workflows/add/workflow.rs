use crate::adapters::db::{LinkQuery, repository};
use crate::adapters::lock::{
    ProcInfo, empty_lock_list_notice, format_still_locked_message, kill_processes,
    list_locking_processes_with_progress, pre_scan_notices, wait_after_kill,
};
use crate::adapters::migrate::MigrationEvent;
use crate::adapters::paths::runtime_paths;
use crate::adapters::symlink;
use crate::domain::error::SymmError;
use crate::domain::model::LinkKind;
use crate::ui::interaction::choice;
use crate::ui::progress::migration_reporter::MigrationProgressReporter;
use crate::workflows::add::adopt;
use crate::workflows::perf;
use inquire::Text;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

pub fn run<W: Write>(
    conn: &rusqlite::Connection,
    link: &Path,
    target: &Path,
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    let link_norm = runtime_paths::normalize_link(link);
    let existing = repository::find_optional(conn, &LinkQuery::link_path_exact(&link_norm))?;
    let mut reporter = MigrationProgressReporter::new(writer);
    ensure_link_not_locked(Path::new(&link_norm), &mut reporter)?;
    let prep = adopt::resolve_add_conflict(Path::new(&link_norm), target, &mut |event| {
        reporter.handle_migration_event(event)
    })?;

    let target_norm = if prep.skip_target_exists_check {
        runtime_paths::normalize_target_known_exists(target)?
    } else {
        runtime_paths::normalize_target(target)?
    };
    let link_kind = if prep.link_exists_at_path {
        existing
            .as_ref()
            .map(|r| r.link_kind)
            .unwrap_or(LinkKind::Symlink)
    } else {
        reporter.handle_migration_event(MigrationEvent::CreatingLink {
            link: link_norm.clone(),
            target: target_norm.clone(),
        })?;
        symlink::create_link(Path::new(&target_norm), Path::new(&link_norm))?
    };

    let default_name = existing.as_ref().map(|r| r.name.as_str()).unwrap_or("");
    let name_input = resolve_add_name(default_name)?;
    reporter.handle_migration_event(MigrationEvent::PersistingDb {
        link: link_norm.clone(),
    })?;
    let name = repository::insert_link(conn, &name_input, &link_norm, &target_norm, link_kind)?;
    if name_input != name && !name_input.is_empty() {
        reporter.write_line(&format!(
            "name「{name_input}」已自动改为「{name}」（纯数字会与记录 ID 查询混淆）"
        ))?;
    }
    reporter.handle_migration_event(MigrationEvent::Done {
        link: link_norm.clone(),
    })?;
    let display_name = if name.is_empty() {
        "(空)"
    } else {
        name.as_str()
    };
    reporter.write_line(&format!("创建成功：{link_norm}（name: {display_name}）"))?;
    perf::log_perf(
        "add",
        started.elapsed(),
        &[
            ("link_path", link_norm),
            ("target_path", target_norm),
            ("name", name),
        ],
    );
    Ok(())
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
    for notice in pre_scan_notices() {
        reporter.write_line(notice)?;
    }
    let procs = list_locking_processes_with_progress(link, |event| {
        reporter.handle_lock_probe_event(event)
    })?;
    if procs.is_empty() {
        if let Some(notice) = empty_lock_list_notice() {
            reporter.write_line(notice)?;
        }
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
    kill_processes(&pids)?;
    reporter.write_line("正在等待占用进程释放句柄")?;
    let remaining = wait_after_kill(link)?;
    if remaining.is_empty() {
        return Ok(());
    }
    Err(SymmError::IoError {
        message: format_still_locked_message(link, &remaining),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LockResolutionAction {
    UnlockAll,
    Cancel,
}

fn select_lock_resolution_action(procs: &[ProcInfo]) -> Result<LockResolutionAction, SymmError> {
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
