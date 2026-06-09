use std::io::Write;
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::process::Command;

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

/// 选择文件（返回文件路径）。
#[cfg(not(target_os = "macos"))]
pub fn pick_path_file(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_title(title).pick_file()
}

/// 选择文件夹（返回目录路径）。
pub fn pick_path_folder(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new().set_title(title).pick_folder()
}

/// macOS：同一对话框可选文件或文件夹。
#[cfg(target_os = "macos")]
pub fn pick_path_file_or_folder(title: &str, prompt: &str) -> Option<PathBuf> {
    pick_path_macos(title, prompt)
}

#[cfg(target_os = "macos")]
fn pick_path_macos(title: &str, prompt: &str) -> Option<PathBuf> {
    let title = escape_js_string(title);
    let prompt = escape_js_string(prompt);
    let script = format!(
        r#"
ObjC.import("AppKit");
var panel = $.NSOpenPanel.openPanel;
panel.setTitle("{title}");
panel.setCanChooseFiles(true);
panel.setCanChooseDirectories(true);
panel.setAllowsMultipleSelection(false);
panel.setPrompt("{prompt}");
if (panel.runModal() === $.NSFileHandlingPanelOKButton) {{
    panel.URL.path.js;
}}
"#
    );
    let out = Command::new("osascript")
        .args(["-l", "JavaScript", "-e", &script])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

#[cfg(target_os = "macos")]
fn escape_js_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
