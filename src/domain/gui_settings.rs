use serde::{Deserialize, Serialize};

/// 正文字号（px，逻辑像素）；其它 `TextStyle` 按固定比例推导。
pub const FONT_SIZE_PT_MIN: f32 = 10.0;
pub const FONT_SIZE_PT_MAX: f32 = 18.0;
pub const FONT_SIZE_PT_DEFAULT: f32 = 14.0;

/// GUI 偏好（持久化于 `data/settings.json`，与链接库 `symm.db` 分离）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GuiSettings {
    pub theme: ThemeMode,
    pub locale: Locale,
    /// 配色方案（与明暗模式独立）。
    pub color_scheme: ColorScheme,
    pub sidebar_width: f32,
    /// 正文字号（px）；非整页倍率缩放。
    pub font_size_pt: f32,
    /// 数据目录；`None` 或空串表示可执行文件旁默认 `data/`。
    #[serde(default)]
    pub data_dir: Option<String>,
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::System,
            locale: Locale::default(),
            color_scheme: ColorScheme::default(),
            sidebar_width: default_sidebar_width(),
            font_size_pt: FONT_SIZE_PT_DEFAULT,
            data_dir: None,
        }
    }
}

pub fn data_dir_from_settings(settings: &GuiSettings) -> String {
    settings
        .data_dir
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_default()
        .to_string()
}

pub fn sanitize_font_size_pt(v: f32) -> f32 {
    if !v.is_finite() {
        return FONT_SIZE_PT_DEFAULT;
    }
    v.clamp(FONT_SIZE_PT_MIN, FONT_SIZE_PT_MAX)
}

fn default_sidebar_width() -> f32 {
    280.0
}

/// 界面语言（`settings.json` 存 `zh-CN` / `en`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Locale {
    #[default]
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en")]
    En,
}

impl Locale {
    pub fn next(self) -> Self {
        match self {
            Locale::ZhCn => Locale::En,
            Locale::En => Locale::ZhCn,
        }
    }

    pub fn toggle_label(self) -> &'static str {
        match self {
            Locale::ZhCn => "中文",
            Locale::En => "EN",
        }
    }
}

/// 明暗：跟随系统 / 浅色 / 深色。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub fn next(self) -> Self {
        match self {
            Self::System => Self::Light,
            Self::Light => Self::Dark,
            Self::Dark => Self::System,
        }
    }

    /// 仅 Light/Dark；`System` 由 GUI 层 `theme::resolve` 解析。
    pub fn is_dark(self) -> bool {
        matches!(self, ThemeMode::Dark)
    }
}

/// 配色方案（参考常见工具主题：中性 / 蓝 / 绿 / 紫 / 暖色）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorScheme {
    #[default]
    Slate,
    Ocean,
    Forest,
    Violet,
    Ember,
}

impl ColorScheme {
    pub const ALL: [ColorScheme; 5] = [
        ColorScheme::Slate,
        ColorScheme::Ocean,
        ColorScheme::Forest,
        ColorScheme::Violet,
        ColorScheme::Ember,
    ];

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|&s| s == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
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
    fn theme_mode_cycles() {
        assert_eq!(ThemeMode::System.next(), ThemeMode::Light);
        assert_eq!(ThemeMode::Light.next(), ThemeMode::Dark);
        assert_eq!(ThemeMode::Dark.next(), ThemeMode::System);
    }

    #[test]
    fn locale_cycles() {
        assert_eq!(Locale::ZhCn.next(), Locale::En);
        assert_eq!(Locale::En.next(), Locale::ZhCn);
    }

    #[test]
    fn color_scheme_cycles() {
        assert_eq!(ColorScheme::Ember.next(), ColorScheme::Slate);
    }

    #[test]
    fn font_size_clamps() {
        assert_eq!(sanitize_font_size_pt(9.0), 10.0);
        assert_eq!(sanitize_font_size_pt(20.0), 18.0);
    }

    #[test]
    fn accepts_unknown_fields_via_default() {
        let raw = r#"{"theme":"dark","locale":"en","color_scheme":"ocean","sidebar_width":300,"font_size_pt":16,"future_flag":true}"#;
        let parsed: GuiSettings = serde_json::from_str(raw).expect("parse");
        assert_eq!(parsed.theme, ThemeMode::Dark);
        assert_eq!(parsed.locale, Locale::En);
        assert_eq!(parsed.color_scheme, ColorScheme::Ocean);
        assert!((parsed.sidebar_width - 300.0).abs() < f32::EPSILON);
        assert!((parsed.font_size_pt - 16.0).abs() < f32::EPSILON);
    }
}
