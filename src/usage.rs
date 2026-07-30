//! Frecency: persisted launch counters so frequent apps rank first.
//!
//! Plain-text state at `~/.local/state/hyprlauncher/usage`, one `count\tid`
//! per line.

use std::{collections::HashMap, env, fs, path::PathBuf};

use anyhow::{Context, Result};

#[derive(Debug, Clone, Default)]
pub struct UsageStats {
    counts: HashMap<String, u32>,
}

impl UsageStats {
    pub fn load() -> Self {
        let counts = state_file_path()
            .and_then(|path| fs::read_to_string(path).ok())
            .map(|content| parse(&content))
            .unwrap_or_default();

        Self { counts }
    }

    pub fn count(&self, id: &str) -> u32 {
        self.counts.get(id).copied().unwrap_or(0)
    }

    pub fn bump(&mut self, id: &str) {
        *self.counts.entry(id.to_owned()).or_insert(0) += 1;
    }

    pub fn save(&self) -> Result<()> {
        let path = state_file_path().context("sin directorio de estado para el uso")?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creando {}", parent.display()))?;
        }

        let mut lines: Vec<String> = self.counts.iter().map(|(id, count)| format!("{count}\t{id}")).collect();
        lines.sort();

        fs::write(&path, lines.join("\n")).with_context(|| format!("escribiendo {}", path.display()))
    }
}

fn parse(content: &str) -> HashMap<String, u32> {
    content
        .lines()
        .filter_map(|line| {
            let (count, id) = line.split_once('\t')?;
            Some((id.to_owned(), count.trim().parse().ok()?))
        })
        .collect()
}

fn state_file_path() -> Option<PathBuf> {
    if let Some(state_home) = env::var_os("XDG_STATE_HOME") {
        return Some(PathBuf::from(state_home).join("hyprlauncher").join("usage"));
    }

    env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("state").join("hyprlauncher").join("usage"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_counts_usage_lines() {
        let counts = parse("3\tfirefox.desktop\n12\tcode.desktop\nbasura\n");

        assert_eq!(counts.get("firefox.desktop"), Some(&3));
        assert_eq!(counts.get("code.desktop"), Some(&12));
        assert_eq!(counts.len(), 2);
    }

    #[test]
    fn bump_increments_from_zero() {
        let mut stats = UsageStats {
            counts: HashMap::new(),
        };

        assert_eq!(stats.count("kitty.desktop"), 0);
        stats.bump("kitty.desktop");
        stats.bump("kitty.desktop");
        assert_eq!(stats.count("kitty.desktop"), 2);
    }
}
