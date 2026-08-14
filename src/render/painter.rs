use fontdue::Font;
use image::{RgbaImage, imageops::FilterType};
use tiny_skia::Pixmap;

use crate::{
    desktop::{DesktopEntry, IconCache},
    geometry::Rect,
    launcher::{ListItem, name_match_range},
    layout::LauncherLayout,
    model::Model,
    style,
    theme::{Color, Theme},
};

use super::{
    draw::{
        draw_clear_icon, draw_image_contain, draw_image_cover_slanted_right, draw_search_icon, fill_round_rect, fill_slanted_band,
        fill_slanted_preview_rect,
    },
    text::{TextSpec, TextSurface, draw_text_center, draw_text_left, measure_text_width},
};

const PREVIEW_SLANT: i32 = 46;
const PREVIEW_DIVIDER_BAND_WIDTH: i32 = 28;
const EMPTY_STATE_QUERY_MAX_CHARS: usize = 24;

fn empty_state_message(query: &str) -> String {
    let query = query.trim();

    if query.is_empty() {
        return "Sin resultados".to_owned();
    }

    let shown: String = if query.chars().count() > EMPTY_STATE_QUERY_MAX_CHARS {
        let truncated: String = query.chars().take(EMPTY_STATE_QUERY_MAX_CHARS).collect();
        format!("{truncated}…")
    } else {
        query.to_owned()
    };

    format!("Sin resultados para «{shown}»")
}

/// First alphanumeric letter of the app name, as icon fallback.
fn entry_initial(entry: &DesktopEntry) -> String {
    entry
        .name
        .chars()
        .find(|ch| ch.is_alphanumeric())
        .map(|ch| ch.to_uppercase().collect::<String>())
        .unwrap_or_else(|| "•".to_owned())
}

pub struct Painter<'a> {
    pixmap: Pixmap,
    scale: f32,
    font: &'a Font,
}

impl<'a> Painter<'a> {
    pub fn new(width: u32, height: u32, scale: f32, font: &'a Font) -> Option<Self> {
        Some(Self {
            pixmap: Pixmap::new(width, height)?,
            scale,
            font,
        })
    }

    pub fn draw(&mut self, model: &Model, theme: &Theme, wallpaper: Option<&RgbaImage>, icons: &mut IconCache) {
        let layout = model.layout();

        self.fill_fullscreen_scrim(theme.background.with_alpha(88));
        self.draw_panel(&layout, theme);
        self.draw_preview(&layout, theme, wallpaper);
        self.draw_diagonal_divider(&layout, theme);
        self.draw_search(&layout, model, theme);
        self.draw_pins(&layout, model, theme, icons);
        self.draw_entries(&layout, model, theme, icons);
        self.draw_hints(&layout, model, theme);
        self.draw_error(&layout, model, theme);
    }

    /// Muted key guide on the scrim, just below the panel.
    fn draw_hints(&mut self, layout: &LauncherLayout, model: &Model, theme: &Theme) {
        // The error banner uses this strip; it wins.
        if model.error.is_some() {
            return;
        }

        let rect = Rect::new(layout.panel.x, layout.panel.y + layout.panel.h + 9, layout.panel.w, 14);

        self.text_center(rect, "↑↓ navegar · Enter abrir · Esc salir", style::font_size::HINT, theme.muted.with_alpha(200));
    }

    /// Fixed tiles with the most-launched apps, pinned between search and results.
    fn draw_pins(&mut self, layout: &LauncherLayout, model: &Model, theme: &Theme, icons: &mut IconCache) {
        let pinned = model.launcher.pinned_entries(style::pins::MAX);

        if pinned.is_empty() {
            return;
        }

        self.text_left(layout.pins_label, "Más usadas", style::font_size::HINT, theme.muted.with_alpha(190));

        for (index, entry) in pinned.iter().enumerate() {
            let tile = layout.pin_rect(index);
            let hovered = model.hovered_pin == Some(index);

            let (border_color, fill_color) = if hovered {
                (theme.accent.with_alpha(150), theme.surface_variant)
            } else {
                (theme.panel_border.with_alpha(90), theme.surface.with_alpha(180))
            };

            self.fill_round(tile, style::pins::RADIUS + 1, border_color);
            self.fill_round(tile.inset(1), style::pins::RADIUS, fill_color);

            // Solo el logo, centrado en el tile.
            let size = style::pins::ICON_SIZE;
            let icon = Rect::new(tile.x + (tile.w - size) / 2, tile.y + (tile.h - size) / 2, size, size);

            if let Some(app_icon) = icons.image_for(entry) {
                self.image_contain_lanczos(icon, 0, &app_icon);
            } else {
                self.text_center(icon, &entry_initial(entry), 12.0, theme.muted);
            }
        }
    }

