#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::core::dynamic_link_disk_cache::*;
    use std::fs;

    #[test]
    fn test_disk_cache_put_get_and_lru_eviction() {
        // Budget for 2 frames of 100 bytes each
        let mut cache = PersistentDiskCache::new(250);

        let f0 = vec![1u8; 100];
        let f1 = vec![2u8; 100];
        let f2 = vec![3u8; 100];

        let k0 = PersistentDiskCache::compute_cache_key("comp1", 0, 1);
        let k1 = PersistentDiskCache::compute_cache_key("comp1", 1, 1);
        let k2 = PersistentDiskCache::compute_cache_key("comp1", 2, 1);

        let _ = cache.put_frame(k0, "comp1".into(), 0, 10, 10, f0, 1000);
        let _ = cache.put_frame(k1, "comp1".into(), 1, 10, 10, f1, 1001);
        assert_eq!(cache.entries.len(), 2);
        assert_eq!(cache.current_bytes, 200);

        // Storing 3rd frame must evict k0 (epoch 1000)
        let _ = cache.put_frame(k2, "comp1".into(), 2, 10, 10, f2, 1002);
        assert_eq!(cache.entries.len(), 2);
        assert_eq!(cache.current_bytes, 200);
        assert!(cache.get_frame(k0, 1003).is_none());
        assert!(cache.get_frame(k1, 1003).is_some());
        assert!(cache.get_frame(k2, 1003).is_some());
    }

    #[test]
    fn test_disk_cache_reinsertion_deducts_old_size() {
        let mut cache = PersistentDiskCache::new(200);
        let k0 = 42;
        let _ = cache.put_frame(k0, "c".into(), 0, 1, 1, vec![0u8; 100], 100);
        assert_eq!(cache.current_bytes, 100);

        // Reinsert same key with new size (80 bytes)
        let _ = cache.put_frame(k0, "c".into(), 0, 1, 1, vec![0u8; 80], 101);
        assert_eq!(cache.current_bytes, 80);
        assert_eq!(cache.entries.len(), 1);
    }

    #[test]
    fn test_disk_cache_rejects_oversized_frame() {
        let mut cache = PersistentDiskCache::new(100);
        let k0 = 99;
        // 150 bytes > 100 bytes budget
        let _ = cache.put_frame(k0, "c".into(), 0, 1, 1, vec![0u8; 150], 100);
        assert_eq!(cache.entries.len(), 0);
        assert_eq!(cache.current_bytes, 0);
    }

    #[test]
    fn test_disk_cache_filesystem_persistence_and_metadata_reload() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "ae_test_cache_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&tmp_dir);

        let mut cache = PersistentDiskCache::with_directory(500, tmp_dir.clone());
        let k0 = 12345;
        let data = vec![7u8; 64];
        let _ = cache.put_frame(k0, "comp_main".into(), 5, 1920, 1080, data.clone(), 100);

        // Clear memory cache only
        cache.entries.clear();
        cache.current_bytes = 0;

        // Reload from filesystem
        let loaded = cache.get_frame(k0, 200);
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap(), data.as_slice());

        // Check restored metadata
        let (meta, _) = cache.entries.get(&k0).unwrap();
        assert_eq!(meta.comp_id, "comp_main");
        assert_eq!(meta.frame, 5);
        assert_eq!(meta.width, 1920);
        assert_eq!(meta.height, 1080);

        cache.clear();
        assert!(!tmp_dir.join(format!("{k0:016x}.cache")).exists());
        assert!(!tmp_dir.join(format!("{k0:016x}.meta")).exists());
        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn startup_inventory_enforces_budget_and_invalidates_cold_compositions() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "kagari_cache_inventory_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&tmp_dir);
        let stale_temp = tmp_dir.join("ab.cache.tmp");
        fs::write(&stale_temp, [9u8; 32]).unwrap();
        let first = PersistentDiskCache::compute_cache_key("a", 0, 1);
        let second = PersistentDiskCache::compute_cache_key("b", 0, 1);
        {
            let mut cache = PersistentDiskCache::with_directory(200, &tmp_dir);
            cache
                .put_frame(first, "a".into(), 0, 1, 1, vec![1; 100], 1)
                .unwrap();
            cache
                .put_frame(second, "b".into(), 0, 1, 1, vec![2; 100], 2)
                .unwrap();
        }

        let mut cache = PersistentDiskCache::with_directory(100, &tmp_dir);
        assert!(!stale_temp.exists());
        assert_eq!(cache.disk_bytes, 100);
        assert!(cache.get_frame(first, 3).is_none());
        assert_eq!(cache.get_frame(second, 3), Some([2; 100].as_slice()));
        assert_eq!(cache.invalidate_comp("b"), 1);
        assert_eq!(cache.disk_bytes, 0);
        assert!(!tmp_dir.join(format!("{second:016x}.cache")).exists());
        let _ = fs::remove_dir_all(tmp_dir);
    }

    #[test]
    fn test_corrupt_metadata_is_not_accepted_as_valid_frame_metadata() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "ae_test_cache_corrupt_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let key = 0xabcdu64;
        let _ = fs::create_dir_all(&tmp_dir);
        fs::write(tmp_dir.join(format!("{key:016x}.cache")), [1u8, 2, 3]).unwrap();
        fs::write(tmp_dir.join(format!("{key:016x}.meta")), b"not-json").unwrap();

        let mut cache = PersistentDiskCache::with_directory(100, &tmp_dir);
        assert!(
            cache.get_frame(key, 1).is_none(),
            "corrupt cache metadata must be ignored"
        );
        let _ = fs::remove_dir_all(tmp_dir);
    }

    #[test]
    fn test_mismatched_metadata_size_is_discarded() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "ae_test_cache_size_mismatch_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let key = 0x1234u64;
        fs::create_dir_all(&tmp_dir).unwrap();
        fs::write(tmp_dir.join(format!("{key:016x}.cache")), [1u8, 2, 3, 4]).unwrap();
        let metadata = DiskCacheMetadata {
            cache_key: key,
            comp_id: "comp".into(),
            frame: 0,
            width: 1,
            height: 1,
            size_bytes: 99,
            last_access_epoch: 1,
        };
        fs::write(
            tmp_dir.join(format!("{key:016x}.meta")),
            serde_json::to_string(&metadata).unwrap(),
        )
        .unwrap();

        let mut cache = PersistentDiskCache::with_directory(100, &tmp_dir);
        assert!(cache.get_frame(key, 2).is_none());
        assert!(!tmp_dir.join(format!("{key:016x}.cache")).exists());
        assert!(!tmp_dir.join(format!("{key:016x}.meta")).exists());
        let _ = fs::remove_dir_all(tmp_dir);
    }

    #[test]
    fn test_mismatched_metadata_key_is_discarded() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "ae_test_cache_key_mismatch_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let key = 0x5678u64;
        fs::create_dir_all(&tmp_dir).unwrap();
        fs::write(tmp_dir.join(format!("{key:016x}.cache")), [5u8, 6, 7]).unwrap();
        let metadata = DiskCacheMetadata {
            cache_key: key + 1,
            comp_id: "comp".into(),
            frame: 0,
            width: 1,
            height: 1,
            size_bytes: 3,
            last_access_epoch: 1,
        };
        fs::write(
            tmp_dir.join(format!("{key:016x}.meta")),
            serde_json::to_string(&metadata).unwrap(),
        )
        .unwrap();

        let mut cache = PersistentDiskCache::with_directory(100, &tmp_dir);
        assert!(cache.get_frame(key, 2).is_none());
        assert!(!tmp_dir.join(format!("{key:016x}.cache")).exists());
        assert!(!tmp_dir.join(format!("{key:016x}.meta")).exists());
        let _ = fs::remove_dir_all(tmp_dir);
    }

    #[test]
    fn test_oversized_disk_entry_is_discarded() {
        let tmp_dir = std::env::temp_dir().join(format!(
            "ae_test_cache_oversized_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let key = 0x9abcu64;
        fs::create_dir_all(&tmp_dir).unwrap();
        fs::write(tmp_dir.join(format!("{key:016x}.cache")), [0u8; 101]).unwrap();
        fs::write(tmp_dir.join(format!("{key:016x}.meta")), b"stale").unwrap();

        let mut cache = PersistentDiskCache::with_directory(100, &tmp_dir);
        assert!(cache.get_frame(key, 1).is_none());
        assert!(!tmp_dir.join(format!("{key:016x}.cache")).exists());
        assert!(!tmp_dir.join(format!("{key:016x}.meta")).exists());
        let _ = fs::remove_dir_all(tmp_dir);
    }

    #[test]
    fn test_invalidate_comp_only_removes_matching_entries() {
        let mut cache = PersistentDiskCache::new(1024);
        cache
            .put_frame(1, "a".into(), 0, 1, 1, vec![1; 10], 1)
            .unwrap();
        cache
            .put_frame(2, "a".into(), 1, 1, 1, vec![2; 20], 2)
            .unwrap();
        cache
            .put_frame(3, "b".into(), 0, 1, 1, vec![3; 30], 3)
            .unwrap();

        assert_eq!(cache.invalidate_comp("a"), 2);
        assert_eq!(cache.current_bytes, 30);
        assert_eq!(cache.entries.len(), 1);
        assert!(cache.entries.contains_key(&3));
    }
}
