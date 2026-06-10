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
    pub border: Color32,
    pub control_border: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub text_hover: Color32,
    pub accent_text: Color32,
    pub accent_soft: Color32,
    pub accent_active: Color32,
    pub shadow: Color32,
}

pub fn for_scheme(scheme: ColorScheme, dark: bool) -> UiPalette {
    let accent = accent_for_scheme(scheme, dark);
    let accent_text = accent_text_for_scheme(scheme, dark);
    if dark {
        dark_palette(accent, accent_text)
    } else {
        light_palette(accent, accent_text)
    }
}

pub fn accent_for_scheme(scheme: ColorScheme, dark: bool) -> Color32 {
    match scheme {
        ColorScheme::Slate => {
            if dark {
                Color32::from_rgb(0x94, 0xA3, 0xB8)
            } else {
                Color32::from_rgb(0x47, 0x55, 0x69)
            }
        }
        ColorScheme::Ocean => {
            if dark {
                Color32::from_rgb(0x60, 0xA5, 0xFA)
            } else {
                Color32::from_rgb(0x3B, 0x82, 0xF6)
            }
        }
        ColorScheme::Forest => {
            if dark {
                Color32::from_rgb(0x34, 0xD3, 0x99)
            } else {
                Color32::from_rgb(0x22, 0xC5, 0x5E)
            }
        }
        ColorScheme::Violet => {
            if dark {
                Color32::from_rgb(0xA7, 0x8B, 0xFA)
            } else {
                Color32::from_rgb(0x8B, 0x5C, 0xF6)
            }
        }
        ColorScheme::Ember => {
            if dark {
                Color32::from_rgb(0xF5, 0xB8, 0x49)
            } else {
                Color32::from_rgb(0xF5, 0x9E, 0x0B)
            }
        }
    }
}

pub fn accent_text_for_scheme(scheme: ColorScheme, dark: bool) -> Color32 {
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
    let surface = Color32::from_rgb(0xFF, 0xFF, 0xFF);
    UiPalette {
        dark: false,
        bg: Color32::from_rgb(0xF8, 0xFA, 0xFC),
        surface,
        surface_alt: Color32::from_rgb(0xF1, 0xF5, 0xF9),
        surface_hover: Color32::from_rgb(0xF8, 0xFA, 0xFC),
        border: Color32::from_rgb(0xE2, 0xE8, 0xF0),
        control_border: Color32::from_rgb(0xCB, 0xD5, 0xE1),
        text: Color32::from_rgb(0x0F, 0x17, 0x2A),
        text_muted: Color32::from_rgb(0x64, 0x74, 0x8B),
        text_hover: accent_text,
        accent_text,
        accent_soft: mix_color(surface, accent, 0.10),
        accent_active: mix_color(surface, accent, 0.16),
        shadow: Color32::from_rgba_premultiplied(15, 23, 42, 28),
    }
}

fn dark_palette(accent: Color32, accent_text: Color32) -> UiPalette {
    let surface = Color32::from_rgb(0x11, 0x18, 0x27);
    UiPalette {
        dark: true,
        bg: Color32::from_rgb(0x0B, 0x12, 0x20),
        surface,
        surface_alt: Color32::from_rgb(0x17, 0x24, 0x35),
        surface_hover: Color32::from_rgb(0x20, 0x2C, 0x3F),
        border: Color32::from_rgb(0x2A, 0x3A, 0x52),
        control_border: Color32::from_rgb(0x52, 0x63, 0x7A),
        text: Color32::from_rgb(0xF8, 0xFA, 0xFC),
        text_muted: Color32::from_rgb(0xA8, 0xB3, 0xC7),
        text_hover: accent_text,
        accent_text,
        accent_soft: mix_color(surface, accent, 0.18),
        accent_active: mix_color(surface, accent, 0.26),
        shadow: Color32::from_rgba_premultiplied(0, 0, 0, 96),
    }
}

fn mix_color(base: Color32, tint: Color32, amount: f32) -> Color32 {
    fn mix_channel(a: u8, b: u8, amount: f32) -> u8 {
        (f32::from(a) + (f32::from(b) - f32::from(a)) * amount).round() as u8
    }

    Color32::from_rgb(
        mix_channel(base.r(), tint.r(), amount),
        mix_channel(base.g(), tint.g(), amount),
        mix_channel(base.b(), tint.b(), amount),
    )
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
