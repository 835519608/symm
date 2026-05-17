//! `add` 无参或缺参时：可选从库中选模板，再交互填写链接 / 目标路径。

use crate::adapters::db::pick_list;
use crate::domain::error::SymmError;
use crate::domain::model::LinkRecord;
use crate::ui::interaction::pick_record;
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
    let entries = pick_list::list_entries(conn)?;
    if entries.is_empty() {
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
