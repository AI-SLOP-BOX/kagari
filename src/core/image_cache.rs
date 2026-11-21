/// Image layer loader and cache for JPEG/PNG images.
///
/// Loads image files from disk, decodes them to RGBA pixel buffers,
/// and caches them for use by the software renderer.
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;

/// Cached decoded image data.
#[derive(Debug, Clone)]
pub struct CachedImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

struct CachedFile {
    revision: u64,
    image: Arc<CachedImage>,
    lru_stamp: u64,
    byte_size: usize,
}

/// Thread-safe image cache.
pub struct ImageCache {
    cache: HashMap<String, CachedFile>,
    current_bytes: usize,
    max_bytes: usize,
    lru_clock: u64,
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageCache {
    const DEFAULT_MAX_BYTES: usize = 1024 * 1024 * 1024;
    const MAX_DECODE_BYTES: u64 = 512 * 1024 * 1024;
    const MAX_SVG_SOURCE_BYTES: u64 = 32 * 1024 * 1024;
    const MAX_SVG_BITMAP_BYTES: usize = 256 * 1024 * 1024;

    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            current_bytes: 0,
            max_bytes: Self::DEFAULT_MAX_BYTES,
            lru_clock: 0,
        }
    }

    /// Set the maximum cache size in bytes.
    pub fn set_max_bytes(&mut self, max: usize) {
        self.max_bytes = max;
        self.evict_if_over_budget();
    }

    fn evict_if_over_budget(&mut self) {
        while self.current_bytes > self.max_bytes && !self.cache.is_empty() {
            // Find the entry with the lowest LRU stamp
            if let Some((worst_key, worst_entry)) = self
                .cache
                .iter()
                .min_by_key(|(_, e)| e.lru_stamp)
                .map(|(k, e)| (k.clone(), e.byte_size))
            {
                self.current_bytes = self.current_bytes.saturating_sub(worst_entry);
                self.cache.remove(&worst_key);
            } else {
                break;
            }
        }
    }

    /// Load an image from a file path. Returns cached result on subsequent calls.
    pub fn load_image(&mut self, path: &str) -> Option<&CachedImage> {
        let revision = match Self::file_revision(path) {
            Some(revision) => revision,
            None => {
                if let Some(removed) = self.cache.remove(path) {
                    self.current_bytes = self.current_bytes.saturating_sub(removed.byte_size);
                }
                return None;
            }
        };
        if self
            .cache
            .get(path)
            .is_some_and(|cached| cached.revision == revision)
        {
            self.lru_clock += 1;
            if let Some(entry) = self.cache.get_mut(path) {
                entry.lru_stamp = self.lru_clock;
            }
            return self.cache.get(path).map(|cached| cached.image.as_ref());
        }

        let img = match Self::decode_image_file(path, self.max_bytes) {
            Some(image) => image,
            None => {
                if let Some(removed) = self.cache.remove(path) {
                    self.current_bytes = self.current_bytes.saturating_sub(removed.byte_size);
                }
                return None;
            }
        };
        // Remove old entry if replacing
        if let Some(removed) = self.cache.remove(path) {
            self.current_bytes = self.current_bytes.saturating_sub(removed.byte_size);
        }
        let byte_size = img.pixels.len();
        self.lru_clock += 1;
        self.cache.insert(
            path.to_string(),
            CachedFile {
                revision,
                image: Arc::new(img),
                lru_stamp: self.lru_clock,
                byte_size,
            },
        );
        self.current_bytes += byte_size;
        self.evict_if_over_budget();
        self.cache.get(path).map(|cached| cached.image.as_ref())
    }

    /// Load an image and return a shared handle so renderers can retain pixels
    /// after releasing the global cache lock without copying the bitmap.
    pub fn load_image_shared(&mut self, path: &str) -> Option<Arc<CachedImage>> {
        self.load_image(path)?;
        self.cache.get(path).map(|cached| Arc::clone(&cached.image))
    }

    /// Maximum pixels per image (16384 x 16384): guards against decompression bombs
    /// and runaway allocations from corrupted or malicious image files.
    pub const MAX_IMAGE_PIXELS: u64 = 16384 * 16384;

    /// Decode an image file to RGBA pixels.
    fn decode_image_file(path: &str, max_bytes: usize) -> Option<CachedImage> {
        if std::path::Path::new(path)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
        {
            return decode_svg_file(path, max_bytes);
        }
        // Check declared dimensions BEFORE decoding to avoid huge allocations
        let reader = image::ImageReader::open(path).ok()?;
        let (declared_w, declared_h) = reader.into_dimensions().ok()?;
        let total = declared_w as u64 * declared_h as u64;
        if declared_w == 0
            || declared_h == 0
            || declared_w > crate::core::software_renderer::MAX_RENDER_DIMENSION
            || declared_h > crate::core::software_renderer::MAX_RENDER_DIMENSION
            || total > Self::MAX_IMAGE_PIXELS
        {
            log::warn!(
                "[ImageCache] Rejecting image {} with dimensions {}x{} ({} px > limit)",
                path,
                declared_w,
                declared_h,
                total
            );
            return None;
        }
        let declared_bytes = total.checked_mul(4)?;
        if declared_bytes > max_bytes as u64 {
            log::warn!(
                "[ImageCache] Rejecting image {} with {} RGBA bytes (cache limit {} bytes)",
                path,
                declared_bytes,
                max_bytes
            );
            return None;
        }

        let mut reader = image::ImageReader::open(path).ok()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(crate::core::software_renderer::MAX_RENDER_DIMENSION);
        limits.max_image_height = Some(crate::core::software_renderer::MAX_RENDER_DIMENSION);
        limits.max_alloc = Some(Self::MAX_DECODE_BYTES.min(max_bytes as u64));
        reader.limits(limits);
        let img = reader.decode().ok()?;
        let (width, height) = (img.width(), img.height());
        let pixel_count = u64::from(width).checked_mul(u64::from(height))?;
        let decoded_rgba_bytes = pixel_count.checked_mul(4)?;
        if width == 0
            || height == 0
            || width > crate::core::software_renderer::MAX_RENDER_DIMENSION
            || height > crate::core::software_renderer::MAX_RENDER_DIMENSION
            || pixel_count > Self::MAX_IMAGE_PIXELS
            || decoded_rgba_bytes > Self::MAX_DECODE_BYTES
            || decoded_rgba_bytes > max_bytes as u64
        {
            return None;
        }
        let expected_bytes = usize::try_from(decoded_rgba_bytes).ok()?;
        let rgba = img.to_rgba8();
        if rgba.dimensions() != (width, height) || rgba.as_raw().len() != expected_bytes {
            return None;
        }
        Some(CachedImage {
            width,
            height,
            pixels: rgba.into_raw(),
        })
    }

    fn file_revision(path: &str) -> Option<u64> {
        let path_buf = std::path::Path::new(path);
        #[cfg(not(test))]
        {
            crate::core::media_revision::existing_media_file_revision(path_buf)
        }
        #[cfg(test)]
        {
            crate::core::media_revision::existing_media_file_revision_uncached(path_buf)
        }
    }

    /// Get a cached image without loading.
    pub fn get(&self, path: &str) -> Option<&CachedImage> {
        let revision = Self::file_revision(path)?;
        self.cache
            .get(path)
            .filter(|cached| cached.revision == revision)
            .map(|cached| cached.image.as_ref())
    }
}

