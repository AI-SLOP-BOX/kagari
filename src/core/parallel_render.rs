use rayon::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// Render queue item representing a composition to be rendered.
#[derive(Debug, Clone)]
pub struct RenderQueueItem {
    pub comp_name: String,
    pub start_frame: u32,
    pub end_frame: u32,
    pub output_path: String,
    pub status: RenderStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderStatus {
    Pending,
    Rendering,
    Done,
    Failed,
}

/// Progress callback: (item_index, frames_done, total_frames).
type ProgressCallback = dyn Fn(usize, u32, u32) + Send + Sync;

/// A render callback failed for one concrete queue item/frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderFailure {
    pub item_index: usize,
    pub frame: u32,
}

fn record_first_failure(slot: &Mutex<Option<RenderFailure>>, failure: RenderFailure) {
    let mut recorded = slot.lock().unwrap_or_else(|error| error.into_inner());
    if recorded.is_none() {
        *recorded = Some(failure);
    }
}

/// Parallel render queue that processes multiple compositions and/or frames
/// using rayon's thread pool.
pub struct ParallelRenderQueue {
    pub items: Vec<RenderQueueItem>,
    /// Total frames rendered across all items.
    pub total_frames_rendered: AtomicU32,
    /// Total frames to render across all items.
    pub total_frames: u32,
    /// Whether the entire queue has been cancelled.
    pub cancelled: AtomicBool,
    progress: Option<Arc<ProgressCallback>>,
}

impl ParallelRenderQueue {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            total_frames_rendered: AtomicU32::new(0),
            total_frames: 0,
            cancelled: AtomicBool::new(false),
            progress: None,
        }
    }

    pub fn set_progress_callback<F: Fn(usize, u32, u32) + Send + Sync + 'static>(&mut self, cb: F) {
        self.progress = Some(Arc::new(cb));
    }

    pub fn add_item(&mut self, item: RenderQueueItem) {
        if item.start_frame <= item.end_frame {
            let frames = item.end_frame - item.start_frame + 1;
            self.total_frames = self.total_frames.saturating_add(frames);
        }
        self.items.push(item);
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Render all queue items in parallel. Each item's frames are rendered
    /// sequentially within the item, but different items run in parallel.
    pub fn render_all<F>(&self, render_frame: F)
    where
        F: Fn(&str, u32) -> Vec<u8> + Sync,
    {
        let external_cancel = AtomicBool::new(false);
        self.render_all_with_external_cancel(&external_cancel, render_frame);
    }

    pub fn render_all_with_external_cancel<F>(&self, external_cancel: &AtomicBool, render_frame: F)
    where
        F: Fn(&str, u32) -> Vec<u8> + Sync,
    {
        let _ = self.render_all_with_external_cancel_checked(external_cancel, render_frame);
    }

    /// Render all items while converting a callback panic into a structured
    /// failure. The legacy wrapper keeps its fire-and-forget API, while callers
    /// that own a background worker can report the exact item/frame.
    pub fn render_all_with_external_cancel_checked<F>(
        &self,
        external_cancel: &AtomicBool,
        render_frame: F,
    ) -> Result<(), RenderFailure>
    where
        F: Fn(&str, u32) -> Vec<u8> + Sync,
    {
        let failure = Arc::new(Mutex::new(None::<RenderFailure>));

        self.items
            .par_iter()
            .enumerate()
            .for_each(|(item_idx, item)| {
                if failure
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .is_some()
                {
                    return;
                }
                if self.is_cancelled() || external_cancel.load(Ordering::Relaxed) {
                    self.cancel();
                    return;
                }

                if item.start_frame > item.end_frame {
                    return;
                }
                for frame in item.start_frame..=item.end_frame {
                    if failure
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .is_some()
                    {
                        return;
                    }
                    if self.is_cancelled() || external_cancel.load(Ordering::Relaxed) {
                        self.cancel();
                        return;
                    }

                    let render_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                        || render_frame(&item.comp_name, frame),
                    ));
                    if render_result.is_err() {
                        self.cancel();
                        record_first_failure(
                            &failure,
                            RenderFailure {
                                item_index: item_idx,
                                frame,
                            },
                        );
                        return;
                    }
                    self.total_frames_rendered.fetch_add(1, Ordering::Relaxed);

                    if let Some(cb) = &self.progress {
                        let done = self.total_frames_rendered.load(Ordering::Relaxed);
                        let callback_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                            || cb(item_idx, done, self.total_frames),
                        ));
                        if callback_result.is_err() {
                            self.cancel();
                            record_first_failure(
                                &failure,
                                RenderFailure {
                                    item_index: item_idx,
                                    frame,
                                },
                            );
                            return;
                        }
                    }
                }
            });

        let result = failure
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
            .map_or(Ok(()), Err);
        result
    }

    /// Multi-frame rendering (MFR): render all frames of all items in parallel
    /// across both items AND frames within each item. This maximizes CPU core
    /// utilization for batch rendering.
    pub fn render_all_mfr<F>(&self, render_frame: F)
    where
        F: Fn(&str, u32) -> Vec<u8> + Sync,
    {
        let external_cancel = AtomicBool::new(false);
        self.render_all_mfr_with_external_cancel(&external_cancel, render_frame);
    }

    /// MFR variant with a cancellation source owned by the caller.
    ///
    /// The callback may be running on several rayon workers at once, so the
    /// flag is checked both before entering an item and immediately before
    /// each frame. It is cooperative: a callback already in progress is
    /// allowed to return, but no new frame is started after cancellation is
    /// observed.
    pub fn render_all_mfr_with_external_cancel<F>(
        &self,
        external_cancel: &AtomicBool,
        render_frame: F,
    ) where
        F: Fn(&str, u32) -> Vec<u8> + Sync,
    {
        let _ = self.render_all_mfr_with_external_cancel_checked(external_cancel, render_frame);
    }

    /// MFR variant that reports callback failures instead of unwinding the
    /// rayon worker pool.
    pub fn render_all_mfr_with_external_cancel_checked<F>(
        &self,
        external_cancel: &AtomicBool,
        render_frame: F,
    ) -> Result<(), RenderFailure>
    where
        F: Fn(&str, u32) -> Vec<u8> + Sync,
    {
        let failure = Arc::new(Mutex::new(None::<RenderFailure>));

        self.items
            .par_iter()
            .enumerate()
            .for_each(|(item_idx, item)| {
                if failure
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .is_some()
                {
                    return;
                }
                if self.is_cancelled() || external_cancel.load(Ordering::Relaxed) {
                    self.cancel();
                    return;
                }

                if item.start_frame > item.end_frame {
                    return;
                }
                let frames: Vec<u32> = (item.start_frame..=item.end_frame).collect();
                frames.par_iter().for_each(|&frame| {
                    if failure
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .is_some()
                    {
                        return;
                    }
                    if self.is_cancelled() || external_cancel.load(Ordering::Relaxed) {
                        self.cancel();
                        return;
                    }

                    let render_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                        || render_frame(&item.comp_name, frame),
                    ));
                    if render_result.is_err() {
                        self.cancel();
                        record_first_failure(
                            &failure,
                            RenderFailure {
                                item_index: item_idx,
                                frame,
                            },
                        );
                        return;
                    }
                    self.total_frames_rendered.fetch_add(1, Ordering::Relaxed);

                    if let Some(cb) = &self.progress {
                        let done = self.total_frames_rendered.load(Ordering::Relaxed);
                        let callback_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                            || cb(item_idx, done, self.total_frames),
                        ));
                        if callback_result.is_err() {
                            self.cancel();
                            record_first_failure(
                                &failure,
                                RenderFailure {
                                    item_index: item_idx,
                                    frame,
                                },
                            );
                        }
                    }
                });
            });

        let result = failure
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
            .map_or(Ok(()), Err);
        result
    }
}

