use crate::adopt;
use crate::cli::{Commands, StatusArg};
use crate::db;
use crate::error::SymmError;
use crate::link_ops;
use crate::model::LinkStatus;
use crate::output;
use crate::paths;
use inquire::Text;
use std::io::Write;
use std::path::Path;

pub fn execute<W: Write>(command: Commands, writer: &mut W) -> Result<(), SymmError> {
    let conn = db::open_db()?;
    match command {
        Commands::Add { link, target } => {
            let link_norm = paths::normalize_link(&link);
            let existing = db::get_by_link_path(&conn, &link_norm)?;
            let prep = adopt::resolve_add_conflict(Path::new(&link_norm), &target)?;

            let target_norm = paths::normalize_target(&target)?;
            let link_kind =
                match link_ops::create_link(Path::new(&target_norm), Path::new(&link_norm)) {
                    Ok(kind) => kind,
                    Err(e) => {
                        let _ = prep.rollback(Path::new(&link_norm), Path::new(&target_norm));
                        return Err(e);
                    }
                };

            let default_name = existing.as_ref().map(|r| r.name.as_str()).unwrap_or("");
            let name = resolve_add_name(default_name)?;
            if let Err(e) = db::insert_link(&conn, &name, &link_norm, &target_norm, link_kind) {
                let _ = link_ops::remove_link(Path::new(&link_norm));
                let _ = prep.rollback(Path::new(&link_norm), Path::new(&target_norm));
                return Err(e);
            }
            prep.commit()?;
            let display_name = if name.is_empty() {
                "(空)"
            } else {
                name.as_str()
            };
            writeln!(writer, "创建成功：{link_norm}（name: {display_name}）").map_err(|e| {
                SymmError::IoError {
                    message: e.to_string(),
                }
            })?;
            Ok(())
        }
        Commands::Rm { selector } => {
            let record = db::get_by_selector(&conn, &selector)?;
            link_ops::remove_link(Path::new(&record.link_path))?;
            db::delete_by_selector(&conn, &selector)?;
            writeln!(writer, "删除成功：{}", record.name).map_err(|e| SymmError::IoError {
                message: e.to_string(),
            })?;
            Ok(())
        }
        Commands::Ls { json, status } => {
            let wanted = status.map(status_to_model);
            if json {
                stream_ls_json(&conn, wanted, writer)
            } else {
                stream_ls_table(&conn, wanted, writer)
            }
        }
        Commands::Show { selector, json } => {
            let view = link_ops::as_view(db::get_by_selector(&conn, &selector)?);
            if json {
                let text = output::render_json(&view)?;
                writeln!(writer, "{text}").map_err(|e| SymmError::IoError {
                    message: e.to_string(),
                })?;
                Ok(())
            } else {
                writer
                    .write_all(output::render_show_table(&view).as_bytes())
                    .map_err(|e| SymmError::IoError {
                        message: e.to_string(),
                    })?;
                Ok(())
            }
        }
    }
}

fn stream_ls_table<W: Write>(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
    writer: &mut W,
) -> Result<(), SymmError> {
    let records = db::list_links(conn)?;
    output::write_list_header(writer)?;
    for record in records {
        let view = link_ops::as_view(record);
        if wanted.is_none_or(|s| view.status == s) {
            output::write_list_row(writer, &view)?;
        }
    }
    Ok(())
}

fn stream_ls_json<W: Write>(
    conn: &rusqlite::Connection,
    wanted: Option<LinkStatus>,
    writer: &mut W,
) -> Result<(), SymmError> {
    let records = db::list_links(conn)?;
    output::write_json_array_start(writer)?;
    let mut first = true;
    for record in records {
        let view = link_ops::as_view(record);
        if wanted.is_none_or(|s| view.status == s) {
            output::write_json_item(writer, &view, first)?;
            first = false;
        }
    }
    output::write_json_array_end(writer)
}

fn status_to_model(arg: StatusArg) -> LinkStatus {
    match arg {
        StatusArg::Ok => LinkStatus::Ok,
        StatusArg::Broken => LinkStatus::Broken,
        StatusArg::Missing => LinkStatus::Missing,
    }
}

fn resolve_add_name(default_name: &str) -> Result<String, SymmError> {
    if let Ok(v) = std::env::var("SYMM_ADD_NAME") {
        return Ok(v.trim().to_string());
    }

    Text::new("可选填写 name（回车保持默认值）:")
        .with_default(default_name)
        .prompt()
        .map(|s| s.trim().to_string())
        .map_err(|e| SymmError::InvalidArgument {
            message: format!("已取消：{e}"),
        })
}
