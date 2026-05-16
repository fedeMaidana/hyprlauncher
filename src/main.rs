use anyhow::Result;

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

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = cli::config_from_args();
    app::AppState::run(config)
}
