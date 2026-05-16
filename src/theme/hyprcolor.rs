use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::Deserialize;

use super::{Color, Theme};

#[derive(Debug, Deserialize)]
struct HyprcolorPalette {
    wallpaper: Option<String>,
    background: String,
    foreground: String,
    surface: String,
    surface_variant: String,
    accent: String,
    accent_1: String,
    accent_2: String,
    accent_3: String,
}

pub fn load(path: &Path) -> Result<Theme> {
    let content = fs::read_to_string(path).with_context(|| format!("no se pudo leer {}", path.display()))?;
    let palette: HyprcolorPalette =
        serde_json::from_str(&content).with_context(|| format!("{} no tiene formato hyprcolor válido", path.display()))?;

    let background = parse("background", &palette.background)?;
    let foreground = parse("foreground", &palette.foreground)?;
    let surface = parse("surface", &palette.surface)?.with_alpha(212);
    let surface_variant = parse("surface_variant", &palette.surface_variant)?.with_alpha(228);
    let accent = parse("accent", &palette.accent)?;
    let accent_1 = parse("accent_1", &palette.accent_1)?;
    let accent_2 = parse("accent_2", &palette.accent_2)?;
    let accent_3 = parse("accent_3", &palette.accent_3)?;

    Ok(Theme {
        background,
        foreground,
        muted: foreground.with_alpha(160),
        panel: background.with_alpha(205),
        panel_border: accent_2.with_alpha(72),
        surface,
        surface_variant,
        accent,
        accent_soft: accent_1.with_alpha(190),
        danger: accent_3.with_alpha(235),
        wallpaper: palette.wallpaper.map(Into::into),
    })
}

fn parse(name: &str, value: &str) -> Result<Color> {
    Color::from_hex(value).with_context(|| format!("color inválido en hyprcolor: {name}={value}"))
}
