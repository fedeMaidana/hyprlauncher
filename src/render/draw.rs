use image::RgbaImage;
use tiny_skia::{
    FillRule, FilterQuality, LineCap, Mask, Paint, Path, PathBuilder, Pixmap, PixmapPaint, PixmapRef, Stroke, Transform,
};

use crate::{
    geometry::{Corners, Rect},
    theme::Color,
};

const CIRCLE_KAPPA: f32 = 0.552_284_8;

pub fn fill_round_rect(pixmap: &mut Pixmap, rect: Rect, radius: i32, color: Color) {
    let Some(path) = round_rect_path(rect, radius, Corners::ALL) else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color_rgba8(color.r, color.g, color.b, color.a);
    paint.anti_alias = true;

    pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
}

pub fn draw_image_cover(pixmap: &mut Pixmap, rect: Rect, radius: i32, image: &RgbaImage) {
    draw_image(pixmap, rect, radius, image, ImageFit::Cover);
}

pub fn draw_image_contain(pixmap: &mut Pixmap, rect: Rect, radius: i32, image: &RgbaImage) {
    draw_image(pixmap, rect, radius, image, ImageFit::Contain);
}

pub fn draw_search_icon(pixmap: &mut Pixmap, rect: Rect, color: Color, stroke_width: i32) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }

    let size = rect.w.min(rect.h) as f32;
    let stroke_width = stroke_width.max(1) as f32;

    let cx = rect.x as f32 + size * 0.42;
    let cy = rect.y as f32 + size * 0.42;
    let radius = size * 0.22;

    let circle = Rect::new(
        (cx - radius).round() as i32,
        (cy - radius).round() as i32,
        (radius * 2.0).round() as i32,
        (radius * 2.0).round() as i32,
    );

    let Some(circle_path) = round_rect_path(circle, circle.w / 2, Corners::ALL) else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color_rgba8(color.r, color.g, color.b, color.a);
    paint.anti_alias = true;

    let stroke = Stroke {
        width: stroke_width,
        line_cap: LineCap::Round,
        ..Default::default()
    };

    pixmap.stroke_path(&circle_path, &paint, &stroke, Transform::identity(), None);

    let mut handle = PathBuilder::new();

    handle.move_to(cx + radius * 0.68, cy + radius * 0.68);
    handle.line_to(rect.x as f32 + size * 0.76, rect.y as f32 + size * 0.76);

    if let Some(handle_path) = handle.finish() {
        pixmap.stroke_path(&handle_path, &paint, &stroke, Transform::identity(), None);
    }
}

#[derive(Debug, Clone, Copy)]
enum ImageFit {
    Cover,
    Contain,
}

fn draw_image(pixmap: &mut Pixmap, rect: Rect, radius: i32, image: &RgbaImage, fit: ImageFit) {
    if rect.w <= 0 || rect.h <= 0 || image.width() == 0 || image.height() == 0 {
        return;
    }

    let Some(image_ref) = PixmapRef::from_bytes(image.as_raw(), image.width(), image.height()) else {
        return;
    };

    let scale_x = rect.w as f32 / image.width() as f32;
    let scale_y = rect.h as f32 / image.height() as f32;

    let scale = match fit {
        ImageFit::Cover => scale_x.max(scale_y),
        ImageFit::Contain => scale_x.min(scale_y),
    };

    let scaled_w = image.width() as f32 * scale;
    let scaled_h = image.height() as f32 * scale;

    let dx = rect.x as f32 + (rect.w as f32 - scaled_w) / 2.0;
    let dy = rect.y as f32 + (rect.h as f32 - scaled_h) / 2.0;

    let transform = Transform::from_scale(scale, scale).post_translate(dx, dy);

    let Some(mut mask) = Mask::new(pixmap.width(), pixmap.height()) else {
        return;
    };

    let Some(clip) = round_rect_path(rect, radius, Corners::ALL) else {
        return;
    };

    mask.fill_path(&clip, FillRule::Winding, true, Transform::identity());

    let paint = PixmapPaint {
        quality: FilterQuality::Bicubic,
        ..PixmapPaint::default()
    };

    pixmap.draw_pixmap(0, 0, image_ref, &paint, transform, Some(&mask));
}

fn round_rect_path(rect: Rect, radius: i32, corners: Corners) -> Option<Path> {
    if rect.w <= 0 || rect.h <= 0 {
        return None;
    }

    let r_max = (rect.w.min(rect.h)) / 2;
    let r = radius.clamp(0, r_max) as f32;

    let x = rect.x as f32;
    let y = rect.y as f32;
    let w = rect.w as f32;
    let h = rect.h as f32;

    let tl = if corners.top_left { r } else { 0.0 };
    let tr = if corners.top_right { r } else { 0.0 };
    let br = if corners.bottom_right { r } else { 0.0 };
    let bl = if corners.bottom_left { r } else { 0.0 };

    let k = CIRCLE_KAPPA;
    let mut pb = PathBuilder::new();

    pb.move_to(x + tl, y);

    pb.line_to(x + w - tr, y);
    if tr > 0.0 {
        pb.cubic_to(x + w - tr + tr * k, y, x + w, y + tr - tr * k, x + w, y + tr);
    }

    pb.line_to(x + w, y + h - br);
    if br > 0.0 {
        pb.cubic_to(x + w, y + h - br + br * k, x + w - br + br * k, y + h, x + w - br, y + h);
    }

    pb.line_to(x + bl, y + h);
    if bl > 0.0 {
        pb.cubic_to(x + bl - bl * k, y + h, x, y + h - bl + bl * k, x, y + h - bl);
    }

    pb.line_to(x, y + tl);
    if tl > 0.0 {
        pb.cubic_to(x, y + tl - tl * k, x + tl - tl * k, y, x + tl, y);
    }

    pb.close();
    pb.finish()
}
