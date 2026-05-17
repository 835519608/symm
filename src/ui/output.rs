use crate::domain::error::SymmError;
use crate::domain::model::LinkView;
use serde::Serialize;
use std::io::Write;

#[derive(Serialize)]
struct ErrorPayload<'a> {
    code: &'a str,
    message: String,
}

pub fn write_list_table<W: Write>(writer: &mut W, items: &[LinkView]) -> Result<(), SymmError> {
    let headers = ["ID", "名称", "状态", "类型", "链接路径", "目标路径"];
    let rows: Vec<Vec<String>> = items
        .iter()
        .map(|item| {
            vec![
                item.id.to_string(),
                item.display_name(),
                item.status.to_string(),
                item.link_kind.to_string(),
                item.link_path.clone(),
                item.target_path.clone(),
            ]
        })
        .collect();
    let table = format_table(&headers, &rows);
    writer.write_all(table.as_bytes()).map_err(io_err)
}

pub fn write_check_table<W: Write>(writer: &mut W, items: &[LinkView]) -> Result<(), SymmError> {
    write_list_table(writer, items)
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

pub fn render_show_table(item: &LinkView) -> String {
    format!(
        "ID: {}\n名称: {}\n状态: {}\n类型: {}\n链接路径: {}\n目标路径: {}\n",
        item.id,
        item.display_name(),
        item.status,
        item.link_kind,
        item.link_path,
        item.target_path
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

fn format_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let ncol = headers.len();
    let mut widths = headers
        .iter()
        .map(|h| h.chars().count())
        .collect::<Vec<_>>();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < ncol {
                widths[i] = widths[i].max(cell.chars().count());
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

fn append_row(out: &mut String, cells: &[&str], widths: &[usize], is_header: bool) {
    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let width = widths.get(i).copied().unwrap_or(0);
        pad_cell(out, cell, width);
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

fn pad_cell(out: &mut String, cell: &str, width: usize) {
    let len = cell.chars().count();
    out.push_str(cell);
    if len < width {
        for _ in 0..width - len {
            out.push(' ');
        }
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
        assert!(lines[0].starts_with("ID"));
        assert!(lines[2].starts_with("1 "));
        assert!(lines[3].starts_with("12"));
    }
}
