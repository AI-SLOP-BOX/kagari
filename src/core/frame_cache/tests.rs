use super::*;
use std::sync::atomic::Ordering;

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
        .arg("core::frame_cache::tests::disk_budget_lock_child_process")
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
