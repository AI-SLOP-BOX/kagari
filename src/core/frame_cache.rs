//! SQLite MVCC-inspired versioned frame render cache.
//!
//! Every time the project changes (a new history commit is made), the cache
//! version increments. Reads always see a consistent snapshot; writes for the
//! new version happen concurrently without invalidating in-progress reads.
//!
//! Includes strict RAM byte-memory limit guards (default 512 MB) & LRU eviction
//! policy to prevent OS Out-Of-Memory (OOM) crashes under 4K/8K video preview.

pub mod disk_cache;
pub mod pool;

use std::collections::HashMap;
use std::sync::Arc;

pub use disk_cache::*;
pub use pool::*;

/// The frame cache. Key is `(frame_index, cache_version)`.
pub struct FrameCache {
    entries: HashMap<(u32, u64), CacheEntry>,
    /// Maximum number of entries before GC triggers automatically.
    max_entries: usize,
    /// Maximum memory limit in bytes for cached RGBA pixel buffers (e.g. 512MB).
    pub max_memory_bytes: usize,
    /// Currently allocated pixel bytes in memory.
    pub current_memory_bytes: usize,
    /// Layer indices that have been modified since last commit.
    dirty_layers: std::collections::HashSet<usize>,
    /// Composition IDs that contain dirty layers.
    dirty_comps: std::collections::HashSet<String>,
    /// Mapping from frame index to the set of layer indices used to render that frame.
    /// Used for partial invalidation: allows reusing previous-version frames when
    /// only some layers have changed.
    frame_layers: std::collections::HashMap<(u32, u64), std::collections::HashSet<usize>>,
    /// When set, this cache ignores the global version counter and uses this
    /// fixed version instead. This prevents parallel tests from polluting each
    /// other's version state.
    version_override: Option<u64>,
    disk_namespace: u128,
}

