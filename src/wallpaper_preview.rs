use std::path::Path;

use image::RgbaImage;

pub fn load(path: Option<&Path>) -> Option<RgbaImage> {
    let path = path?;
    match image::open(path) {
        Ok(image) => Some(image.to_rgba8()),
        Err(err) => {
            log::warn!("no se pudo cargar wallpaper preview {}: {err:#}", path.display());
            None
        }
    }
}
