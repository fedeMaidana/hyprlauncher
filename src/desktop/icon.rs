use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
};

use image::{RgbaImage, imageops::FilterType};

use super::DesktopEntry;

const ICON_RENDER_SIZE: u32 = 256;
const MAX_SCAN_DEPTH: usize = 8;

#[derive(Debug, Default)]
pub struct IconCache {
    icons: HashMap<String, RgbaImage>,
}

impl IconCache {
    pub fn load_for_entries(entries: &[DesktopEntry]) -> Self {
        let icon_names = unique_icon_names(entries);
        let index = IconIndex::build();
        let mut icons = HashMap::new();

        for name in icon_names {
            let Some(path) = index.lookup(&name) else {
                continue;
            };

            match load_icon(&path) {
                Some(image) => {
                    icons.insert(name, image);
                }
                None => {
                    log::debug!("no se pudo cargar icono desde {}", path.display());
                }
            }
        }

        Self { icons }
    }

    pub fn image_for(&self, entry: &DesktopEntry) -> Option<&RgbaImage> {
        entry.icon.as_ref().and_then(|name| self.icons.get(name))
    }
}

#[derive(Debug, Default)]
struct IconIndex {
    paths: HashMap<String, IconCandidate>,
}

impl IconIndex {
    fn build() -> Self {
        let mut index = Self::default();

        for root in icon_roots() {
            index.scan_root(&root);
        }

        index
    }

    fn lookup(&self, name: &str) -> Option<PathBuf> {
        let path = Path::new(name);

        if path.is_absolute() && path.exists() && icon_format(path).is_some() {
            return Some(path.to_path_buf());
        }

        self.paths.get(name).map(|candidate| candidate.path.clone())
    }

    fn scan_root(&mut self, root: &Path) {
        if root.exists() {
            self.scan_dir(root, 0);
        }
    }

    fn scan_dir(&mut self, dir: &Path, depth: usize) {
        if depth > MAX_SCAN_DEPTH {
            return;
        }

        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                self.scan_dir(&path, depth + 1);
                continue;
            }

            let Some(format) = icon_format(&path) else {
                continue;
            };

            let Some(stem) = path.file_stem().and_then(|value| value.to_str()).map(str::to_owned) else {
                continue;
            };

            let candidate = IconCandidate {
                score: score_icon_path(&path, format),
                path,
            };

            match self.paths.get(&stem) {
                Some(existing) if existing.score >= candidate.score => {}
                _ => {
                    self.paths.insert(stem, candidate);
                }
            }
        }
    }
}

#[derive(Debug)]
struct IconCandidate {
    score: i32,
    path: PathBuf,
}

#[derive(Debug, Clone, Copy)]
enum IconFormat {
    Raster,
    Svg,
}

fn unique_icon_names(entries: &[DesktopEntry]) -> Vec<String> {
    let mut seen = HashSet::new();

    entries
        .iter()
        .filter_map(|entry| entry.icon.as_deref())
        .filter(|name| !name.trim().is_empty())
        .filter(|name| seen.insert((*name).to_owned()))
        .map(str::to_owned)
        .collect()
}

fn icon_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(data_home) = env::var_os("XDG_DATA_HOME") {
        roots.push(PathBuf::from(data_home).join("icons"));
    } else if let Some(home) = env::var_os("HOME") {
        roots.push(PathBuf::from(home).join(".local/share/icons"));
    }

    if let Some(home) = env::var_os("HOME") {
        roots.push(PathBuf::from(home).join(".icons"));
    }

    let data_dirs = env::var_os("XDG_DATA_DIRS")
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_owned());

    for base in data_dirs.split(':') {
        roots.push(PathBuf::from(base).join("icons"));
        roots.push(PathBuf::from(base).join("pixmaps"));
    }

    roots
}

fn icon_format(path: &Path) -> Option<IconFormat> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png" | "jpg" | "jpeg" | "webp") => Some(IconFormat::Raster),
        Some("svg") => Some(IconFormat::Svg),
        _ => None,
    }
}

fn score_icon_path(path: &Path, format: IconFormat) -> i32 {
    let path = path.to_string_lossy().to_ascii_lowercase();

    let symbolic_penalty = if path.contains("symbolic") || path.contains("-symbolic") {
        -10_000
    } else {
        0
    };

    let scalable_score = if path.contains("/scalable/") { 4_000 } else { 0 };

    let app_score = if path.contains("/apps/") || path.contains("/pixmaps/") {
        2_000
    } else {
        0
    };

    let format_score = match format {
        IconFormat::Svg => 3_000,
        IconFormat::Raster => 0,
    };

    let size_score = [1024, 512, 256, 192, 128, 96, 72, 64, 48, 32, 24, 22, 16]
        .into_iter()
        .find(|size| path.contains(&format!("{size}x{size}")))
        .unwrap_or(0);

    symbolic_penalty + scalable_score + app_score + format_score + size_score
}

fn load_icon(path: &Path) -> Option<RgbaImage> {
    match icon_format(path)? {
        IconFormat::Raster => load_raster_icon(path),
        IconFormat::Svg => render_svg_icon(path),
    }
}

fn load_raster_icon(path: &Path) -> Option<RgbaImage> {
    let image = image::open(path).ok()?.to_rgba8();
    Some(resize_icon(image))
}

fn render_svg_icon(path: &Path) -> Option<RgbaImage> {
    let data = fs::read(path).ok()?;
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(&data, &options).ok()?;

    let size = tree.size();
    let source_w = size.width();
    let source_h = size.height();

    if source_w <= 0.0 || source_h <= 0.0 {
        return None;
    }

    let source_max = source_w.max(source_h);
    let scale = ICON_RENDER_SIZE as f32 / source_max;

    let rendered_w = source_w * scale;
    let rendered_h = source_h * scale;

    let dx = (ICON_RENDER_SIZE as f32 - rendered_w) / 2.0;
    let dy = (ICON_RENDER_SIZE as f32 - rendered_h) / 2.0;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(ICON_RENDER_SIZE, ICON_RENDER_SIZE)?;
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale).post_translate(dx, dy);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    RgbaImage::from_raw(ICON_RENDER_SIZE, ICON_RENDER_SIZE, pixmap.take())
}

fn resize_icon(image: RgbaImage) -> RgbaImage {
    if image.width() == ICON_RENDER_SIZE && image.height() == ICON_RENDER_SIZE {
        return image;
    }

    image::imageops::resize(&image, ICON_RENDER_SIZE, ICON_RENDER_SIZE, FilterType::Lanczos3)
}