impl FrameCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
            max_memory_bytes: 512 * 1024 * 1024, // 512 MB default
            current_memory_bytes: 0,
            dirty_layers: std::collections::HashSet::new(),
            dirty_comps: std::collections::HashSet::new(),
            frame_layers: std::collections::HashMap::new(),
            version_override: None,
            disk_namespace: next_disk_namespace(),
        }
    }

    /// Create a cache with a fixed version for isolated testing.
    /// This bypasses the global version counter entirely, preventing
    /// parallel test pollution.
    pub fn with_version(max_entries: usize, version: u64) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
            max_memory_bytes: 512 * 1024 * 1024,
            current_memory_bytes: 0,
            dirty_layers: std::collections::HashSet::new(),
            dirty_comps: std::collections::HashSet::new(),
            frame_layers: std::collections::HashMap::new(),
            version_override: Some(version),
            disk_namespace: next_disk_namespace(),
        }
    }

    /// Return the effective version: either the override or the global counter.
    fn effective_version(&self) -> u64 {
        self.version_override.unwrap_or_else(current_version)
    }

    /// Mark a layer as dirty (modified). Only frames containing this layer need re-rendering.
    pub fn mark_layer_dirty(&mut self, layer_idx: usize) {
        self.dirty_layers.insert(layer_idx);
    }

    /// Mark a composition as dirty (contains modified layers).
    pub fn mark_comp_dirty(&mut self, comp_id: &str) {
        self.dirty_comps.insert(comp_id.to_string());
    }

    /// Mark layers as dirty for partial invalidation.
    pub fn invalidate_layers(&mut self, layer_indices: &[usize]) {
        for &idx in layer_indices {
            self.dirty_layers.insert(idx);
        }
    }

    /// Check if a specific layer is dirty.
    pub fn is_layer_dirty(&self, layer_idx: usize) -> bool {
        self.dirty_layers.contains(&layer_idx)
    }

    /// Check if a frame should be invalidated based on its layer composition.
    /// A frame is dirty if any of the layers used to render it are marked dirty.
    pub fn is_frame_dirty(&self, _frame: u32, layers: &[usize]) -> bool {
        for &layer_idx in layers {
            if self.is_layer_dirty(layer_idx) {
                return true;
            }
        }
        false
    }

    /// Clear all dirty flags (call after a full re-render or commit).
    pub fn clear_dirty(&mut self) {
        self.dirty_layers.clear();
        self.dirty_comps.clear();
    }

    /// Get the set of dirty layer indices.
    pub fn dirty_layers(&self) -> &std::collections::HashSet<usize> {
        &self.dirty_layers
    }

    /// Try to retrieve a cached frame for the current global version (updates LRU timestamp).
    /// Checks RAM first, then falls back to disk cache on miss.
    /// Entries are only valid for the exact current project version.
    pub fn get(&mut self, frame: u32) -> Option<&CacheEntry> {
        self.get_with_layers(frame, &[])
    }

    /// Try to retrieve a cached frame for the current project version.
    /// `frame_layers` is retained as provenance for future content-addressed
    /// invalidation, but never authorizes cross-version reuse by itself.
    pub fn get_with_layers(&mut self, frame: u32, frame_layers: &[usize]) -> Option<&CacheEntry> {
        let ver = self.effective_version();
        let key = (frame, ver);

        // Check if we have a cached entry for this frame in current version
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.lru_stamp = lru_tick();
            return self.entries.get(&key);
        }

        // Check disk cache on miss
        if let Some((pixels, w, h)) = disk_cache::read_frame_in_namespace_with_limit(
            &key,
            self.disk_namespace,
            self.max_memory_bytes,
        ) {
            if pixels.len() > self.max_memory_bytes {
                return None;
            }
            if self.current_memory_bytes.saturating_add(pixels.len()) > self.max_memory_bytes {
                self.collect_garbage();
                if self.current_memory_bytes.saturating_add(pixels.len()) > self.max_memory_bytes {
                    return None;
                }
            }
            self.current_memory_bytes += pixels.len();
            self.entries.insert(
                key,
                CacheEntry {
                    version: ver,
                    width: w,
                    height: h,
                    pixels: Arc::new(pixels),
                    lru_stamp: lru_tick(),
                },
            );
            self.frame_layers
                .insert(key, frame_layers.iter().cloned().collect());
            // Evict from disk once loaded into RAM
            disk_cache::remove_frame_in_namespace(&key, self.disk_namespace);
            return self.entries.get(&key);
        }
        None
    }

    /// Immutable check if frame is cached without updating LRU timestamp.
    pub fn is_cached(&self, frame: u32) -> bool {
        let ver = self.effective_version();
        self.entries.contains_key(&(frame, ver))
    }

    /// Returns the number of cached frames currently stored.
    pub fn cached_count(&self) -> usize {
        self.entries.len()
    }

    /// Store a rendered frame for the current global version.
    pub fn insert(&mut self, frame: u32, width: u32, height: u32, pixels: Vec<u8>) {
        self.insert_with_layers(frame, width, height, pixels, &[]);
    }

    /// Store a rendered frame and record which layers contributed to it.
    /// Layer provenance is metadata only until a complete content-addressed
    /// render key can prove that all other render inputs are unchanged.
    pub fn insert_with_layers(
        &mut self,
        frame: u32,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
        layer_indices: &[usize],
    ) {
        let Some(expected_bytes) = expected_pixel_bytes(width, height) else {
            return;
        };
        if pixels.len() != expected_bytes {
            return;
        }
        let ver = self.effective_version();
        let key = (frame, ver);
        let bytes_size = pixels.len();

        // If replacing existing entry, subtract old size first
        if let Some(old) = self.entries.remove(&key) {
            self.current_memory_bytes = self.current_memory_bytes.saturating_sub(old.pixels.len());
        }
        self.frame_layers.remove(&key);

        self.current_memory_bytes += bytes_size;
        self.entries.insert(
            key,
            CacheEntry {
                version: ver,
                width,
                height,
                pixels: Arc::new(pixels),
                lru_stamp: lru_tick(),
            },
        );

        if !layer_indices.is_empty() {
            self.frame_layers
                .insert(key, layer_indices.iter().copied().collect());
        }

        if self.entries.len() > self.max_entries
            || self.current_memory_bytes > self.max_memory_bytes
        {
            self.collect_garbage();
        }
    }

    /// Record which layers contributed to an already-inserted frame.
    pub fn set_frame_layers(&mut self, frame: u32, layer_indices: &[usize]) {
        let key = (frame, self.effective_version());
        if !layer_indices.is_empty() {
            self.frame_layers
                .insert(key, layer_indices.iter().copied().collect());
        }
    }

    /// Discard stale version entries and LRU evict unaccessed frames when memory budget is exceeded.
    pub fn collect_garbage(&mut self) {
        let current = self.effective_version();
        self.collect_garbage_below(current);

        if self.current_memory_bytes > self.max_memory_bytes {
            let target_memory = (self.max_memory_bytes as f64 * 0.75) as usize;
            let mut keys_by_access: Vec<((u32, u64), u64)> = self
                .entries
                .iter()
                .map(|(k, v)| (*k, v.lru_stamp))
                .collect();

            keys_by_access.sort_by_key(|(_k, accessed)| *accessed);

            for (key, _accessed) in keys_by_access {
                if self.current_memory_bytes <= target_memory {
                    break;
                }
                if let Some(removed) = self.entries.remove(&key) {
                    let _ = disk_cache::write_frame_in_namespace(
                        &key,
                        self.disk_namespace,
                        &removed.pixels,
                        removed.width,
                        removed.height,
                    );
                    self.frame_layers.remove(&key);
                    self.current_memory_bytes = self
                        .current_memory_bytes
                        .saturating_sub(removed.pixels.len());
                }
            }
        }
    }

    /// Discard all cache entries whose version is strictly older than `target_version`.
    pub fn collect_garbage_below(&mut self, target_version: u64) {
        let mut evicted: Vec<_> = self
            .entries
            .keys()
            .filter(|(_, version)| *version < target_version)
            .copied()
            .collect();
        evicted.sort_unstable();
        for key in evicted {
            let Some(entry) = self.entries.remove(&key) else {
                continue;
            };
            let _ = disk_cache::write_frame_in_namespace(
                &key,
                self.disk_namespace,
                &entry.pixels,
                entry.width,
                entry.height,
            );
            self.frame_layers.remove(&key);
            self.current_memory_bytes =
                self.current_memory_bytes.saturating_sub(entry.pixels.len());
        }
    }

    /// Discard the entire cache (RAM + disk).
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
        self.frame_layers.clear();
        self.current_memory_bytes = 0;
        disk_cache::clear_namespace(self.disk_namespace);
    }

    /// How many entries are currently held (all versions combined).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many entries exist for the current version.
    pub fn current_version_len(&self) -> usize {
        let ver = self.effective_version();
        self.entries.keys().filter(|(_, v)| *v == ver).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_miss_on_new_version() {
        let mut cache = FrameCache::new(256);
        let pixels = vec![0u8; 4];
        const FRAME: u32 = 2_000_001;

        cache.insert(FRAME, 1, 1, pixels.clone());
        let insert_ver = cache
            .entries
            .keys()
            .find(|(f, _)| *f == FRAME)
            .map(|(_, v)| *v)
            .expect("entry should exist after insert");

        bump_version();
        if current_version() != insert_ver {
            assert!(
                cache.get(FRAME).is_none(),
                "get() should miss when current version != insert version"
            );
        }
    }

    #[test]
    fn test_layer_metadata_never_reuses_cross_version_pixels() {
        let mut cache = FrameCache::new(16);
        const FRAME: u32 = 2_000_002;
        cache.insert_with_layers(FRAME, 1, 1, vec![17, 34, 51, 255], &[0, 1]);
        let inserted_version = cache
            .entries
            .keys()
            .find(|(frame, _)| *frame == FRAME)
            .map(|(_, version)| *version)
            .expect("inserted frame must exist");

        while current_version() == inserted_version {
            bump_version();
        }

        assert!(
            cache.get_with_layers(FRAME, &[0, 1]).is_none(),
            "layer indices alone cannot prove that resolution, color, effects, sources, and backend match"
        );
    }

    #[test]
    fn test_gc_removes_old_versions() {
        let mut cache = FrameCache::new(1024);
        let pixels = Arc::new(vec![0u8; 4]);
        const FRAME: u32 = 1_000_002;

        let old_ver = 1;
        let cur_ver = 2;

        cache.entries.insert(
            (FRAME, old_ver),
            CacheEntry {
                version: old_ver,
                width: 1,
                height: 1,
                pixels: pixels.clone(),
                lru_stamp: lru_tick(),
            },
        );
        cache.entries.insert(
            (FRAME, cur_ver),
            CacheEntry {
                version: cur_ver,
                width: 1,
                height: 1,
                pixels: pixels.clone(),
                lru_stamp: lru_tick(),
            },
        );

        assert_eq!(cache.len(), 2, "should have 2 versioned entries before GC");
        cache.collect_garbage_below(cur_ver);

        assert!(
            cache.entries.values().all(|e| e.version >= cur_ver),
            "all remaining entries must be >= cur_ver"
        );
        assert!(
            cache.entries.contains_key(&(FRAME, cur_ver)),
            "current version entry must survive GC"
        );
        assert!(
            !cache.entries.contains_key(&(FRAME, old_ver)),
            "old version entry must be removed by GC"
        );
    }

    #[test]
    fn invalid_pixel_shape_is_rejected_without_replacing_a_valid_frame() {
        let mut cache = FrameCache::with_version(16, 7);
        cache.insert_with_layers(3, 1, 1, vec![1, 2, 3, 255], &[4]);

        cache.insert_with_layers(3, 2, 2, vec![9, 9, 9], &[8]);
        cache.insert(4, 0, 1, vec![]);

        let entry = cache
            .get(3)
            .expect("valid entry must survive invalid writes");
        assert_eq!((entry.width, entry.height), (1, 1));
        assert_eq!(entry.pixels.as_ref(), &[1, 2, 3, 255]);
        assert_eq!(cache.cached_count(), 1);
        assert_eq!(cache.current_memory_bytes, 4);
        assert_eq!(cache.frame_layers.get(&(3, 7)).cloned(), Some([4].into()));
    }

    #[test]
    fn stale_frame_layer_metadata_is_removed_with_evicted_pixels() {
        let mut cache = FrameCache::with_version(16, 2);
        cache.insert_with_layers(9, 1, 1, vec![0, 0, 0, 255], &[2, 5]);
        assert!(cache.frame_layers.contains_key(&(9, 2)));

        cache.collect_garbage_below(3);

        assert!(!cache.entries.contains_key(&(9, 2)));
        assert!(!cache.frame_layers.contains_key(&(9, 2)));
        assert_eq!(cache.current_memory_bytes, 0);
    }

    #[test]
    fn test_lru_memory_limit_purging() {
        let mut cache = FrameCache::new(1024);
        cache.max_memory_bytes = 100;

        let pixels = vec![255u8; 40];
        cache.insert(1, 10, 1, pixels.clone());
        cache.insert(2, 10, 1, pixels.clone());
        cache.insert(3, 10, 1, pixels.clone());

        assert!(
            cache.current_memory_bytes <= 100,
            "LRU memory limit should automatically purge old entries"
        );
    }

    #[test]
    fn test_pixel_buffer_pool_recycling() {
        let pool = PixelBufferPool::new();
        let buf = pool.acquire(1024);
        assert!(buf.capacity() >= 1024);
        pool.recycle(buf);
        let recycled = pool.acquire(512);
        assert!(
            recycled.capacity() >= 1024,
            "Recycled buffer should retain its allocated capacity"
        );
    }
}

