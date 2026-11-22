use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock, RwLock};

static MAX_DISK_BYTES: AtomicU64 = AtomicU64::new(50 * 1024 * 1024 * 1024); // 50 GB default
static WRITE_COUNTER: AtomicU64 = AtomicU64::new(0);
static CACHE_DIRECTORY: OnceLock<RwLock<Option<PathBuf>>> = OnceLock::new();
static DISK_WRITE_LOCK: Mutex<()> = Mutex::new(());

/// Set the maximum disk cache size in bytes. Called from Preferences.
pub fn set_max_disk_bytes(bytes: u64) {
    MAX_DISK_BYTES.store(bytes, Ordering::Relaxed);
}

pub fn cache_directory() -> PathBuf {
    CACHE_DIRECTORY
        .get_or_init(|| RwLock::new(None))
        .read()
        .map(|dir| dir.clone().unwrap_or_else(default_cache_dir))
        .unwrap_or_else(|_| default_cache_dir())
}

pub fn set_cache_directory(path: Option<PathBuf>) -> Result<(), String> {
    validate_cache_directory(path.as_deref())?;
    let mut configured = CACHE_DIRECTORY
        .get_or_init(|| RwLock::new(None))
        .write()
        .map_err(|error| error.to_string())?;
    *configured = path;
    Ok(())
}

pub fn validate_cache_directory(path: Option<&std::path::Path>) -> Result<(), String> {
    use std::io::Write;
    use std::sync::atomic::Ordering;

    static PROBE_COUNTER: AtomicU64 = AtomicU64::new(0);

    if let Some(dir) = path {
        std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
        let probe = dir.join(format!(
            ".kagari-cache-write-{}.{}",
            std::process::id(),
            PROBE_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let mut owns_probe = false;
        let result = (|| {
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&probe).map_err(|error| error.to_string())?;
            owns_probe = true;
            file.write_all(&[]).map_err(|error| error.to_string())?;
            file.sync_all().map_err(|error| error.to_string())
        })();
        if owns_probe {
            if let Err(error) = std::fs::remove_file(&probe) {
                if result.is_ok() {
                    return Err(format!("could not remove cache directory probe: {error}"));
                }
            }
        }
        result?;
    }
    Ok(())
}

fn default_cache_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join("Library/Caches/Kagari/FrameCache");
        }
        std::env::temp_dir().join(format!("kagari_frame_cache_{}", std::process::id()))
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(cache_home) =
            std::env::var_os("XDG_CACHE_HOME").filter(|path| PathBuf::from(path).is_absolute())
        {
            return PathBuf::from(cache_home).join("kagari/frame-cache");
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(".cache/kagari/frame-cache");
        }
        std::env::temp_dir().join(format!("kagari_frame_cache_{}", std::process::id()))
    }
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(".cache/kagari/frame-cache");
        }
        std::env::temp_dir().join(format!("kagari_frame_cache_{}", std::process::id()))
    }
    #[cfg(not(unix))]
    std::env::temp_dir().join("kagari_frame_cache")
}

fn cache_dir() -> PathBuf {
    let dir = cache_directory();
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        if dir == default_cache_dir() && !dir.exists() {
            let _ = std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&dir);
        }
        if dir == default_cache_dir() {
            let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
        } else {
            let _ = std::fs::create_dir_all(&dir);
        }
    }
    #[cfg(not(unix))]
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn frame_path(key: &(u32, u64), namespace: u128) -> PathBuf {
    frame_path_in_dir(&cache_dir(), key, namespace)
}

fn frame_path_in_dir(dir: &std::path::Path, key: &(u32, u64), namespace: u128) -> PathBuf {
    if namespace == 0 {
        dir.join(format!("frame_{}_{}.rgba", key.0, key.1))
    } else {
        dir.join(format!("frame_{namespace:032x}_{}_{}.rgba", key.0, key.1))
    }
}

