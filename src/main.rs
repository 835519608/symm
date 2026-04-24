mod cli;
mod db;
mod error;
mod link_ops;
mod model;
mod output;
mod paths;
mod processes;
mod adopt;
mod service;

use anyhow::Result;
use clap::Parser;

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
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    service::execute(command, &mut lock)?;
    Ok(())
}