#[cfg(test)]
mod memory_bound_tests {
    use super::*;
    use std::sync::atomic::Ordering;

    fn with_isolated_version<F: FnOnce()>(f: F) {
        let saved = GLOBAL_CACHE_VERSION.load(Ordering::SeqCst);
        f();
        GLOBAL_CACHE_VERSION.store(saved, Ordering::SeqCst);
    }

    #[test]
    fn test_memory_budget_enforced_by_lru_eviction() {
        with_isolated_version(|| {
            let mut cache = FrameCache::with_version(1000, 0x00B0_D6E7);
            cache.max_memory_bytes = 1_000_000;

            let frame_bytes = 500 * 50 * 4;
            for i in 0..20u32 {
                cache.insert(i, 500, 50, vec![0u8; frame_bytes]);
                let _ = cache.get(0);
            }

            assert!(
                cache.current_memory_bytes <= cache.max_memory_bytes + frame_bytes,
                "memory must stay near budget, got {}",
                cache.current_memory_bytes
            );
            assert!(
                cache.entries.keys().any(|(f, _)| *f == 0),
                "LRU-hot frame must not be evicted"
            );
        });
    }

    #[test]
    fn test_stale_versions_are_discarded() {
        with_isolated_version(|| {
            let mut cache = FrameCache::new(100);
            cache.max_memory_bytes = usize::MAX;

            bump_version();
            cache.insert(5, 32, 32, vec![0u8; 4096]);
            assert!(cache.is_cached(5));

            bump_version();
            cache.collect_garbage();
            assert_eq!(cache.cached_count(), 0, "stale entries must be discarded");
        });
    }
}

