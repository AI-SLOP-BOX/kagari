//! Persistent disk cache manager with byte budget and LRU eviction.

use super::types::{DiskCacheMetadata, DiskCacheWrite};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Intelligent persistent disk cache manager with strict byte budget and disk backing.
pub struct PersistentDiskCache {
    pub budget_bytes: usize,
    pub current_bytes: usize,
    pub entries: HashMap<u64, (DiskCacheMetadata, Vec<u8>)>,
    pub cache_dir: Option<PathBuf>,
    pub(crate) disk_inventory: HashMap<u64, DiskCacheMetadata>,
    pub(crate) disk_bytes: usize,
}

impl PersistentDiskCache {
    pub fn new(budget_bytes: usize) -> Self {
        Self {
            budget_bytes,
            current_bytes: 0,
            entries: HashMap::new(),
            cache_dir: None,
            disk_inventory: HashMap::new(),
            disk_bytes: 0,
        }
    }

    pub fn with_directory<P: AsRef<Path>>(budget_bytes: usize, dir: P) -> Self {
        let dir_buf = dir.as_ref().to_path_buf();
        let _ = fs::create_dir_all(&dir_buf);
        let mut cache = Self {
            budget_bytes,
            current_bytes: 0,
            entries: HashMap::new(),
            cache_dir: Some(dir_buf),
            disk_inventory: HashMap::new(),
            disk_bytes: 0,
        };
        cache.scan_disk_inventory();
        cache.evict_disk_for_bytes(0);
        cache
    }

    fn scan_disk_inventory(&mut self) {
        let Some(dir) = self.cache_dir.as_ref() else {
            return;
        };
        let Ok(files) = fs::read_dir(dir) else {
            return;
        };
        for file in files.flatten() {
            let metadata_path = file.path();
            if metadata_path.extension().and_then(|ext| ext.to_str()) != Some("meta") {
                continue;
            }
            let parsed = fs::metadata(&metadata_path)
                .ok()
                .filter(|metadata| metadata.len() <= 1024 * 1024)
                .and_then(|_| fs::read_to_string(&metadata_path).ok())
                .and_then(|json| serde_json::from_str::<DiskCacheMetadata>(&json).ok())
                .filter(|metadata| {
                    metadata_path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .and_then(|stem| u64::from_str_radix(stem, 16).ok())
                        == Some(metadata.cache_key)
                })
                .filter(|metadata| {
                    let frame_path = dir.join(format!("{:016x}.cache", metadata.cache_key));
                    fs::metadata(frame_path)
                        .ok()
                        .is_some_and(|frame| frame.len() == metadata.size_bytes as u64)
                });
            if let Some(metadata) = parsed {
                self.disk_bytes = self.disk_bytes.saturating_add(metadata.size_bytes);
                self.disk_inventory.insert(metadata.cache_key, metadata);
            } else {
                let _ = fs::remove_file(metadata_path);
            }
        }
        if let Ok(files) = fs::read_dir(dir) {
            for file in files.flatten() {
                let frame_path = file.path();
                let extension = frame_path.extension().and_then(|ext| ext.to_str());
                if extension == Some("tmp") {
                    let _ = fs::remove_file(frame_path);
                    continue;
                }
                if extension != Some("cache") {
                    continue;
                }
                let key = frame_path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .and_then(|stem| u64::from_str_radix(stem, 16).ok());
                if key.is_none_or(|key| !self.disk_inventory.contains_key(&key)) {
                    let _ = fs::remove_file(frame_path);
                }
            }
        }
    }

    fn evict_disk_for_bytes(&mut self, needed_bytes: usize) {
        while self
            .disk_bytes
            .checked_add(needed_bytes)
            .is_none_or(|total| total > self.budget_bytes)
            && !self.disk_inventory.is_empty()
        {
            let Some(oldest_key) = self
                .disk_inventory
                .iter()
                .min_by_key(|(&key, metadata)| (metadata.last_access_epoch, key))
                .map(|(&key, _)| key)
            else {
                break;
            };
            if let Some(metadata) = self.disk_inventory.remove(&oldest_key) {
                self.disk_bytes = self.disk_bytes.saturating_sub(metadata.size_bytes);
                if let Some(dir) = &self.cache_dir {
                    let _ = fs::remove_file(dir.join(format!("{oldest_key:016x}.cache")));
                    let _ = fs::remove_file(dir.join(format!("{oldest_key:016x}.meta")));
                }
            }
        }
    }

    fn discard_disk_entry(&mut self, key: u64) {
        if let Some(metadata) = self.disk_inventory.remove(&key) {
            self.disk_bytes = self.disk_bytes.saturating_sub(metadata.size_bytes);
        }
        if let Some(dir) = &self.cache_dir {
            let _ = fs::remove_file(dir.join(format!("{key:016x}.cache")));
            let _ = fs::remove_file(dir.join(format!("{key:016x}.meta")));
        }
    }

