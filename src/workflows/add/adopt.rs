//! `add` 冲突与接管：通过 [`migration_service`] 的 staging / `move_path_with_retry` /
//! `migrate_path` 完成文件迁移，失败时由本模块 `rollback` 恢复（与 `rm` 共用同一套 staging 约定）。
use crate::adapters::fs::migration_service::{
    self as migration, MigrationEvent, move_path_with_retry, staging_path,
};
use crate::adapters::fs::path_ops;
use crate::domain::error::SymmError;
use crate::ui::interaction::choice;
use std::fs;
use std::path::{Path, PathBuf};

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

pub struct AddPreparation {
    adopted_link_to_target: bool,
    staged_target: Option<PathBuf>,
    staged_link: Option<PathBuf>,
}

impl AddPreparation {
    pub fn prepare<F>(link: &Path, target: &Path, reporter: &mut F) -> Result<Self, SymmError>
    where
        F: FnMut(MigrationEvent) -> Result<(), SymmError>,
    {
        let mut plan = AddPreparation {
            adopted_link_to_target: false,
            staged_target: None,
            staged_link: None,
        };
        let link_meta = fs::symlink_metadata(link).ok();
        let link_exists = link_meta.is_some();
        let target_exists = target.exists();

        match (link_exists, target_exists) {
            (false, false) | (false, true) => Ok(plan),
            (true, false) => {
                if link_meta
                    .as_ref()
                    .is_some_and(|m| m.file_type().is_symlink())
                {
                    return Err(SymmError::TargetNotFound {
                        path: target.to_string_lossy().to_string(),
                    });
                }
                adopt_link_to_target(link, target, reporter)?;
                plan.adopted_link_to_target = true;
                Ok(plan)
            }
            (true, true) => {
                let link_is_symlink = link_meta
                    .as_ref()
                    .is_some_and(|m| m.file_type().is_symlink());
                if link_is_symlink {
                    plan.prepare_symlink_exist(link, target)?;
                } else {
                    plan.prepare_both_exist(link, target, reporter)?;
                }
                Ok(plan)
            }
        }
    }

    fn prepare_symlink_exist(&mut self, link: &Path, target: &Path) -> Result<(), SymmError> {
        if symlink_points_to_target(link, target)? {
            return Ok(());
        }
        match select_symlink_conflict_choice()? {
            SymlinkConflictChoice::Retarget => {
                let link_staging = staging_path(link, ".__symm_staging__");
                move_path_with_retry(link, &link_staging, "link")?;
                self.staged_link = Some(link_staging);
                Ok(())
            }
            SymlinkConflictChoice::Cancel => Err(SymmError::InvalidArgument {
                message: "用户取消：未执行 add".to_string(),
            }),
        }
    }

    fn prepare_both_exist<F>(
        &mut self,
        link: &Path,
        target: &Path,
        reporter: &mut F,
    ) -> Result<(), SymmError>
    where
        F: FnMut(MigrationEvent) -> Result<(), SymmError>,
    {
        match select_conflict_choice()? {
            ConflictChoice::KeepLink => {
                let target_staging = staging_path(target, ".__symm_staging__");
                move_path_with_retry(target, &target_staging, "target")?;
                self.staged_target = Some(target_staging);
                if let Err(e) = adopt_link_to_target(link, target, reporter) {
                    self.rollback(link, target)?;
                    return Err(e);
                }
                self.adopted_link_to_target = true;
                Ok(())
            }
            ConflictChoice::KeepTarget => {
                let link_staging = staging_path(link, ".__symm_staging__");
                move_path_with_retry(link, &link_staging, "link")?;
                self.staged_link = Some(link_staging);
                Ok(())
            }
            ConflictChoice::Cancel => Err(SymmError::InvalidArgument {
                message: "用户取消：未执行 add".to_string(),
            }),
        }
    }

    pub fn commit(&self) -> Result<(), SymmError> {
        if let Some(path) = &self.staged_target {
            path_ops::remove_path_any(path)?;
        }
        if let Some(path) = &self.staged_link {
            path_ops::remove_path_any(path)?;
        }
        Ok(())
    }

