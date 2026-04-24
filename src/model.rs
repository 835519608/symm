use serde::Serialize;
use std::fmt::{Display, Formatter};
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
}

impl Display for LinkStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkStatus::Ok => write!(f, "ok"),
            LinkStatus::Broken => write!(f, "broken"),
            LinkStatus::Missing => write!(f, "missing"),
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
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LinkRecord {
    pub name: String,
    pub link_path: String,
    pub target_path: String,
    pub link_kind: LinkKind,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkView {
    pub name: String,
    pub link_path: String,
    pub target_path: String,
    pub link_kind: LinkKind,
    pub status: LinkStatus,
}
