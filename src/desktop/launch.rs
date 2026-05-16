use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::desktop::DesktopEntry;

pub fn launch_entry(entry: &DesktopEntry) -> Result<()> {
    let command_line = strip_desktop_field_codes(&entry.exec);
    let parts = shlex::split(&command_line)
        .with_context(|| format!("no pude interpretar Exec de {}: {}", entry.source.display(), entry.exec))?;

    let Some((program, args)) = parts.split_first() else {
        bail!("Exec vacío en {}", entry.source.display());
    };

    log::info!("launching {}", entry.name);
    Command::new(program)
        .args(args)
        .spawn()
        .with_context(|| format!("no se pudo ejecutar {}", entry.name))?;

    Ok(())
}

fn strip_desktop_field_codes(exec: &str) -> String {
    let mut out = String::with_capacity(exec.len());
    let mut chars = exec.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            match chars.peek().copied() {
                Some('%') => {
                    chars.next();
                    out.push('%');
                }
                Some(code) if is_field_code(code) => {
                    chars.next();
                }
                _ => out.push(ch),
            }
        } else {
            out.push(ch);
        }
    }

    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_field_code(code: char) -> bool {
    matches!(code, 'f' | 'F' | 'u' | 'U' | 'i' | 'c' | 'k' | 'v' | 'm' | 'd' | 'D' | 'n' | 'N')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_desktop_field_codes() {
        assert_eq!(strip_desktop_field_codes("firefox %u"), "firefox");
        assert_eq!(strip_desktop_field_codes("app --name %c %%"), "app --name %");
    }
}
