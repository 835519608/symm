use crate::domain::error::SymmError;
use crate::domain::model::LinkView;
use serde::Serialize;
use std::io::Write;
use unicode_width::UnicodeWidthStr;

const LINK_TABLE_HEADERS: [&str; 6] = ["序号", "名称", "类型", "链接路径", "目标路径", "状态"];
const LINK_TABLE_WIDTHS: [usize; 6] = [6, 24, 8, 56, 56, 12];

pub struct ListPageInfo<'a> {
    pub emitted: usize,
    pub limit: u32,
    pub offset: u32,
    pub has_more: bool,
    pub status: Option<&'a str>,
}

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
    let name = item.display_name();
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

pub fn write_list_table_footer<W: Write>(
    writer: &mut W,
    page: ListPageInfo<'_>,
) -> Result<(), SymmError> {
    let start = if page.emitted == 0 {
        0
    } else {
        page.offset + 1
    };
    let end = page.offset + page.emitted as u32;
    let range_label = if page.status.is_some() {
        "显示匹配结果"
    } else {
        "显示结果"
    };
    writeln!(
        writer,
        "\n{range_label} {start}-{end}；表格序号是全库 ls 序号，可用于 show/rm/restore。"
    )
    .map_err(io_err)?;
    if page.has_more {
        let next_offset = page.offset + page.limit;
        let mut command = format!(
            "symm-cli ls --limit {} --offset {}",
            page.limit, next_offset
        );
        if let Some(status) = page.status {
            command.push_str(&format!(" --status {status}"));
        }
        writeln!(writer, "下一页：{command}").map_err(io_err)?;
    }
    Ok(())
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
        "序号: {}\n名称: {}\n类型: {}\n链接路径: {}\n目标路径: {}\n状态: {}\n{}",
        item.index,
        item.display_name(),
        item.link_kind.label_zh(),
        item.link_path,
        item.target_path,
        item.status.label_zh(),
        item.status_error
            .as_ref()
            .map(|err| format!("状态错误: {err}\n"))
            .unwrap_or_default(),
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

fn cell_width(s: &str) -> usize {
    s.width()
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

    fn sample_view(name: &str) -> LinkView {
        LinkView {
            record: LinkRecord {
                id: 1,
                name: name.to_string(),
                link_path: "/tmp/link".to_string(),
                target_path: "/tmp/target".to_string(),
                link_kind: LinkKind::Symlink,
                created_at: 0,
                updated_at: 0,
            },
            index: 1,
            status: LinkStatus::Ok,
            status_error: None,
        }
    }

    #[test]
    fn list_table_aligns_production_header_with_ascii_cells() {
        let mut out = Vec::new();
        write_list_table(&mut out, &[sample_view("demo")]).expect("write table");
        let table = String::from_utf8(out).expect("utf8");
        let lines: Vec<_> = table.lines().collect();
        assert_eq!(cell_width(lines[0]), cell_width(lines[2]));
    }

    #[test]
    fn list_table_item_does_not_truncate_long_paths() {
        let long_path = format!("/tmp/{}", "a".repeat(120));
        let mut view = sample_view("long");
        view.record.link_path = long_path.clone();
        view.record.target_path = long_path.clone();
        let mut out = Vec::new();
        write_list_table(&mut out, &[view]).expect("write table");
        let text = String::from_utf8(out).expect("utf8");
        assert!(text.contains(&long_path));
    }
}
