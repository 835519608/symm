//! 将 CLI 选择器（name / `ls` 序号）解析为 [`LinkRecord`]。
use crate::adapters::db::link_query::LinkQuery;
use crate::adapters::db::repository;
use crate::domain::error::SymmError;
use crate::domain::model::LinkRecord;

/// 解析 `rm` / `show` 参数：纯数字 = `ls` 全表序号（1-based）；否则按 **name 精确匹配**。
pub fn resolve_cli_record(
    conn: &rusqlite::Connection,
    token: &str,
) -> Result<LinkRecord, SymmError> {
    let token = token.trim();
    if token.is_empty() {
        return Err(SymmError::InvalidArgument {
            message: "选择器不能为空".to_string(),
        });
    }
    if token.chars().all(|c| c.is_ascii_digit()) {
        let index = token
            .parse::<u32>()
            .map_err(|_| SymmError::InvalidArgument {
                message: format!("序号无效：{token}"),
            })?;
        return resolve_list_index(conn, index);
    }
    repository::find_one(conn, &LinkQuery::name_exact(token))
}

/// 与 `ls` 相同顺序（`id` 升序）下的 1-based 序号。
pub fn resolve_list_index(
    conn: &rusqlite::Connection,
    index: u32,
) -> Result<LinkRecord, SymmError> {
    if index == 0 {
        return Err(SymmError::InvalidArgument {
            message: "序号从 1 开始".to_string(),
        });
    }
    let selector = index.to_string();
    repository::list_links(conn)?
        .into_iter()
        .nth((index - 1) as usize)
        .ok_or(SymmError::NotFound { selector })
}

/// 计算记录在 `ls` 全表中的序号（用于 `show` 展示）。
pub fn list_index_for_record(
    conn: &rusqlite::Connection,
    record: &LinkRecord,
) -> Result<u32, SymmError> {
    for (i, row) in repository::list_links(conn)?.into_iter().enumerate() {
        if row.id == record.id {
            return Ok(i as u32 + 1);
        }
    }
    Err(SymmError::NotFound {
        selector: record.link_path.clone(),
    })
}
