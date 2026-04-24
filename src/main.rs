mod cli;
mod db;
mod error;
mod link_ops;
mod model;
mod output;

use anyhow::Result;
use clap::Parser;
use std::path::Path;

fn main() -> Result<()> {
    if let Err(err) = run() {
        eprintln!("{}", output::render_error_json(&err));
        std::process::exit(1);
    }
    Ok(())
}

fn run() -> Result<(), error::SymmError> {
    let cli = cli::Cli::parse();
    let command = cli
        .command
        .ok_or_else(|| error::SymmError::InvalidArgument {
            message: "未提供命令，请使用 --help 查看帮助".to_string(),
        })?;
    let conn = db::open_db()?;

    match command {
        cli::Commands::Add { name, target, link } => {
            if name.trim().is_empty() {
                return Err(error::SymmError::InvalidArgument {
                    message: "名称不能为空".to_string(),
                });
            }
            let target_norm = db::normalize_target(&target)?;
            let link_norm = db::normalize_link(&link);
            let link_kind = link_ops::create_link(Path::new(&target_norm), Path::new(&link_norm))?;

            if let Err(e) = db::insert_link(&conn, &name, &link_norm, &target_norm, link_kind) {
                let _ = link_ops::remove_link(Path::new(&link_norm));
                return Err(e);
            }
            println!("创建成功：{name}");
        }
        cli::Commands::Rm { selector } => {
            let record = db::get_by_selector(&conn, &selector)?;
            link_ops::remove_link(Path::new(&record.link_path))?;
            db::delete_by_selector(&conn, &selector)?;
            println!("删除成功：{}", record.name);
        }
        cli::Commands::Ls { json, status } => {
            let mut views: Vec<_> = db::list_links(&conn)?
                .into_iter()
                .map(link_ops::as_view)
                .collect();
            if let Some(s) = status {
                let wanted = match s {
                    cli::StatusArg::Ok => model::LinkStatus::Ok,
                    cli::StatusArg::Broken => model::LinkStatus::Broken,
                    cli::StatusArg::Missing => model::LinkStatus::Missing,
                };
                views.retain(|v| v.status == wanted);
            }

            if json {
                println!("{}", output::render_json(&views)?);
            } else {
                print!("{}", output::render_list_table(&views));
            }
        }
        cli::Commands::Show { selector, json } => {
            let view = link_ops::as_view(db::get_by_selector(&conn, &selector)?);
            if json {
                println!("{}", output::render_json(&view)?);
            } else {
                print!("{}", output::render_show_table(&view));
            }
        }
    }

    Ok(())
}
