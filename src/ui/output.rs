use crate::domain::error::SymmError;
use crate::domain::model::LinkView;
use serde::Serialize;
use std::borrow::Cow;
use std::io::Write;
use std::path::Path;
use unicode_width::UnicodeWidthStr;

const LINK_TABLE_HEADERS: [&str; 6] = ["序号", "名称", "类型", "链接路径", "目标路径", "状态"];
const LINK_TABLE_WIDTHS: [usize; 6] = [6, 24, 8, 56, 56, 12];

#[derive(Serialize)]
struct ErrorPayload<'a> {
    code: &'a str,
    message: String,
}

pub fn write_list_table<W: Write>(writer: &mut W, items: &[LinkView]) -> Result<(), SymmError> {
    write_list_table_header(writer)?;
    for row in items {
        write_list_table_item(writer, row)?;
    }
    Ok(())
}

pub fn write_list_table_header<W: Write>(writer: &mut W) -> Result<(), SymmError> {
    write_row(writer, &LINK_TABLE_HEADERS, &LINK_TABLE_WIDTHS, true)
}

pub fn write_list_table_item<W: Write>(writer: &mut W, item: &LinkView) -> Result<(), SymmError> {
    let index = item.index.to_string();
    let name = view_display_name(item);
    let cells = [
        index.as_str(),
        name.as_ref(),
        item.link_kind.label_zh(),
        item.link_path.as_str(),
        item.target_path.as_str(),
        item.status.label_zh(),
    ];
    write_row(writer, &cells, &LINK_TABLE_WIDTHS, false)
}

fn view_display_name(item: &LinkView) -> Cow<'_, str> {
    if !item.name.is_empty() {
        return Cow::Borrowed(item.name.as_str());
    }
    Path::new(&item.link_path)
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .map(Cow::Borrowed)
        .unwrap_or_else(|| Cow::Borrowed(item.link_path.as_str()))
}

pub fn write_json_array_start<W: Write>(writer: &mut W) -> Result<(), SymmError> {
    writer.write_all(b"[").map_err(io_err)
}

pub fn write_json_array_end<W: Write>(writer: &mut W) -> Result<(), SymmError> {
    writer.write_all(b"]\n").map_err(io_err)
}

pub fn write_json_item<W: Write>(
    writer: &mut W,
    item: &LinkView,
    is_first: bool,
) -> Result<(), SymmError> {
    if !is_first {
        writer.write_all(b",").map_err(io_err)?;
    }
    serde_json::to_writer(writer, item).map_err(|e| SymmError::IoError {
        message: e.to_string(),
    })
}

pub fn render_show_detail(item: &LinkView) -> String {
    format!(
        "序号: {}\n名称: {}\n类型: {}\n链接路径: {}\n目标路径: {}\n状态: {}\n",
        item.index,
        item.display_name(),
        item.link_kind.label_zh(),
        item.link_path,
        item.target_path,
        item.status.label_zh(),
    )
}

pub fn render_json<T: Serialize>(value: &T) -> Result<String, SymmError> {
    serde_json::to_string_pretty(value).map_err(|e| SymmError::IoError {
        message: e.to_string(),
    })
}

pub fn render_error_json(err: &SymmError) -> String {
    let payload = ErrorPayload {
        code: err.code(),
        message: err.to_string(),
    };
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| {
        "{\"code\":\"io_error\",\"message\":\"错误信息序列化失败\"}".to_string()
    })
}

fn write_row<W: Write>(
    writer: &mut W,
    cells: &[&str],
    widths: &[usize],
    is_header: bool,
) -> Result<(), SymmError> {
    let mut line = String::new();
    append_row_min_width(&mut line, cells, widths, is_header);
    writer.write_all(line.as_bytes()).map_err(io_err)
}

#[cfg(test)]
fn format_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let ncol = headers.len();
    let mut widths = headers.iter().map(|h| cell_width(h)).collect::<Vec<_>>();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < ncol {
                widths[i] = widths[i].max(cell_width(cell));
            }
        }
    }

    let mut out = String::new();
    append_row(&mut out, headers, &widths, true);
    for row in rows {
        let cells: Vec<&str> = row.iter().map(String::as_str).collect();
        append_row(&mut out, &cells, &widths, false);
    }
    out
}

fn cell_width(s: &str) -> usize {
    s.width()
}

#[cfg(test)]
fn append_row(out: &mut String, cells: &[&str], widths: &[usize], is_header: bool) {
    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let width = widths.get(i).copied().unwrap_or(0);
        pad_cell_left(out, cell, width);
    }
    out.push('\n');
    if is_header {
        for (i, &width) in widths.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            for _ in 0..width {
                out.push('-');
            }
        }
        out.push('\n');
    }
}

fn append_row_min_width(out: &mut String, cells: &[&str], widths: &[usize], is_header: bool) {
    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let width = widths.get(i).copied().unwrap_or(0);
        pad_cell_left(out, cell, width);
    }
    out.push('\n');
    if is_header {
        for (i, &width) in widths.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            for _ in 0..width {
                out.push('-');
            }
        }
        out.push('\n');
    }
}

fn pad_cell_left(out: &mut String, cell: &str, width: usize) {
    out.push_str(cell);
    for _ in 0..width.saturating_sub(cell_width(cell)) {
        out.push(' ');
    }
}

fn io_err(e: std::io::Error) -> SymmError {
    SymmError::IoError {
        message: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::{LinkKind, LinkRecord, LinkStatus};

    #[test]
    fn table_aligns_columns() {
        let table = format_table(
            &["ID", "名称"],
            &[
                vec!["1".into(), "ab".into()],
                vec!["12".into(), "xyz".into()],
            ],
        );
        let lines: Vec<_> = table.lines().collect();
        assert_eq!(cell_width(lines[0]), cell_width(lines[2]));
        assert_eq!(cell_width(lines[0]), cell_width(lines[3]));
    }

    #[test]
    fn table_aligns_cjk_headers_with_ascii_cells() {
        let table = format_table(&["序号", "名称"], &[vec!["1".into(), "demo".into()]]);
        let lines: Vec<_> = table.lines().collect();
        assert_eq!(cell_width(lines[0]), cell_width(lines[2]));
    }

    #[test]
    fn list_table_item_does_not_truncate_long_paths() {
        let long_path = format!("/tmp/{}", "a".repeat(120));
        let view = LinkView {
            record: LinkRecord {
                id: 1,
                name: "long".to_string(),
                link_path: long_path.clone(),
                target_path: long_path.clone(),
                link_kind: LinkKind::Symlink,
                created_at: 0,
                updated_at: 0,
            },
            index: 1,
            status: LinkStatus::Ok,
        };
        let mut out = Vec::new();
        write_list_table(&mut out, &[view]).expect("write table");
        let text = String::from_utf8(out).expect("utf8");
        assert!(text.contains(&long_path));
    }
}
