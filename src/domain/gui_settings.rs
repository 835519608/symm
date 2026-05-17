use serde::{Deserialize, Serialize};

/// GUI 偏好（持久化于 `data/settings.json`，与链接库 `symm.db` 分离）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GuiSettings {
    pub theme: ThemeMode,
    /// 界面语言（预留 i18n），如 `zh-CN`、`en`。
    pub locale: String,
    pub sidebar_width: f32,
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::System,
            locale: default_locale(),
            sidebar_width: default_sidebar_width(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

fn default_locale() -> String {
    "zh-CN".to_string()
}

fn default_sidebar_width() -> f32 {
    280.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_serialize_round_trip() {
        let json = serde_json::to_string(&GuiSettings::default()).expect("serialize");
        let parsed: GuiSettings = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed, GuiSettings::default());
    }

    #[test]
    fn accepts_unknown_fields_via_default() {
        let raw = r#"{"theme":"dark","locale":"en","sidebar_width":300,"future_flag":true}"#;
        let parsed: GuiSettings = serde_json::from_str(raw).expect("parse");
        assert_eq!(parsed.theme, ThemeMode::Dark);
        assert_eq!(parsed.locale, "en");
        assert!((parsed.sidebar_width - 300.0).abs() < f32::EPSILON);
    }
}