    /// Computes unique 64-bit content hash key for a composition frame.
    pub fn compute_cache_key(comp_id: &str, frame: u32, project_version: u64) -> u64 {
        let mut h = 0xcbf29ce484222325u64; // FNV-1a
        for b in comp_id.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h ^= frame as u64;
        h = h.wrapping_mul(0x100000001b3);
        h ^= project_version;
        h = h.wrapping_mul(0x100000001b3);
        h
    }

    /// Helper to evict oldest LRU entries until `needed_bytes` fits within budget.
    fn evict_for_bytes(&mut self, needed_bytes: usize) {
        while self.current_bytes + needed_bytes > self.budget_bytes && !self.entries.is_empty() {
            let oldest_key = self
                .entries
                .iter()
                .min_by_key(|(_, (meta, _))| meta.last_access_epoch)
                .map(|(&k, _)| k);

            if let Some(k) = oldest_key {
                if let Some((meta, _)) = self.entries.remove(&k) {
                    self.current_bytes = self.current_bytes.saturating_sub(meta.size_bytes);
                    if let Some(ref dir) = self.cache_dir {
                        let file_path = dir.join(format!("{:016x}.cache", k));
                        let meta_path = dir.join(format!("{:016x}.meta", k));
                        let _ = fs::remove_file(file_path);
                        let _ = fs::remove_file(meta_path);
                    }
                    if let Some(disk_meta) = self.disk_inventory.remove(&k) {
                        self.disk_bytes = self.disk_bytes.saturating_sub(disk_meta.size_bytes);
                    }
                }
            } else {
                break;
            }
        }
    }

    /// Retrieves frame from disk cache if present, evicting LRU if needed when loading from disk.
    pub fn get_frame(&mut self, key: u64, current_epoch: u64) -> Option<&[u8]> {
        if self.entries.contains_key(&key) {
            let (meta, data) = self.entries.get_mut(&key).unwrap();
            meta.last_access_epoch = current_epoch;
            if let Some(disk_meta) = self.disk_inventory.get_mut(&key) {
                disk_meta.last_access_epoch = current_epoch;
            }
            return Some(data.as_slice());
        }

        if let Some(dir) = self.cache_dir.clone() {
            if !self.disk_inventory.contains_key(&key) {
                return None;
            }
            // Attempt reload from persistent disk file if present
            let file_path = dir.join(format!("{:016x}.cache", key));
            let meta_path = dir.join(format!("{:016x}.meta", key));
            let file_size = fs::metadata(&file_path).ok().map(|meta| meta.len());
            if file_size.is_some_and(|size| size > self.budget_bytes as u64) {
                self.discard_disk_entry(key);
                return None;
            }
            if let Some(file_size) = file_size {
                use std::io::Read;
                let mut bytes = Vec::with_capacity(file_size as usize);
                let read_result = fs::File::open(&file_path).and_then(|file| {
                    file.take(self.budget_bytes.saturating_add(1) as u64)
                        .read_to_end(&mut bytes)
                });
                if read_result.is_err() || bytes.len() as u64 != file_size {
                    self.discard_disk_entry(key);
                    return None;
                }
                let size = bytes.len();
                if size <= self.budget_bytes {
                    // Try parsing metadata; if metadata file is missing or corrupted, discard corrupted cache
                    let meta_res = fs::metadata(&meta_path)
                        .ok()
                        .filter(|meta| meta.len() <= 1024 * 1024)
                        .and_then(|_| fs::read_to_string(&meta_path).ok())
                        .and_then(|json| serde_json::from_str::<DiskCacheMetadata>(&json).ok());

                    if let Some(mut meta) =
                        meta_res.filter(|meta| meta.cache_key == key && meta.size_bytes == size)
                    {
                        meta.last_access_epoch = current_epoch;
                        if let Some(disk_meta) = self.disk_inventory.get_mut(&key) {
                            disk_meta.last_access_epoch = current_epoch;
                        }
                        self.evict_for_bytes(size);
                        self.current_bytes += size;
                        self.entries.insert(key, (meta, bytes));
                        return self.entries.get(&key).map(|(_, d)| d.as_slice());
                    } else {
                        // Corrupted cache: remove both files
                        self.discard_disk_entry(key);
                    }
                } else {
                    // A file larger than the current budget can never become a valid entry.
                    self.discard_disk_entry(key);
                }
            }
        }

        None
    }

    /// Stores a frame in the disk cache, evicting oldest frames if over budget.
    /// Rejects entries that exceed total budget to prevent budget overflow.
    #[allow(clippy::too_many_arguments)]
    pub fn put_frame(
        &mut self,
        key: u64,
        comp_id: String,
        frame: u32,
        width: u32,
        height: u32,
        data: Vec<u8>,
        current_epoch: u64,
    ) -> Result<(), std::io::Error> {
        self.put_frame_record(DiskCacheWrite {
            cache_key: key,
            comp_id,
            frame,
            width,
            height,
            data,
            current_epoch,
        })
    }