/// Decode an SVG through resvg while keeping the same cache and allocation limits
/// used for raster images. External file references are ignored by usvg's nested
/// document parser, so importing an SVG never reads files adjacent to it.
pub(crate) fn decode_svg_file(path: &str, max_bytes: usize) -> Option<CachedImage> {
    let tree = load_svg_tree(path)?;
    let size = tree.size();
    let width_f = size.width().ceil();
    let height_f = size.height().ceil();
    if !width_f.is_finite() || !height_f.is_finite() || width_f < 1.0 || height_f < 1.0 {
        return None;
    }
    let (width, height) = (width_f as u32, height_f as u32);
    let pixels = u64::from(width).checked_mul(u64::from(height))?;
    let byte_len = pixels.checked_mul(4)?;
    if pixels > ImageCache::MAX_IMAGE_PIXELS
        || byte_len > max_bytes.min(ImageCache::MAX_SVG_BITMAP_BYTES) as u64
    {
        return None;
    }
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    let pixels = pixmap.take_demultiplied();
    if pixels.len() != byte_len as usize {
        return None;
    }
    Some(CachedImage {
        width,
        height,
        pixels,
    })
}

/// Read dimensions without allocating the decoded bitmap. SVG dimensions are
/// parsed with the same source and pixel limits used by the renderer.
pub fn image_dimensions(path: &str) -> Option<(u32, u32)> {
    let is_svg = std::path::Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"));
    let (width, height) = if is_svg {
        let size = load_svg_tree(path)?.size();
        let width = size.width().ceil();
        let height = size.height().ceil();
        if !width.is_finite() || !height.is_finite() || width < 1.0 || height < 1.0 {
            return None;
        }
        (width as u32, height as u32)
    } else {
        image::image_dimensions(path).ok()?
    };
    let pixels = u64::from(width).checked_mul(u64::from(height))?;
    let bytes = pixels.checked_mul(4)?;
    let byte_limit = if is_svg {
        ImageCache::MAX_SVG_BITMAP_BYTES
    } else {
        ImageCache::DEFAULT_MAX_BYTES
    };
    (width > 0
        && height > 0
        && pixels <= ImageCache::MAX_IMAGE_PIXELS
        && bytes <= byte_limit as u64)
        .then_some((width, height))
}

