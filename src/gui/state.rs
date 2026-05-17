use crate::domain::model::{LinkKind, LinkStatus, LinkView};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainView {
    Dashboard,
    Detail,
}

#[derive(Debug)]
pub struct AppState {
    pub search: String,
    pub selected_id: Option<i64>,
    pub expanded_groups: HashSet<String>,
    pub main_view: MainView,
    pub sidebar_width: f32,
    pub toast: Option<String>,
    pub data_home: Option<PathBuf>,
    pub db_error: Option<String>,
}

#[derive(Debug, Default)]
pub struct LinkSnapshot {
    pub views: Vec<LinkView>,
    pub scanned: usize,
}

impl LinkSnapshot {
    pub fn total(&self) -> usize {
        self.scanned
    }

    pub fn ok_count(&self) -> usize {
        self.views
            .iter()
            .filter(|v| v.status == LinkStatus::Ok)
            .count()
    }

    pub fn kind_counts(&self) -> (usize, usize) {
        let mut symlink = 0usize;
        let mut junction = 0usize;
        for v in &self.views {
            match v.link_kind {
                LinkKind::Symlink => symlink += 1,
                LinkKind::Junction => junction += 1,
            }
        }
        (symlink, junction)
    }

    pub fn filtered<'a>(&'a self, search: &str) -> Vec<&'a LinkView> {
        let q = search.trim().to_lowercase();
        self.views
            .iter()
            .filter(|v| {
                if q.is_empty() {
                    return true;
                }
                v.display_name().to_lowercase().contains(&q)
                    || v.link_path.to_lowercase().contains(&q)
                    || v.target_path.to_lowercase().contains(&q)
            })
            .collect()
    }

    pub fn selected_view(&self, id: Option<i64>) -> Option<&LinkView> {
        let id = id?;
        self.views.iter().find(|v| v.id == id)
    }

    pub fn groups(&self, search: &str) -> Vec<(String, Vec<&LinkView>)> {
        use std::collections::BTreeMap;
        let mut map: BTreeMap<String, Vec<&LinkView>> = BTreeMap::new();
        for view in self.filtered(search) {
            let group = PathBuf::from(&view.link_path)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "（根目录）".to_string());
            map.entry(group).or_default().push(view);
        }
        map.into_iter().collect()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            search: String::new(),
            selected_id: None,
            expanded_groups: HashSet::new(),
            main_view: MainView::Dashboard,
            sidebar_width: 260.0,
            toast: None,
            data_home: None,
            db_error: None,
        }
    }
}
