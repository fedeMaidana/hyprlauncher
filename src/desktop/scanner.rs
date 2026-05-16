use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use super::{DesktopEntry, parser::parse_desktop_entry};

pub fn scan_desktop_entries() -> Result<Vec<DesktopEntry>> {
    let mut by_id = HashMap::<String, DesktopEntry>::new();

    for dir in application_dirs() {
        scan_dir(&dir, &mut by_id).with_context(|| format!("leyendo {}", dir.display()))?;
    }

    let mut entries: Vec<_> = by_id.into_values().collect();
    entries.sort_by_key(|entry| entry.name.to_lowercase());

    Ok(entries)
}

fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(data_home) = env::var_os("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(data_home).join("applications"));
    } else if let Some(home) = env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }

    let data_dirs = env::var_os("XDG_DATA_DIRS")
        .and_then(|v| v.into_string().ok())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_owned());

    dirs.extend(data_dirs.split(':').map(|base| PathBuf::from(base).join("applications")));
    dirs
}

fn scan_dir(dir: &Path, by_id: &mut HashMap<String, DesktopEntry>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            scan_dir(&path, by_id)?;
            continue;
        }

        if path.extension().and_then(|v| v.to_str()) != Some("desktop") {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) => {
                log::debug!("no se pudo leer {}: {err:#}", path.display());
                continue;
            }
        };

        if let Some(entry) = parse_desktop_entry(&path, &content) {
            by_id.entry(entry.id.clone()).or_insert(entry);
        }
    }

    Ok(())
}
