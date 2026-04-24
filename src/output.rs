use crate::error::SymmError;
use crate::model::LinkView;
use serde::Serialize;
use std::fmt::Write;

#[derive(Serialize)]
struct ErrorPayload<'a> {
    code: &'a str,
    message: String,
}

pub fn render_list_table(items: &[LinkView]) -> String {
    let mut out = String::with_capacity(items.len().saturating_mul(64) + 64);
    out.push_str("名称\t状态\t类型\t链接路径\t目标路径\n");
    for item in items {
        let _ = writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}",
            item.name, item.status, item.link_kind, item.link_path, item.target_path
        );
    }
    out
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
