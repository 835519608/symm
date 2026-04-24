use crate::cli::{Commands, StatusArg};
use crate::db;
use crate::error::SymmError;
use crate::link_ops;
use crate::model::{LinkStatus, LinkView};
use crate::output;
use crate::paths;
use rayon::prelude::*;
use std::path::Path;

pub fn execute(command: Commands) -> Result<String, SymmError> {
    let conn = db::open_db()?;
    match command {
        Commands::Add { name, target, link } => {
            if name.trim().is_empty() {
                return Err(SymmError::InvalidArgument {
                    message: "名称不能为空".to_string(),
                });
            }
            let target_norm = paths::normalize_target(&target)?;
            let link_norm = paths::normalize_link(&link);
            let link_kind = link_ops::create_link(Path::new(&target_norm), Path::new(&link_norm))?;

            if let Err(e) = db::insert_link(&conn, &name, &link_norm, &target_norm, link_kind) {
                let _ = link_ops::remove_link(Path::new(&link_norm));
                return Err(e);
            }
            Ok(format!("创建成功：{name}\n"))
        }
        Commands::Rm { selector } => {
            let record = db::get_by_selector(&conn, &selector)?;
            link_ops::remove_link(Path::new(&record.link_path))?;
            db::delete_by_selector(&conn, &selector)?;
            Ok(format!("删除成功：{}\n", record.name))
        }
        Commands::Ls { json, status } => {
            let wanted = status.map(status_to_model);
            let views = list_views(&conn, wanted)?;
            if json {
                output::render_json(&views).map(|s| format!("{s}\n"))
            } else {
                Ok(output::render_list_table(&views))
            }
        }
        Commands::Show { selector, json } => {
            let view = link_ops::as_view(db::get_by_selector(&conn, &selector)?);
            if json {
                output::render_json(&view).map(|s| format!("{s}\n"))
            } else {
                Ok(output::render_show_table(&view))
            }
        }
    }
}

fn list_views(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
) -> Result<Vec<LinkView>, SymmError> {
    let records = db::list_links(conn)?;
    const PARALLEL_THRESHOLD: usize = 128;

    if records.len() < PARALLEL_THRESHOLD {
        return Ok(records
            .into_iter()
            .map(link_ops::as_view)
            .filter(|view| wanted.is_none_or(|s| view.status == s))
            .collect());
    }

    Ok(records
        .into_par_iter()
        .map(link_ops::as_view)
        .filter(|view| wanted.is_none_or(|s| view.status == s))
        .collect())
}

fn status_to_model(arg: StatusArg) -> LinkStatus {
    match arg {
        StatusArg::Ok => LinkStatus::Ok,
        StatusArg::Broken => LinkStatus::Broken,
        StatusArg::Missing => LinkStatus::Missing,
    }
}
