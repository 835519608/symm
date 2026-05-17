//! `add` 无参或缺参时：可选从库中选模板，再交互填写 link / target。

use crate::adapters::db::repository;
use crate::adapters::db::resolve;
use crate::domain::error::SymmError;
use crate::domain::model::LinkRecord;
use crate::ui::interaction::pick_record::{format_pick_label, parse_pick_index};
use inquire::Select;
use inquire::Text;
use std::path::{Path, PathBuf};

const MANUAL_OPTION: &str = "(手动输入新路径)";

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
        link = Some(prompt_path("link 路径（软链接位置）", default)?);
    }
    if target.is_none() {
        let default = template.as_ref().map(|r| r.target_path.as_str());
        target = Some(prompt_path("target 路径（实体数据位置）", default)?);
    }

    Ok((
        link.expect("link resolved"),
        target.expect("target resolved"),
    ))
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

fn pick_optional_template(conn: &rusqlite::Connection) -> Result<Option<LinkRecord>, SymmError> {
    let records = repository::list_links(conn)?;
    if records.is_empty() {
        return Ok(None);
    }

    let mut options = vec![MANUAL_OPTION.to_string()];
    options.extend(
        records
            .iter()
            .enumerate()
            .map(|(i, record)| format_pick_label(i as u32 + 1, record)),
    );

    let selected = Select::new("选择模板记录（可选）", options)
        .with_help_message("↑↓ 移动 Enter 确认；选已有记录可预填 link / target")
        .prompt()
        .map_err(|e| SymmError::InvalidArgument {
            message: format!("已取消：{e}"),
        })?;

    if selected.starts_with("(手动") {
        return Ok(None);
    }

    let index = parse_pick_index(&selected).ok_or_else(|| SymmError::InvalidArgument {
        message: "无法解析所选记录".to_string(),
    })?;
    Ok(Some(resolve::record_at_index(conn, index)?))
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
