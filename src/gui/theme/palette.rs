use crate::domain::gui_settings::ColorScheme;
use egui::Color32;

/// 当前界面调色板（极简工具风）。
#[derive(Debug, Clone, Copy)]
pub struct UiPalette {
    pub dark: bool,
    pub bg: Color32,
    pub surface: Color32,
    pub surface_alt: Color32,
    pub surface_hover: Color32,
    pub surface_active: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub text_hover: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub accent_soft: Color32,
    pub shadow: Color32,
}

pub fn for_scheme(scheme: ColorScheme, dark: bool) -> UiPalette {
    let accent = accent_for(scheme, dark);
    let accent_text = accent_text_for(scheme, dark);
    if dark {
        dark_palette(accent, accent_text)
    } else {
        light_palette(accent, accent_text)
    }
}

fn accent_for(scheme: ColorScheme, dark: bool) -> Color32 {
    match scheme {
        ColorScheme::Slate => {
            if dark {
                Color32::from_rgb(0x94, 0xA3, 0xB8)
            } else {
                Color32::from_rgb(0x47, 0x55, 0x69)
            }
        }
        ColorScheme::Ocean => Color32::from_rgb(0x3B, 0x82, 0xF6),
        ColorScheme::Forest => Color32::from_rgb(0x22, 0xC5, 0x5E),
        ColorScheme::Violet => Color32::from_rgb(0x8B, 0x5C, 0xF6),
        ColorScheme::Ember => Color32::from_rgb(0xF5, 0x9E, 0x0B),
    }
}

fn accent_text_for(scheme: ColorScheme, dark: bool) -> Color32 {
    if dark {
        return match scheme {
            ColorScheme::Slate => Color32::from_rgb(0xCB, 0xD5, 0xE1),
            ColorScheme::Ocean => Color32::from_rgb(0x93, 0xC5, 0xFD),
            ColorScheme::Forest => Color32::from_rgb(0x86, 0xEF, 0xAC),
            ColorScheme::Violet => Color32::from_rgb(0xC4, 0xB5, 0xFD),
            ColorScheme::Ember => Color32::from_rgb(0xFD, 0xBA, 0x74),
        };
    }
    match scheme {
        ColorScheme::Slate => Color32::from_rgb(0x47, 0x55, 0x69),
        ColorScheme::Ocean => Color32::from_rgb(0x1D, 0x4E, 0xD8),
        ColorScheme::Forest => Color32::from_rgb(0x16, 0x65, 0x34),
        ColorScheme::Violet => Color32::from_rgb(0x6D, 0x28, 0xD9),
        ColorScheme::Ember => Color32::from_rgb(0x92, 0x40, 0x0E),
    }
}

fn light_palette(accent: Color32, accent_text: Color32) -> UiPalette {
    UiPalette {
        dark: false,
        bg: Color32::from_rgb(0xF4, 0xF4, 0xF5),
        surface: Color32::from_rgb(0xFF, 0xFF, 0xFF),
        surface_alt: Color32::from_rgb(0xFA, 0xFA, 0xFA),
        surface_hover: Color32::from_rgb(0xFF, 0xFF, 0xFF),
        surface_active: Color32::from_rgb(0xF1, 0xF5, 0xF9),
        border: Color32::from_rgb(0xE4, 0xE4, 0xE7),
        text: Color32::from_rgb(0x18, 0x18, 0x1B),
        text_muted: Color32::from_rgb(0x71, 0x71, 0x7A),
        text_hover: accent_text,
        accent,
        accent_text,
        accent_soft: accent.gamma_multiply(0.12),
        shadow: Color32::from_rgba_premultiplied(15, 23, 42, 28),
    }
}

fn dark_palette(accent: Color32, accent_text: Color32) -> UiPalette {
    UiPalette {
        dark: true,
        bg: Color32::from_rgb(0x09, 0x09, 0x0B),
        surface: Color32::from_rgb(0x18, 0x18, 0x1B),
        surface_alt: Color32::from_rgb(0x12, 0x12, 0x14),
        surface_hover: Color32::from_rgb(0x27, 0x27, 0x2A),
        surface_active: Color32::from_rgb(0x3F, 0x3F, 0x46),
        border: Color32::from_rgb(0x3F, 0x3F, 0x46),
        text: Color32::from_rgb(0xFA, 0xFA, 0xFA),
        text_muted: Color32::from_rgb(0xA1, 0xA1, 0xAA),
        text_hover: accent_text,
        accent,
        accent_text,
        accent_soft: accent.gamma_multiply(0.22),
        shadow: Color32::from_rgba_premultiplied(0, 0, 0, 80),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accent_text_has_body_text_contrast_on_surfaces() {
        for scheme in ColorScheme::ALL {
            for dark in [false, true] {
                let p = for_scheme(scheme, dark);
                assert!(
                    contrast_ratio(p.accent_text, p.surface) >= 4.5,
                    "{scheme:?} dark={dark} accent_text {:?} surface {:?}",
                    p.accent_text,
                    p.surface
                );
                assert!(
                    contrast_ratio(p.accent_text, p.bg) >= 4.5,
                    "{scheme:?} dark={dark} accent_text {:?} bg {:?}",
                    p.accent_text,
                    p.bg
                );
            }
        }
    }

    fn contrast_ratio(a: Color32, b: Color32) -> f32 {
        let (lighter, darker) = {
            let a = relative_luminance(a);
            let b = relative_luminance(b);
            if a > b { (a, b) } else { (b, a) }
        };
        (lighter + 0.05) / (darker + 0.05)
    }

    fn relative_luminance(c: Color32) -> f32 {
        fn channel(v: u8) -> f32 {
            let v = f32::from(v) / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
    }
}
