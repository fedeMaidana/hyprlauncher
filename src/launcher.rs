use crate::desktop::DesktopEntry;
use crate::usage::UsageStats;

/// Boost cap below the rank tier gap (10): frequent use reorders results
/// within a tier but never lets a weaker match beat an exact/prefix one.
const USAGE_BOOST_CAP: u32 = 9;

/// Una fila de la lista: separador de letra del abecedario, o una app.
#[derive(Debug, Clone)]
pub enum ListItem<'a> {
    Header(char),
    Entry { index: usize, entry: &'a DesktopEntry },
}

/// Letra de agrupación: la inicial alfabética en mayúscula, o `#` para
/// nombres que arrancan con números u otros símbolos.
fn group_letter(name: &str) -> char {
    match name.chars().find(|ch| ch.is_alphanumeric()) {
        Some(ch) if ch.is_alphabetic() => ch.to_uppercase().next().unwrap_or(ch),
        _ => '#',
    }
}

#[derive(Debug, Clone)]
pub struct Launcher {
    entries: Vec<DesktopEntry>,
    query: String,
    selected: usize,
    hovered: Option<usize>,
    max_results: usize,
    scroll_offset: usize,
    usage: UsageStats,
}

impl Launcher {
    pub fn new(entries: Vec<DesktopEntry>, max_results: usize, usage: UsageStats) -> Self {
        Self {
            entries,
            query: String::new(),
            selected: 0,
            hovered: None,
            max_results: max_results.max(1),
            scroll_offset: 0,
            usage,
        }
    }

    /// Bumps and persists the launch counter for frecency ranking.
    pub fn record_launch(&mut self, id: &str) -> anyhow::Result<()> {
        self.usage.bump(id);
        self.usage.save()
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn hovered(&self) -> Option<usize> {
        self.hovered
    }

    pub fn selected(&self) -> usize {
        self.selected.min(self.result_count().saturating_sub(1))
    }

    pub fn result_count(&self) -> usize {
        self.ranked_entries().len()
    }

    pub fn visible_entries(&self) -> Vec<&DesktopEntry> {
        self.ranked_entries().into_iter().map(|(_, entry)| entry).collect()
    }

    /// Sin búsqueda activa, la lista completa agrupada por inicial con
    /// separadores de letra; buscando, el ranking plano de siempre.
    pub fn list_items(&self) -> Vec<ListItem<'_>> {
        let ranked = self.ranked_entries();

        if !self.browse_mode() {
            return ranked
                .into_iter()
                .enumerate()
                .map(|(index, (_, entry))| ListItem::Entry { index, entry })
                .collect();
        }

        let mut items = Vec::with_capacity(ranked.len());
        let mut current: Option<char> = None;

        for (index, (_, entry)) in ranked.into_iter().enumerate() {
            let letter = group_letter(&entry.name);

            if current != Some(letter) {
                items.push(ListItem::Header(letter));
                current = Some(letter);
            }

            items.push(ListItem::Entry { index, entry });
        }

        items
    }

    pub fn window_items(&self, window_size: usize) -> Vec<ListItem<'_>> {
        let items = self.list_items();
        let count = items.len();

        if count == 0 || window_size == 0 {
            return Vec::new();
        }

        let start = self.scroll_offset.min(count.saturating_sub(1));
        let start = start.min(count.saturating_sub(window_size));

