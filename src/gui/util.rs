use crate::domain::error::SymmError;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn open_path(path: &Path) -> Result<(), SymmError> {
    open_str(&path.to_string_lossy())
}

pub fn open_str(path: &str) -> Result<(), SymmError> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| SymmError::IoError {
                message: e.to_string(),
            })?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| SymmError::IoError {
                message: e.to_string(),
            })?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| SymmError::IoError {
                message: e.to_string(),
            })?;
    }
    Ok(())
}

pub struct VecWriter(pub Vec<u8>);

impl Write for VecWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl VecWriter {
    pub fn into_log(self) -> String {
        String::from_utf8_lossy(&self.0).trim().to_string()
    }
}

pub fn pick_file() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_file()
}

pub fn pick_folder() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_folder()
}
