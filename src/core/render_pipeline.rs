#![allow(dead_code)]
/// Lazy evaluation render pipeline with generation-based stale request filtering.
///
/// The UI thread never blocks on GPU work. Instead, it enqueues `RenderCommand`
/// messages. A background `RenderWorker` thread drains the queue, performs GPU
/// rendering, and writes results to FrameCache.
/// Flush skips queued work from earlier generations. A callback already running cannot be
/// interrupted because the callback API does not receive a cancellation token.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

/// Commands the UI thread sends to the render worker.
#[derive(Debug, Clone)]
pub enum RenderCommand {
    /// Render a specific frame at a specific cache version.
    RenderFrame { frame: u32, version: u64 },
    /// Render a contiguous range of frames (background pre-fetch).
    PrefetchRange { start: u32, end: u32, version: u64 },
    /// Mark queued work from earlier generations as stale (e.g., project changed).
    Flush,
    /// Shut down the worker thread cleanly.
    Shutdown,
}

/// Notification from the worker back to the UI thread.
#[derive(Debug)]
pub enum RenderResult {
    /// A frame finished rendering and is now in the FrameCache.
    FrameReady { frame: u32, cache_version: u64 },
    /// A batch of frames is ready.
    BatchReady { start: u32, end: u32 },
}

/// Manages the command channel to the background render worker.
pub struct RenderPipeline {
    cmd_tx: Sender<QueuedCommand>,
    result_rx: Receiver<RenderResult>,
    generation: Arc<AtomicU64>,
    command_gate: Mutex<()>,
}

struct QueuedCommand {
    generation: u64,
    command: RenderCommand,
}

impl RenderPipeline {
    /// Spawn the background render worker and return the pipeline handle.
    pub fn new<F>(render_fn: F) -> Self
    where
        F: Fn(RenderCommand, &Sender<RenderResult>) + Send + 'static,
    {
        let (cmd_tx, cmd_rx) = mpsc::channel::<QueuedCommand>();
        let (result_tx, result_rx) = mpsc::channel::<RenderResult>();
        let generation = Arc::new(AtomicU64::new(1));
        let worker_generation = Arc::clone(&generation);

        thread::Builder::new()
            .name("render_worker".to_string())
            .spawn(move || {
                log::info!("[RenderWorker] started");
                while let Ok(queued) = cmd_rx.recv() {
                    let QueuedCommand { generation, command } = queued;
                    match &command {
                        RenderCommand::Shutdown => {
                            log::info!("[RenderWorker] shutting down");
                            break;
                        }
                        RenderCommand::Flush => {
                            log::debug!("[RenderWorker] flush — obsolete queued work will be skipped");
                        }
                        RenderCommand::RenderFrame { version, .. }
                        | RenderCommand::PrefetchRange { version, .. } => {
                            if generation != worker_generation.load(Ordering::SeqCst) {
                                log::debug!("[RenderWorker] skipping stale render command (project version {})", version);
                                continue;
                            }
                            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                render_fn(command, &result_tx);
                            }));
                            if let Err(e) = res {
                                log::error!("[RenderWorker] Caught panic during render task execution: {:?}", e);
                            }
                        }
                    }
                }
            })
            .expect("Failed to spawn render worker thread");

        Self {
            cmd_tx,
            result_rx,
            generation,
            command_gate: Mutex::new(()),
        }
    }

    /// Request an asynchronous render of `frame`.
    pub fn request_frame(&self, frame: u32, version: u64) {
        let _gate = self.command_gate.lock().unwrap_or_else(|e| e.into_inner());
        let generation = self.generation.load(Ordering::SeqCst);
        let _ = self.cmd_tx.send(QueuedCommand {
            generation,
            command: RenderCommand::RenderFrame { frame, version },
        });
    }

    /// Pre-fetch a range of frames in the background.
    pub fn prefetch_range(&self, start: u32, end: u32, version: u64) {
        let _gate = self.command_gate.lock().unwrap_or_else(|e| e.into_inner());
        let generation = self.generation.load(Ordering::SeqCst);
        let _ = self.cmd_tx.send(QueuedCommand {
            generation,
            command: RenderCommand::PrefetchRange {
                start,
                end,
                version,
            },
        });
    }

    /// Skip queued commands from earlier generations and accept subsequent requests.
    pub fn flush(&self) {
        let _gate = self.command_gate.lock().unwrap_or_else(|e| e.into_inner());
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let _ = self.cmd_tx.send(QueuedCommand {
            generation,
            command: RenderCommand::Flush,
        });
    }

    /// Poll for completed frames without blocking.
    pub fn poll_results(&self) -> Vec<RenderResult> {
        let mut results = Vec::new();
        while let Ok(r) = self.result_rx.try_recv() {
            results.push(r);
        }
        results
    }

    /// Gracefully shut down the worker thread.
    pub fn shutdown(&self) {
        let _gate = self.command_gate.lock().unwrap_or_else(|e| e.into_inner());
        let generation = self.generation.load(Ordering::SeqCst);
        let _ = self.cmd_tx.send(QueuedCommand {
            generation,
            command: RenderCommand::Shutdown,
        });
    }
}

impl Drop for RenderPipeline {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub struct LazyFrameEvaluator {
    in_flight: std::collections::HashSet<u32>,
}

impl Default for LazyFrameEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl LazyFrameEvaluator {
    pub fn new() -> Self {
        Self {
            in_flight: std::collections::HashSet::new(),
        }
    }

    pub fn needs_render(
        &mut self,
        frame: u32,
        cache: &crate::core::frame_cache::FrameCache,
    ) -> bool {
        if cache.is_cached(frame) {
            self.in_flight.remove(&frame);
            return false;
        }
        if self.in_flight.contains(&frame) {
            return false;
        }
        self.in_flight.insert(frame);
        true
    }

    pub fn mark_delivered(&mut self, frame: u32) {
        self.in_flight.remove(&frame);
    }
}

#[cfg(test)]
mod tests {
    use super::{RenderCommand, RenderPipeline};
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::Duration;

    #[test]
    fn flush_skips_waiting_old_requests_but_keeps_new_requests() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (rendered_tx, rendered_rx) = mpsc::channel();
        let release_rx = Arc::new(Mutex::new(release_rx));
        let worker_release = Arc::clone(&release_rx);
        let pipeline = RenderPipeline::new(move |command, _| {
            if let RenderCommand::RenderFrame { frame, .. } = command {
                let _ = rendered_tx.send(frame);
                if frame == 1 {
                    let _ = started_tx.send(());
                    let _ = worker_release.lock().unwrap().recv();
                }
            }
        });

        pipeline.request_frame(1, 1);
        started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("first render should begin");
        pipeline.request_frame(2, 1);
        pipeline.flush();
        pipeline.request_frame(3, 1);
        release_tx.send(()).expect("worker should be waiting");

        assert_eq!(rendered_rx.recv_timeout(Duration::from_secs(2)), Ok(1));
        assert_eq!(rendered_rx.recv_timeout(Duration::from_secs(2)), Ok(3));
        assert!(rendered_rx.recv_timeout(Duration::from_millis(50)).is_err());
        pipeline.shutdown();
    }
}