impl Default for ParallelRenderQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Multi-frame render statistics for the status bar.
#[derive(Debug, Clone, Default)]
pub struct RenderStats {
    pub frames_rendered: u32,
    pub total_frames: u32,
    pub elapsed_ms: f64,
    pub avg_frame_ms: f64,
    pub active_threads: usize,
}

impl RenderStats {
    pub fn progress_pct(&self) -> f32 {
        if self.total_frames == 0 {
            0.0
        } else {
            self.frames_rendered as f32 / self.total_frames as f32 * 100.0
        }
    }

    pub fn fps(&self) -> f32 {
        if self.elapsed_ms <= 0.0 {
            0.0
        } else {
            self.frames_rendered as f32 / (self.elapsed_ms as f32 / 1000.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_parallel_render_queue_basics() {
        let mut queue = ParallelRenderQueue::new();
        queue.add_item(RenderQueueItem {
            comp_name: "Comp 1".into(),
            start_frame: 0,
            end_frame: 9,
            output_path: "/tmp/out1.mp4".into(),
            status: RenderStatus::Pending,
        });
        queue.add_item(RenderQueueItem {
            comp_name: "Comp 2".into(),
            start_frame: 0,
            end_frame: 4,
            output_path: "/tmp/out2.mp4".into(),
            status: RenderStatus::Pending,
        });
        assert_eq!(queue.total_frames, 15);
        assert!(!queue.is_cancelled());
    }

    #[test]
    fn test_cancel() {
        let queue = ParallelRenderQueue::new();
        assert!(!queue.is_cancelled());
        queue.cancel();
        assert!(queue.is_cancelled());
    }

    #[test]
    fn test_render_stats() {
        let stats = RenderStats {
            frames_rendered: 50,
            total_frames: 100,
            elapsed_ms: 1000.0,
            avg_frame_ms: 20.0,
            active_threads: 4,
        };
        assert_eq!(stats.progress_pct(), 50.0);
        assert!((stats.fps() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_parallel_render_executes() {
        let counter = AtomicUsize::new(0);
        let mut queue = ParallelRenderQueue::new();
        queue.add_item(RenderQueueItem {
            comp_name: "Test".into(),
            start_frame: 0,
            end_frame: 3,
            output_path: "/tmp/test.mp4".into(),
            status: RenderStatus::Pending,
        });
        queue.render_all(|_comp, _frame| {
            counter.fetch_add(1, Ordering::Relaxed);
            vec![0u8; 4]
        });
        assert_eq!(counter.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn invalid_frame_range_does_not_inflate_total_or_render() {
        let mut queue = ParallelRenderQueue::new();
        queue.add_item(RenderQueueItem {
            comp_name: "Invalid".into(),
            start_frame: 20,
            end_frame: 10,
            output_path: "/tmp/invalid".into(),
            status: RenderStatus::Pending,
        });
        assert_eq!(queue.total_frames, 0);

        let counter = AtomicUsize::new(0);
        queue.render_all(|_, _| {
            counter.fetch_add(1, Ordering::SeqCst);
            Vec::new()
        });
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn external_cancel_stops_parallel_render_before_next_frame() {
        let mut queue = ParallelRenderQueue::new();
        queue.add_item(RenderQueueItem {
            comp_name: "Cancelable".into(),
            start_frame: 0,
            end_frame: 100,
            output_path: "/tmp/cancelable".into(),
            status: RenderStatus::Pending,
        });
        let external_cancel = AtomicBool::new(false);
        let counter = AtomicUsize::new(0);
        queue.render_all_with_external_cancel(&external_cancel, |_, _| {
            let rendered = counter.fetch_add(1, Ordering::SeqCst) + 1;
            if rendered == 1 {
                external_cancel.store(true, Ordering::SeqCst);
            }
            Vec::new()
        });
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert!(queue.is_cancelled());
    }

    #[test]
    fn render_callback_panic_is_reported_without_unwinding() {
        let mut queue = ParallelRenderQueue::new();
        queue.add_item(RenderQueueItem {
            comp_name: "panic".into(),
            start_frame: 7,
            end_frame: 9,
            output_path: "/tmp/panic".into(),
            status: RenderStatus::Pending,
        });
        let external_cancel = AtomicBool::new(false);

        let result = queue.render_all_with_external_cancel_checked(&external_cancel, |_, _| {
            panic!("synthetic render failure")
        });

        assert_eq!(
            result,
            Err(RenderFailure {
                item_index: 0,
                frame: 7,
            })
        );
        assert!(queue.is_cancelled());
        assert_eq!(queue.total_frames_rendered.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn mfr_progress_panic_is_reported_without_unwinding() {
        let mut queue = ParallelRenderQueue::new();
        queue.add_item(RenderQueueItem {
            comp_name: "progress panic".into(),
            start_frame: 2,
            end_frame: 4,
            output_path: "/tmp/progress-panic".into(),
            status: RenderStatus::Pending,
        });
        queue.set_progress_callback(|_, _, _| panic!("synthetic progress failure"));
        let external_cancel = AtomicBool::new(false);

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        let result = pool.install(|| {
            queue.render_all_mfr_with_external_cancel_checked(&external_cancel, |_, _| {
                vec![0u8; 4]
            })
        });

        assert_eq!(
            result,
            Err(RenderFailure {
                item_index: 0,
                frame: 2,
            })
        );
        assert!(queue.is_cancelled());
        assert_eq!(queue.total_frames_rendered.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn external_cancel_stops_mfr_render_before_next_frame() {
        let mut queue = ParallelRenderQueue::new();
        queue.add_item(RenderQueueItem {
            comp_name: "MFR cancel".into(),
            start_frame: 0,
            end_frame: 100,
            output_path: "/tmp/mfr-cancel".into(),
            status: RenderStatus::Pending,
        });
        let external_cancel = AtomicBool::new(false);
        let counter = AtomicUsize::new(0);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        pool.install(|| {
            queue.render_all_mfr_with_external_cancel(&external_cancel, |_, _| {
                let rendered = counter.fetch_add(1, Ordering::SeqCst) + 1;
                if rendered == 1 {
                    external_cancel.store(true, Ordering::SeqCst);
                }
                Vec::new()
            });
        });
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert!(queue.is_cancelled());
    }

    #[test]
    fn invalid_frame_range_does_not_render_in_mfr_mode() {
        let mut queue = ParallelRenderQueue::new();
        queue.add_item(RenderQueueItem {
            comp_name: "Invalid MFR".into(),
            start_frame: 9,
            end_frame: 3,
            output_path: "/tmp/invalid-mfr".into(),
            status: RenderStatus::Pending,
        });
        let counter = AtomicUsize::new(0);
        queue.render_all_mfr(|_, _| {
            counter.fetch_add(1, Ordering::SeqCst);
            Vec::new()
        });
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }
}
