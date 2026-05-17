use crate::domain::error::SymmError;
use crate::ui::cli::Commands;
use crate::workflows;
use std::io::Write;

pub fn execute<W: Write>(command: Commands, writer: &mut W) -> Result<(), SymmError> {
    let conn = crate::adapters::db::repository::open_db()?;
    match command {
        Commands::Add { link, target } => {
            workflows::add::workflow::run(&conn, &link, &target, writer)
        }
        Commands::Rm { selectors } => workflows::rm::workflow::run(&conn, &selectors, writer),
        Commands::Ls {
            json,
            status,
            limit,
            offset,
        } => {
            let wanted = status.map(|value| value.to_model());
            workflows::ls::workflow::run(&conn, json, wanted, limit, offset, writer)
        }
        Commands::Show { selector, json } => {
            workflows::show::workflow::run(&conn, selector.as_deref(), json, writer)
        }
        Commands::ElevatedListLocks { .. } | Commands::ElevatedKill { .. } => {
            Err(SymmError::InvalidArgument {
                message: "内部提权子命令应由 CLI 入口直接处理".to_string(),
            })
        }
        #[cfg(windows)]
        Commands::ElevatedCreateLink { .. } => Err(SymmError::InvalidArgument {
            message: "内部提权子命令应由 CLI 入口直接处理".to_string(),
        }),
    }
}