    fn fill_slanted_preview(&mut self, rect: Rect, radius: i32, slant: i32, color: Color) {
        let rect = self.scale_rect(rect);
        let radius = self.scale_len(radius);
        let slant = self.scale_len(slant);

        fill_slanted_preview_rect(&mut self.pixmap, rect, radius, slant, color);
    }

    fn image_cover_slanted_right(&mut self, rect: Rect, radius: i32, slant: i32, image: &RgbaImage) {
        let rect = self.scale_rect(rect);
        let radius = self.scale_len(radius);
        let slant = self.scale_len(slant);

        draw_image_cover_slanted_right(&mut self.pixmap, rect, radius, slant, image);
    }

    fn slanted_band(&mut self, rect: Rect, slant: i32, color: Color) {
        let rect = self.scale_rect(rect);
        let slant = self.scale_len(slant);

        fill_slanted_band(&mut self.pixmap, rect, slant, color);
    }

    pub fn copy_to_wayland_canvas(&self, canvas: &mut [u8]) {
        copy_rgba_to_bgra(canvas, self.pixmap.data());
    }

    fn fill_fullscreen_scrim(&mut self, color: Color) {
        let rect = Rect::new(0, 0, self.logical(self.pixmap.width()), self.logical(self.pixmap.height()));

        self.fill_round(rect, 0, color);
    }

    fn draw_panel(&mut self, layout: &LauncherLayout, theme: &Theme) {
        self.fill_round(layout.panel, style::surface::PANEL_RADIUS + 1, theme.panel_border);
        self.fill_round(layout.panel.inset(1), style::surface::PANEL_RADIUS, theme.panel);
    }

    fn draw_preview(&mut self, layout: &LauncherLayout, theme: &Theme, wallpaper: Option<&RgbaImage>) {
        self.fill_slanted_preview(layout.preview, style::surface::PREVIEW_RADIUS, PREVIEW_SLANT, theme.surface);

        if let Some(wallpaper) = wallpaper {
            self.image_cover_slanted_right(layout.preview, style::surface::PREVIEW_RADIUS, PREVIEW_SLANT, wallpaper);

            self.fill_slanted_preview(layout.preview, style::surface::PREVIEW_RADIUS, PREVIEW_SLANT, theme.background.with_alpha(64));
        } else {
            self.fill_slanted_preview(
                layout.preview.inset(12),
                style::surface::PREVIEW_RADIUS - 6,
                PREVIEW_SLANT,
                theme.surface_variant.with_alpha(120),
            );
        }
    }

    fn draw_entries(&mut self, layout: &LauncherLayout, model: &Model, theme: &Theme, icons: &mut IconCache) {
        let visible = model.launcher.window_items(layout.visible_rows());

        if visible.is_empty() {
            let rect = Rect::new(layout.list.x, layout.list.y + 28, layout.list.w, 40);
            self.text_center(rect, &empty_state_message(model.launcher.query()), style::font_size::QUERY, theme.muted);
            return;
        }

        for (row_index, item) in visible.iter().enumerate() {
            let row = layout.row_rect(row_index);

            match item {
                ListItem::Header(letter) => self.draw_group_header(row, *letter, theme),
                ListItem::Entry { index, entry } => {
                    let selected = *index == model.launcher.selected();
                    let hovered = model.launcher.hovered() == Some(*index);

                    let row_color = if selected {
                        theme.accent_soft
                    } else if hovered {
                        theme.surface_variant.with_alpha(170)
                    } else {
                        Color::from_rgba(0, 0, 0, 0)
                    };

                    if row_color.a > 0 {
                        self.fill_round(row, style::surface::ITEM_RADIUS, row_color);
                    }

                    let app_icon = icons.image_for(entry);
                    self.draw_entry(row, entry, selected, model.launcher.query(), theme, app_icon.as_deref());
                }
            }
        }

        self.draw_overflow_indicator(layout, model, theme, &visible);
    }