fn load_svg_tree(path: &str) -> Option<resvg::usvg::Tree> {
    use std::io::Read;

    let file = std::fs::File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    if metadata.len() == 0 || metadata.len() > ImageCache::MAX_SVG_SOURCE_BYTES {
        return None;
    }
    let mut data = Vec::new();
    file.take(ImageCache::MAX_SVG_SOURCE_BYTES + 1)
        .read_to_end(&mut data)
        .ok()?;
    if data.len() as u64 > ImageCache::MAX_SVG_SOURCE_BYTES {
        return None;
    }
    let options = svg_options();
    resvg::usvg::Tree::from_data_nested(&data, &options).ok()
}

fn svg_options() -> resvg::usvg::Options<'static> {
    static FONT_DB: OnceLock<std::sync::Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();
    let fonts = FONT_DB.get_or_init(|| {
        let mut database = resvg::usvg::fontdb::Database::new();
        database.load_system_fonts();
        std::sync::Arc::new(database)
    });
    resvg::usvg::Options {
        fontdb: std::sync::Arc::clone(fonts),
        ..Default::default()
    }
}

/// Global image cache instance.
static GLOBAL_IMAGE_CACHE: OnceLock<std::sync::Mutex<ImageCache>> = OnceLock::new();

/// Access the global image cache.
pub fn with_image_cache<F, R>(f: F) -> R
where
    F: FnOnce(&mut ImageCache) -> R,
{
    let cache = GLOBAL_IMAGE_CACHE.get_or_init(|| std::sync::Mutex::new(ImageCache::new()));
    let mut lock = cache.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut lock)
}

/// Per-path decode locks prevent concurrent requests for one large image from
/// multiplying its temporary decode memory, without serializing other paths.
fn image_decode_lock(path: &str) -> Arc<std::sync::Mutex<()>> {
    static DECODE_LOCKS: OnceLock<
        std::sync::Mutex<HashMap<String, std::sync::Weak<std::sync::Mutex<()>>>>,
    > = OnceLock::new();
    let locks = DECODE_LOCKS.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    let mut locks = locks.lock().unwrap_or_else(|error| error.into_inner());
    locks.retain(|_, lock| lock.strong_count() > 0);
    if let Some(lock) = locks.get(path).and_then(std::sync::Weak::upgrade) {
        return lock;
    }
    let lock = Arc::new(std::sync::Mutex::new(()));
    locks.insert(path.to_string(), Arc::downgrade(&lock));
    lock
}

