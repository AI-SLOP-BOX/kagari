#[test]
fn invert_and_tint_have_pixel_level_semantics() {
    let mut inverted = vec![12, 80, 200, 37, 255, 0, 17, 91];
    apply_layer_effects(
        None,
        None,
        &mut inverted,
        2,
        1,
        &[Effect {
            id: "invert".into(),
            name: "Invert".into(),
            effect_type: EffectType::Invert {
                invert_alpha: false,
            },
            enabled: true,
        }],
        0,
        24,
    );
    assert_eq!(inverted, vec![243, 175, 55, 37, 0, 255, 238, 91]);

    let mut tinted = vec![100, 50, 0, 23];
    apply_layer_effects(
        None,
        None,
        &mut tinted,
        1,
        1,
        &[Effect {
            id: "tint".into(),
            name: "Color Tint".into(),
            effect_type: EffectType::ColorTint {
                color: constant([1.0, 0.0, 0.0, 1.0]),
                intensity: constant(50.0),
            },
            enabled: true,
        }],
        0,
        24,
    );
    assert_eq!(tinted, vec![178, 25, 0, 23]);
}
#[test]
fn mfr_render_reports_each_expected_frame_once() {
    let mut queue = ParallelRenderQueue::new();
    let item_count = 4;
    let frames_per_item = 17;
    for item in 0..item_count {
        queue.add_item(RenderQueueItem {
            comp_name: format!("comp-{item}"),
            start_frame: 3,
            end_frame: 3 + frames_per_item - 1,
            output_path: format!("/tmp/semantic-{item}.rgba"),
            status: RenderStatus::Pending,
        });
    }

    let rendered = Arc::new(Mutex::new(Vec::<(String, u32)>::new()));
    let rendered_for_callback = Arc::clone(&rendered);
    let external_cancel = std::sync::atomic::AtomicBool::new(false);
    let result =
        queue.render_all_mfr_with_external_cancel_checked(&external_cancel, move |name, frame| {
            rendered_for_callback
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push((name.to_owned(), frame));
            vec![0u8; 4]
        });
    assert_eq!(result, Ok(()));

    let rendered = rendered.lock().unwrap_or_else(|error| error.into_inner());
    assert_eq!(rendered.len(), item_count * frames_per_item as usize);
    let unique: HashSet<_> = rendered.iter().cloned().collect();
    assert_eq!(
        unique.len(),
        rendered.len(),
        "MFR rendered a duplicate frame"
    );
    for item in 0..item_count {
        let name = format!("comp-{item}");
        let frames: Vec<_> = rendered
            .iter()
            .filter_map(|(rendered_name, frame)| (rendered_name == &name).then_some(*frame))
            .collect();
        assert_eq!(
            frames.len(),
            frames_per_item as usize,
            "{name} dropped a frame"
        );
        assert!(frames.iter().all(|frame| (3..=19).contains(frame)));
    }
    assert_eq!(
        queue
            .total_frames_rendered
            .load(std::sync::atomic::Ordering::SeqCst),
        u64::from(item_count as u32) * u64::from(frames_per_item)
    );
}
#[test]
fn mfr_cancellation_keeps_frame_identity_and_accounting_consistent() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    let mut queue = ParallelRenderQueue::new();
    for item in 0..4 {
        queue.add_item(RenderQueueItem {
            comp_name: format!("cancel-{item}"),
            start_frame: 0,
            end_frame: 255,
            output_path: format!("/tmp/cancel-{item}.rgba"),
            status: RenderStatus::Pending,
        });
    }

    let external_cancel = Arc::new(AtomicBool::new(false));
    let first_callback = AtomicUsize::new(0);
    let seen = Arc::new(Mutex::new(Vec::<(String, u32)>::new()));
    let seen_for_callback = Arc::clone(&seen);
    let cancel_for_callback = Arc::clone(&external_cancel);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("test rayon pool must build");
    let result = pool.install(|| {
        queue.render_all_mfr_with_external_cancel_checked(&external_cancel, move |name, frame| {
            seen_for_callback
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push((name.to_owned(), frame));
            if first_callback.fetch_add(1, Ordering::SeqCst) == 0 {
                cancel_for_callback.store(true, Ordering::SeqCst);
            }
            std::thread::yield_now();
            vec![0u8; 4]
        })
    });

    assert_eq!(
        result,
        Ok(()),
        "cooperative cancellation is not a render error"
    );
    assert!(queue.is_cancelled(), "workers must publish cancellation");
    let seen = seen.lock().unwrap_or_else(|error| error.into_inner());
    let unique: HashSet<_> = seen.iter().cloned().collect();
    assert_eq!(unique.len(), seen.len(), "cancellation duplicated a frame");
    assert!(!seen.is_empty(), "the first callback must have started");
    assert!(
        seen.len() <= queue.total_frames as usize,
        "cancellation rendered more callbacks than the queue total"
    );
    assert_eq!(
        queue.total_frames_rendered.load(Ordering::SeqCst) as usize,
        seen.len(),
        "progress accounting diverged from callback execution"
    );
}
#[test]
fn render_callback_failure_is_reported_with_item_and_frame_identity() {
    let mut queue = ParallelRenderQueue::new();
    queue.add_item(RenderQueueItem {
        comp_name: "broken-comp".into(),
        start_frame: 0,
        end_frame: 4,
        output_path: "/tmp/broken-comp.rgba".into(),
        status: RenderStatus::Pending,
    });

    let external_cancel = std::sync::atomic::AtomicBool::new(false);
    let result = queue.render_all_with_external_cancel_checked(&external_cancel, |_name, frame| {
        assert_ne!(
            frame, 2,
            "frame 2 must be converted to a structured failure"
        );
        vec![0u8; 4]
    });

    assert_eq!(
        result,
        Err(RenderFailure {
            item_index: 0,
            frame: 2,
        })
    );
    assert!(
        queue.is_cancelled(),
        "a worker failure must cancel the queue"
    );
    assert_eq!(
        queue
            .total_frames_rendered
            .load(std::sync::atomic::Ordering::SeqCst),
        2,
        "frames after the failed callback must not be counted as rendered"
    );
}
