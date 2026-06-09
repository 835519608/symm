use crate::domain::model::LinkStatus;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Parser)]
#[command(name = "symm-cli", version, about = "链接管理命令行工具")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Add {
        /// link 路径；省略则交互填写（可先选库中模板）
        link: Option<PathBuf>,
        /// 实体数据位置；省略则交互填写
        target: Option<PathBuf>,
    },
    Adopt {
        /// 要接管的真实文件/目录位置，接管后会变成链接位置
        link: Option<PathBuf>,
        /// 真实数据迁移到的位置
        target: Option<PathBuf>,
    },
    Point {
        /// 已存在的链接位置
        link: Option<PathBuf>,
        /// 新的真实数据位置
        target: Option<PathBuf>,
    },
    Rm {
        /// ls 序号（纯数字）或 name；可多个；省略则交互多选
        #[arg(value_name = "SELECTOR")]
        selectors: Vec<String>,
    },
    Restore {
        /// ls 序号（纯数字）或 name；可多个；省略则交互多选
        #[arg(value_name = "SELECTOR")]
        selectors: Vec<String>,
    },
    Ls {
        #[arg(long)]
        json: bool,
        #[arg(long, value_parser = parse_status_arg)]
        status: Option<LinkStatus>,
        #[arg(long, value_parser = parse_positive_limit_arg, allow_hyphen_values = true)]
        limit: Option<u32>,
        #[arg(long, default_value_t = 0, value_parser = parse_offset_arg, allow_hyphen_values = true)]
        offset: u32,
    },
    Show {
        /// ls 序号（纯数字）或 name；省略则交互选择
        selector: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// 内部：提权子进程扫描占用（用户勿直接调用）
    #[command(hide = true, name = "__elevated-list-locks")]
    ElevatedListLocks {
        #[arg(long)]
        out: PathBuf,
        path: PathBuf,
        /// 内部：提权子进程错误日志路径
        #[arg(long = "elevated-log", hide = true)]
        elevated_log: Option<PathBuf>,
        /// 内部：提权子进程进度 JSONL 路径（父进程读取并显示）
        #[arg(long = "elevated-progress", hide = true)]
        elevated_progress: Option<PathBuf>,
    },
    /// 内部：提权子进程结束占用（用户勿直接调用）
    #[command(hide = true, name = "__elevated-kill")]
    ElevatedKill {
        #[arg(value_delimiter = ',')]
        pids: Vec<u32>,
    },
    /// 内部：提权子进程创建链接（仅 Windows，用户勿直接调用）
    #[cfg(windows)]
    #[command(hide = true, name = "__elevated-create-link")]
    ElevatedCreateLink {
        #[arg(long = "link-kind", hide = true)]
        link_kind: Option<String>,
        target: PathBuf,
        link: PathBuf,
    },
}

fn parse_status_arg(raw: &str) -> Result<LinkStatus, String> {
    LinkStatus::from_str(raw).map_err(|_| {
        format!("状态无效：{raw}（可选：ok / broken / missing / stale / drift / unknown）")
    })
}

fn parse_positive_limit_arg(raw: &str) -> Result<u32, String> {
    let limit = raw
        .parse::<u32>()
        .map_err(|_| format!("limit 无效：{raw}（必须是正整数）"))?;
    if limit == 0 {
        return Err("limit 无效：0（必须是正整数）".to_string());
    }
    Ok(limit)
}

fn parse_offset_arg(raw: &str) -> Result<u32, String> {
    raw.parse::<u32>()
        .map_err(|_| format!("offset 无效：{raw}（必须是非负整数）"))
}
