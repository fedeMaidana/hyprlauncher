use fontdue::Font;
use image::RgbaImage;
use tiny_skia::Pixmap;

use crate::{
    desktop::DesktopEntry,
    geometry::Rect,
    layout::LauncherLayout,
    model::Model,
    style,
    theme::{Color, Theme},
};

use super::{
    draw::{draw_image_cover, fill_round_rect},
    text::{TextSpec, TextSurface, draw_text_center, draw_text_left},
};

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

    pub fn draw(&mut self, model: &Model, theme: &Theme, wallpaper: Option<&RgbaImage>) {
        let layout = LauncherLayout::new(
            model.logical_width,
            model.logical_height,
            model.preferred_width,
            model.preferred_height,
        );

        self.fill_fullscreen_scrim(theme.background.with_alpha(88));
        self.draw_panel(&layout, theme);
        self.draw_preview(&layout, theme, wallpaper);
        self.draw_search(&layout, model, theme);
        self.draw_entries(&layout, model, theme);
        self.draw_error(&layout, model, theme);
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
        self.fill_round(layout.preview, style::surface::PREVIEW_RADIUS, theme.surface);

        if let Some(wallpaper) = wallpaper {
            self.image_cover(layout.preview, style::surface::PREVIEW_RADIUS, wallpaper);
            self.fill_round(layout.preview, style::surface::PREVIEW_RADIUS, theme.background.with_alpha(64));
        } else {
            self.fill_round(
                layout.preview.inset(12),
                style::surface::PREVIEW_RADIUS - 6,
                theme.surface_variant.with_alpha(120),
            );
        }

        let title =
            Rect::new(layout.preview.x + 20, layout.preview.y + layout.preview.h - 62, layout.preview.w - 40, 28);
        let subtitle = Rect::new(layout.preview.x + 20, title.y + 26, layout.preview.w - 40, 22);
        self.text_left(title, "hyprlauncher", style::font_size::TITLE, theme.foreground);
        self.text_left(subtitle, "Rust + SCTK", style::font_size::HINT, theme.muted);
    }

    fn draw_search(&mut self, layout: &LauncherLayout, model: &Model, theme: &Theme) {
        self.fill_round(layout.search, style::surface::ITEM_RADIUS, theme.surface_variant);

        let orb = Rect::new(
            layout.search.x + 15,
            layout.search.y + (layout.search.h - style::spacing::ICON_SIZE) / 2,
            style::spacing::ICON_SIZE,
            style::spacing::ICON_SIZE,
        );
        self.fill_round(orb, style::spacing::ICON_SIZE / 2, theme.accent);
        self.text_center(orb, "⌕", 16.0, theme.background.with_alpha(230));

        let text_rect = Rect::new(layout.search.x + 54, layout.search.y, layout.search.w - 70, layout.search.h);
        if model.launcher.query().is_empty() {
            self.text_left(text_rect, "Buscar aplicación", style::font_size::QUERY, theme.muted);
        } else {
            self.text_left(text_rect, model.launcher.query(), style::font_size::QUERY, theme.foreground);
        }
    }

    fn draw_entries(&mut self, layout: &LauncherLayout, model: &Model, theme: &Theme) {
        let visible = model.launcher.visible_entries();
        if visible.is_empty() {
            let rect = Rect::new(layout.list.x, layout.list.y + 28, layout.list.w, 40);
            self.text_center(rect, "Sin resultados", style::font_size::QUERY, theme.muted);
            return;
        }

        for (index, entry) in visible.iter().enumerate() {
            let row = layout.row_rect(index);
            let selected = index == model.launcher.selected();
            let hovered = model.launcher.hovered() == Some(index);
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

            self.draw_entry(row, entry, selected, theme);
        }
    }

    fn draw_entry(&mut self, row: Rect, entry: &DesktopEntry, selected: bool, theme: &Theme) {
        let icon = Rect::new(row.x + 14, row.y + (row.h - 28) / 2, 28, 28);
        let icon_bg = if selected {
            theme.background.with_alpha(225)
        } else {
            theme.surface_variant
        };
        let icon_fg = if selected { theme.accent } else { theme.foreground };
        self.fill_round(icon, 14, icon_bg);

        let initial = entry
            .name
            .chars()
            .find(|ch| ch.is_alphanumeric())
            .map(|ch| ch.to_uppercase().collect::<String>())
            .unwrap_or_else(|| "•".to_owned());
        self.text_center(icon, &initial, 13.0, icon_fg);

        let text_x = icon.x + icon.w + 12;
        let title = Rect::new(text_x, row.y + 4, row.x + row.w - text_x - 12, 22);
        let subtitle = Rect::new(text_x, row.y + 23, row.x + row.w - text_x - 12, 18);
        let fg = if selected {
            theme.background.with_alpha(245)
        } else {
            theme.foreground
        };
        let muted = if selected {
            theme.background.with_alpha(170)
        } else {
            theme.muted
        };

        self.text_left(title, &entry.name, style::font_size::TITLE, fg);
        self.text_left(subtitle, entry.subtitle(), style::font_size::HINT, muted);
    }

    fn draw_error(&mut self, layout: &LauncherLayout, model: &Model, theme: &Theme) {
        let Some(error) = model.error.as_deref() else {
            return;
        };

        let rect = Rect::new(layout.panel.x + 22, layout.panel.y + layout.panel.h - 28, layout.panel.w - 44, 18);
        self.text_left(rect, error, style::font_size::HINT, theme.danger);
    }

    fn fill_round(&mut self, rect: Rect, radius: i32, color: Color) {
        let rect = self.scale_rect(rect);
        let radius = self.scale_len(radius);

        fill_round_rect(&mut self.pixmap, rect, radius, color);
    }

    fn image_cover(&mut self, rect: Rect, radius: i32, image: &RgbaImage) {
        let rect = self.scale_rect(rect);
        let radius = self.scale_len(radius);

        draw_image_cover(&mut self.pixmap, rect, radius, image);
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
}

fn copy_rgba_to_bgra(dst: &mut [u8], src: &[u8]) {
    for (d, s) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
        d[0] = s[2];
        d[1] = s[1];
        d[2] = s[0];
        d[3] = s[3];
    }
}