#[cfg(test)]
mod disk_cache_tests {
    use super::disk_cache;

    #[test]
    fn disk_budget_lock_child_process() {
        let Ok(lock_path) = std::env::var("KAGARI_TEST_DISK_LOCK_PATH") else {
            return;
        };
        let started_path =
            std::path::PathBuf::from(std::env::var("KAGARI_TEST_DISK_LOCK_STARTED").unwrap());
        let acquired_path =
            std::path::PathBuf::from(std::env::var("KAGARI_TEST_DISK_LOCK_ACQUIRED").unwrap());
        std::fs::write(&started_path, []).unwrap();
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(lock_path)
            .unwrap();
        lock.lock().unwrap();
        std::fs::write(acquired_path, []).unwrap();
    }

    #[test]
    fn disk_budget_lock_serializes_separate_processes() {
        let root = std::env::temp_dir().join(format!(
            "kagari_disk_lock_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let lock_path = root.join(".budget.lock");
        let started_path = root.join("started");
        let acquired_path = root.join("acquired");
        let parent_lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .unwrap();
        parent_lock.lock().unwrap();

        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("core::frame_cache::disk_cache_tests::disk_budget_lock_child_process")
            .arg("--nocapture")
            .env("KAGARI_TEST_DISK_LOCK_PATH", &lock_path)
            .env("KAGARI_TEST_DISK_LOCK_STARTED", &started_path)
            .env("KAGARI_TEST_DISK_LOCK_ACQUIRED", &acquired_path)
            .spawn()
            .unwrap();
        let started_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !started_path.exists() && std::time::Instant::now() < started_deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            started_path.exists(),
            "child did not reach the lock attempt"
        );
        std::thread::sleep(std::time::Duration::from_millis(40));
        assert!(!acquired_path.exists(), "child bypassed the held OS lock");

        drop(parent_lock);
        let status = child.wait().unwrap();
        assert!(status.success());
        assert!(acquired_path.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_dimensions_do_not_overflow_or_panic() {
        let key = (u32::MAX, u64::MAX);
        let path = {
            let dir = std::env::temp_dir().join("kagari_frame_cache");
            std::fs::create_dir_all(&dir).unwrap();
            dir.join(format!("frame_{}_{}.rgba", key.0, key.1))
        };
        std::fs::write(&path, [0xff; 8]).unwrap();
        assert!(disk_cache::read_frame(&key).is_none());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn trailing_disk_data_is_rejected() {
        let key = (u32::MAX - 1, u64::MAX - 1);
        let path = {
            let dir = std::env::temp_dir().join("kagari_frame_cache");
            std::fs::create_dir_all(&dir).unwrap();
            dir.join(format!("frame_{}_{}.rgba", key.0, key.1))
        };
        let mut data = Vec::from(1u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&[0, 0, 0, 255, 42]);
        std::fs::write(&path, data).unwrap();
        assert!(disk_cache::read_frame(&key).is_none());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn zero_dimension_disk_frame_is_rejected_and_removed() {
        let key = (u32::MAX - 2, u64::MAX - 2);
        let dir = std::env::temp_dir().join("kagari_frame_cache");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("frame_{}_{}.rgba", key.0, key.1));
        std::fs::write(&path, [0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
        assert!(disk_cache::read_frame(&key).is_none());
        assert!(!path.exists());
    }
}
