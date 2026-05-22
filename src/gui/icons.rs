//! Phosphor 图标（`egui-phosphor` regular）。

use egui_phosphor::regular;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Add,
    Refresh,
    FolderOpen,
    Trash,
    Monitor,
    Sun,
    Moon,
    Link,
    Clear,
    Search,
    Globe,
    Palette,
    TextAa,
    Gear,
    Info,
}

impl Icon {
    pub fn glyph(self) -> &'static str {
        match self {
            Icon::Add => regular::PLUS,
            Icon::Refresh => regular::ARROWS_CLOCKWISE,
            Icon::FolderOpen => regular::FOLDER_OPEN,
            Icon::Trash => regular::TRASH,
            Icon::Monitor => regular::MONITOR,
            Icon::Sun => regular::SUN,
            Icon::Moon => regular::MOON,
            Icon::Link => regular::LINK,
            Icon::Clear => regular::BROOM,
            Icon::Search => regular::MAGNIFYING_GLASS,
            Icon::Globe => regular::GLOBE,
            Icon::Palette => regular::PALETTE,
            Icon::TextAa => regular::TEXT_AA,
            Icon::Gear => regular::GEAR,
            Icon::Info => regular::INFO,
        }
    }
}
