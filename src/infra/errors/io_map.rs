use crate::domain::error::SymmError;

pub fn ioe(error: std::io::Error) -> SymmError {
    SymmError::IoError {
        message: error.to_string(),
    }
}

pub fn io_ctx(context: &str, error: std::io::Error) -> SymmError {
    SymmError::IoError {
        message: format!("{context}：{error}"),
    }
}
