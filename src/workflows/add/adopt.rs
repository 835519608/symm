//! `add` 冲突与接管：直接腾路径并 `migrate_path`，无 staging 旁路文件。
use crate::adapters::migrate::{self, MigrationEvent};
use crate::adapters::paths::remove;
use crate::adapters::symlink;
use crate::domain::error::SymmError;
use crate::ui::interaction::choice;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddPrepareOutcome {
    /// `link` 路径上是否已有条目（通常为软链），调用方无需再 `create_link`。
    pub link_exists_at_path: bool,
    /// 若 true，`add` 可对 `target` 调用 `normalize_target_known_exists`，跳过第二次 `exists()`。
    pub skip_target_exists_check: bool,
}

fn finish_outcome(
    link_exists_at_path: bool,
    target_existed_at_start: bool,
    target_removed_during_prepare: bool,
    target_created_during_prepare: bool,
) -> AddPrepareOutcome {
    let skip_target_exists_check =
        target_existed_at_start && !target_removed_during_prepare && !target_created_during_prepare;
    AddPrepareOutcome {
        link_exists_at_path,
        skip_target_exists_check,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConflictChoice {
    KeepLink,
    KeepTarget,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SymlinkConflictChoice {
    Retarget,
    Cancel,
}

pub fn resolve_add_conflict<F>(
    link: &Path,
    target: &Path,
    reporter: &mut F,
) -> Result<AddPrepareOutcome, SymmError>
where
    F: FnMut(MigrationEvent) -> Result<(), SymmError>,
{
    let link_meta = fs::symlink_metadata(link).ok();
    let link_exists = link_meta.is_some();
    let link_is_symlink = link_meta
        .as_ref()
        .is_some_and(|meta| meta.file_type().is_symlink());
    let target_existed_at_start = target.exists();

    match (link_exists, target_existed_at_start) {
        (false, false) => Ok(finish_outcome(false, false, false, false)),
        (false, true) => Ok(finish_outcome(false, true, false, false)),
        (true, false) => {
            if link_is_symlink {
                return Err(SymmError::TargetNotFound {
                    path: target.to_string_lossy().to_string(),
                });
            }
            adopt_link_to_target(link, target, reporter, true, true)?;
            Ok(finish_outcome(false, false, false, true))
        }
        (true, true) => {
            if link_is_symlink {
                prepare_symlink_exist(link, target, target_existed_at_start)
            } else {
                prepare_both_exist(link, target, reporter, target_existed_at_start)
            }
        }
    }
}

fn prepare_symlink_exist(
    link: &Path,
    target: &Path,
    target_existed_at_start: bool,
) -> Result<AddPrepareOutcome, SymmError> {
    if symlink_points_to_target(link, target)? {
        return Ok(finish_outcome(true, target_existed_at_start, false, false));
    }
    match select_symlink_conflict_choice()? {
        SymlinkConflictChoice::Retarget => {
            symlink::unlink(link)?;
            Ok(finish_outcome(false, target_existed_at_start, false, false))
        }
        SymlinkConflictChoice::Cancel => Err(SymmError::InvalidArgument {
            message: "已取消".to_string(),
        }),
    }
}

fn prepare_both_exist<F>(
    link: &Path,
    target: &Path,
    reporter: &mut F,
    target_existed_at_start: bool,
) -> Result<AddPrepareOutcome, SymmError>
where
    F: FnMut(MigrationEvent) -> Result<(), SymmError>,
{
    match select_conflict_choice()? {
        ConflictChoice::KeepLink => {
            remove::remove_any(target)?;
            adopt_link_to_target(link, target, reporter, true, true)?;
            Ok(finish_outcome(false, target_existed_at_start, true, true))
        }
        ConflictChoice::KeepTarget => {
            remove::remove_any(link)?;
            Ok(finish_outcome(false, target_existed_at_start, false, false))
        }
        ConflictChoice::Cancel => Err(SymmError::InvalidArgument {
            message: "已取消".to_string(),
        }),
    }
}

pub fn adopt_link_to_target<F>(
    link: &Path,
    target: &Path,
    reporter: &mut F,
    link_is_entity: bool,
    target_known_absent: bool,
) -> Result<(), SymmError>
where
    F: FnMut(MigrationEvent) -> Result<(), SymmError>,
{
    if !target_known_absent && target.exists() {
        return Err(SymmError::InvalidArgument {
            message: "接管失败：目标位置已有文件".to_string(),
        });
    }
    ensure_target_parent_dir(target)?;
    if !link_is_entity {
        let meta = fs::symlink_metadata(link).map_err(|e| SymmError::IoError {
            message: format!("接管失败：无法读取链接路径：{e}"),
        })?;
        if meta.file_type().is_symlink() {
            return Err(SymmError::InvalidArgument {
                message: "接管失败：该路径已是软链".to_string(),
            });
        }
    }
    migrate::migrate_path(link, target, reporter).map_err(|e| SymmError::IoError {
        message: format!("接管失败：无法把链接内容移到目标位置：{e}"),
    })
}

fn ensure_target_parent_dir(target: &Path) -> Result<(), SymmError> {
    let parent = target.parent().ok_or_else(|| SymmError::InvalidArgument {
        message: format!("无法解析目标路径的父目录：{}", target.display()),
    })?;
    if parent.exists() {
        return Ok(());
    }
    fs::create_dir_all(parent).map_err(|e| SymmError::IoError {
        message: format!("接管失败：无法创建目录 {}：{e}", parent.display()),
    })
}

fn select_conflict_choice() -> Result<ConflictChoice, SymmError> {
    choice::choose_with_env(
        "SYMM_ADD_CONFLICT_CHOICE",
        parse_conflict_choice,
        "链接位置和目标位置都已存在，请选择：",
        "↑↓ 移动  Enter 确认  Esc 取消",
        vec![
            (
                "留链接这边（不要目标那边）".to_string(),
                ConflictChoice::KeepLink,
            ),
            (
                "留目标那边（不要链接这边）".to_string(),
                ConflictChoice::KeepTarget,
            ),
            ("取消".to_string(), ConflictChoice::Cancel),
        ],
    )
}

fn parse_conflict_choice(raw: &str) -> Result<ConflictChoice, SymmError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "link" | "keep_link" => Ok(ConflictChoice::KeepLink),
        "target" | "keep_target" => Ok(ConflictChoice::KeepTarget),
        "cancel" | "abort" => Ok(ConflictChoice::Cancel),
        _ => Err(SymmError::InvalidArgument {
            message: format!(
                "环境变量 SYMM_ADD_CONFLICT_CHOICE 无效：{raw}（可选：link / target / cancel）"
            ),
        }),
    }
}

fn select_symlink_conflict_choice() -> Result<SymlinkConflictChoice, SymmError> {
    choice::choose_with_env(
        "SYMM_ADD_SYMLINK_CONFLICT_CHOICE",
        parse_symlink_conflict_choice,
        "该路径已是软链，但指向与目标不一致，请选择：",
        "↑↓ 移动  Enter 确认  Esc 取消",
        vec![
            (
                "改成指向新目标".to_string(),
                SymlinkConflictChoice::Retarget,
            ),
            ("取消".to_string(), SymlinkConflictChoice::Cancel),
        ],
    )
}

fn parse_symlink_conflict_choice(raw: &str) -> Result<SymlinkConflictChoice, SymmError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "retarget" | "target" | "replace" => Ok(SymlinkConflictChoice::Retarget),
        "cancel" | "abort" => Ok(SymlinkConflictChoice::Cancel),
        _ => Err(SymmError::InvalidArgument {
            message: format!(
                "环境变量 SYMM_ADD_SYMLINK_CONFLICT_CHOICE 无效：{raw}（可选：retarget / cancel）"
            ),
        }),
    }
}

fn symlink_points_to_target(link: &Path, target: &Path) -> Result<bool, SymmError> {
    let pointed = fs::read_link(link).map_err(|e| SymmError::IoError {
        message: format!("无法读取 link 指向：{e}"),
    })?;
    let resolved = if pointed.is_absolute() {
        pointed
    } else {
        let parent = link.parent().ok_or_else(|| SymmError::InvalidArgument {
            message: "无法解析 link 父目录".to_string(),
        })?;
        parent.join(pointed)
    };
    let resolved_canonical = fs::canonicalize(&resolved).map_err(|e| SymmError::IoError {
        message: format!("无法解析 link 指向路径：{e}"),
    })?;
    let target_canonical = fs::canonicalize(target).map_err(|e| SymmError::IoError {
        message: format!("无法解析 target 路径：{e}"),
    })?;
    Ok(resolved_canonical == target_canonical)
}
