//! 与 `ls` 顺序一致的交互候选列表。

use crate::adapters::db::link_store;
use crate::domain::error::SymmError;
use crate::domain::model::LinkRecord;
use std::ops::Deref;

const INTERACTIVE_ENTRY_LIMIT: u32 = 1000;

#[derive(Debug, Clone)]
pub struct PickEntry {
    pub index: u32,
    pub record: LinkRecord,
}

#[derive(Debug, Clone)]
pub struct PickEntries {
    items: Vec<PickEntry>,
    total: usize,
}

impl PickEntries {
    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn option_limit(&self) -> usize {
        INTERACTIVE_ENTRY_LIMIT as usize
    }

    pub fn is_truncated(&self) -> bool {
        self.total > self.items.len()
    }

    pub fn labels(&self) -> Vec<String> {
        self.items.iter().map(format_label).collect()
    }

    pub fn record_for_label(&self, label: &str) -> Option<&PickEntry> {
        entry_for_label(&self.items, label)
    }
}

impl Deref for PickEntries {
    type Target = [PickEntry];

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

pub fn list_entries(conn: &rusqlite::Connection) -> Result<PickEntries, SymmError> {
    let total = link_store::count(conn)?;
    if total > INTERACTIVE_ENTRY_LIMIT as usize {
        return Ok(PickEntries {
            items: Vec::new(),
            total,
        });
    }
    let items = link_store::list_paginated(conn, Some(INTERACTIVE_ENTRY_LIMIT), 0)?
        .into_iter()
        .enumerate()
        .map(|(i, record)| PickEntry {
            index: i as u32 + 1,
            record,
        })
        .collect();
    Ok(PickEntries { items, total })
}

pub fn format_label(entry: &PickEntry) -> String {
    format!("#{}  {}", entry.index, entry.record.display_name())
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
