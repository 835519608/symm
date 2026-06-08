use crate::domain::error::SymmError;
use crate::ui::cli::Commands;
use crate::workflows;
use std::io::Write;

pub fn execute<W: Write>(command: Commands, writer: &mut W) -> Result<(), SymmError> {
    let conn = crate::adapters::db::link_store::open()?;
    match command {
        Commands::Add { link, target } => {
            let (link, target) = crate::app::cli_decisions::resolve_add_paths(
                &conn,
                link.as_deref(),
                target.as_deref(),
            )?;
            let mut decisions = crate::app::cli_decisions::CliAddDecisions;
            workflows::add::workflow::run_with_decisions(
                &conn,
                &link,
                &target,
                &mut decisions,
                writer,
            )
        }
        Commands::Rm { selectors } => workflows::rm::workflow::run_with_mode_picker(
            &conn,
            &selectors,
            crate::app::cli_decisions::select_rm_mode,
            writer,
        ),
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
