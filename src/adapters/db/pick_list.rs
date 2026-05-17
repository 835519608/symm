//! 与 `ls` 顺序一致的索引列表，供 workflow 交互选择（含状态标签）。

use crate::adapters::db::repository;
use crate::adapters::status;
use crate::domain::error::SymmError;
use crate::domain::model::LinkRecord;

#[derive(Debug, Clone)]
pub struct PickEntry {
    pub index: u32,
    pub record: LinkRecord,
}

pub fn list_entries(conn: &rusqlite::Connection) -> Result<Vec<PickEntry>, SymmError> {
    repository::list_links(conn)?
        .into_iter()
        .enumerate()
        .map(|(i, record)| {
            Ok(PickEntry {
                index: i as u32 + 1,
                record,
            })
        })
        .collect()
}

pub fn format_label(entry: &PickEntry) -> String {
    let view = status::to_view(entry.record.clone());
    format!(
        "#{}  {}  [{}]",
        entry.index,
        entry.record.display_name(),
        view.status.label_zh()
    )
}

pub fn entry_for_label<'a>(entries: &'a [PickEntry], label: &str) -> Option<&'a PickEntry> {
    let index = parse_label_index(label)?;
    entries.iter().find(|e| e.index == index)
}

pub fn parse_label_index(label: &str) -> Option<u32> {
    let rest = label.strip_prefix('#')?;
    rest.split_whitespace().next()?.parse().ok()
}

pub fn records_for_labels(
    entries: &[PickEntry],
    labels: &[String],
) -> Result<Vec<LinkRecord>, SymmError> {
    let mut picked = Vec::with_capacity(labels.len());
    let mut seen = std::collections::HashSet::new();
    for label in labels {
        let entry = entry_for_label(entries, label).ok_or_else(|| SymmError::InvalidArgument {
            message: format!("无法识别所选记录：{label}"),
        })?;
        if seen.insert(entry.index) {
            picked.push(entry.record.clone());
        }
    }
    Ok(picked)
}