    /// Separador del abecedario: la letra del grupo y una línea fina.
    fn draw_group_header(&mut self, row: Rect, letter: char, theme: &Theme) {
        let label = Rect::new(row.x + 16, row.y, 22, row.h);

        self.text_left(label, &letter.to_string(), 12.0, theme.accent);

        let line = Rect::new(row.x + 44, row.y + row.h / 2, (row.w - 56).max(0), 1);

        self.fill_round(line, 0, theme.panel_border.with_alpha(110));
    }

    /// Muted "n más…" below the panel's bottom-right corner when results overflow.
    fn draw_overflow_indicator(&mut self, layout: &LauncherLayout, model: &Model, theme: &Theme, visible: &[ListItem<'_>]) {
        let total = model.launcher.result_count();

        let last_shown = visible
            .iter()
            .filter_map(|item| match item {
                ListItem::Entry { index, .. } => Some(index + 1),
                ListItem::Header(_) => None,
            })
            .max()
            .unwrap_or(0);

        if last_shown >= total {
            return;
        }

        let text = format!("{} más…", total - last_shown);
        let width = self.measure_text_width(&text, style::font_size::HINT);
        let x = layout.panel.x + layout.panel.w - 4 - width;
        let rect = Rect::new(x, layout.panel.y + layout.panel.h + 9, width + 4, 14);

        self.text_left(rect, &text, style::font_size::HINT, theme.muted.with_alpha(200));
    }

    fn draw_entry(&mut self, row: Rect, entry: &DesktopEntry, selected: bool, query: &str, theme: &Theme, app_icon: Option<&RgbaImage>) {
        let size = style::spacing::ICON_SIZE;
        let icon = Rect::new(row.x + 16, row.y + (row.h - size) / 2, size, size);

        if let Some(app_icon) = app_icon {
            self.image_contain_lanczos(icon, 0, app_icon);
        } else {
            self.text_center(icon, &entry_initial(entry), 11.0, theme.muted);
        }

        let text_x = icon.x + icon.w + 16;
        let title = Rect::new(text_x, row.y + 4, row.x + row.w - text_x - 12, 22);
        let subtitle = Rect::new(text_x, row.y + 23, row.x + row.w - text_x - 12, 18);

        let fg = if selected {
            Color::from_rgba(255, 255, 255, 245)
        } else {
            theme.foreground
        };

        let muted = if selected {
            Color::from_rgba(255, 255, 255, 175)
        } else {
            theme.muted
        };

        // Matched letters light up in accent so the ranking explains itself.
        let highlight = if selected { fg } else { theme.accent };

        match name_match_range(&entry.name, query) {
            Some((start, end)) => self.text_left_highlighted(title, &entry.name, style::font_size::TITLE, fg, highlight, start, end),
            None => self.text_left(title, &entry.name, style::font_size::TITLE, fg),
        }

        self.text_left(subtitle, entry.subtitle(), style::font_size::HINT, muted);
    }

    #[allow(clippy::too_many_arguments)]
    fn text_left_highlighted(&mut self, rect: Rect, text: &str, size: f32, base: Color, accent: Color, start: usize, end: usize) {
        let chars: Vec<char> = text.chars().collect();
        let start = start.min(chars.len());
        let end = end.clamp(start, chars.len());

        let segments = [
            (chars[..start].iter().collect::<String>(), base),
            (chars[start..end].iter().collect::<String>(), accent),
            (chars[end..].iter().collect::<String>(), base),
        ];

        let mut x = rect.x;

        for (segment, color) in segments {
            if segment.is_empty() {
                continue;
            }

            if x >= rect.x + rect.w {
                break;
            }

            let width = (rect.x + rect.w - x).max(0);
            self.text_left(Rect::new(x, rect.y, width, rect.h), &segment, size, color);
            x += self.measure_text_width(&segment, size);
        }
    }

    fn draw_error(&mut self, layout: &LauncherLayout, model: &Model, theme: &Theme) {
        let Some(error) = model.error.as_deref() else {
            return;
        };

        let rect = Rect::new(layout.panel.x + 4, layout.panel.y + layout.panel.h + 7, layout.panel.w - 8, 18);

        self.text_left(rect, error, style::font_size::HINT, theme.danger);
    }

    fn fill_round(&mut self, rect: Rect, radius: i32, color: Color) {
        let rect = self.scale_rect(rect);
        let radius = self.scale_len(radius);

        fill_round_rect(&mut self.pixmap, rect, radius, color);
    }

    fn image_contain_lanczos(&mut self, rect: Rect, radius: i32, image: &RgbaImage) {
        let rect = self.scale_rect(rect);
        let radius = self.scale_len(radius);

        if rect.w <= 0 || rect.h <= 0 || image.width() == 0 || image.height() == 0 {
            return;
        }

        let src_w = image.width() as f32;
        let src_h = image.height() as f32;

        let scale = (rect.w as f32 / src_w).min(rect.h as f32 / src_h);

        let out_w = (src_w * scale).round().max(1.0) as u32;
        let out_h = (src_h * scale).round().max(1.0) as u32;

        let resized = image::imageops::resize(image, out_w, out_h, FilterType::Lanczos3);

        let draw_rect = Rect::new(rect.x + (rect.w - out_w as i32) / 2, rect.y + (rect.h - out_h as i32) / 2, out_w as i32, out_h as i32);

        draw_image_contain(&mut self.pixmap, draw_rect, radius, &resized);
    }

    /// Input de búsqueda: lupa sin adornos, foco con anillo suave y una ×
    /// para limpiar cuando hay texto. El ícono acompaña, no protagoniza.
    fn draw_search(&mut self, layout: &LauncherLayout, model: &Model, theme: &Theme) {
        let focused = model.search_focused;
        let query = model.launcher.query();

        let (border_color, fill_color, icon_color) = if focused {
            (theme.accent.with_alpha(150), theme.surface_variant.with_alpha(225), theme.accent)
        } else {
            (theme.panel_border.with_alpha(70), theme.surface.with_alpha(150), theme.muted.with_alpha(200))
        };

        // Halo de foco, estilo focus-ring: apenas se insinúa.
        if focused {
            self.fill_round(layout.search.inset(-2), style::surface::SEARCH_RADIUS + 3, theme.accent.with_alpha(45));
        }

        self.fill_round(layout.search, style::surface::SEARCH_RADIUS + 1, border_color);
        self.fill_round(layout.search.inset(1), style::surface::SEARCH_RADIUS, fill_color);

        let size = style::spacing::SEARCH_ICON_SIZE;
        let icon = Rect::new(layout.search.x + 14, layout.search.y + (layout.search.h - size) / 2, size, size);

        self.search_icon(icon, icon_color);

        let text_x = icon.x + icon.w + 10;
        let text_rect = Rect::new(text_x, layout.search.y, layout.search.x + layout.search.w - 44 - text_x, layout.search.h);

        if query.is_empty() {
            let placeholder_color = if focused { theme.muted } else { theme.muted.with_alpha(170) };

            // Corrido 8px: deja lugar al caret a su izquierda, en ambos estados
            // (posición estable, sin saltos al enfocar).
            let placeholder_rect = Rect::new(text_rect.x + 8, text_rect.y, (text_rect.w - 8).max(0), text_rect.h);

            self.text_left(placeholder_rect, "Buscar aplicación", style::font_size::QUERY, placeholder_color);
        } else {
            let text_color = if focused { theme.foreground } else { theme.foreground.with_alpha(220) };

            self.text_left(text_rect, query, style::font_size::QUERY, text_color);
            self.clear_icon(layout.search_clear_rect().inset(6), theme.muted.with_alpha(200));
        }

        if focused && model.caret_visible {
            self.draw_search_caret(text_rect, model, theme);
        }
    }

    fn draw_search_caret(&mut self, text_rect: Rect, model: &Model, theme: &Theme) {
        let query = model.launcher.query();

        // Vacío: a la izquierda del placeholder. Con texto: pegado al último
        // glifo, como en cualquier input de verdad.
        let x = if query.is_empty() {
            text_rect.x
        } else {
            text_rect.x + self.measure_text_width(query, style::font_size::QUERY) + 2
        };

        // Que el caret no se escape del input con queries largas.
        let x = x.min(text_rect.x + text_rect.w - 2);

        let caret = Rect::new(x, text_rect.y + (text_rect.h - 18) / 2, 2, 18);

        self.fill_round(caret, 1, theme.accent);
    }

    fn clear_icon(&mut self, rect: Rect, color: Color) {
        let rect = self.scale_rect(rect);
        let stroke_width = self.scale_len(2);

        draw_clear_icon(&mut self.pixmap, rect, color, stroke_width);
    }

    /// Misma medición que usa el renderer de texto (incluye letter-spacing),
    /// así el caret y los anchos calculados coinciden con lo dibujado.
    fn measure_text_width(&self, text: &str, size: f32) -> i32 {
        measure_text_width(self.font, text, size).round() as i32
    }

    fn search_icon(&mut self, rect: Rect, color: Color) {
        let rect = self.scale_rect(rect);
        let stroke_width = self.scale_len(2);

        draw_search_icon(&mut self.pixmap, rect, color, stroke_width);
    }

    fn text_left(&mut self, rect: Rect, text: &str, size: f32, color: Color) {
        let width = self.pixmap.width();
        let height = self.pixmap.height();
        let rect = self.scale_rect(rect);

        let mut surface = TextSurface::new(self.pixmap.data_mut(), width, height);

        draw_text_left(
            &mut surface,
            self.font,
            TextSpec {
                text,
                font_size: size * self.scale,
                rect,
                color,
            },
        );
    }

    fn text_center(&mut self, rect: Rect, text: &str, size: f32, color: Color) {
        let width = self.pixmap.width();
        let height = self.pixmap.height();
        let rect = self.scale_rect(rect);

        let mut surface = TextSurface::new(self.pixmap.data_mut(), width, height);

        draw_text_center(
            &mut surface,
            self.font,
            TextSpec {
                text,
                font_size: size * self.scale,
                rect,
                color,
            },
        );
    }

    fn scale_rect(&self, rect: Rect) -> Rect {
        Rect {
            x: (rect.x as f32 * self.scale).round() as i32,
            y: (rect.y as f32 * self.scale).round() as i32,
            w: (rect.w as f32 * self.scale).round() as i32,
            h: (rect.h as f32 * self.scale).round() as i32,
        }
    }

    fn scale_len(&self, value: i32) -> i32 {
        (value as f32 * self.scale).round() as i32
    }

    fn logical(&self, value: u32) -> i32 {
        (value as f32 / self.scale).round() as i32
    }

    fn draw_diagonal_divider(&mut self, layout: &LauncherLayout, theme: &Theme) {
        let bottom_cut_x = layout.preview.x + layout.preview.w - PREVIEW_SLANT;

        let band = Rect::new(bottom_cut_x - PREVIEW_DIVIDER_BAND_WIDTH, layout.preview.y, PREVIEW_DIVIDER_BAND_WIDTH, layout.preview.h);

        self.slanted_band(band, PREVIEW_SLANT, theme.panel.with_alpha(96));
    }
}

fn copy_rgba_to_bgra(dst: &mut [u8], src: &[u8]) {
    for (d, s) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
        d[0] = s[2];
        d[1] = s[1];
        d[2] = s[0];
        d[3] = s[3];
    }
}
