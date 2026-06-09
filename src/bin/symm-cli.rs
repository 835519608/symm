use anyhow::Result;
use clap::Parser;
use clap::error::ErrorKind;
use symm::ui::cli::Cli;

fn main() -> Result<()> {
    if let Err(err) = run() {
        eprintln!("{}", symm::ui::output::render_error_json(&err));
        std::process::exit(1);
    }
    Ok(())
}

fn run() -> Result<(), symm::domain::error::SymmError> {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err)
            if matches!(
                err.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            err.print()
                .map_err(|e| symm::domain::error::SymmError::IoError {
                    message: e.to_string(),
                })?;
            std::process::exit(0);
        }
        Err(err) => {
            return Err(symm::domain::error::SymmError::InvalidArgument {
                message: err.to_string(),
            });
        }
    };
    let command = cli
        .command
        .ok_or_else(|| symm::domain::error::SymmError::InvalidArgument {
            message: "未提供命令，请使用 --help 查看帮助".to_string(),
        })?;

    if symm::app::dispatch::is_elevated_command(&command) {
        return symm::app::dispatch::execute_elevated(command);
    }

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    symm::app::dispatch::execute(command, &mut lock)
}
