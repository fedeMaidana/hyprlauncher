use crate::{geometry::Rect, style};

#[derive(Debug, Clone)]
pub struct LauncherLayout {
    pub panel: Rect,
    pub preview: Rect,
    pub search: Rect,
    pub list: Rect,
}

impl LauncherLayout {
    pub fn new(surface_w: u32, surface_h: u32, preferred_w: u32, preferred_h: u32) -> Self {
        let panel_w = preferred_w.min(surface_w.saturating_sub(48)).max(360) as i32;
        let panel_h = preferred_h.min(surface_h.saturating_sub(48)).max(240) as i32;
        let surface_w = surface_w as i32;
        let surface_h = surface_h as i32;

        let panel = Rect::new((surface_w - panel_w) / 2, (surface_h - panel_h) / 2, panel_w, panel_h);
        let inner = panel.inset(style::spacing::PANEL_PADDING);
        let preview_w = ((inner.w as f32) * 0.45).round() as i32;
        let preview = Rect::new(inner.x, inner.y, preview_w, inner.h);
        let right_x = preview.x + preview.w + style::spacing::GAP;
        let right_w = inner.x + inner.w - right_x;
        let search = Rect::new(right_x, inner.y, right_w, style::spacing::SEARCH_HEIGHT);
        let list_y = search.y + search.h + style::spacing::ROW_GAP;
        let list = Rect::new(right_x, list_y, right_w, inner.y + inner.h - list_y);

        Self {
            panel,
            preview,
            search,
            list,
        }
    }

    pub fn row_rect(&self, index: usize) -> Rect {
        let step = style::spacing::ROW_HEIGHT + style::spacing::ROW_GAP;
        Rect::new(self.list.x, self.list.y + index as i32 * step, self.list.w, style::spacing::ROW_HEIGHT)
    }

    pub fn row_at(&self, x: f64, y: f64, count: usize) -> Option<usize> {
        (0..count).find(|idx| self.row_rect(*idx).contains(x, y))
    }
}
