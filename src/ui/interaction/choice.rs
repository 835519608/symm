use crate::domain::error::SymmError;
use inquire::Select;

pub fn choose_with_env<T, F>(
    env_key: &str,
    parse_env: F,
    prompt: &str,
    help_message: &str,
    options: Vec<(String, T)>,
) -> Result<T, SymmError>
where
    T: Copy,
    F: FnOnce(&str) -> Result<T, SymmError>,
{
    if let Ok(raw) = std::env::var(env_key) {
        return parse_env(&raw);
    }
    choose_from_options(prompt, help_message, options)
}

pub fn choose_from_options<T>(
    prompt: &str,
    help_message: &str,
    options: Vec<(String, T)>,
) -> Result<T, SymmError>
where
    T: Copy,
{
    let labels = options
        .iter()
        .map(|(label, _)| label.clone())
        .collect::<Vec<_>>();
    let selected = Select::new(prompt, labels)
        .with_help_message(help_message)
        .prompt()
        .map_err(|e| SymmError::InvalidArgument {
            message: format!("已取消：{e}"),
        })?;

    for (label, value) in options {
        if label == selected {
            return Ok(value);
        }
    }
    Err(SymmError::InvalidArgument {
        message: "无效选择".to_string(),
    })
}
