use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
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
        write!(f, "{}", self.label_zh())
    }
}

impl LinkKind {
    /// 终端表格/详情用；JSON 仍为 `symlink` / `junction`。
    pub fn label_zh(self) -> &'static str {
        match self {
            LinkKind::Symlink => "软链接",
            LinkKind::Junction => "目录联接",
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
        write!(f, "{}", self.label_zh())
    }
}

impl LinkStatus {
    /// 终端表格/详情用；JSON / `--status` 仍为英文枚举。
    pub fn label_zh(self) -> &'static str {
        match self {
            LinkStatus::Ok => "正常",
            LinkStatus::Broken => "目标没了",
            LinkStatus::Missing => "链接没了",
            LinkStatus::Stale => "不是软链",
            LinkStatus::Drift => "指向不对",
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

/// 纯数字 name 入库前加此前缀，避免与 `show 1` / `rm 1` 等按 id 查询混淆。
pub const LINK_NAME_DIGIT_PREFIX: &str = "link-";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedLinkName {
    pub stored: String,
    pub digit_prefix_applied: bool,
}

/// 空 name 保持为空；非空纯 ASCII 数字则加 `link-` 前缀后入库。
pub fn prepare_link_name_for_storage(name: &str) -> PreparedLinkName {
    if name.is_empty() {
        return PreparedLinkName {
            stored: String::new(),
            digit_prefix_applied: false,
        };
    }
    if name.chars().all(|c| c.is_ascii_digit()) {
        return PreparedLinkName {
            stored: format!("{LINK_NAME_DIGIT_PREFIX}{name}"),
            digit_prefix_applied: true,
        };
    }
    PreparedLinkName {
        stored: name.to_string(),
        digit_prefix_applied: false,
    }
}

/// 与 `links` 表一一对应的行模型（改表主要改此结构 + repository 映射）。
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

/// `ls` / `show` 展示：表行 + 运行时字段。`Deref` 到 [`LinkRecord`]，代码里可直接 `view.link_path`；
/// JSON 用 `flatten` 摊平，不出现 `record` 嵌套层。
#[derive(Debug, Clone, Serialize)]
pub struct LinkView {
    #[serde(flatten)]
    pub record: LinkRecord,
    pub index: u32,
    pub status: LinkStatus,
}

impl Deref for LinkView {
    type Target = LinkRecord;

    fn deref(&self) -> &Self::Target {
        &self.record
    }
}

#[cfg(test)]
mod name_tests {
    use super::{LINK_NAME_DIGIT_PREFIX, LinkRecord, LinkView, prepare_link_name_for_storage};
    use crate::domain::model::LinkKind;

    #[test]
    fn empty_name_unchanged() {
        let p = prepare_link_name_for_storage("");
        assert_eq!(p.stored, "");
        assert!(!p.digit_prefix_applied);
    }

    #[test]
    fn pure_digit_gets_prefix() {
        let p = prepare_link_name_for_storage("42");
        assert_eq!(p.stored, format!("{LINK_NAME_DIGIT_PREFIX}42"));
        assert!(p.digit_prefix_applied);
    }

    #[test]
    fn non_digit_unchanged() {
        let p = prepare_link_name_for_storage("cursor-data");
        assert_eq!(p.stored, "cursor-data");
        assert!(!p.digit_prefix_applied);
    }

    #[test]
    fn already_prefixed_unchanged() {
        let p = prepare_link_name_for_storage("link-42");
        assert_eq!(p.stored, "link-42");
        assert!(!p.digit_prefix_applied);
    }

    #[test]
    fn link_view_deref_exposes_record_fields() {
        let view = LinkView {
            record: LinkRecord {
                id: 7,
                name: "demo".to_string(),
                link_path: "/tmp/link".to_string(),
                target_path: "/tmp/target".to_string(),
                link_kind: LinkKind::Symlink,
                created_at: 0,
                updated_at: 0,
            },
            index: 2,
            status: super::LinkStatus::Ok,
        };
        assert_eq!(view.id, 7);
        assert_eq!(view.name, "demo");
        assert_eq!(view.display_name(), "demo");
        assert_eq!(view.index, 2);
    }
}
