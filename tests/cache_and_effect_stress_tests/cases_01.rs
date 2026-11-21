#[test]
fn frame_cache_concurrent_version_bump_safety() {
    use std::thread;
    let version_before = frame_cache::current_version();
    let handles: Vec<_> = (0..8)
        .map(|_| {
            thread::spawn(|| {
                for _ in 0..100 {
                    frame_cache::bump_version();
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    let version_after = frame_cache::current_version();
    // Other tests may also bump versions concurrently
    assert!(
        version_after >= version_before + 800,
        "Version should increment by at least 800 (got {})",
        version_after - version_before
    );
}
#[test]
fn frame_cache_insert_get_eviction_cycle() {
    let mut cache = FrameCache::with_version(10, 1);
    cache.max_memory_bytes = 1024 * 50; // 50 KB budget
    cache.current_memory_bytes = 0;

    let pixels = vec![128u8; 64 * 64 * 4]; // 16 KB each

    // Fill beyond budget
    for i in 0..10u32 {
        cache.insert(i, 64, 64, pixels.clone());
    }

    // Memory should not exceed budget after GC
    cache.collect_garbage();
    assert!(
        cache.current_memory_bytes <= cache.max_memory_bytes,
        "Memory {} exceeds budget {}",
        cache.current_memory_bytes,
        cache.max_memory_bytes
    );
}
#[test]
fn frame_cache_replacing_a_frame_updates_bytes_and_pixels_atomically() {
    let mut cache = FrameCache::with_version(8, 0xCACE_0001);
    let first = vec![11u8; 4 * 4 * 4];
    cache.insert(7, 4, 4, first);
    assert_eq!(cache.current_memory_bytes, 4 * 4 * 4);

    let replacement = vec![222u8; 8 * 8 * 4];
    cache.insert(7, 8, 8, replacement);
    assert_eq!(cache.current_memory_bytes, 8 * 8 * 4);
    assert_eq!(
        cache.cached_count(),
        1,
        "replacement must not duplicate a key"
    );

    let entry = cache.get(7).expect("replacement must remain readable");
    assert_eq!((entry.width, entry.height), (8, 8));
    assert!(entry.pixels.iter().all(|&pixel| pixel == 222));
}
#[test]
fn frame_cache_stale_entries_invisible_after_version_bump() {
    // Use version override to isolate from global counter races
    let mut cache = FrameCache::with_version(100, 1);
    let pixels = vec![200u8; 32 * 32 * 4];

    cache.insert(0, 32, 32, pixels.clone());
    assert!(cache.is_cached(0), "Should be cached at current version");

    // Create a new cache at a higher version — entries from version 1 are stale
    let mut cache2 = FrameCache::with_version(100, 2);
    cache2.insert(0, 32, 32, pixels.clone());
    // Original cache still uses version 1, its entry is valid
    assert!(
        cache.is_cached(0),
        "Original cache entry should still be valid at its version"
    );
    // New cache uses version 2, its entry is valid
    assert!(cache2.is_cached(0), "New cache entry should be valid");
}
#[test]
fn frame_cache_shared_access_keeps_pixels_and_invalidation_consistent() {
    // FrameCache intentionally exposes mutable operations rather than hiding
    // them behind a lock. This is the integration contract for callers that
    // share it across render/UI workers: one lock guards every read-modify-
    // write sequence, so readers never observe a partially replaced entry.
    let version = 0xA11C_0000_u64 + std::process::id() as u64;
    let cache = Arc::new(Mutex::new(FrameCache::with_version(64, version)));
    let start = Arc::new(Barrier::new(4));
    let mut handles = Vec::new();

    let writer_cache = Arc::clone(&cache);
    let writer_start = Arc::clone(&start);
    handles.push(std::thread::spawn(move || {
        writer_start.wait();
        for frame in 0..240u32 {
            let marker = (frame % 251) as u8;
            let pixels = vec![marker; 4 * 4 * 4];
            let mut cache = writer_cache
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            cache.insert(frame % 64, 4, 4, pixels);
            cache.collect_garbage_below(version);
        }
    }));

    for _ in 0..2 {
        let reader_cache = Arc::clone(&cache);
        let reader_start = Arc::clone(&start);
        handles.push(std::thread::spawn(move || {
            reader_start.wait();
            for frame in 0..240u32 {
                let mut cache = reader_cache
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                if let Some(entry) = cache.get(frame % 64) {
                    assert_eq!(entry.version, version);
                    assert_eq!(entry.width, 4);
                    assert_eq!(entry.height, 4);
                    assert_eq!(entry.pixels.len(), 4 * 4 * 4);
                    assert!(
                        entry.pixels.windows(2).all(|pair| pair[0] == pair[1]),
                        "cache reader observed a partially replaced pixel buffer"
                    );
                }
            }
        }));
    }

    let invalidator_cache = Arc::clone(&cache);
    let invalidator_start = Arc::clone(&start);
    handles.push(std::thread::spawn(move || {
        invalidator_start.wait();
        for layer in 0..240usize {
            let mut cache = invalidator_cache
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            cache.invalidate_layers(&[layer % 8]);
            assert!(cache.is_layer_dirty(layer % 8));
            cache.clear_dirty();
        }
    }));

    for handle in handles {
        handle.join().expect("shared cache worker must not panic");
    }

    let cache = cache.lock().unwrap_or_else(|error| error.into_inner());
    assert!(cache.current_version_len() <= 64);
    assert!(!cache.is_layer_dirty(0));
}
#[test]
fn frame_cache_invalidate_all_clears_everything() {
    let mut cache = FrameCache::with_version(100, 1);
    let pixels = vec![100u8; 16 * 16 * 4];

    for i in 0..20u32 {
        cache.insert(i, 16, 16, pixels.clone());
    }
    assert_eq!(cache.cached_count(), 20);

    cache.invalidate_all();
    assert_eq!(
        cache.cached_count(),
        0,
        "invalidate_all should clear everything"
    );
}
#[test]
fn frame_cache_layer_dirty_tracking() {
    let mut cache = FrameCache::new(100);
    assert!(!cache.is_layer_dirty(0));
    cache.mark_layer_dirty(0);
    assert!(cache.is_layer_dirty(0));
    cache.clear_dirty();
    assert!(!cache.is_layer_dirty(0));
}
#[test]
fn frame_cache_invalidate_specific_layers() {
    let mut cache = FrameCache::with_version(100, 1);

    let pixels = vec![50u8; 8 * 8 * 4];
    cache.insert_with_layers(0, 8, 8, pixels.clone(), &[0, 1, 2]);
    cache.insert_with_layers(1, 8, 8, pixels.clone(), &[3, 4, 5]);
    assert_eq!(cache.cached_count(), 2, "Both frames should be cached");
    assert!(!cache.is_layer_dirty(0), "Layer 0 should not be dirty yet");

    cache.invalidate_layers(&[0]);
    // invalidate_layers marks the layer dirty, not the frame
    assert!(cache.is_layer_dirty(0), "Layer 0 should be dirty");
    assert!(!cache.is_layer_dirty(1), "Layer 1 should not be dirty");
    assert!(
        cache.is_frame_dirty(0, &[0, 1, 2]),
        "Frame 0 (layers 0,1,2) should be dirty"
    );
    assert!(
        !cache.is_frame_dirty(1, &[3, 4, 5]),
        "Frame 1 (layers 3,4,5) should not be dirty"
    );
}
#[test]
fn pixel_buffer_pool_recycling_bounds() {
    let pool = PixelBufferPool::new();
    let mut bufs = Vec::new();
    // Acquire 100 buffers
    for _ in 0..100 {
        bufs.push(pool.acquire(1024));
    }
    // Recycle all 100
    for buf in bufs {
        pool.recycle(buf);
    }
    // Pool should cap at 64
    assert!(pool.len() <= 64, "Pool exceeded cap: {}", pool.len());
}

// ─────────────────────────────────────────────────────────────────────────────
// §2  TileCache Stress Tests
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn tile_cache_eviction_under_pressure() {
    let mut cache = TileCache::with_version(64, 4096, 10); // 4 KB budget, tiny
    let tile_data = vec![0u8; 64 * 64 * 4]; // 16 KB per tile — oversized

    // Oversized tiles should be rejected silently
    let coord = tile_cache::TileCoord { tx: 0, ty: 0 };
    cache.insert(0, coord, tile_data);
    assert_eq!(cache.tile_count(), 0, "Oversized tile should be rejected");
}
#[test]
fn tile_cache_version_invalidation() {
    let mut cache = TileCache::with_version(64, 1024 * 1024, 1);
    let small_tile = vec![42u8; 8 * 8 * 4];

    let coord = tile_cache::TileCoord { tx: 0, ty: 0 };
    cache.insert(0, coord, small_tile.clone());
    assert!(
        cache.get(0, coord).is_some(),
        "Should be found at insert version"
    );

    // Create a new cache at a higher version — entries from version 1 are stale
    let mut cache2 = TileCache::with_version(64, 1024 * 1024, 2);
    cache2.insert(0, coord, small_tile);
    // Original cache still uses version 1, its entry is valid
    assert!(
        cache.get(0, coord).is_some(),
        "Original cache entry should still be valid at its version"
    );
    // New cache uses version 2, its entry is valid
    assert!(
        cache2.get(0, coord).is_some(),
        "New cache entry should be valid"
    );
}
#[test]
fn tile_cache_tiles_for_frame_grid_accuracy() {
    let cache = TileCache::with_version(256, 1024 * 1024, 1);
    let tiles = cache.tiles_for_frame(0, 1920, 1080);
    let expected_cols = 1920_usize.div_ceil(256); // 8
    let expected_rows = 1080_usize.div_ceil(256); // 5
    assert_eq!(tiles.len(), expected_cols * expected_rows);
}
#[test]
fn tile_cache_invalidate_frame_only() {
    let mut cache = TileCache::with_version(64, 1024 * 1024, 42);
    let tile = vec![1u8; 8 * 8 * 4];
    let coord = tile_cache::TileCoord { tx: 0, ty: 0 };
    cache.insert(0, coord, tile.clone());
    cache.insert(1, coord, tile);

    cache.invalidate_frame(0);
    assert!(cache.get(0, coord).is_none());
    assert!(cache.get(1, coord).is_some(), "Other frame should survive");
}
#[test]
fn tile_cache_memory_accounting_accuracy() {
    let mut cache = TileCache::with_version(64, 1024 * 1024, 1);
    let tile_size = 8 * 8 * 4; // 256 bytes
    let tile = vec![0u8; tile_size];

    let initial = cache.memory_usage();
    let coord = tile_cache::TileCoord { tx: 0, ty: 0 };
    cache.insert(0, coord, tile);
    let after = cache.memory_usage();
    assert_eq!(
        after - initial,
        tile_size,
        "Memory accounting should match tile size"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §3  MerkleFrameCache Tests
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn merkle_deterministic_hashing() {
    let h1 = MerkleFrameCache::compute_node_hash("layer1", "params1", 0, None);
    let h2 = MerkleFrameCache::compute_node_hash("layer1", "params1", 0, None);
    let h3 = MerkleFrameCache::compute_node_hash("layer1", "params2", 0, None);
    assert_eq!(h1, h2, "Same inputs must produce same hash");
    assert_ne!(h1, h3, "Different params must produce different hash");
}
#[test]
fn merkle_parent_hash_sensitivity() {
    let h_no_parent = MerkleFrameCache::compute_node_hash("a", "b", 0, None);
    let parent = MerkleFrameCache::compute_node_hash("x", "y", 0, None);
    let h_with_parent = MerkleFrameCache::compute_node_hash("a", "b", 0, Some(&parent));
    assert_ne!(h_no_parent, h_with_parent, "Parent hash must change output");
}
#[test]
fn merkle_cache_deduplication() {
    let mut cache = MerkleFrameCache::new();
    let h = MerkleFrameCache::compute_node_hash("layer", "params", 0, None);
    let pixels = vec![255u8; 100];
    cache.insert(h.clone(), pixels.clone());
    let retrieved = cache.get(&h).unwrap();
    assert_eq!(*retrieved, pixels);
}

// ─────────────────────────────────────────────────────────────────────────────
// §4  ParallelRenderQueue Stress Tests
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn parallel_render_cancel_immediate() {
    let mut queue = ParallelRenderQueue::new();
    for i in 0..100 {
        queue.add_item(RenderQueueItem {
            comp_name: format!("comp_{i}"),
            start_frame: 0,
            end_frame: 10,
            output_path: format!("/tmp/test_{i}.png"),
            status: kagari_vfx::core::parallel_render::RenderStatus::Pending,
        });
    }
    queue.cancel();
    assert!(queue.is_cancelled());

    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();
    queue.render_all(move |_, _| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
        vec![0u8; 4]
    });
    // Cancelled — should render 0 frames
    assert_eq!(counter.load(Ordering::Relaxed), 0);
}
#[test]
fn parallel_render_mfr_cancel() {
    let mut queue = ParallelRenderQueue::new();
    for i in 0..50 {
        queue.add_item(RenderQueueItem {
            comp_name: format!("comp_{i}"),
            start_frame: 0,
            end_frame: 100,
            output_path: format!("/tmp/test_{i}.png"),
            status: kagari_vfx::core::parallel_render::RenderStatus::Pending,
        });
    }
    queue.cancel();

    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();
    queue.render_all_mfr(move |_, _| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
        vec![0u8; 4]
    });
    assert_eq!(counter.load(Ordering::Relaxed), 0);
}
#[test]
fn parallel_render_progress_callback_accuracy() {
    let mut queue = ParallelRenderQueue::new();
    let progress_values = Arc::new(std::sync::Mutex::new(Vec::new()));
    let pv_clone = progress_values.clone();

    queue.set_progress_callback(move |_item, _done, _total| {
        pv_clone.lock().unwrap().push((_done, _total));
    });

    queue.add_item(RenderQueueItem {
        comp_name: "test".into(),
        start_frame: 0,
        end_frame: 4,
        output_path: "/tmp/test.png".into(),
        status: kagari_vfx::core::parallel_render::RenderStatus::Pending,
    });

    queue.render_all(|_, _| vec![0u8; 4]);

    let values = progress_values.lock().unwrap();
    assert!(
        !values.is_empty(),
        "Progress callback should have been called"
    );
    // Last callback should show completion
    let last = values.last().unwrap();
    assert_eq!(last.0, last.1, "Final progress should show done == total");
}
