//! 记录选择菜单（纯 inquire；选项由 workflow / adapter 准备）。

use crate::domain::error::SymmError;
use inquire::{MultiSelect, Select, Text};

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

pub fn prompt_one_selector(total: usize, option_limit: usize) -> Result<String, SymmError> {
    let raw = Text::new("记录较多，请输入名称或列表序号")
        .with_help_message(&format!(
            "当前 {total} 条，菜单最多显示 {option_limit} 条；纯数字按 ls 序号解析"
        ))
        .prompt()
        .map_err(|e| SymmError::InvalidArgument {
            message: format!("已取消：{e}"),
        })?;
    let selector = raw.trim();
    if selector.is_empty() {
        return Err(SymmError::InvalidArgument {
            message: "选择器不能为空".to_string(),
        });
    }
    Ok(selector.to_string())
}

pub fn prompt_many_selectors(total: usize, option_limit: usize) -> Result<Vec<String>, SymmError> {
    let raw = Text::new("记录较多，请输入要删除的名称或序号")
        .with_help_message(&format!(
            "当前 {total} 条，菜单最多显示 {option_limit} 条；多个选择器用逗号分隔"
        ))
        .prompt()
        .map_err(|e| SymmError::InvalidArgument {
            message: format!("已取消：{e}"),
        })?;
    let selectors = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if selectors.is_empty() {
        return Err(SymmError::InvalidArgument {
            message: "未选择任何记录".to_string(),
        });
    }
    Ok(selectors)
}
