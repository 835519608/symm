//! 检测库内记录与磁盘实际状态是否一致。

use crate::adapters::db::repository;
use crate::adapters::fs::link_status;
use crate::domain::error::SymmError;
use crate::domain::model::{LinkStatus, LinkView};
use crate::ui::output;
use crate::workflows::perf;
use std::io::Write;
use std::time::Instant;

pub fn run<W: Write>(
    conn: &rusqlite::Connection,
    json: bool,
    prune: bool,
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    let records = repository::list_links(conn)?;
    let views: Vec<LinkView> = records.into_iter().map(link_status::as_view).collect();

    let issues: Vec<&LinkView> = views
        .iter()
        .filter(|v| v.status != LinkStatus::Ok)
        .collect();

    if json {
        let payload = serde_json::json!({
            "total": views.len(),
            "issues": issues,
        });
        let text = output::render_json(&payload)?;
        writeln!(writer, "{text}").map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })?;
    } else if issues.is_empty() {
        writeln!(writer, "共 {} 条记录，全部正常。", views.len()).map_err(io_err)?;
    } else {
        writeln!(
            writer,
            "共 {} 条记录，其中 {} 条异常：",
            views.len(),
            issues.len()
        )
        .map_err(io_err)?;
        let owned: Vec<LinkView> = issues.into_iter().cloned().collect();
        output::write_check_table(writer, &owned)?;
    }

    let mut pruned = 0usize;
    if prune {
        for view in &views {
            if view.status == LinkStatus::Stale && repository::delete_by_id(conn, view.id)? {
                pruned += 1;
            }
        }
        if !json && pruned > 0 {
            writeln!(
                writer,
                "已从库中移除 {pruned} 条 stale 记录（未动磁盘文件）。"
            )
            .map_err(io_err)?;
        }
    }

    perf::log_perf(
        "check",
        started.elapsed(),
        &[
            ("json", json.to_string()),
            ("prune", prune.to_string()),
            ("total", views.len().to_string()),
            (
                "issues",
                views
                    .iter()
                    .filter(|v| v.status != LinkStatus::Ok)
                    .count()
                    .to_string(),
            ),
            ("pruned", pruned.to_string()),
        ],
    );
    Ok(())
}

fn io_err(e: std::io::Error) -> SymmError {
    SymmError::IoError {
        message: e.to_string(),
    }
}
