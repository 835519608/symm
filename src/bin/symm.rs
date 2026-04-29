use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    if let Err(err) = run() {
        eprintln!("{}", symm::ui::output::render_error_json(&err));
        std::process::exit(1);
    }
    Ok(())
}

fn run() -> Result<(), symm::domain::error::SymmError> {
    let cli = symm::ui::cli::Cli::parse();
    let command = cli
        .command
        .ok_or_else(|| symm::domain::error::SymmError::InvalidArgument {
            message: "未提供命令，请使用 --help 查看帮助".to_string(),
        })?;
    #[cfg(windows)]
    if !symm::adapters::platform::admin::is_elevated() {
        return Err(symm::domain::error::SymmError::PermissionDenied {
            message: "需要管理员权限运行（将通过 UAC 请求授权）".to_string(),
        });
    }
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    symm::app::service::execute(command, &mut lock)?;
    Ok(())
}