/// Load a shared image without holding the global cache mutex during file I/O or
/// decoding. Requests for the same uncached path share one decode, while
/// unrelated paths can be decoded concurrently.
pub fn load_image_shared(path: &str) -> Option<Arc<CachedImage>> {
    let cache = GLOBAL_IMAGE_CACHE.get_or_init(|| std::sync::Mutex::new(ImageCache::new()));
    let revision = ImageCache::file_revision(path);
    if let Some(revision) = revision.as_ref() {
        let mut lock = cache.lock().unwrap_or_else(|error| error.into_inner());
        if lock
            .cache
            .get(path)
            .is_some_and(|entry| &entry.revision == revision)
        {
            lock.lru_clock = lock.lru_clock.saturating_add(1);
            let stamp = lock.lru_clock;
            if let Some(entry) = lock.cache.get_mut(path) {
                entry.lru_stamp = stamp;
                return Some(Arc::clone(&entry.image));
            }
        }
    } else {
        let mut lock = cache.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(removed) = lock.cache.remove(path) {
            lock.current_bytes = lock.current_bytes.saturating_sub(removed.byte_size);
        }
        return None;
    }

    let decode_lock = image_decode_lock(path);
    let _decode_guard = decode_lock
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let Some(revision) = ImageCache::file_revision(path) else {
        let mut lock = cache.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(removed) = lock.cache.remove(path) {
            lock.current_bytes = lock.current_bytes.saturating_sub(removed.byte_size);
        }
        return None;
    };
    let max_bytes = {
        let mut lock = cache.lock().unwrap_or_else(|error| error.into_inner());
        if lock
            .cache
            .get(path)
            .is_some_and(|entry| entry.revision == revision)
        {
            lock.lru_clock = lock.lru_clock.saturating_add(1);
            let stamp = lock.lru_clock;
            if let Some(entry) = lock.cache.get_mut(path) {
                entry.lru_stamp = stamp;
                return Some(Arc::clone(&entry.image));
            }
        }
        if let Some(removed) = lock.cache.remove(path) {
            lock.current_bytes = lock.current_bytes.saturating_sub(removed.byte_size);
        }
        lock.max_bytes
    };

    let decoded = Arc::new(ImageCache::decode_image_file(path, max_bytes)?);
    let mut lock = cache.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(entry) = lock
        .cache
        .get(path)
        .filter(|entry| entry.revision == revision)
    {
        return Some(Arc::clone(&entry.image));
    }
    if let Some(removed) = lock.cache.remove(path) {
        lock.current_bytes = lock.current_bytes.saturating_sub(removed.byte_size);
    }
    let byte_size = decoded.pixels.len();
    if byte_size <= lock.max_bytes {
        lock.lru_clock = lock.lru_clock.saturating_add(1);
        let lru_stamp = lock.lru_clock;
        lock.cache.insert(
            path.to_string(),
            CachedFile {
                revision,
                image: Arc::clone(&decoded),
                lru_stamp,
                byte_size,
            },
        );
        lock.current_bytes = lock.current_bytes.saturating_add(byte_size);
        lock.evict_if_over_budget();
    }
    Some(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_cache_creation() {
        let c = ImageCache::new();
        assert!(c.cache.is_empty());
    }

    #[test]
    fn test_load_nonexistent_image() {
        let mut c = ImageCache::new();
        assert!(c.load_image("/nonexistent/path/image.png").is_none());
    }

    #[test]
    fn test_reloads_image_when_file_changes() {
        let dir = std::env::temp_dir().join(format!(
            "kagari_image_cache_revision_{}_{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("replace.png");

        image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]))
            .save(&path)
            .unwrap();
        let mut cache = ImageCache::new();
        assert_eq!(
            cache.load_image(path.to_str().unwrap()).unwrap().pixels[0],
            255
        );

        image::RgbaImage::from_pixel(2, 1, image::Rgba([0, 255, 0, 255]))
            .save(&path)
            .unwrap();
        let reloaded = cache.load_image(path.to_str().unwrap()).unwrap();
        assert_eq!((reloaded.width, reloaded.height), (2, 1));
        assert_eq!(&reloaded.pixels[..4], &[0, 255, 0, 255]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn failed_reload_and_deleted_source_do_not_expose_stale_pixels() {
        let dir =
            std::env::temp_dir().join(format!("kagari_image_cache_stale_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stale.png");
        image::RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 255]))
            .save(&path)
            .unwrap();
        let key = path.to_str().unwrap();
        let mut cache = ImageCache::new();
        assert!(cache.load_image(key).is_some());
        std::fs::write(&path, b"corrupt").unwrap();
        assert!(cache.load_image(key).is_none());
        assert!(cache.get(key).is_none());
        let _ = std::fs::remove_file(&path);
        assert!(cache.get(key).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod robustness_tests {
    use super::*;

    #[test]
    fn test_missing_and_invalid_files_return_none() {
        let mut cache = ImageCache::new();
        // Missing file
        assert!(cache.load_image("/nonexistent/path/img.png").is_none());
        // Directory instead of file
        assert!(cache.load_image("/tmp").is_none());
        // Corrupted image payload with valid extension
        let bad = std::env::temp_dir().join("kagari_bad_image.png");
        std::fs::write(&bad, b"not a real png at all").unwrap();
        assert!(cache.load_image(bad.to_str().unwrap()).is_none());
        let _ = std::fs::remove_file(&bad);
    }
}
