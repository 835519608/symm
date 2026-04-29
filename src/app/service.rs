use crate::domain::error::SymmError;
use crate::ui::cli::Commands;
use crate::workflows;
use std::io::Write;

pub fn execute<W: Write>(command: Commands, writer: &mut W) -> Result<(), SymmError> {
    let conn = crate::adapters::db::repository::open_db()?;
    if let Err(err) = workflows::recovery::workflow::recover_pending_operations(&conn, writer) {
        writeln!(writer, "恢复扫描失败（已跳过，不阻断当前命令）：{err}").map_err(|e| {
            SymmError::IoError {
                message: e.to_string(),
            }
        })?;
    }
    match command {
        Commands::Add { link, target } => {
            workflows::add::workflow::run(&conn, &link, &target, writer)
        }
        Commands::Rm { selector } => workflows::rm::workflow::run(&conn, &selector, writer),
        Commands::Ls { json, status } => {
            let wanted = status.map(|value| value.to_model());
            workflows::ls::workflow::run(&conn, json, wanted, writer)
        }
        Commands::Show { selector, json } => {
            workflows::show::workflow::run(&conn, &selector, json, writer)
        }
    }
}
