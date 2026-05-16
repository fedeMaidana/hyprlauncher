use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use fontdue::{Font, FontSettings};

const FONT_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/TTF/JetBrainsMono-Regular.ttf",
    "/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Regular.ttf",
    "/usr/share/fonts/TTF/FiraCode-Regular.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
];

pub fn load_ui_font() -> Result<Font> {
    for candidate in FONT_CANDIDATES {
        let path = Path::new(candidate);
        if !path.exists() {
            continue;
        }

        let bytes = fs::read(path).with_context(|| format!("no se pudo leer {}", path.display()))?;
        return Font::from_bytes(bytes, FontSettings::default())
            .map_err(|err| anyhow::anyhow!("no se pudo cargar fuente {}: {err}", path.display()));
    }

    bail!("no encontré una fuente UI usable; instalá JetBrains Mono, Fira Code o DejaVu Sans")
}
