mod cli;
mod db;
mod error;
mod link_ops;
mod model;
mod output;

use anyhow::Result;

fn main() -> Result<()> {
    let _ = cli::Cli::parse_args();
    Ok(())
}
