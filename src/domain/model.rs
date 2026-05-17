use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LinkKind {
    #[serde(rename = "symlink")]
    Symlink,
    #[serde(rename = "junction")]
    Junction,
}

impl Display for LinkKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkKind::Symlink => write!(f, "symlink"),
            LinkKind::Junction => write!(f, "junction"),
        }
    }
}

impl FromStr for LinkKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "symlink" => Ok(LinkKind::Symlink),
            "junction" => Ok(LinkKind::Junction),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LinkStatus {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "broken")]
    Broken,
    #[serde(rename = "missing")]
    Missing,
    /// 路径存在，但已不是软链/junction（库内陈旧记录）
    #[serde(rename = "stale")]
    Stale,
    /// 仍是软链，但指向与库中 target 不一致
    #[serde(rename = "drift")]
    Drift,
}

impl Display for LinkStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkStatus::Ok => write!(f, "ok"),
            LinkStatus::Broken => write!(f, "broken"),
            LinkStatus::Missing => write!(f, "missing"),
            LinkStatus::Stale => write!(f, "stale"),
            LinkStatus::Drift => write!(f, "drift"),
        }
    }
}

impl FromStr for LinkStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ok" => Ok(LinkStatus::Ok),
            "broken" => Ok(LinkStatus::Broken),
            "missing" => Ok(LinkStatus::Missing),
            "stale" => Ok(LinkStatus::Stale),
            "drift" => Ok(LinkStatus::Drift),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LinkRecord {
    pub id: i64,
    pub name: String,
    pub link_path: String,
    pub target_path: String,
    pub link_kind: LinkKind,
    pub created_at: i64,
    pub updated_at: i64,
}

impl LinkRecord {
    pub fn display_name(&self) -> String {
        if !self.name.is_empty() {
            return self.name.clone();
        }
        Path::new(&self.link_path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.link_path.clone())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkView {
    pub id: i64,
    pub name: String,
    pub link_path: String,
    pub target_path: String,
    pub link_kind: LinkKind,
    pub status: LinkStatus,
}

impl LinkView {
    pub fn display_name(&self) -> String {
        if !self.name.is_empty() {
            return self.name.clone();
        }
        Path::new(&self.link_path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.link_path.clone())
    }
}
