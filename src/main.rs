use anyhow::{Context, Result};

mod app;
mod cli;
mod config;
mod desktop;
mod font;
mod geometry;
mod launcher;
mod layout;
mod model;
mod render;
mod style;
mod theme;
mod wallpaper_preview;

use cli::CliAction;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    match cli::action_from_args() {
        CliAction::Run(config) => app::AppState::run(config),
        CliAction::WarmIconCache => {
            let entries = desktop::scan_desktop_entries().context("no se pudieron leer aplicaciones .desktop")?;

            desktop::warm_icon_cache(&entries);

            Ok(())
        }
    }
}
