use anyhow::Result;
use clap::Parser;
use symm::ui::cli::{Cli, Commands};

fn main() -> Result<()> {
    if let Err(err) = run() {
        eprintln!("{}", symm::ui::output::render_error_json(&err));
        std::process::exit(1);
    }
    Ok(())
}

fn run() -> Result<(), symm::domain::error::SymmError> {
    let cli = Cli::parse();
    let command = cli
        .command
        .ok_or_else(|| symm::domain::error::SymmError::InvalidArgument {
            message: "未提供命令，请使用 --help 查看帮助".to_string(),
        })?;

    match command {
        Commands::ElevatedListLocks {
            out,
            path,
            elevated_log,
            elevated_progress,
        } => match symm::adapters::lock::elevated_list_locks_entry(
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
        Commands::ElevatedKill { pids } => symm::adapters::lock::elevated_kill_entry(&pids),
        #[cfg(windows)]
        Commands::ElevatedCreateLink { target, link } => {
            symm::adapters::platform::host::elevated_create_link_entry(&target, &link)
        }
        other => {
            let stdout = std::io::stdout();
            let mut lock = stdout.lock();
            symm::app::dispatch::execute(other, &mut lock)
        }
    }
}
