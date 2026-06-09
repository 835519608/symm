//! 将 CLI 选择器（name / `ls` 序号）解析为 [`LinkRecord`]。

use crate::adapters::db::link_store;
use crate::domain::error::SymmError;
use crate::domain::model::LinkRecord;
use std::collections::{BTreeMap, HashSet};

/// 解析 `rm` / `show` 参数：纯数字 = `ls` 全表序号（1-based）；否则按 **name 精确匹配**。
pub fn record_from_token(
    conn: &rusqlite::Connection,
    token: &str,
) -> Result<LinkRecord, SymmError> {
    let token = token.trim();
    if token.is_empty() {
        return Err(SymmError::InvalidArgument {
            message: "选择器不能为空".to_string(),
        });
    }
    if let Some(index) = parse_list_index(token)? {
        return record_at_index(conn, index);
    }
    link_store::find_by_name(conn, token)
}

pub fn records_from_tokens(
    conn: &rusqlite::Connection,
    tokens: &[String],
) -> Result<Vec<LinkRecord>, SymmError> {
    let mut resolved = vec![None; tokens.len()];
    let mut numeric_positions: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    let mut name_positions: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    for (pos, raw) in tokens.iter().enumerate() {
        let token = raw.trim();
        if token.is_empty() {
            return Err(SymmError::InvalidArgument {
                message: "选择器不能为空".to_string(),
            });
        }
        if let Some(index) = parse_list_index(token)? {
            numeric_positions.entry(index).or_default().push(pos);
        } else {
            name_positions
                .entry(token.to_string())
                .or_default()
                .push(pos);
        }
    }

    if !numeric_positions.is_empty() {
        fill_numeric_records(conn, &numeric_positions, &mut resolved)?;
    }
    if !name_positions.is_empty() {
        fill_name_records(conn, &name_positions, &mut resolved)?;
    }

    let mut records = Vec::with_capacity(tokens.len());
    let mut seen_ids = HashSet::new();
    for (pos, record) in resolved.into_iter().enumerate() {
        let record = record.ok_or_else(|| SymmError::NotFound {
            selector: tokens[pos].trim().to_string(),
        })?;
        if seen_ids.insert(record.id) {
            records.push(record);
        }
    }
    Ok(records)
}

/// 与 `ls` 相同顺序（`id` 升序）下的 1-based 序号。
pub fn record_at_index(conn: &rusqlite::Connection, index: u32) -> Result<LinkRecord, SymmError> {
    if index == 0 {
        return Err(SymmError::InvalidArgument {
            message: "序号从 1 开始".to_string(),
        });
    }
    let selector = index.to_string();
    link_store::list_paginated(conn, Some(1), index - 1)?
        .into_iter()
        .next()
        .ok_or(SymmError::NotFound { selector })
}

/// 计算记录在 `ls` 全表中的序号（用于 `show` 展示）。
pub fn index_in_list(conn: &rusqlite::Connection, record: &LinkRecord) -> Result<u32, SymmError> {
    link_store::index_for_id(conn, record.id)?.ok_or_else(|| SymmError::NotFound {
        selector: record.link_path.clone(),
    })
}

fn parse_list_index(token: &str) -> Result<Option<u32>, SymmError> {
    if !token.chars().all(|c| c.is_ascii_digit()) {
        return Ok(None);
    }
    let index = token
        .parse::<u32>()
        .map_err(|_| SymmError::InvalidArgument {
            message: format!("序号无效：{token}"),
        })?;
    if index == 0 {
        return Err(SymmError::InvalidArgument {
            message: "序号从 1 开始".to_string(),
        });
    }
    Ok(Some(index))
}

fn fill_numeric_records(
    conn: &rusqlite::Connection,
    numeric_positions: &BTreeMap<u32, Vec<usize>>,
    resolved: &mut [Option<LinkRecord>],
) -> Result<(), SymmError> {
    let max_index = numeric_positions.keys().next_back().copied().unwrap_or(0);
    let mut row_index = 0u32;
    link_store::for_each(conn, |record| {
        row_index = row_index.saturating_add(1);
        if let Some(positions) = numeric_positions.get(&row_index) {
            for &pos in positions {
                resolved[pos] = Some(record.clone());
            }
        }
        Ok(row_index < max_index)
    })?;
    Ok(())
}

fn fill_name_records(
    conn: &rusqlite::Connection,
    name_positions: &BTreeMap<String, Vec<usize>>,
    resolved: &mut [Option<LinkRecord>],
) -> Result<(), SymmError> {
    let names = name_positions.keys().cloned().collect::<Vec<_>>();
    for record in link_store::find_existing_by_names(conn, &names)? {
        if let Some(positions) = name_positions.get(&record.name) {
            for &pos in positions {
                resolved[pos] = Some(record.clone());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::db::{link_store, schema};
    use crate::domain::model::LinkKind;
    use rusqlite::Connection;

    #[test]
    fn mixed_missing_selectors_report_first_input_order_miss() {
        let conn = Connection::open_in_memory().expect("open memory db");
        schema::migrate(&conn).expect("migrate");
        link_store::upsert_link(
            &conn,
            "exists",
            "/tmp/link",
            "/tmp/target",
            LinkKind::Symlink,
        )
        .expect("insert");

        let selectors = vec!["999".to_string(), "aaa-missing".to_string()];
        let err = records_from_tokens(&conn, &selectors).expect_err("missing selector");

        assert!(matches!(err, SymmError::NotFound { selector } if selector == "999"));
    }
}
