#[cfg(windows)]
use std::path::Path;

#[cfg(windows)]
pub(crate) fn verbatim_wide_path(path: &Path) -> Option<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;

    fn wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s).encode_wide().collect()
    }

    let absolute;
    let path = if path.is_absolute() {
        path
    } else {
        absolute = std::env::current_dir().ok()?.join(path);
        absolute.as_path()
    };

    let raw: Vec<u16> = path.as_os_str().encode_wide().collect();
    let verbatim = wide(r"\\?\");
    let nt_verbatim = wide(r"\??\");
    if raw.starts_with(&verbatim) || raw.starts_with(&nt_verbatim) {
        let mut out = raw;
        out.push(0);
        return Some(out);
    }

    let unc = wide(r"\\");
    let mut out = if raw.starts_with(&unc) {
        let mut prefixed = wide(r"\\?\UNC\");
        prefixed.extend_from_slice(&raw[unc.len()..]);
        prefixed
    } else {
        let mut prefixed = verbatim;
        prefixed.extend_from_slice(&raw);
        prefixed
    };
    out.push(0);
    Some(out)
}
