use crate::app::commands;
use crate::domain::error::SymmError;
use crate::interface::cli::Commands;
use std::io::Write;

pub fn execute<W: Write>(command: Commands, writer: &mut W) -> Result<(), SymmError> {
    let conn = crate::infra::db::repository::open_db()?;
    match command {
        Commands::Add { link, target } => commands::add::run(&conn, &link, &target, writer),
        Commands::Rm { selector } => commands::rm::run(&conn, &selector, writer),
        Commands::Ls { json, status } => {
            let wanted = status.map(commands::add::status_to_model);
            commands::ls::run(&conn, json, wanted, writer)
        }
        Commands::Show { selector, json } => commands::show::run(&conn, &selector, json, writer),
    }
}
