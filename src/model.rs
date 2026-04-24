#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    Symlink,
    Junction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkStatus {
    Ok,
    Broken,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkRecord {
    pub name: String,
    pub link_path: String,
    pub target_path: String,
    pub link_kind: LinkKind,
}
