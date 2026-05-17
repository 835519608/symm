//! `links` 表查询条件：各字段可选，非空字段 AND 组合（供 repository / 未来 HTTP 服务共用）。

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StringMatch {
    #[default]
    Exact,
    Contains,
}

#[derive(Debug, Clone, Default)]
pub struct LinkQuery {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub name_match: StringMatch,
    pub link_path: Option<String>,
    pub link_path_match: StringMatch,
    pub target_path: Option<String>,
    pub target_path_match: StringMatch,
}

impl LinkQuery {
    pub fn id(id: i64) -> Self {
        Self {
            id: Some(id),
            ..Self::default()
        }
    }

    pub fn name_exact(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            name_match: StringMatch::Exact,
            ..Self::default()
        }
    }

    pub fn link_path_exact(link_path: impl Into<String>) -> Self {
        Self {
            link_path: Some(link_path.into()),
            link_path_match: StringMatch::Exact,
            ..Self::default()
        }
    }

    pub fn has_predicate(&self) -> bool {
        self.id.is_some()
            || self.name.is_some()
            || self.link_path.is_some()
            || self.target_path.is_some()
    }

    pub fn describe(&self) -> String {
        let mut parts = Vec::new();
        if let Some(id) = self.id {
            parts.push(format!("id={id}"));
        }
        if let Some(name) = &self.name {
            parts.push(format!(
                "name {} {:?}",
                match self.name_match {
                    StringMatch::Exact => "=",
                    StringMatch::Contains => "LIKE",
                },
                name
            ));
        }
        if let Some(path) = &self.link_path {
            parts.push(format!(
                "link_path {} {:?}",
                match self.link_path_match {
                    StringMatch::Exact => "=",
                    StringMatch::Contains => "LIKE",
                },
                path
            ));
        }
        if let Some(path) = &self.target_path {
            parts.push(format!(
                "target_path {} {:?}",
                match self.target_path_match {
                    StringMatch::Exact => "=",
                    StringMatch::Contains => "LIKE",
                },
                path
            ));
        }
        if parts.is_empty() {
            "(all)".to_string()
        } else {
            parts.join(" AND ")
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ListOptions {
    pub limit: Option<u32>,
    pub offset: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_sets_fields() {
        let q = LinkQuery::name_exact("demo");
        assert_eq!(q.name.as_deref(), Some("demo"));
        assert_eq!(q.name_match, StringMatch::Exact);
        assert!(q.id.is_none());
    }
}
