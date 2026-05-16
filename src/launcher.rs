use crate::desktop::DesktopEntry;

#[derive(Debug, Clone)]
pub struct Launcher {
    entries: Vec<DesktopEntry>,
    query: String,
    selected: usize,
    hovered: Option<usize>,
    max_results: usize,
}

impl Launcher {
    pub fn new(entries: Vec<DesktopEntry>, max_results: usize) -> Self {
        Self {
            entries,
            query: String::new(),
            selected: 0,
            hovered: None,
            max_results: max_results.max(1),
        }
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn hovered(&self) -> Option<usize> {
        self.hovered
    }

    pub fn selected(&self) -> usize {
        self.selected.min(self.visible_count().saturating_sub(1))
    }

    pub fn visible_count(&self) -> usize {
        self.visible_entries().len()
    }

    pub fn visible_entries(&self) -> Vec<&DesktopEntry> {
        let mut ranked: Vec<_> = self
            .entries
            .iter()
            .filter_map(|entry| entry.rank(&self.query).map(|rank| (rank, entry)))
            .collect();

        ranked.sort_by(|(rank_a, a), (rank_b, b)| {
            rank_a
                .cmp(rank_b)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        ranked
            .into_iter()
            .take(self.max_results)
            .map(|(_, entry)| entry)
            .collect()
    }

    pub fn push_char(&mut self, ch: char) -> bool {
        if ch.is_control() {
            return false;
        }
        self.query.push(ch);
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
        let count = self.visible_count();
        if count == 0 {
            return false;
        }
        let before = self.selected;
        self.selected = (self.selected + 1) % count;
        before != self.selected
    }

    pub fn select_prev(&mut self) -> bool {
        let count = self.visible_count();
        if count == 0 {
            return false;
        }
        let before = self.selected;
        self.selected = if self.selected == 0 {
            count - 1
        } else {
            self.selected - 1
        };
        before != self.selected
    }

    pub fn select_index(&mut self, index: usize) -> bool {
        if index >= self.visible_count() {
            return false;
        }
        let before = self.selected;
        self.selected = index;
        before != self.selected
    }

    pub fn hover_index(&mut self, index: Option<usize>) -> bool {
        let normalized = index.filter(|idx| *idx < self.visible_count());
        let changed = self.hovered != normalized;
        self.hovered = normalized;
        changed
    }

    pub fn selected_entry(&self) -> Option<DesktopEntry> {
        self.visible_entries()
            .get(self.selected())
            .map(|entry| DesktopEntry::clone(*entry))
    }

    fn reset_selection(&mut self) {
        self.selected = 0;
        self.hovered = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn app(name: &str) -> DesktopEntry {
        DesktopEntry {
            id: format!("{name}.desktop"),
            name: name.to_owned(),
            generic_name: None,
            comment: None,
            exec: name.to_lowercase(),
            icon: None,
            source: PathBuf::from(format!("{name}.desktop")),
        }
    }

    #[test]
    fn filters_by_query_and_resets_selection() {
        let mut launcher = Launcher::new(vec![app("Firefox"), app("Files")], 8);

        assert!(launcher.push_char('f'));
        assert_eq!(launcher.visible_entries().len(), 2);
        assert!(launcher.push_char('i'));
        assert_eq!(launcher.visible_entries()[0].name, "Files");
        assert_eq!(launcher.selected(), 0);
    }
}