    pub fn rollback(&self, link: &Path, target: &Path) -> Result<(), SymmError> {
        if self.adopted_link_to_target && target.exists() {
            if link.exists() {
                path_ops::remove_path_any(link)?;
            }
            migration::move_path_without_progress(target, link).map_err(|e| {
                SymmError::IoError {
                    message: format!("回滚失败：无法恢复 link：{e}"),
                }
            })?;
        }
        if let Some(path) = &self.staged_target
            && path.exists()
        {
            move_path_with_retry(path, target, "target")?;
        }
        if let Some(path) = &self.staged_link
            && path.exists()
        {
            if link.exists() {
                path_ops::remove_path_any(link)?;
            }
            move_path_with_retry(path, link, "link")?;
        }
        Ok(())
    }
}

pub fn resolve_add_conflict<F>(
    link: &Path,
    target: &Path,
    reporter: &mut F,
) -> Result<AddPreparation, SymmError>
where
    F: FnMut(MigrationEvent) -> Result<(), SymmError>,
{
    AddPreparation::prepare(link, target, reporter)
}

pub fn adopt_link_to_target<F>(
    link: &Path,
    target: &Path,
    reporter: &mut F,
) -> Result<(), SymmError>
where
    F: FnMut(MigrationEvent) -> Result<(), SymmError>,
{
    if target.exists() {
        return Err(SymmError::InvalidArgument {
            message: "接管失败：目标路径已存在".to_string(),
        });
    }
    ensure_target_parent_dir(target)?;
    let meta = fs::symlink_metadata(link).map_err(|e| SymmError::IoError {
        message: format!("接管失败：无法读取 link 元数据：{e}"),
    })?;
    if meta.file_type().is_symlink() {
        return Err(SymmError::InvalidArgument {
            message: "接管失败：link 已经是软链接".to_string(),
        });
    }
    let staging = staging_path(link, ".__symm_staging__");
    move_path_with_retry(link, &staging, "link")?;
    if let Err(e) = migration::migrate_path(&staging, target, reporter) {
        let _ = fs::rename(&staging, link);
        return Err(SymmError::IoError {
            message: format!("接管失败：无法移动到 target（已回滚）：{e}"),
        });
    }
    Ok(())
}

fn ensure_target_parent_dir(target: &Path) -> Result<(), SymmError> {
    let parent = target.parent().ok_or_else(|| SymmError::InvalidArgument {
        message: format!("无法解析 target 父目录：{}", target.display()),
    })?;
    if parent.exists() {
        return Ok(());
    }
    fs::create_dir_all(parent).map_err(|e| SymmError::IoError {
        message: format!("接管失败：无法创建 target 父目录 {}：{e}", parent.display()),
    })
}

fn select_conflict_choice() -> Result<ConflictChoice, SymmError> {
    choice::choose_with_env(
        "SYMM_ADD_CONFLICT_CHOICE",
        parse_conflict_choice,
        "检测到 target 与 link 都已存在，请选择处理方式：",
        "↑↓ 移动  Enter 确认  Esc 取消",
        vec![
            (
                "保留 link（放弃 target）".to_string(),
                ConflictChoice::KeepLink,
            ),
            (
                "保留 target（放弃 link）".to_string(),
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
                "环境变量 SYMM_ADD_CONFLICT_CHOICE 值无效：{raw}（可选：link/target/cancel）"
            ),
        }),
    }
}

fn select_symlink_conflict_choice() -> Result<SymlinkConflictChoice, SymmError> {
    choice::choose_with_env(
        "SYMM_ADD_SYMLINK_CONFLICT_CHOICE",
        parse_symlink_conflict_choice,
        "检测到 link 已是软链接但当前指向与 target 不一致，请选择处理方式：",
        "↑↓ 移动  Enter 确认  Esc 取消",
        vec![
            (
                "改为指向新的 target".to_string(),
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
                "环境变量 SYMM_ADD_SYMLINK_CONFLICT_CHOICE 值无效：{raw}（可选：retarget/cancel）"
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
