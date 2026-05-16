use std::env;

use crate::config::Config;

pub fn config_from_args() -> Config {
    let mut config = Config::load();
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--max-results" => {
                if let Some(value) = args.next().and_then(|v| v.parse::<usize>().ok()) {
                    config.max_results = value.max(1);
                }
            }
            "--width" => {
                if let Some(value) = args.next().and_then(|v| v.parse::<u32>().ok()) {
                    config.width = value.max(360);
                }
            }
            "--height" => {
                if let Some(value) = args.next().and_then(|v| v.parse::<u32>().ok()) {
                    config.height = value.max(240);
                }
            }
            "--hyprcolor" => {
                if let Some(value) = args.next() {
                    config.hyprcolor_json = value.into();
                }
            }
            _ => {}
        }
    }

    config
}
