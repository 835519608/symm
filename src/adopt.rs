use crate::error::SymmError;
use crate::migration::{MigrationEvent, migrate_path, move_path_without_progress};
use inquire::Select;
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
                let link_staging = staging_path(link);
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
                let target_staging = staging_path(target);
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
                let link_staging = staging_path(link);
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
            remove_path_any(path)?;
        }
        if let Some(path) = &self.staged_link {
            remove_path_any(path)?;
        }
        Ok(())
    }

    pub fn rollback(&self, link: &Path, target: &Path) -> Result<(), SymmError> {
        if self.adopted_link_to_target && target.exists() {
            // add 失败后，若 link 已被创建，先移除，再把 target 中的内容回滚到 link。
            if link.exists() {
                remove_path_any(link)?;
            }
            move_path_without_progress(target, link).map_err(|e| SymmError::IoError {
                message: format!("回滚失败：无法恢复 link：{e}"),
            })?;
        }

        if let Some(path) = &self.staged_target
            && path.exists()
        {
            fs::rename(path, target).map_err(|e| SymmError::IoError {
                message: format!("回滚失败：无法恢复 target：{e}"),
            })?;
        }

        if let Some(path) = &self.staged_link
            && path.exists()
        {
            if link.exists() {
                remove_path_any(link)?;
            }
            fs::rename(path, link).map_err(|e| SymmError::IoError {
                message: format!("回滚失败：无法恢复 link 备份：{e}"),
            })?;
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

    let meta = fs::symlink_metadata(link).map_err(|e| SymmError::IoError {
        message: format!("接管失败：无法读取 link 元数据：{e}"),
    })?;
    if meta.file_type().is_symlink() {
        return Err(SymmError::InvalidArgument {
            message: "接管失败：link 已经是软链接".to_string(),
        });
    }

    let staging = staging_path(link);
    move_path_with_retry(link, &staging, "link")?;

    if let Err(e) = migrate_path(&staging, target, reporter) {
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

fn select_conflict_choice() -> Result<ConflictChoice, SymmError> {
    if let Ok(raw) = std::env::var("SYMM_ADD_CONFLICT_CHOICE") {
        return parse_conflict_choice(&raw);
    }

    let options = vec![
        ("保留 link（放弃 target）", ConflictChoice::KeepLink),
        ("保留 target（放弃 link）", ConflictChoice::KeepTarget),
        ("取消", ConflictChoice::Cancel),
    ];
    let labels: Vec<&str> = options.iter().map(|(label, _)| *label).collect();
    let selected = Select::new("检测到 target 与 link 都已存在，请选择处理方式：", labels)
        .with_help_message("↑↓ 移动  Enter 确认  Esc 取消")
        .prompt()
        .map_err(|e| SymmError::InvalidArgument {
            message: format!("已取消：{e}"),
        })?;

    for (label, choice) in options {
        if label == selected {
            return Ok(choice);
        }
    }
    Err(SymmError::InvalidArgument {
        message: "无效选择".to_string(),
    })
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
    if let Ok(raw) = std::env::var("SYMM_ADD_SYMLINK_CONFLICT_CHOICE") {
        return parse_symlink_conflict_choice(&raw);
    }

    let options = vec![
        ("改为指向新的 target", SymlinkConflictChoice::Retarget),
        ("取消", SymlinkConflictChoice::Cancel),
    ];
    let labels: Vec<&str> = options.iter().map(|(label, _)| *label).collect();
    let selected = Select::new(
        "检测到 link 已是软链接但当前指向与 target 不一致，请选择处理方式：",
        labels,
    )
    .with_help_message("↑↓ 移动  Enter 确认  Esc 取消")
    .prompt()
    .map_err(|e| SymmError::InvalidArgument {
        message: format!("已取消：{e}"),
    })?;

    for (label, choice) in options {
        if label == selected {
            return Ok(choice);
        }
    }
    Err(SymmError::InvalidArgument {
        message: "无效选择".to_string(),
    })
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

fn move_path_with_retry(src: &Path, dst: &Path, role: &str) -> Result<(), SymmError> {
    fs::rename(src, dst).map_err(|e| {
        let mut message = format!("无法移动 {role}：{e}");
        if e.raw_os_error() == Some(5) {
            message.push_str(
                "。系统拒绝访问（os error 5），可能仍有占用未被识别，或当前进程权限不足（可尝试以管理员身份运行）",
            );
        }
        SymmError::IoError { message }
    })?;
    Ok(())
}

fn remove_path_any(path: &Path) -> Result<(), SymmError> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_dir() {
                fs::remove_dir_all(path).map_err(|e| SymmError::IoError {
                    message: e.to_string(),
                })?;
            } else {
                fs::remove_file(path).map_err(|e| SymmError::IoError {
                    message: e.to_string(),
                })?;
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(SymmError::IoError {
            message: e.to_string(),
        }),
    }
}
