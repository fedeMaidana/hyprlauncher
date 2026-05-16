use crate::desktop::DesktopEntry;

#[derive(Debug, Clone)]
pub struct Launcher {
    entries: Vec<DesktopEntry>,
    query: String,
    selected: usize,
    hovered: Option<usize>,
    max_results: usize,
    scroll_offset: usize,
}

impl Launcher {
    pub fn new(entries: Vec<DesktopEntry>, max_results: usize) -> Self {
        Self {
            entries,
            query: String::new(),
            selected: 0,
            hovered: None,
            max_results: max_results.max(1),
            scroll_offset: 0,
        }
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

    pub fn window_entries(&self, window_size: usize) -> Vec<(usize, &DesktopEntry)> {
        let ranked = self.ranked_entries();
        let count = ranked.len();

        if count == 0 || window_size == 0 {
            return Vec::new();
        }

        let start = self.scroll_offset.min(count.saturating_sub(1));
        let start = start.min(count.saturating_sub(window_size));

        ranked
            .into_iter()
            .enumerate()
            .skip(start)
            .take(window_size)
            .map(|(index, (_, entry))| (index, entry))
            .collect()
    }

    pub fn window_count(&self, window_size: usize) -> usize {
        self.result_count().min(window_size)
    }

    pub fn index_for_window_row(&self, row: usize, window_size: usize) -> Option<usize> {
        if row >= self.window_count(window_size) {
            return None;
        }

        Some(self.scroll_offset + row)
    }

    pub fn ensure_selected_visible(&mut self, window_size: usize) -> bool {
        let count = self.result_count();

        if count == 0 || window_size == 0 {
            let changed = self.scroll_offset != 0;
            self.scroll_offset = 0;
            return changed;
        }

        let max_offset = count.saturating_sub(window_size);
        let before = self.scroll_offset;
        let selected = self.selected();

        if selected < self.scroll_offset {
            self.scroll_offset = selected;
        } else if selected >= self.scroll_offset + window_size {
            self.scroll_offset = selected + 1 - window_size;
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

    fn ranked_entries(&self) -> Vec<(i32, &DesktopEntry)> {
        let mut ranked: Vec<_> = self
            .entries
            .iter()
            .filter_map(|entry| entry.rank(&self.query).map(|rank| (rank, entry)))
            .collect();

        ranked.sort_by(|(rank_a, a), (rank_b, b)| rank_a.cmp(rank_b).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));

        let limit = self.max_results.min(ranked.len());
        ranked.truncate(limit);

        ranked
    }

    fn reset_selection(&mut self) {
        self.selected = 0;
        self.hovered = None;
        self.scroll_offset = 0;
    }
}
