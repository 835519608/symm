use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "symm", version, about = "软链接管理命令行工具")]
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
        status: Option<StatusArg>,
    },
    Show {
        selector: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum StatusArg {
    Ok,
    Broken,
    Missing,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
