use std::{collections::HashMap, path::Path};

use crate::desktop::DesktopEntry;

pub fn parse_desktop_entry(path: &Path, content: &str) -> Option<DesktopEntry> {
    let values = parse_desktop_values(content);

    if values.get("Type").map(String::as_str) != Some("Application") {
        return None;
    }
    if is_true(values.get("NoDisplay")) || is_true(values.get("Hidden")) {
        return None;
    }

    let name = values.get("Name")?.trim().to_owned();
    let exec = values.get("Exec")?.trim().to_owned();
    if name.is_empty() || exec.is_empty() {
        return None;
    }

    Some(DesktopEntry {
        id: path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("application.desktop")
            .to_owned(),
        name,
        generic_name: values.get("GenericName").cloned().filter(|v| !v.is_empty()),
        comment: values.get("Comment").cloned().filter(|v| !v.is_empty()),
        exec,
        icon: values.get("Icon").cloned().filter(|v| !v.is_empty()),
        source: path.to_owned(),
    })
}

fn parse_desktop_values(content: &str) -> HashMap<String, String> {
    let mut in_desktop_entry = false;
    let mut values = HashMap::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }

        if !in_desktop_entry {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        if key.contains('[') {
            continue;
        }

        values.insert(key.to_owned(), unescape(value));
    }

    values
}

fn unescape(value: &str) -> String {
    value
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r")
        .replace("\\\\", "\\")
}

fn is_true(value: Option<&String>) -> bool {
    value.map(|v| v.eq_ignore_ascii_case("true")).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_visible_application() {
        let content = r#"
[Desktop Entry]
Type=Application
Name=Ark
GenericName=Archive Manager
Comment=Work with archives
Exec=ark %U
Icon=ark
"#;

        let entry = parse_desktop_entry(Path::new("ark.desktop"), content).unwrap();

        assert_eq!(entry.name, "Ark");
        assert_eq!(entry.exec, "ark %U");
        assert_eq!(entry.icon.as_deref(), Some("ark"));
    }

    #[test]
    fn skips_hidden_entries() {
        let content = r#"
[Desktop Entry]
Type=Application
Name=Hidden App
Exec=hidden
NoDisplay=true
"#;

        assert!(parse_desktop_entry(Path::new("hidden.desktop"), content).is_none());
    }
}
