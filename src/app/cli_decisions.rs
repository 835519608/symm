use crate::adapters::lock::ProcInfo;
use crate::domain::error::SymmError;
use crate::domain::model::LinkRecord;
use crate::ui::interaction::{choice, pick_record};
use crate::workflows::add::workflow::{
    AddConflictChoice, AddDecisionProvider, AddLockChoice, AddSymlinkConflictChoice,
};
use crate::workflows::pick_list;
use crate::workflows::rm::workflow::RemoveMode;
use inquire::Text;
use std::path::{Path, PathBuf};

const MANUAL_OPTION: &str = "(自己输入路径)";

pub fn resolve_add_paths(
    conn: &rusqlite::Connection,
    link: Option<&Path>,
    target: Option<&Path>,
) -> Result<(PathBuf, PathBuf), SymmError> {
    let mut link = link
        .map(Path::to_path_buf)
        .or_else(|| env_path("SYMM_ADD_LINK"));
    let mut target = target
        .map(Path::to_path_buf)
        .or_else(|| env_path("SYMM_ADD_TARGET"));

    if let (Some(link), Some(target)) = (&link, &target) {
        return Ok((link.clone(), target.clone()));
    }

    let template = pick_optional_template(conn)?;

    if link.is_none() {
        let default = template.as_ref().map(|r| r.link_path.as_str());
        link = Some(prompt_path("链接放在哪（软链路径）", default)?);
    }
    if target.is_none() {
        let default = template.as_ref().map(|r| r.target_path.as_str());
        target = Some(prompt_path("真实文件/目录在哪（目标路径）", default)?);
    }

    match (link, target) {
        (Some(link), Some(target)) => Ok((link, target)),
        _ => Err(SymmError::InvalidArgument {
            message: "链接路径和目标路径不能为空".to_string(),
        }),
    }
}

pub struct CliAddDecisions;

impl AddDecisionProvider for CliAddDecisions {
    fn name(&mut self, default_name: &str) -> Result<String, SymmError> {
        if let Ok(v) = std::env::var("SYMM_ADD_NAME") {
            return Ok(v.trim().to_string());
        }
        Text::new("名称（可选，回车沿用默认）:")
            .with_default(default_name)
            .prompt()
            .map(|s| s.trim().to_string())
            .map_err(|e| SymmError::InvalidArgument {
                message: format!("已取消：{e}"),
            })
    }

    fn lock_choice(&mut self, procs: &[ProcInfo]) -> Result<AddLockChoice, SymmError> {
        let occupied = procs
            .iter()
            .map(|proc| format!("  - {}", proc))
            .collect::<Vec<_>>()
            .join("\n");
        let prompt = format!("以下进程占用了链接位置：\n{occupied}\n请选择：");
        choice::choose_with_env(
            "SYMM_ADD_LOCK_CHOICE",
            parse_lock_choice,
            &prompt,
            "↑↓ 移动  Enter 确认  Esc 取消",
            vec![
                (
                    format!("结束占用并继续（{} 个进程）", procs.len()),
                    AddLockChoice::Unlock,
                ),
                ("取消".to_string(), AddLockChoice::Cancel),
            ],
        )
    }

    fn conflict_choice(&mut self) -> Result<AddConflictChoice, SymmError> {
        choice::choose_with_env(
            "SYMM_ADD_CONFLICT_CHOICE",
            parse_conflict_choice,
            "链接位置和目标位置都已存在，请选择：",
            "↑↓ 移动  Enter 确认  Esc 取消",
            vec![
                (
                    "留链接这边（不要目标那边）".to_string(),
                    AddConflictChoice::KeepLink,
                ),
                (
                    "留目标那边（不要链接这边）".to_string(),
                    AddConflictChoice::KeepTarget,
                ),
                ("取消".to_string(), AddConflictChoice::Cancel),
            ],
        )
    }

    fn symlink_conflict_choice(&mut self) -> Result<AddSymlinkConflictChoice, SymmError> {
        choice::choose_with_env(
            "SYMM_ADD_SYMLINK_CONFLICT_CHOICE",
            parse_symlink_conflict_choice,
            "该路径已是软链，但指向与目标不一致，请选择：",
            "↑↓ 移动  Enter 确认  Esc 取消",
            vec![
                (
                    "改成指向新目标".to_string(),
                    AddSymlinkConflictChoice::Retarget,
                ),
                ("取消".to_string(), AddSymlinkConflictChoice::Cancel),
            ],
        )
    }
}

