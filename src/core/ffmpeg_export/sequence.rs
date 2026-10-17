use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use super::types::{ExportEvent, RenderFrameFn};

/// Multi-comp parallel export using `ParallelRenderQueue`.
pub fn start_parallel_export(
    items: Vec<crate::core::parallel_render::RenderQueueItem>,
    tx: Sender<ExportEvent>,
    cancel_flag: Arc<std::sync::atomic::AtomicBool>,
    render_frame_fn: RenderFrameFn,
) -> Result<(), String> {
    if items.is_empty() {
        let _ = tx.send(ExportEvent::Error("No render queue items".to_string()));
        return Err("No items".to_string());
    }

    let total: u64 = items
        .iter()
        .filter(|i| i.start_frame <= i.end_frame)
        .map(|i| u64::from(i.end_frame) - u64::from(i.start_frame) + 1)
        .fold(0u64, u64::saturating_add);
    if total == 0 {
        let _ = tx.send(ExportEvent::Error("Total frames is zero".to_string()));
        return Err("Zero frames".to_string());
    }

    std::thread::Builder::new()
        .name("parallel_export".to_string())
        .spawn(move || {
            let mut queue = crate::core::parallel_render::ParallelRenderQueue::new();
            for item in items {
                queue.add_item(item);
            }
            let tx_clone = tx.clone();
            let total_frames = total;
            queue.set_progress_callback(move |_item_idx, done, _total| {
                let pct = done as f64 / total_frames.max(1) as f64;
                let _ = tx_clone.send(ExportEvent::Progress(
                    pct as f32,
                    format!("Parallel: {}/{} frames", done, total_frames),
                ));
            });

            // MFR on: rayon frames within and across items; MFR off: items in
            // parallel but each item's frames rendered sequentially.
            let render_result = if crate::core::parallel_render::mfr_enabled() {
                queue.render_all_mfr_with_external_cancel_checked(&cancel_flag, |comp_name, frame| {
                    render_frame_fn(comp_name, frame)
                })
            } else {
                queue.render_all_with_external_cancel_checked(&cancel_flag, |comp_name, frame| {
                    render_frame_fn(comp_name, frame)
                })
            };

            if let Err(failure) = render_result {
                let _ = tx.send(ExportEvent::Error(format!(
                    "Parallel export failed at item {} frame {}",
                    failure.item_index, failure.frame
                )));
            } else if cancel_flag.load(Ordering::SeqCst) || queue.is_cancelled() {
                let _ = tx.send(ExportEvent::Error("Parallel export canceled".to_string()));
            } else {
                let _ = tx.send(ExportEvent::Finished(format!(
                    "Parallel render complete: {} items, {} frames",
                    queue.items.len(),
                    total_frames
                )));
            }
        })
        .map_err(|e| format!("Failed to spawn parallel export thread: {}", e))?;

    Ok(())
}

/// PNG image-sequence export: renders every frame and writes
/// `{stem}_{frame:04}.png` into `dir` using the `image` crate.
#[allow(clippy::too_many_arguments)]
pub fn start_png_sequence_export<F>(
    dir: std::path::PathBuf,
    stem: String,
    width: u32,
    height: u32,
    total_frames: u32,
    first_index: u32,
    tx: Sender<ExportEvent>,
    cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    mut render_frame: F,
) where
    F: FnMut(u32) -> Vec<u8> + Send + 'static,
{
    std::thread::spawn(move || {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            let _ = tx.send(ExportEvent::Error(format!(
                "Cannot create dir {}: {}",
                dir.display(),
                e
            )));
            return;
        }
        let last = total_frames.saturating_sub(1);
        for i in 0..total_frames {
            let f = i + first_index;
            if cancel_flag.load(Ordering::SeqCst) {
                let _ = tx.send(ExportEvent::Error("Export canceled".to_string()));
                return;
            }
            let pixels =
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| render_frame(f))) {
                    Ok(pixels) => pixels,
                    Err(_) => {
                        let _ = tx.send(ExportEvent::Error(format!(
                            "Render callback failed at frame {}",
                            f
                        )));
                        return;
                    }
                };
            let path = dir.join(format!("{}_{:04}.png", stem, f));
            if let Err(e) =
                image::save_buffer(&path, &pixels, width, height, image::ColorType::Rgba8)
            {
                let _ = tx.send(ExportEvent::Error(format!(
                    "Failed writing {}: {}",
                    path.display(),
                    e
                )));
                return;
            }
            if i % 2 == 0 || i == last {
                let _ = tx.send(ExportEvent::Progress(
                    (i + 1) as f32 / total_frames.max(1) as f32,
                    format!("Frame {} / {} → {}", f + 1, total_frames, path.display()),
                ));
            }
        }
        let _ = tx.send(ExportEvent::Finished(format!(
            "PNG sequence exported: {} frames → {}",
            total_frames,
            dir.join(format!("{}_", stem)).display()
        )));
    });
}
