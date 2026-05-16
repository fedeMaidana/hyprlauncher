use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
};

use image::{RgbaImage, imageops::FilterType};

use super::DesktopEntry;

const ICON_RENDER_SIZE: u32 = 256;
const MAX_SCAN_DEPTH: usize = 8;

#[derive(Debug)]
pub struct IconCache {
    icons: Arc<Mutex<HashMap<String, Arc<RgbaImage>>>>,
    requested: HashSet<String>,
    tx: mpsc::Sender<String>,
    pending: Arc<AtomicUsize>,
    changed: Arc<AtomicBool>,
}

impl IconCache {
    pub fn new() -> Self {
        let icons = Arc::new(Mutex::new(HashMap::new()));
        let pending = Arc::new(AtomicUsize::new(0));
        let changed = Arc::new(AtomicBool::new(false));
        let requested = HashSet::new();
        let (tx, rx) = mpsc::channel();

        let worker_icons = Arc::clone(&icons);
        let worker_pending = Arc::clone(&pending);
        let worker_changed = Arc::clone(&changed);

        if let Err(err) = std::thread::Builder::new()
            .name("hyprlauncher-icon-worker".to_owned())
            .spawn(move || icon_worker(rx, worker_icons, worker_pending, worker_changed))
        {
            log::warn!("no se pudo iniciar icon worker: {err:?}");
        }

        Self {
            icons,
            requested,
            tx,
            pending,
            changed,
        }
    }

    pub fn preload_entries<'a, I>(&mut self, entries: I)
    where
        I: IntoIterator<Item = &'a DesktopEntry>,
    {
        for entry in entries {
            self.request_entry(entry);
        }
    }

    pub fn image_for(&mut self, entry: &DesktopEntry) -> Option<Arc<RgbaImage>> {
        let name = entry.icon.as_deref()?.trim();

        if name.is_empty() {
            return None;
        }

        if let Some(image) = self.cached_image(name) {
            return Some(image);
        }

        self.request_icon(name);

        None
    }

    pub fn needs_redraw(&self) -> bool {
        self.pending.load(Ordering::Relaxed) > 0 || self.changed.swap(false, Ordering::Relaxed)
    }

    fn request_entry(&mut self, entry: &DesktopEntry) {
        let Some(name) = entry.icon.as_deref() else {
            return;
        };

        let name = name.trim();

        if !name.is_empty() {
            self.request_icon(name);
        }
    }

    fn request_icon(&mut self, name: &str) {
        if self.cached_image(name).is_some() {
            return;
        }

        if !self.requested.insert(name.to_owned()) {
            return;
        }

        self.pending.fetch_add(1, Ordering::Relaxed);

        if self.tx.send(name.to_owned()).is_err() {
            self.pending.fetch_sub(1, Ordering::Relaxed);
        }
    }

    fn cached_image(&self, name: &str) -> Option<Arc<RgbaImage>> {
        self.icons.lock().ok()?.get(name).cloned()
    }
}

impl Default for IconCache {
    fn default() -> Self {
        Self::new()
    }
}

pub fn warm_icon_cache(entries: &[DesktopEntry]) {
    let index = IconIndex::build();
    let mut warmed = 0usize;
    let mut skipped = 0usize;
    let mut missing = 0usize;
    let mut seen = HashSet::new();

    for entry in entries {
        let Some(name) = entry.icon.as_deref().map(str::trim) else {
            continue;
        };

        if name.is_empty() || !seen.insert(name.to_owned()) {
            continue;
        }

        if load_cached_icon(name).is_some() {
            skipped += 1;
            continue;
        }

        let Some(path) = index.lookup(name) else {
            missing += 1;
            continue;
        };

        let Some(image) = load_icon(&path) else {
            missing += 1;
            continue;
        };

        store_cached_icon(name, &image);
        warmed += 1;
    }

    log::info!("icon cache warm finished: warmed={warmed}, cached={skipped}, missing={missing}");
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

fn icon_worker(
    rx: mpsc::Receiver<String>,
    icons: Arc<Mutex<HashMap<String, Arc<RgbaImage>>>>,
    pending: Arc<AtomicUsize>,
    changed: Arc<AtomicBool>,
) {
    let mut index: Option<IconIndex> = None;

    while let Ok(name) = rx.recv() {
        let image = load_cached_icon(&name).or_else(|| {
            let index = index.get_or_insert_with(IconIndex::build);
            let path = index.lookup(&name)?;
            let image = load_icon(&path)?;

            store_cached_icon(&name, &image);

            Some(image)
        });

        if let Some(image) = image
            && let Ok(mut icons) = icons.lock()
        {
            icons.insert(name, Arc::new(image));
            changed.store(true, Ordering::Relaxed);
        }

        pending.fetch_sub(1, Ordering::Relaxed);
    }
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

fn load_cached_icon(name: &str) -> Option<RgbaImage> {
    let path = cached_icon_path(name);
    image::open(path).ok().map(|image| image.to_rgba8())
}

fn store_cached_icon(name: &str, image: &RgbaImage) {
    let path = cached_icon_path(name);

    if let Some(parent) = path.parent()
        && let Err(err) = fs::create_dir_all(parent)
    {
        log::debug!("no se pudo crear cache de iconos: {err:#}");
        return;
    }

    if let Err(err) = image.save(&path) {
        log::debug!("no se pudo guardar icono cacheado {}: {err:#}", path.display());
    }
}

fn cached_icon_path(name: &str) -> PathBuf {
    cache_dir()
        .join("hyprlauncher")
        .join("icons")
        .join(format!("{}.png", sanitize_icon_name(name)))
}

fn cache_dir() -> PathBuf {
    if let Some(cache_home) = env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(cache_home);
    }

    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cache")
}

fn sanitize_icon_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect();

    if sanitized.is_empty() { "unknown".to_owned() } else { sanitized }
}
