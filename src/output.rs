use crate::error::SymmError;
use crate::model::LinkView;
use serde::Serialize;
use std::io::Write;

#[derive(Serialize)]
struct ErrorPayload<'a> {
    code: &'a str,
    message: String,
}

pub fn write_list_header<W: Write>(writer: &mut W) -> Result<(), SymmError> {
    writer
        .write_all("名称\t状态\t类型\t链接路径\t目标路径\n".as_bytes())
        .map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })
}

pub fn write_list_row<W: Write>(writer: &mut W, item: &LinkView) -> Result<(), SymmError> {
    writeln!(
        writer,
        "{}\t{}\t{}\t{}\t{}",
        item.name, item.status, item.link_kind, item.link_path, item.target_path
    )
    .map_err(|e| SymmError::IoError {
        message: e.to_string(),
    })
}

pub fn write_json_array_start<W: Write>(writer: &mut W) -> Result<(), SymmError> {
    writer.write_all(b"[").map_err(|e| SymmError::IoError {
        message: e.to_string(),
    })
}

pub fn write_json_array_end<W: Write>(writer: &mut W) -> Result<(), SymmError> {
    writer.write_all(b"]\n").map_err(|e| SymmError::IoError {
        message: e.to_string(),
    })
}

pub fn write_json_item<W: Write>(
    writer: &mut W,
    item: &LinkView,
    is_first: bool,
) -> Result<(), SymmError> {
    if !is_first {
        writer.write_all(b",").map_err(|e| SymmError::IoError {
            message: e.to_string(),
        })?;
    }
    serde_json::to_writer(writer, item).map_err(|e| SymmError::IoError {
        message: e.to_string(),
    })
}

pub fn render_show_table(item: &LinkView) -> String {
    format!(
        "名称: {}\n状态: {}\n类型: {}\n链接路径: {}\n目标路径: {}\n",
        item.name, item.status, item.link_kind, item.link_path, item.target_path
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
