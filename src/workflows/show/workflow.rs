use crate::adapters::db::repository;
use crate::adapters::fs::link_status;
use crate::domain::error::SymmError;
use crate::domain::model::LinkRecord;
use crate::ui::output;
use crate::workflows::perf;
use inquire::Select;
use std::io::Write;
use std::time::Instant;

pub fn run<W: Write>(
    conn: &rusqlite::Connection,
    selector: Option<&str>,
    json: bool,
    writer: &mut W,
) -> Result<(), SymmError> {
    let started = Instant::now();
    let selector = resolve_selector(conn, selector)?;
    let view = link_status::as_view(repository::get_by_selector(conn, &selector)?);
    if json {
        let text = output::render_json(&view)?;
        writeln!(writer, "{text}").map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })?;
    } else {
        writer
            .write_all(output::render_show_table(&view).as_bytes())
            .map_err(|e| SymmError::IoError {
                message: e.to_string(),
            })?;
    }
    perf::log_perf(
        "show",
        started.elapsed(),
        &[("selector", selector), ("json", json.to_string())],
    );
    Ok(())
}

fn resolve_selector(
    conn: &rusqlite::Connection,
    selector: Option<&str>,
) -> Result<String, SymmError> {
    if let Some(selector) = selector {
        if !selector.is_empty() {
            return Ok(selector.to_string());
        }
    }
    pick_selector_interactive(conn)
}

fn pick_selector_interactive(conn: &rusqlite::Connection) -> Result<String, SymmError> {
    let records = repository::list_links(conn)?;
    if records.is_empty() {
        return Err(SymmError::NotFound {
            selector: "(空库)".to_string(),
        });
    }

    let options = records.iter().map(format_pick_label).collect::<Vec<_>>();
    let selected = Select::new("选择要查看的记录（可输入筛选）", options)
        .with_help_message("↑↓ 移动，Enter 确认；输入文字可过滤名称/ID")
        .prompt()
        .map_err(|e| SymmError::InvalidArgument {
            message: format!("已取消：{e}"),
        })?;

    parse_pick_label(&selected).ok_or_else(|| SymmError::InvalidArgument {
        message: "无法解析所选记录".to_string(),
    })
}

fn format_pick_label(record: &LinkRecord) -> String {
    let view = link_status::as_view(record.clone());
    format!(
        "#{}  {}  [{}]",
        record.id,
        record.display_name(),
        view.status
    )
}

fn parse_pick_label(label: &str) -> Option<String> {
    let rest = label.strip_prefix('#')?;
    let id = rest.split_whitespace().next()?;
    Some(id.to_string())
}
