//! 记录选择菜单（纯 inquire；选项由 workflow / adapter 准备）。

use crate::domain::error::SymmError;
use inquire::{MultiSelect, Select};

pub fn pick_one_option(options: &[String]) -> Result<String, SymmError> {
    Select::new("选择一条记录", options.to_vec())
        .with_help_message("↑↓ 移动 Enter 确认；行首 # 为列表序号")
        .prompt()
        .map_err(|e| SymmError::InvalidArgument {
            message: format!("已取消：{e}"),
        })
}

pub fn pick_many_options(options: &[String]) -> Result<Vec<String>, SymmError> {
    MultiSelect::new("选择要删除的记录", options.to_vec())
        .with_help_message("空格切换选中 Enter 确认")
        .prompt()
        .map_err(|e| SymmError::InvalidArgument {
            message: format!("已取消：{e}"),
        })
}