    pub fn put_frame_record(&mut self, write: DiskCacheWrite) -> Result<(), std::io::Error> {
        let DiskCacheWrite {
            cache_key: key,
            comp_id,
            frame,
            width,
            height,
            data,
            current_epoch,
        } = write;
        let size = data.len();

        // If the single frame exceeds total cache budget, reject insertion
        if size > self.budget_bytes {
            return Ok(());
        }

        // If key already exists, remove it and subtract its size first
        if let Some((old_meta, _)) = self.entries.remove(&key) {
            self.current_bytes = self.current_bytes.saturating_sub(old_meta.size_bytes);
        }
        if let Some(old_meta) = self.disk_inventory.remove(&key) {
            self.disk_bytes = self.disk_bytes.saturating_sub(old_meta.size_bytes);
        }
        if let Some(dir) = &self.cache_dir {
            let _ = fs::remove_file(dir.join(format!("{key:016x}.cache")));
            let _ = fs::remove_file(dir.join(format!("{key:016x}.meta")));
        }

        // Evict LRU entries if adding new size exceeds budget
        self.evict_for_bytes(size);
        self.evict_disk_for_bytes(size);

        let meta = DiskCacheMetadata {
            cache_key: key,
            comp_id,
            frame,
            width,
            height,
            size_bytes: size,
            last_access_epoch: current_epoch,
        };

        // Persist to disk file atomically if directory is configured
        if let Some(ref dir) = self.cache_dir {
            let file_path = dir.join(format!("{:016x}.cache", key));
            let meta_path = dir.join(format!("{:016x}.meta", key));
            let tmp_file_path = dir.join(format!("{:016x}.cache.tmp", key));
            let tmp_meta_path = dir.join(format!("{:016x}.meta.tmp", key));

            // Write to temporary files first
            if let Err(e) = fs::write(&tmp_file_path, &data) {
                let _ = fs::remove_file(&tmp_file_path);
                return Err(e);
            }
            let meta_json = serde_json::to_string(&meta)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            if let Err(e) = fs::write(&tmp_meta_path, meta_json) {
                let _ = fs::remove_file(&tmp_file_path);
                let _ = fs::remove_file(&tmp_meta_path);
                return Err(e);
            }

            // Atomic rename to final file destinations
            if let Err(e) = fs::rename(&tmp_file_path, &file_path) {
                let _ = fs::remove_file(&tmp_file_path);
                let _ = fs::remove_file(&tmp_meta_path);
                return Err(e);
            }
            if let Err(e) = fs::rename(&tmp_meta_path, &meta_path) {
                let _ = fs::remove_file(&file_path);
                let _ = fs::remove_file(&tmp_meta_path);
                return Err(e);
            }
            self.disk_bytes = self.disk_bytes.saturating_add(size);
            self.disk_inventory.insert(key, meta.clone());
        }

        self.current_bytes += size;
        self.entries.insert(key, (meta, data));
        Ok(())
    }

    /// Removes every cached frame belonging to one composition.
    pub fn invalidate_comp(&mut self, comp_id: &str) -> usize {
        let mut keys = self
            .disk_inventory
            .iter()
            .filter_map(|(&key, meta)| (meta.comp_id == comp_id).then_some(key))
            .collect::<Vec<_>>();
        for (&key, (meta, _)) in &self.entries {
            if meta.comp_id == comp_id && !keys.contains(&key) {
                keys.push(key);
            }
        }

        for key in &keys {
            if let Some((meta, _)) = self.entries.remove(key) {
                self.current_bytes = self.current_bytes.saturating_sub(meta.size_bytes);
            }
            if let Some(meta) = self.disk_inventory.remove(key) {
                self.disk_bytes = self.disk_bytes.saturating_sub(meta.size_bytes);
            }
            if let Some(dir) = &self.cache_dir {
                let _ = fs::remove_file(dir.join(format!("{key:016x}.cache")));
                let _ = fs::remove_file(dir.join(format!("{key:016x}.meta")));
            }
        }
        keys.len()
    }

    /// Clears all memory and disk cache files.
    pub fn clear(&mut self) {
        if let Some(ref dir) = self.cache_dir {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if matches!(
                        entry.path().extension().and_then(|s| s.to_str()),
                        Some("cache") | Some("meta") | Some("tmp")
                    ) {
                        let _ = fs::remove_file(entry.path());
                    }
                }
            }
        }
        self.entries.clear();
        self.current_bytes = 0;
        self.disk_inventory.clear();
        self.disk_bytes = 0;
    }
}
