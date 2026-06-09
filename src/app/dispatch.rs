use crate::domain::error::SymmError;
use crate::ui::cli::Commands;
use crate::workflows;
use crate::workflows::link_ops::workflow::LinkOperation;
use std::io::Write;
use std::path::PathBuf;

pub fn execute<W: Write>(command: Commands, writer: &mut W) -> Result<(), SymmError> {
    match command {
        Commands::Add { .. }
        | Commands::Adopt { .. }
        | Commands::Point { .. }
        | Commands::Rm { .. }
        | Commands::Restore { .. }
        | Commands::Ls { .. }
        | Commands::Show { .. } => {
            let conn = crate::adapters::db::link_store::open()?;
            execute_with_conn(&conn, command, writer)
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

fn execute_with_conn<W: Write>(
    conn: &rusqlite::Connection,
    command: Commands,
    writer: &mut W,
) -> Result<(), SymmError> {
    match command {
        Commands::Add { link, target } => {
            execute_link_operation(conn, LinkOperation::Add, link, target, writer)
        }
        Commands::Adopt { link, target } => {
            execute_link_operation(conn, LinkOperation::Adopt, link, target, writer)
        }
        Commands::Point { link, target } => {
            execute_link_operation(conn, LinkOperation::Point, link, target, writer)
        }
        Commands::Rm { selectors } => workflows::rm::workflow::run_rm(conn, &selectors, writer),
        Commands::Restore { selectors } => {
            workflows::rm::workflow::run_restore(conn, &selectors, writer)
        }
        Commands::Ls {
            json,
            status: wanted,
            limit,
            offset,
        } => workflows::ls::workflow::run(conn, json, wanted, limit, offset, writer),
        Commands::Show { selector, json } => {
            workflows::show::workflow::run(conn, selector.as_deref(), json, writer)
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

fn execute_link_operation<W: Write>(
    conn: &rusqlite::Connection,
    operation: LinkOperation,
    link: Option<PathBuf>,
    target: Option<PathBuf>,
    writer: &mut W,
) -> Result<(), SymmError> {
    let (link, target) =
        crate::ui::cli_decisions::resolve_link_op_paths(conn, link.as_deref(), target.as_deref())?;
    let mut decisions = crate::ui::cli_decisions::CliLinkOpDecisions;
    workflows::link_ops::workflow::run_operation(
        conn,
        operation,
        &link,
        &target,
        &mut decisions,
        writer,
    )
}
