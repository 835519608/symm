//! `add` 前链接路径占用检测与解除。

use crate::adapters::lock::{
    ProcInfo, empty_lock_list_notice, format_still_locked_message, kill_processes,
    list_locking_processes_with_progress, pre_scan_notices, wait_after_kill,
};
use crate::domain::error::SymmError;
use crate::ui::progress::migration_reporter::MigrationProgressReporter;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LockResolutionAction {
    UnlockAll,
    Cancel,
}

pub(crate) fn ensure_link_not_locked_with_choice<W: Write>(
    link: &Path,
    reporter: &mut MigrationProgressReporter<'_, W>,
    choose_action: &mut impl FnMut(&[ProcInfo]) -> Result<LockResolutionAction, SymmError>,
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
    let action = choose_action(&procs)?;
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
