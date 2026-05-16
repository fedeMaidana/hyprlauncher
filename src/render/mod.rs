mod draw;
mod painter;
mod text;

use fontdue::Font;
use image::RgbaImage;

use crate::{desktop::IconCache, model::Model, theme::Theme};

use painter::Painter;

pub struct RenderRequest<'a> {
    pub canvas: &'a mut [u8],
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub model: &'a Model,
    pub theme: &'a Theme,
    pub wallpaper: Option<&'a RgbaImage>,
    pub icons: &'a IconCache,
    pub font: &'a Font,
}

pub fn render_launcher(request: RenderRequest<'_>) {
    let Some(mut painter) = Painter::new(request.width, request.height, request.scale, request.font) else {
        log::error!("no se pudo crear pixmap {}x{}", request.width, request.height);
        return;
    };

    painter.draw(request.model, request.theme, request.wallpaper, request.icons);
    painter.copy_to_wayland_canvas(request.canvas);
}