        items.into_iter().skip(start).take(window_size).collect()
    }

    pub fn window_len(&self, window_size: usize) -> usize {
        self.window_items(window_size).len()
    }

    /// Índice de app (para selección) en la fila `row` de la ventana visible;
    /// `None` si esa fila es un separador de letra.
    pub fn entry_index_at_row(&self, row: usize, window_size: usize) -> Option<usize> {
        match self.window_items(window_size).get(row) {
            Some(ListItem::Entry { index, .. }) => Some(*index),
            _ => None,
        }
    }

    pub fn ensure_selected_visible(&mut self, window_size: usize) -> bool {
        let selected = self.selected();

        // Leer todo lo que depende de `items` antes de mutar el offset.
        let (count, item_pos, header_above) = {
            let items = self.list_items();

            let item_pos = items
                .iter()
                .position(|item| matches!(item, ListItem::Entry { index, .. } if *index == selected))
                .unwrap_or(0);

            let header_above = item_pos > 0 && matches!(items[item_pos - 1], ListItem::Header(_));

            (items.len(), item_pos, header_above)
        };

        if count == 0 || window_size == 0 {
            let changed = self.scroll_offset != 0;
            self.scroll_offset = 0;
            return changed;
        }

        let max_offset = count.saturating_sub(window_size);
        let before = self.scroll_offset;

        if item_pos < self.scroll_offset {
            // Si justo arriba está el separador del grupo, que también se vea.
            self.scroll_offset = if header_above { item_pos - 1 } else { item_pos };
        } else if item_pos >= self.scroll_offset + window_size {
            self.scroll_offset = item_pos + 1 - window_size;
        }

        self.scroll_offset = self.scroll_offset.min(max_offset);
        before != self.scroll_offset
    }

    pub fn push_char(&mut self, ch: char) -> bool {
        if ch.is_control() {
            return false;
        }

        self.query.push(ch);
        self.reset_selection();
        true
    }

    pub fn clear_query(&mut self) -> bool {
        if self.query.is_empty() {
            return false;
        }

        self.query.clear();
        self.reset_selection();
        true
    }

    pub fn backspace(&mut self) -> bool {
        let changed = self.query.pop().is_some();

        if changed {
            self.reset_selection();
        }

        changed
    }

    pub fn select_next(&mut self) -> bool {
        let count = self.result_count();

        if count == 0 {
            return false;
        }

        let before = self.selected;
        self.selected = (self.selected + 1) % count;
        before != self.selected
    }

    pub fn select_prev(&mut self) -> bool {
        let count = self.result_count();

        if count == 0 {
            return false;
        }

        let before = self.selected;

        self.selected = if self.selected == 0 { count - 1 } else { self.selected - 1 };

        before != self.selected
    }

    pub fn select_index(&mut self, index: usize) -> bool {
        if index >= self.result_count() {
            return false;
        }

        let before = self.selected;
        self.selected = index;
        before != self.selected
    }

    pub fn hover_index(&mut self, index: Option<usize>) -> bool {
        let normalized = index.filter(|idx| *idx < self.result_count());
        let changed = self.hovered != normalized;

        self.hovered = normalized;
        changed
    }

    pub fn selected_entry(&self) -> Option<DesktopEntry> {
        self.visible_entries().get(self.selected()).map(|entry| DesktopEntry::clone(*entry))
    }

    /// Las `limit` apps con más lanzamientos registrados; solo califican las
    /// que se usaron al menos una vez.
    pub fn pinned_entries(&self, limit: usize) -> Vec<&DesktopEntry> {
        let mut used: Vec<(u32, &DesktopEntry)> = self
            .entries
            .iter()
            .filter_map(|entry| {
                let count = self.usage.count(&entry.id);
                (count > 0).then_some((count, entry))
            })
            .collect();

        used.sort_by(|(count_a, a), (count_b, b)| count_b.cmp(count_a).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        used.truncate(limit);

        used.into_iter().map(|(_, entry)| entry).collect()
    }

    pub fn pinned_count(&self, limit: usize) -> usize {
        self.pinned_entries(limit).len()
    }

    pub fn pinned_entry(&self, index: usize, limit: usize) -> Option<DesktopEntry> {
        self.pinned_entries(limit).get(index).map(|entry| DesktopEntry::clone(*entry))
    }

    /// Navegando (sin query) el abecedario manda: orden alfabético puro, sin
    /// boost de uso. El boost solo reordena resultados de búsqueda.
    fn browse_mode(&self) -> bool {
        self.query.trim().is_empty()
    }

    fn ranked_entries(&self) -> Vec<(i32, &DesktopEntry)> {
        let browse = self.browse_mode();

        let mut ranked: Vec<_> = self
            .entries
            .iter()
            .filter_map(|entry| {
                entry.rank(&self.query).map(|rank| {
                    let boost = if browse { 0 } else { self.usage_boost(entry) };
                    (rank - boost, entry)
                })
            })
            .collect();

        ranked.sort_by(|(rank_a, a), (rank_b, b)| rank_a.cmp(rank_b).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));

        let limit = self.max_results.min(ranked.len());
        ranked.truncate(limit);

        ranked
    }

    fn usage_boost(&self, entry: &DesktopEntry) -> i32 {
        self.usage.count(&entry.id).min(USAGE_BOOST_CAP) as i32
    }

    fn reset_selection(&mut self) {
        self.selected = 0;
        self.hovered = None;
        self.scroll_offset = 0;
    }
}

