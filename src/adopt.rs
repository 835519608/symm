use crate::error::SymmError;
use crate::processes;
use inquire::MultiSelect;
use std::fs;
use std::path::{Path, PathBuf};

pub fn adopt_link_to_target(link: &Path, target: &Path) -> Result<(), SymmError> {
    // 只在 link 存在实体、target 不存在时调用
    if target.exists() {
        return Err(SymmError::InvalidArgument {
            message: "接管失败：目标路径已存在".to_string(),
        });
    }

    let meta = fs::symlink_metadata(link).map_err(|e| SymmError::IoError {
        message: format!("接管失败：无法读取 link 元数据：{e}"),
    })?;
    if meta.file_type().is_symlink() {
        return Err(SymmError::InvalidArgument {
            message: "接管失败：link 已经是软链接".to_string(),
        });
    }

    let staging = staging_path(link);

    // 第一步：原子改名 link -> staging（失败则可能是占用/权限）
    if let Err(e) = fs::rename(link, &staging) {
        // 尝试列出占用进程并按 pnpm 风格多选 kill
        let procs = processes::list_locking_processes(link)?;
        if procs.is_empty() {
            return Err(SymmError::IoError {
                message: format!("无法移动 link（可能被占用）：{e}"),
            });
        }

        let options = procs.iter().map(|p| p.display.clone()).collect::<Vec<_>>();
        let selected = MultiSelect::new("检测到可能占用该路径的进程，请用空格选择要结束的进程，回车确认：", options)
            .with_help_message("↑↓ 移动  空格 选择/取消  Enter 确认  Esc 取消")
            .prompt()
            .map_err(|e| SymmError::InvalidArgument {
                message: format!("已取消：{e}"),
            })?;

        let mut pids = Vec::new();
        for s in selected {
            if let Some(pid) = extract_pid(&s) {
                pids.push(pid);
            }
        }
        if pids.is_empty() {
            return Err(SymmError::InvalidArgument {
                message: "未选择任何进程，已取消".to_string(),
            });
        }

        processes::kill_processes(&pids)?;
        fs::rename(link, &staging).map_err(|e| SymmError::IoError {
            message: format!("结束进程后仍无法移动 link：{e}"),
        })?;
    }

    // 第二步：移动 staging -> target（失败必须回滚 staging -> link）
    if let Err(e) = fs::rename(&staging, target) {
        let _ = fs::rename(&staging, link);
        return Err(SymmError::IoError {
            message: format!("接管失败：无法移动到 target（已回滚）：{e}"),
        });
    }

    Ok(())
}

fn staging_path(link: &Path) -> PathBuf {
    let mut p = link.to_path_buf();
    let file_name = link
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "link".to_string());
    p.set_file_name(format!("{file_name}.__symm_staging__"));
    p
}

fn extract_pid(line: &str) -> Option<u32> {
    // 形如 "PID 1234  xxx"
    let mut it = line.split_whitespace();
    let a = it.next()?;
    let b = it.next()?;
    if a.eq_ignore_ascii_case("pid") {
        b.parse::<u32>().ok()
    } else {
        None
    }
}

