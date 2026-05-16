mod color;
mod hyprcolor;

pub use color::Color;

use std::path::PathBuf;

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub muted: Color,
    pub panel: Color,
    pub panel_border: Color,
    pub surface: Color,
    pub surface_variant: Color,
    pub accent: Color,
    pub accent_soft: Color,
    pub danger: Color,
    pub wallpaper: Option<PathBuf>,
}

impl Theme {
    pub fn load(config: &Config) -> Self {
        hyprcolor::load(&config.hyprcolor_json).unwrap_or_else(|err| {
            log::warn!("no se pudo cargar paleta hyprcolor: {err:#}");
            Self::fallback()
        })
    }

    pub fn fallback() -> Self {
        Self {
            background: Color::from_rgb(12, 20, 26),
            foreground: Color::from_rgb(224, 232, 238),
            muted: Color::from_rgb(148, 165, 176),
            panel: Color::from_rgba(18, 32, 40, 210),
            panel_border: Color::from_rgba(170, 215, 225, 60),
            surface: Color::from_rgba(34, 51, 61, 205),
            surface_variant: Color::from_rgba(54, 75, 86, 210),
            accent: Color::from_rgb(223, 171, 154),
            accent_soft: Color::from_rgba(223, 171, 154, 185),
            danger: Color::from_rgb(255, 119, 119),
            wallpaper: None,
        }
    }
}
