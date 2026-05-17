//! 软链 rebase 的路径计算（无 OS 调用，供 `fs::rebase` 与 `platform::fs` 共用）。

use std::path::{Path, PathBuf};

/// 将 `raw_target`（相对或绝对）映射到 `dst_root` 下的等价路径。
pub fn internal_target(
    dst_root: &Path,
    src_link: &Path,
    raw_target: &Path,
    source_roots: &[PathBuf],
) -> PathBuf {
    let resolved = if raw_target.is_absolute() {
        raw_target.to_path_buf()
    } else {
        src_link
            .parent()
            .unwrap_or_else(|| {
                source_roots
                    .first()
                    .map(PathBuf::as_path)
                    .unwrap_or(dst_root)
            })
            .join(raw_target)
    };

    for base in source_roots {
        if let Ok(rel) = resolved.strip_prefix(base) {
            return dst_root.join(rel);
        }
    }

    raw_target.to_path_buf()
}

/// 迁移/rebase 时用于匹配旧根路径的根目录列表（含 staging 别名）。
pub fn source_roots(src_root: &Path) -> Vec<PathBuf> {
    let mut roots = vec![src_root.to_path_buf()];
    let Some(name) = src_root.file_name().and_then(|n| n.to_str()) else {
        return roots;
    };
    const STAGING_SUFFIX: &str = ".__symm_staging__";
    if let Some(original_name) = name.strip_suffix(STAGING_SUFFIX) {
        let mut original = src_root.to_path_buf();
        original.set_file_name(original_name);
        roots.push(original);
    }
    roots
}

#[cfg(test)]
mod tests {
    use super::{internal_target, source_roots};
    use std::path::PathBuf;

    #[test]
    fn source_roots_includes_staging_alias() {
        let staging = PathBuf::from("/tmp/agent.__symm_staging__");
        let roots = source_roots(&staging);
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0], staging);
        assert_eq!(roots[1], PathBuf::from("/tmp/agent"));
    }

    #[test]
    fn internal_target_rebases_absolute_under_root() {
        let src_root = PathBuf::from("/data/agent");
        let dst_root = PathBuf::from("/data/agent1");
        let roots = source_roots(&src_root);
        let target = internal_target(
            &dst_root,
            &src_root.join("link"),
            &PathBuf::from("/data/agent/nested/file.txt"),
            &roots,
        );
        assert_eq!(target, dst_root.join("nested").join("file.txt"));
    }
}
