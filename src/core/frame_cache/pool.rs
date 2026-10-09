use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub(crate) fn expected_pixel_bytes(width: u32, height: u32) -> Option<usize> {
    if width == 0 || height == 0 {
        return None;
    }
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
}

/// A monotonically increasing version counter.
pub(crate) static GLOBAL_CACHE_VERSION: AtomicU64 = AtomicU64::new(1);

/// Bump the global cache version — call this after every `history.commit`.
pub fn bump_version() -> u64 {
    GLOBAL_CACHE_VERSION.fetch_add(1, Ordering::SeqCst) + 1
}

/// Read the current version without bumping.
pub fn current_version() -> u64 {
    GLOBAL_CACHE_VERSION.load(Ordering::SeqCst)
}

/// Thread-safe reusable buffer pool for RGBA pixel vectors.
#[allow(dead_code)]
pub struct PixelBufferPool {
    pool: std::sync::Mutex<Vec<Vec<u8>>>,
}

#[allow(dead_code)]
impl Default for PixelBufferPool {
    fn default() -> Self {
        Self::new()
    }
}

impl PixelBufferPool {
    pub fn new() -> Self {
        Self {
            pool: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Acquire a buffer with capacity for `size_bytes`.
    pub fn acquire(&self, size_bytes: usize) -> Vec<u8> {
        let mut pool = self.pool.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(mut buf) = pool.pop() {
            buf.clear();
            if buf.capacity() < size_bytes {
                buf.reserve(size_bytes - buf.capacity());
            }
            buf
        } else {
            Vec::with_capacity(size_bytes)
        }
    }

    /// Recycle a vector back into the pool.
    pub fn recycle(&self, mut buf: Vec<u8>) {
        buf.clear();
        let mut pool = self.pool.lock().unwrap_or_else(|e| e.into_inner());
        if pool.len() < 64 {
            pool.push(buf);
        }
    }

    /// Number of recycled buffers currently in the pool.
    pub fn len(&self) -> usize {
        self.pool.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A single cached frame entry: raw RGBA pixel bytes for one frame at one version.
#[derive(Clone)]
pub struct CacheEntry {
    pub version: u64,
    pub width: u32,
    pub height: u32,
    pub pixels: Arc<Vec<u8>>,
    pub lru_stamp: u64,
}

static LRU_CLOCK: AtomicU64 = AtomicU64::new(1);
static CACHE_INSTANCE_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(crate) fn next_disk_namespace() -> u128 {
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or_default();
    let process = std::process::id() as u64;
    let session = time ^ process.rotate_left(17);
    let instance = CACHE_INSTANCE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    ((session as u128) << 64) | instance as u128
}

pub(crate) fn lru_tick() -> u64 {
    LRU_CLOCK.fetch_add(1, Ordering::Relaxed)
}
