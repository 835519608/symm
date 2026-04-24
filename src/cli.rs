use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "symm", version, about = "Symlink manager CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Add {
        name: String,
        target: PathBuf,
        link: PathBuf,
    },
    Rm {
        selector: String,
    },
    Ls {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        status: Option<String>,
    },
    Show {
        selector: String,
        #[arg(long)]
        json: bool,
    },
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