pub fn select_rm_mode() -> Result<RemoveMode, SymmError> {
    choice::choose_with_env(
        "SYMM_RM_ACTION",
        parse_rm_mode,
        "是否把目标移回链接位置？",
        "↑↓ 移动  Enter 确认  Esc 取消",
        vec![
            ("只删软链和记录".to_string(), RemoveMode::DeleteLinkOnly),
            (
                "删软链，并把目标移回链接位置".to_string(),
                RemoveMode::RestoreTargetToLink,
            ),
        ],
    )
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

fn pick_optional_template(conn: &rusqlite::Connection) -> Result<Option<LinkRecord>, SymmError> {
    let entries = pick_list::list_entries(conn)?;
    if entries.is_empty() || entries.is_truncated() {
        return Ok(None);
    }

    let mut options = vec![MANUAL_OPTION.to_string()];
    options.extend(entries.iter().map(pick_list::format_label));

    let selected = pick_record::pick_one_option(&options)?;
    if selected.starts_with("(自己") {
        return Ok(None);
    }

    let entry = pick_list::entry_for_label(&entries, &selected).ok_or_else(|| {
        SymmError::InvalidArgument {
            message: "无法识别所选记录".to_string(),
        }
    })?;
    Ok(Some(entry.record.clone()))
}

fn prompt_path(label: &str, default: Option<&str>) -> Result<PathBuf, SymmError> {
    let mut prompt = Text::new(label);
    if let Some(default) = default.filter(|s| !s.is_empty()) {
        prompt = prompt.with_default(default);
    }
    let raw = prompt.prompt().map_err(|e| SymmError::InvalidArgument {
        message: format!("已取消：{e}"),
    })?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(SymmError::InvalidArgument {
            message: format!("{label} 不能为空"),
        });
    }
    Ok(PathBuf::from(trimmed))
}

fn parse_lock_choice(raw: &str) -> Result<AddLockChoice, SymmError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "unlock" | "unlock_all" | "kill" | "continue" => Ok(AddLockChoice::Unlock),
        "cancel" | "abort" => Ok(AddLockChoice::Cancel),
        _ => Err(SymmError::InvalidArgument {
            message: format!("环境变量 SYMM_ADD_LOCK_CHOICE 无效：{raw}（可选：unlock / cancel）"),
        }),
    }
}

fn parse_conflict_choice(raw: &str) -> Result<AddConflictChoice, SymmError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "link" | "keep_link" => Ok(AddConflictChoice::KeepLink),
        "target" | "keep_target" => Ok(AddConflictChoice::KeepTarget),
        "cancel" | "abort" => Ok(AddConflictChoice::Cancel),
        _ => Err(SymmError::InvalidArgument {
            message: format!(
                "环境变量 SYMM_ADD_CONFLICT_CHOICE 无效：{raw}（可选：link / target / cancel）"
            ),
        }),
    }
}

fn parse_symlink_conflict_choice(raw: &str) -> Result<AddSymlinkConflictChoice, SymmError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "retarget" | "target" | "replace" => Ok(AddSymlinkConflictChoice::Retarget),
        "cancel" | "abort" => Ok(AddSymlinkConflictChoice::Cancel),
        _ => Err(SymmError::InvalidArgument {
            message: format!(
                "环境变量 SYMM_ADD_SYMLINK_CONFLICT_CHOICE 无效：{raw}（可选：retarget / cancel）"
            ),
        }),
    }
}

fn parse_rm_mode(raw: &str) -> Result<RemoveMode, SymmError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "no" | "n" | "delete" | "delete_only" => Ok(RemoveMode::DeleteLinkOnly),
        "yes" | "y" | "restore" | "restore_target" => Ok(RemoveMode::RestoreTargetToLink),
        _ => Err(SymmError::InvalidArgument {
            message: format!(
                "环境变量 SYMM_RM_ACTION 无效：{raw}（可选：delete / restore 或 no / yes）"
            ),
        }),
    }
}
