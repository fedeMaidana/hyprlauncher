use crate::{geometry::Rect, style};

#[derive(Debug, Clone)]
pub struct LauncherLayout {
    pub panel: Rect,
    pub preview: Rect,
    pub search: Rect,
    pub pins_label: Rect,
    pub pins: Rect,
    pub list: Rect,
}

impl LauncherLayout {
    pub fn new(surface_w: u32, surface_h: u32, preferred_w: u32, preferred_h: u32, pin_count: usize) -> Self {
        let panel_w = preferred_w.min(surface_w.saturating_sub(48)).max(360) as i32;
        let panel_h = preferred_h.min(surface_h.saturating_sub(48)).max(240) as i32;
        let surface_w = surface_w as i32;
        let surface_h = surface_h as i32;

        let panel = Rect::new((surface_w - panel_w) / 2, (surface_h - panel_h) / 2, panel_w, panel_h);

        // El preview va a sangre: pegado a los bordes izquierdo, superior e
        // inferior del panel (1px adentro para no tapar el borde).
        let preview_w = ((panel.w as f32) * 0.45).round() as i32;
        let preview = Rect::new(panel.x + 1, panel.y + 1, preview_w, panel.h - 2);

        let inner = panel.inset(style::spacing::PANEL_PADDING);
        let right_x = preview.x + preview.w + style::spacing::GAP;
        let right_w = inner.x + inner.w - right_x;

        let search = Rect::new(right_x, inner.y, right_w, style::spacing::SEARCH_HEIGHT);

        let mut next_y = search.y + search.h + style::spacing::ROW_GAP;

        let (pins_label, pins) = if pin_count > 0 {
            let label = Rect::new(right_x, next_y, right_w, style::pins::LABEL_HEIGHT);
            let tiles = Rect::new(right_x, label.y + label.h + style::pins::LABEL_GAP, right_w, style::pins::TILE_HEIGHT);

            next_y = tiles.y + tiles.h + style::spacing::ROW_GAP;
            (label, tiles)
        } else {
            (Rect::new(right_x, next_y, right_w, 0), Rect::new(right_x, next_y, right_w, 0))
        };

        let list = Rect::new(right_x, next_y, right_w, inner.y + inner.h - next_y);

        Self {
            panel,
            preview,
            search,
            pins_label,
            pins,
            list,
        }
    }

    /// Botón × para limpiar la búsqueda, pegado al borde derecho del input.
    pub fn search_clear_rect(&self) -> Rect {
        let size = 20;

        Rect::new(self.search.x + self.search.w - size - 12, self.search.y + (self.search.h - size) / 2, size, size)
    }

    pub fn pin_rect(&self, index: usize) -> Rect {
        let gap = style::pins::GAP;
        let slots = style::pins::MAX as i32;
        let w = (self.pins.w - gap * (slots - 1)) / slots;

        Rect::new(self.pins.x + index as i32 * (w + gap), self.pins.y, w, self.pins.h)
    }

    pub fn pin_at(&self, x: f64, y: f64, count: usize) -> Option<usize> {
        if self.pins.h <= 0 {
            return None;
        }

        (0..count.min(style::pins::MAX)).find(|idx| self.pin_rect(*idx).contains(x, y))
    }

    pub fn visible_rows(&self) -> usize {
        let step = style::spacing::ROW_HEIGHT + style::spacing::ROW_GAP;

        if self.list.h <= 0 || step <= 0 {
            return 0;
        }

        ((self.list.h + style::spacing::ROW_GAP) / step) as usize
    }

    pub fn row_rect(&self, index: usize) -> Rect {
        let step = style::spacing::ROW_HEIGHT + style::spacing::ROW_GAP;

        Rect::new(self.list.x, self.list.y + index as i32 * step, self.list.w, style::spacing::ROW_HEIGHT)
    }

    pub fn row_at(&self, x: f64, y: f64, count: usize) -> Option<usize> {
        let count = count.min(self.visible_rows());
        (0..count).find(|idx| self.row_rect(*idx).contains(x, y))
    }
}
