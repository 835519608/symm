//! `add` 前链接路径占用检测与解除。

use crate::adapters::lock::{
    ProcInfo, empty_lock_list_notice, format_still_locked_message, kill_processes,
    list_locking_processes_with_progress, pre_scan_notices, wait_after_kill,
};
use crate::domain::error::SymmError;
use crate::ui::interaction::choice;
use crate::ui::progress::migration_reporter::MigrationProgressReporter;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LockResolutionAction {
    UnlockAll,
    Cancel,
}

pub fn ensure_link_not_locked<W: Write>(
    link: &Path,
    reporter: &mut MigrationProgressReporter<'_, W>,
) -> Result<(), SymmError> {
    reporter.write_line(&format!("正在检查链接是否被占用：{}", link.display()))?;
    if !link.exists() {
        reporter.write_line("链接路径尚不存在，跳过占用检测")?;
        return Ok(());
    }
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
    reporter.write_line("检测到占用，请选择是否结束占用进程")?;
    let action = select_lock_resolution_action(&procs)?;
    if action == LockResolutionAction::Cancel {
        return Err(SymmError::InvalidArgument {
            message: format!("链接位置仍被占用，已取消：{}", link.display()),
        });
    }
    reporter.write_line("正在结束占用进程…")?;
    let pids = procs.iter().map(|proc| proc.pid).collect::<Vec<_>>();
    kill_processes(&pids)?;
    reporter.write_line("等待程序释放文件…")?;
    let remaining = wait_after_kill(link)?;
    if remaining.is_empty() {
        return Ok(());
    }
    Err(SymmError::IoError {
        message: format_still_locked_message(link, &remaining),
    })
}

fn select_lock_resolution_action(procs: &[ProcInfo]) -> Result<LockResolutionAction, SymmError> {
    let occupied = procs
        .iter()
        .map(|proc| format!("  - {}", proc))
        .collect::<Vec<_>>()
        .join("\n");
    let prompt = format!("以下进程占用了链接位置：\n{occupied}\n请选择：");
    choice::choose_with_env(
        "SYMM_ADD_LOCK_CHOICE",
        parse_lock_resolution_action,
        &prompt,
        "↑↓ 移动  Enter 确认  Esc 取消",
        vec![
            (
                format!("结束占用并继续（{} 个进程）", procs.len()),
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
            message: format!("环境变量 SYMM_ADD_LOCK_CHOICE 无效：{raw}（可选：unlock / cancel）"),
        }),
    }
}