fn frame_content_hash(width: u32, height: u32, pixels: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in width.to_le_bytes().into_iter().chain(height.to_le_bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    for byte in pixels {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Current total bytes on disk across all cached frames.
pub fn current_disk_usage() -> u64 {
    current_disk_usage_in(&cache_dir())
}

fn current_disk_usage_in(dir: &std::path::Path) -> u64 {
    let mut total: u64 = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.contains(".rgba.tmp.") {
                let is_stale = meta
                    .modified()
                    .ok()
                    .and_then(|modified| modified.elapsed().ok())
                    .is_some_and(|age| age > std::time::Duration::from_secs(24 * 60 * 60));
                if is_stale {
                    let _ = std::fs::remove_file(path);
                }
            } else if path
                .extension()
                .is_some_and(|extension| extension == "rgba")
            {
                total = total.saturating_add(meta.len());
            }
        }
    }
    total
}

/// Evict oldest files (by mtime) until disk usage fits within the budget.
fn evict_if_over_budget(
    dir: &std::path::Path,
    new_frame_bytes: u64,
    replacing_path: &std::path::Path,
) {
    let budget = MAX_DISK_BYTES.load(Ordering::Relaxed);
    let current = current_disk_usage_in(dir);
    let replaced_bytes = std::fs::metadata(replacing_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let projected_usage = current.saturating_sub(replaced_bytes);
    if projected_usage.saturating_add(new_frame_bytes) <= budget {
        return;
    }
    let mut files: Vec<(PathBuf, u64)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path != replacing_path && path.extension().is_some_and(|e| e == "rgba") {
                if let Ok(meta) = entry.metadata() {
                    let mtime = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    files.push((path, mtime));
                }
            }
        }
    }
    files.sort_by_key(|(_, mtime)| *mtime);
    let mut freed: u64 = 0;
    let needed = projected_usage
        .saturating_add(new_frame_bytes)
        .saturating_sub(budget);
    for (path, _) in &files {
        if freed >= needed {
            break;
        }
        if let Ok(meta) = std::fs::metadata(path) {
            freed += meta.len();
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Write frame pixels to disk. Returns true on success.
pub fn write_frame(key: &(u32, u64), pixels: &[u8], width: u32, height: u32) -> bool {
    write_frame_in_namespace(key, 0, pixels, width, height)
}

pub fn write_frame_in_namespace(
    key: &(u32, u64),
    namespace: u128,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> bool {
    let _writer = DISK_WRITE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = cache_dir();
    let mut lock_options = std::fs::OpenOptions::new();
    lock_options
        .read(true)
        .write(true)
        .create(true)
        .truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        lock_options.mode(0o600);
    }
    let lock_file = match lock_options.open(dir.join(".budget.lock")) {
        Ok(file) if file.lock().is_ok() => file,
        _ => return false,
    };
    let Some(pixel_bytes) = super::expected_pixel_bytes(width, height) else {
        return false;
    };
    if pixels.len() != pixel_bytes {
        return false;
    }
    let Some(frame_bytes) = pixel_bytes.checked_add(16) else {
        return false;
    };
    let frame_bytes = frame_bytes as u64;
    let budget = MAX_DISK_BYTES.load(Ordering::Relaxed);
    if frame_bytes > budget {
        return false;
    }
    let path = frame_path_in_dir(&dir, key, namespace);
    evict_if_over_budget(&dir, frame_bytes, &path);
    let replaced_bytes = std::fs::metadata(&path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    if current_disk_usage_in(&dir)
        .saturating_sub(replaced_bytes)
        .saturating_add(frame_bytes)
        > budget
    {
        return false;
    }

    let tmp_path = path.with_extension(format!(
        "rgba.tmp.{}.{}",
        std::process::id(),
        WRITE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let mut owns_temporary = false;
    let write_result = (|| {
        use std::io::Write;

        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(&tmp_path).ok()?;
        owns_temporary = true;
        let mut writer = std::io::BufWriter::new(file);
        writer.write_all(&width.to_le_bytes()).ok()?;
        writer.write_all(&height.to_le_bytes()).ok()?;
        writer
            .write_all(&frame_content_hash(width, height, pixels).to_le_bytes())
            .ok()?;
        writer.write_all(pixels).ok()?;
        writer.flush().ok()?;
        writer.get_ref().sync_all().ok()?;
        drop(writer);
        crate::core::atomic_file::replace_file(&tmp_path, &path).ok()?;
        owns_temporary = false;
        Some(())
    })();
    if write_result.is_none() && owns_temporary {
        let _ = std::fs::remove_file(&tmp_path);
    }
    drop(lock_file);
    write_result.is_some()
}

/// Read frame pixels from disk. Returns (pixels, width, height) on success.
pub fn read_frame(key: &(u32, u64)) -> Option<(Vec<u8>, u32, u32)> {
    read_frame_in_namespace(key, 0)
}

pub fn read_frame_in_namespace(key: &(u32, u64), namespace: u128) -> Option<(Vec<u8>, u32, u32)> {
    read_frame_in_namespace_with_limit(key, namespace, 512 * 1024 * 1024)
}

pub fn read_frame_in_namespace_with_limit(
    key: &(u32, u64),
    namespace: u128,
    max_pixel_bytes: usize,
) -> Option<(Vec<u8>, u32, u32)> {
    use std::io::Read;
    let path = frame_path(key, namespace);
    let result = (|| {
        let mut file = std::fs::File::open(&path).ok()?;
        let file_len = file.metadata().ok()?.len();
        if file_len < 16 || file_len > (max_pixel_bytes as u64).saturating_add(16) {
            return None;
        }
        let mut header = [0u8; 8];
        file.read_exact(&mut header).ok()?;
        let width = u32::from_le_bytes(header[0..4].try_into().ok()?);
        let height = u32::from_le_bytes(header[4..8].try_into().ok()?);
        if width == 0 || height == 0 {
            return None;
        }
        let pixel_bytes = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))?;
        let expected = pixel_bytes.checked_add(8)?.checked_add(8)?;
        if expected as u64 != file_len || pixel_bytes > max_pixel_bytes {
            return None;
        }
        let mut checksum_bytes = [0u8; 8];
        file.read_exact(&mut checksum_bytes).ok()?;
        let checksum = u64::from_le_bytes(checksum_bytes);
        let mut pixels = Vec::new();
        pixels.try_reserve_exact(pixel_bytes).ok()?;
        pixels.resize(pixel_bytes, 0);
        file.read_exact(&mut pixels).ok()?;
        if checksum != frame_content_hash(width, height, &pixels) {
            return None;
        }
        Some((pixels, width, height))
    })();
    if result.is_none() {
        let _ = std::fs::remove_file(path);
    }
    result
}

/// Remove a frame from disk.
pub fn remove_frame(key: &(u32, u64)) {
    remove_frame_in_namespace(key, 0);
}

pub fn remove_frame_in_namespace(key: &(u32, u64), namespace: u128) {
    let path = frame_path(key, namespace);
    let _ = std::fs::remove_file(&path);
}

pub fn clear_namespace(namespace: u128) {
    let prefix = format!("frame_{namespace:032x}_");
    if let Ok(entries) = std::fs::read_dir(cache_dir()) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(&prefix) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

/// Remove cache frames without deleting the user-selected cache directory or
/// any unrelated files that happen to be stored beside the frames.
pub fn clear_all() {
    let dir = cache_dir();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("frame_")
            && (name.ends_with(".rgba") || name.contains(".rgba.tmp."))
            && entry.file_type().is_ok_and(|kind| kind.is_file())
        {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}
