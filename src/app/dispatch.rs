use crate::domain::error::SymmError;
use crate::ui::cli::{Cli, Commands};
use crate::workflows;
use crate::workflows::link_ops::workflow::LinkOperation;
use clap::Parser;
use std::io::Write;
use std::path::PathBuf;

pub fn try_execute_elevated_from_args() -> Result<bool, SymmError> {
    let Some(first_arg) = std::env::args_os().nth(1) else {
        return Ok(false);
    };
    if !first_arg.to_string_lossy().starts_with("__elevated-") {
        return Ok(false);
    }
    let cli = Cli::try_parse().map_err(|err| SymmError::InvalidArgument {
        message: err.to_string(),
    })?;
    let command = cli.command.ok_or_else(|| SymmError::InvalidArgument {
        message: "提权子进程缺少命令".to_string(),
    })?;
    execute_elevated(command)?;
    Ok(true)
}

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

pub fn execute_elevated(command: Commands) -> Result<(), SymmError> {
    match command {
        Commands::ElevatedListLocks {
            out,
            path,
            elevated_log,
            elevated_progress,
        } => match crate::adapters::lock::elevated_list_locks_entry(
            &path,
            &out,
            elevated_progress.as_deref(),
        ) {
            Ok(()) => Ok(()),
            Err(err) => {
                if let Some(log) = elevated_log {
                    let _ = std::fs::write(&log, err.to_string());
                }
                Err(err)
            }
        },
        Commands::ElevatedKill { pids } => crate::adapters::lock::elevated_kill_entry(&pids),
        #[cfg(windows)]
        Commands::ElevatedCreateLink {
            link_kind,
            target,
            link,
        } => crate::adapters::platform::host::elevated_create_link_entry(
            &target,
            &link,
            link_kind.as_deref(),
        ),
        _ => Err(SymmError::InvalidArgument {
            message: "非提权内部命令不能由提权入口执行".to_string(),
        }),
    }
}

pub fn is_elevated_command(command: &Commands) -> bool {
    matches!(
        command,
        Commands::ElevatedListLocks { .. } | Commands::ElevatedKill { .. }
    ) || {
        #[cfg(windows)]
        {
            matches!(command, Commands::ElevatedCreateLink { .. })
        }
        #[cfg(not(windows))]
        {
            false
        }
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
