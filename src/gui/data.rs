use crate::adapters::db::repository;
use crate::adapters::paths::runtime_paths;
use crate::domain::error::SymmError;
use crate::gui::state::LinkSnapshot;
use crate::workflows::list_views;
use rusqlite::Connection;

pub struct DataStore {
    conn: Option<Connection>,
}

impl DataStore {
    pub fn new() -> Self {
        Self { conn: None }
    }

    pub fn reload(&mut self) -> Result<LinkSnapshot, SymmError> {
        try_reload(&mut self.conn)
    }
}

fn try_reload(conn_slot: &mut Option<Connection>) -> Result<LinkSnapshot, SymmError> {
    let conn = match conn_slot {
        Some(c) => c,
        None => {
            let c = repository::open_db()?;
            *conn_slot = Some(c);
            conn_slot.as_mut().expect("just inserted")
        }
    };
    let collected = list_views::collect_all(conn, None, None, 0)?;
    Ok(LinkSnapshot {
        views: collected.items,
        scanned: collected.scanned,
    })
}

pub fn data_home_display() -> Result<String, SymmError> {
    runtime_paths::data_home().map(|p| p.display().to_string())
}
