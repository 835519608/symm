use crate::adapters::db::repository;
use crate::adapters::db::resolve;
use crate::adapters::status;
use crate::domain::error::SymmError;
use crate::domain::model::LinkRecord;
use inquire::{MultiSelect, Select};

pub fn format_pick_label(index: u32, record: &LinkRecord) -> String {
    let view = status::to_view(record.clone());
    format!("#{}  {}  [{}]", index, record.display_name(), view.status)
}

pub fn parse_pick_index(label: &str) -> Option<u32> {
    let rest = label.strip_prefix('#')?;
    rest.split_whitespace().next()?.parse().ok()
}

pub fn pick_one(conn: &rusqlite::Connection) -> Result<String, SymmError> {
    let records = repository::list_links(conn)?;
    if records.is_empty() {
        return Err(SymmError::NotFound {
            selector: "(空库)".to_string(),
        });
    }

    let options = records
        .iter()
        .enumerate()
        .map(|(i, record)| format_pick_label(i as u32 + 1, record))
        .collect::<Vec<_>>();
    let selected = Select::new("选择记录（可输入筛选）", options)
        .with_help_message("↑↓ 移动，Enter 确认；# 后为 ls 序号")
        .prompt()
        .map_err(|e| SymmError::InvalidArgument {
            message: format!("已取消：{e}"),
        })?;

    let index = parse_pick_index(&selected).ok_or_else(|| SymmError::InvalidArgument {
        message: "无法解析所选记录".to_string(),
    })?;
    Ok(index.to_string())
}

pub fn pick_many(conn: &rusqlite::Connection) -> Result<Vec<LinkRecord>, SymmError> {
    let records = repository::list_links(conn)?;
    if records.is_empty() {
        return Err(SymmError::NotFound {
            selector: "(空库)".to_string(),
        });
    }

    let options = records
        .iter()
        .enumerate()
        .map(|(i, record)| format_pick_label(i as u32 + 1, record))
        .collect::<Vec<_>>();
    let selected = MultiSelect::new("选择要删除的记录（空格切换，Enter 确认）", options)
        .with_help_message("↑↓ 移动，空格选中/取消，Enter 确认")
        .prompt()
        .map_err(|e| SymmError::InvalidArgument {
            message: format!("已取消：{e}"),
        })?;

    if selected.is_empty() {
        return Err(SymmError::InvalidArgument {
            message: "未选择任何记录".to_string(),
        });
    }

    let mut picked = Vec::with_capacity(selected.len());
    let mut seen = std::collections::HashSet::new();
    for label in selected {
        let index = parse_pick_index(&label).ok_or_else(|| SymmError::InvalidArgument {
            message: format!("无法解析所选记录：{label}"),
        })?;
        if !seen.insert(index) {
            continue;
        }
        picked.push(resolve::record_at_index(conn, index)?);
    }
    Ok(picked)
}
