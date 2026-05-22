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

/// 一次打开系统选择器：macOS 可在同一对话框中选文件或文件夹；其它平台为原生「打开」对话框。
pub fn pick_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        if let Some(path) = pick_path_macos() {
            return Some(path);
        }
    }
    rfd::FileDialog::new().set_title("选择路径").pick_file()
}

#[cfg(target_os = "macos")]
fn pick_path_macos() -> Option<PathBuf> {
    let script = r#"
ObjC.import("AppKit");
var panel = $.NSOpenPanel.openPanel;
panel.setTitle("选择路径");
panel.setCanChooseFiles(true);
panel.setCanChooseDirectories(true);
panel.setAllowsMultipleSelection(false);
panel.setPrompt("选择");
if (panel.runModal() === $.NSFileHandlingPanelOKButton) {
    panel.URL.path.js;
}
"#;
    let out = Command::new("osascript")
        .args(["-l", "JavaScript", "-e", script])
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