/// Char range of `query` inside `name` (case-insensitive), for highlighting.
pub fn name_match_range(name: &str, query: &str) -> Option<(usize, usize)> {
    let query = query.trim();

    if query.is_empty() {
        return None;
    }

    let name_chars: Vec<char> = name.chars().map(lower_char).collect();
    let query_chars: Vec<char> = query.chars().map(lower_char).collect();

    if query_chars.len() > name_chars.len() {
        return None;
    }

    (0..=name_chars.len() - query_chars.len())
        .find(|&start| name_chars[start..start + query_chars.len()] == query_chars[..])
        .map(|start| (start, start + query_chars.len()))
}

fn lower_char(ch: char) -> char {
    ch.to_lowercase().next().unwrap_or(ch)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn entry(id: &str, name: &str) -> DesktopEntry {
        DesktopEntry {
            id: id.to_owned(),
            name: name.to_owned(),
            generic_name: None,
            comment: None,
            exec: String::new(),
            icon: None,
            source: PathBuf::new(),
        }
    }

    #[test]
    fn pinned_entries_ranks_by_usage_and_truncates() {
        let mut usage = UsageStats::default();

        for _ in 0..5 {
            usage.bump("zed.desktop");
        }
        for _ in 0..3 {
            usage.bump("spotify.desktop");
        }
        usage.bump("discord.desktop");
        usage.bump("dolphin.desktop");
        usage.bump("dolphin.desktop");

        let launcher = Launcher::new(
            vec![
                entry("claude.desktop", "Claude"),
                entry("discord.desktop", "Discord"),
                entry("dolphin.desktop", "Dolphin"),
                entry("spotify.desktop", "Spotify"),
                entry("zed.desktop", "Zed"),
            ],
            10,
            usage,
        );

        let pinned: Vec<&str> = launcher.pinned_entries(3).into_iter().map(|entry| entry.id.as_str()).collect();

        assert_eq!(pinned, vec!["zed.desktop", "spotify.desktop", "dolphin.desktop"]);
        assert_eq!(launcher.pinned_count(3), 3);
        assert_eq!(launcher.pinned_entry(0, 3).map(|entry| entry.id), Some("zed.desktop".to_owned()));
    }

    #[test]
    fn list_items_groups_by_initial_when_browsing() {
        let mut usage = UsageStats::default();

        // El boost de uso no debe romper el orden alfabético al navegar.
        for _ in 0..5 {
            usage.bump("blender.desktop");
        }

        let launcher = Launcher::new(
            vec![
                entry("alacritty.desktop", "Alacritty"),
                entry("blender.desktop", "Blender"),
                entry("onepassword.desktop", "1Password"),
                entry("amberol.desktop", "Amberol"),
            ],
            10,
            usage,
        );

        let repr: Vec<String> = launcher
            .list_items()
            .iter()
            .map(|item| match item {
                ListItem::Header(letter) => format!("H:{letter}"),
                ListItem::Entry { entry, .. } => entry.name.clone(),
            })
            .collect();

        assert_eq!(repr, vec!["H:#", "1Password", "H:A", "Alacritty", "Amberol", "H:B", "Blender"]);
    }

    #[test]
    fn list_items_stays_flat_while_searching() {
        let mut launcher = Launcher::new(
            vec![entry("alacritty.desktop", "Alacritty"), entry("blender.desktop", "Blender")],
            10,
            UsageStats::default(),
        );

        launcher.push_char('b');

        let items = launcher.list_items();

        assert_eq!(items.len(), 1);
        assert!(items.iter().all(|item| matches!(item, ListItem::Entry { .. })));
    }

    #[test]
    fn entry_index_at_row_skips_headers() {
        let launcher = Launcher::new(
            vec![entry("alacritty.desktop", "Alacritty"), entry("blender.desktop", "Blender")],
            10,
            UsageStats::default(),
        );

        // Ventana: H:A, Alacritty, H:B, Blender.
        assert_eq!(launcher.entry_index_at_row(0, 4), None);
        assert_eq!(launcher.entry_index_at_row(1, 4), Some(0));
        assert_eq!(launcher.entry_index_at_row(2, 4), None);
        assert_eq!(launcher.entry_index_at_row(3, 4), Some(1));
    }

    #[test]
    fn pinned_entries_hides_unused_apps() {
        let launcher = Launcher::new(vec![entry("claude.desktop", "Claude")], 10, UsageStats::default());

        assert!(launcher.pinned_entries(3).is_empty());
        assert_eq!(launcher.pinned_count(3), 0);
        assert!(launcher.pinned_entry(0, 3).is_none());
    }
}
