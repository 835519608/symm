use std::path::{Component, Path, PathBuf};

pub(crate) fn clean(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if let Some(last) = components.last()
                    && matches!(last, Component::Normal(_))
                {
                    components.pop();
                    continue;
                }
                if !components
                    .iter()
                    .any(|component| matches!(component, Component::RootDir))
                {
                    components.push(component);
                }
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                components.push(component);
            }
        }
    }
    components
        .iter()
        .map(|component| component.as_os_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::clean;
    use std::path::Path;

    #[test]
    fn clean_preserves_unmatched_leading_parent_dir() {
        assert_eq!(clean(Path::new("../x")), Path::new("../x"));
    }

    #[test]
    fn clean_preserves_multiple_unmatched_leading_parent_dirs() {
        assert_eq!(clean(Path::new("../../x")), Path::new("../../x"));
    }

    #[test]
    fn clean_preserves_parent_dirs_after_unmatched_parent() {
        assert_eq!(clean(Path::new("../a/../../x")), Path::new("../../x"));
    }

    #[test]
    fn clean_collapses_parent_dir_after_normal_component() {
        assert_eq!(clean(Path::new("a/../x")), Path::new("x"));
    }

    #[cfg(unix)]
    #[test]
    fn clean_does_not_preserve_parent_dir_above_root() {
        assert_eq!(clean(Path::new("/../x")), Path::new("/x"));
    }
}
