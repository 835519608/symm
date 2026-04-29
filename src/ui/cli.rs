use crate::domain::model::LinkStatus;
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
        link: PathBuf,
        target: PathBuf,
    },
    Rm {
        selector: String,
    },
    Ls {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        status: Option<StatusArg>,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long, default_value_t = 0)]
        offset: u32,
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

impl StatusArg {
    pub fn to_model(self) -> LinkStatus {
        match self {
            StatusArg::Ok => LinkStatus::Ok,
            StatusArg::Broken => LinkStatus::Broken,
            StatusArg::Missing => LinkStatus::Missing,
        }
    }
}
