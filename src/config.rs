use std::{env, path::PathBuf};

use crate::style;

#[derive(Debug, Clone)]
pub struct Config {
    pub width: u32,
    pub height: u32,
    pub max_results: usize,
    pub hyprcolor_json: PathBuf,
}

impl Config {
    pub fn load() -> Self {
        Self {
            width: style::surface::WIDTH_HINT,
            height: style::surface::HEIGHT_HINT,
            max_results: usize::MAX,
            hyprcolor_json: default_hyprcolor_json(),
        }
    }
}

fn default_hyprcolor_json() -> PathBuf {
    if let Some(cache_home) = env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(cache_home).join("hyprcolors").join("colors.json");
    }

    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cache")
        .join("hyprcolors")
        .join("colors.json")
}
