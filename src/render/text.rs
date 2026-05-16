use fontdue::Font;

use crate::{geometry::Rect, theme::Color};

const LETTER_SPACING: f32 = 0.1;

pub struct TextSurface<'a> {
    canvas: &'a mut [u8],
    width: u32,
    height: u32,
}

impl<'a> TextSurface<'a> {
    pub fn new(canvas: &'a mut [u8], width: u32, height: u32) -> Self {
        Self { canvas, width, height }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TextSpec<'a> {
    pub text: &'a str,
    pub font_size: f32,
    pub rect: Rect,
    pub color: Color,
}

#[derive(Debug, Clone, Copy)]
struct TextOrigin {
    x: f32,
    baseline: f32,
}

pub fn draw_text_left(surface: &mut TextSurface<'_>, font: &Font, spec: TextSpec<'_>) {
    let text = fit_text_to_width(font, spec.text, spec.font_size, spec.rect.w as f32);
    let Some(metrics) = font.horizontal_line_metrics(spec.font_size) else {
        return;
    };

    let line_h = metrics.ascent - metrics.descent;
    let origin = TextOrigin {
        x: spec.rect.x as f32,
        baseline: spec.rect.y as f32 + (spec.rect.h as f32 - line_h) / 2.0 + metrics.ascent,
    };

    draw_text_baseline(surface, font, &text, spec.font_size, origin, spec.color);
}

pub fn draw_text_center(surface: &mut TextSurface<'_>, font: &Font, spec: TextSpec<'_>) {
    let text = fit_text_to_width(font, spec.text, spec.font_size, spec.rect.w as f32);
    let text_w = measure_text_width(font, &text, spec.font_size);

    let Some(metrics) = font.horizontal_line_metrics(spec.font_size) else {
        return;
    };

    let line_h = metrics.ascent - metrics.descent;
    let origin = TextOrigin {
        x: spec.rect.x as f32 + (spec.rect.w as f32 - text_w) / 2.0,
        baseline: spec.rect.y as f32 + (spec.rect.h as f32 - line_h) / 2.0 + metrics.ascent,
    };

    draw_text_baseline(surface, font, &text, spec.font_size, origin, spec.color);
}

fn draw_text_baseline(surface: &mut TextSurface<'_>, font: &Font, text: &str, font_size: f32, origin: TextOrigin, color: Color) {
    let mut pen_x = origin.x;

    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, font_size);
        let glyph_x = pen_x.round() as i32 + metrics.xmin;
        let glyph_y = (origin.baseline - metrics.ymin as f32 - metrics.height as f32).round() as i32;

        for gy in 0..metrics.height {
            for gx in 0..metrics.width {
                let coverage = bitmap[gy * metrics.width + gx];
                if coverage == 0 {
                    continue;
                }

                let px = glyph_x + gx as i32;
                let py = glyph_y + gy as i32;
                if px < 0 || py < 0 || px >= surface.width as i32 || py >= surface.height as i32 {
                    continue;
                }

                blend_pixel_rgba(
                    surface.canvas,
                    surface.width,
                    px,
                    py,
                    Color {
                        a: scale_alpha(color.a, coverage),
                        ..color
                    },
                );
            }
        }

        pen_x += metrics.advance_width;
        if !ch.is_whitespace() {
            pen_x += LETTER_SPACING;
        }
    }
}

fn measure_text_width(font: &Font, text: &str, font_size: f32) -> f32 {
    let mut width = 0.0;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        let metrics = font.metrics(ch, font_size);
        width += metrics.advance_width;
        if chars.peek().is_some() && !ch.is_whitespace() {
            width += LETTER_SPACING;
        }
    }

    width.max(0.0)
}

fn fit_text_to_width(font: &Font, text: &str, font_size: f32, max_width: f32) -> String {
    if measure_text_width(font, text, font_size) <= max_width {
        return text.to_owned();
    }

    let ellipsis = "…";
    let ellipsis_w = measure_text_width(font, ellipsis, font_size);
    let mut fitted = String::new();

    for ch in text.chars() {
        let next = format!("{fitted}{ch}");
        if measure_text_width(font, &next, font_size) + ellipsis_w > max_width {
            break;
        }
        fitted.push(ch);
    }

    fitted.push('…');
    fitted
}

fn scale_alpha(alpha: u8, coverage: u8) -> u8 {
    ((alpha as u16 * coverage as u16) / 255) as u8
}

fn blend_pixel_rgba(canvas: &mut [u8], surface_w: u32, x: i32, y: i32, color: Color) {
    if color.a == 0 || x < 0 || y < 0 {
        return;
    }

    let idx = ((y as u32 * surface_w + x as u32) * 4) as usize;
    if idx + 3 >= canvas.len() {
        return;
    }

    let dst_r = canvas[idx] as u16;
    let dst_g = canvas[idx + 1] as u16;
    let dst_b = canvas[idx + 2] as u16;
    let dst_a = canvas[idx + 3] as u16;

    let a = color.a as u16;
    let inv_a = 255 - a;

    canvas[idx] = ((color.r as u16 * a + dst_r * inv_a) / 255) as u8;
    canvas[idx + 1] = ((color.g as u16 * a + dst_g * inv_a) / 255) as u8;
    canvas[idx + 2] = ((color.b as u16 * a + dst_b * inv_a) / 255) as u8;
    canvas[idx + 3] = (a + (dst_a * inv_a) / 255) as u8;
}
