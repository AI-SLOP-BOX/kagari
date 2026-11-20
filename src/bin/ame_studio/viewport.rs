use super::*;

const MAX_VIEWPORT_PREVIEW_CACHE_ENTRIES: usize = 120;
const MAX_VIEWPORT_PREVIEW_CACHE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_SCALED_VIEWPORT_PREVIEW_CACHE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

fn viewport_preview_cache_limits(memory_limit_gb: f64) -> (usize, u64) {
    let clamped_gb = if memory_limit_gb.is_finite() {
        memory_limit_gb.clamp(1.0, 64.0)
    } else {
        12.0
    };
    let quarter_bytes = (clamped_gb * 0.25 * 1024.0 * 1024.0 * 1024.0) as u64;
    let max_bytes = quarter_bytes.clamp(
        MAX_VIEWPORT_PREVIEW_CACHE_BYTES,
        MAX_SCALED_VIEWPORT_PREVIEW_CACHE_BYTES,
    );
    let max_entries = ((max_bytes / (1024 * 1024)) as usize).clamp(MAX_VIEWPORT_PREVIEW_CACHE_ENTRIES, 1800);
    (max_entries, max_bytes)
}

fn viewport_preview_cache_needs_eviction_with_limits(
    entry_count: usize,
    cached_bytes: u64,
    incoming: u64,
    max_entries: usize,
    max_bytes: u64,
) -> bool {
    entry_count >= max_entries || cached_bytes.saturating_add(incoming) > max_bytes
}

fn viewport_preview_job_should_cancel(
    active: ViewportPreviewKey,
    requested: ViewportPreviewKey,
    is_playing: bool,
) -> bool {
    active.revision != requested.revision
        || active.width != requested.width
        || active.height != requested.height
        || (!is_playing
            && (active.frame != requested.frame
                || active.media_revision != requested.media_revision))
}

fn viewport_preview_cache_entry_bytes(width: u32, height: u32) -> u64 {
    u64::from(width)
        .saturating_mul(u64::from(height))
        .saturating_mul(4)
}

#[cfg(test)]
fn viewport_preview_cache_needs_eviction(
    entry_count: usize,
    cached_bytes: u64,
    incoming: u64,
) -> bool {
    viewport_preview_cache_needs_eviction_with_limits(
        entry_count,
        cached_bytes,
        incoming,
        MAX_VIEWPORT_PREVIEW_CACHE_ENTRIES,
        MAX_VIEWPORT_PREVIEW_CACHE_BYTES,
    )
}

fn viewport_preview_cache_eviction_key(
    keys: &[(u64, u32, u32, u32, u64)],
    current_revision: u64,
    current_frame: u32,
) -> Option<(u64, u32, u32, u32, u64)> {
    keys.iter()
        .find(|key| key.0 != current_revision)
        .copied()
        .or_else(|| {
            keys.iter()
                .copied()
                .max_by_key(|key| key.3.abs_diff(current_frame))
        })
}

impl AfterEffectsStudio {
    pub(crate) fn viewport_preview_limit(&self) -> f64 {
        let mut preview_limit: f64 = match self.playback_quality.as_str() {
            "full" => 960.0,
            "high" => 768.0,
            "balanced" => 640.0,
            "performance" => 480.0,
            _ => {
                if self.adaptive_preview && self.is_playing {
                    320.0
                } else {
                    640.0
                }
            }
        };
        if let Some(limit) = match self.preview_resolution.as_str() {
            "full" => Some(960.0_f64),
            "half" => Some(480.0),
            "quarter" => Some(320.0),
            _ => None,
        } {
            preview_limit = preview_limit.min(limit);
        }
        if !self.high_quality_preview {
            preview_limit = preview_limit.min(480.0);
        }
        preview_limit
    }

    pub fn draw_viewport(&mut self, scene: &mut Scene, x: f64, y: f64, width: f64, height: f64) {
        let viewport = Rect::new(x, y, x + width, y + height);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(8, 9, 11),
            None,
            &viewport,
        );
        let preview_limit = self.viewport_preview_limit();
        let composition = self.project_model.active_composition();
        let source_width = composition.width.max(1);
        let source_height = composition.height.max(1);
        let pixel_aspect = f64::from(composition.pixel_aspect).clamp(0.1, 10.0);
        let proxy_scale = f64::from(kagari_vfx::core::proxy::composition_preview_scale(
            composition,
        ));
        let render_scale = (preview_limit / (f64::from(source_width) * pixel_aspect))
            .min(preview_limit / f64::from(source_height))
            .min(1.0)
            * proxy_scale;
        let preview_width = (f64::from(source_width) * render_scale).round().max(1.0) as u32;
        let preview_height = (f64::from(source_height) * render_scale).round().max(1.0) as u32;
        let frame_index = self
            .current_frame
            .min(composition.duration_frames.saturating_sub(1));
        let media_revision =
            kagari_vfx::core::media_revision::composition_media_revision(composition, frame_index);
        let preview_key = ViewportPreviewKey {
            revision: self.preview_revision,
            media_revision,
            frame: frame_index,
            width: preview_width,
            height: preview_height,
        };
        let current_k = (
            preview_key.revision,
            preview_key.width,
            preview_key.height,
            preview_key.frame,
            preview_key.media_revision,
        );
        let current_preview_cached = self.preview_cache.contains_key(&current_k);
        if self.viewport_preview_active_key.is_some_and(|active| {
            let cancel_for_playhead = !current_preview_cached
                && viewport_preview_job_should_cancel(active, preview_key, self.is_playing);
            let cancel_for_config = active.revision != preview_key.revision
                || active.width != preview_key.width
                || active.height != preview_key.height;
            cancel_for_playhead || cancel_for_config
        }) {
            if let Some(cancel) = &self.viewport_preview_cancel {
                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
        loop {
            match self.viewport_preview_result_rx.try_recv() {
                Ok(result) => {
                    if self.viewport_preview_active_key == Some(result.key) {
                        self.viewport_preview_inflight = false;
                        self.viewport_preview_active_key = None;
                        self.viewport_preview_cancel = None;
                    }
                    if result.key == preview_key {
                        if let Some(pixels) = result.pixels.filter(|pixels| {
                            pixels.len()
                                == (preview_width as usize)
                                    .saturating_mul(preview_height as usize)
                                    .saturating_mul(4)
                        }) {
                            let pixels = if self.node_graph_live_preview {
                                self.evaluate_compositor_graph_rgba8(
                                    preview_width,
                                    preview_height,
                                    pixels,
                                )
                            } else {
                                pixels
                            };
                            let image = vello::peniko::ImageData {
                                data: vello::peniko::Blob::new(pixels),
                                format: vello::peniko::ImageFormat::Rgba8,
                                alpha_type: vello::peniko::ImageAlphaType::Alpha,
                                width: preview_width,
                                height: preview_height,
                            };
                            let cache_k = (
                                result.key.revision,
                                result.key.width,
                                result.key.height,
                                result.key.frame,
                                result.key.media_revision,
                            );
                            let incoming_bytes = viewport_preview_cache_entry_bytes(
                                result.key.width,
                                result.key.height,
                            );
                            let mut cached_bytes = self
                                .preview_cache
                                .keys()
                                .map(|key| viewport_preview_cache_entry_bytes(key.1, key.2))
                                .fold(0u64, u64::saturating_add);
                            let (max_cache_entries, max_cache_bytes) =
                                viewport_preview_cache_limits(self.memory_limit_gb);
                            while viewport_preview_cache_needs_eviction_with_limits(
                                self.preview_cache.len(),
                                cached_bytes,
                                incoming_bytes,
                                max_cache_entries,
                                max_cache_bytes,
                            ) {
                                let cache_keys =
                                    self.preview_cache.keys().copied().collect::<Vec<_>>();
                                let evict_key = viewport_preview_cache_eviction_key(
                                    &cache_keys,
                                    preview_key.revision,
                                    preview_key.frame,
                                );
                                let Some(evict_key) = evict_key else {
                                    break;
                                };
                                if self.preview_cache.remove(&evict_key).is_some() {
                                    cached_bytes = cached_bytes.saturating_sub(
                                        viewport_preview_cache_entry_bytes(
                                            evict_key.1,
                                            evict_key.2,
                                        ),
                                    );
                                }
                            }
                            self.preview_cache.insert(cache_k, image);
                            self.viewport_preview_failed = None;
                        } else {
                            self.viewport_preview_failed = Some(preview_key);
                        }
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.viewport_preview_inflight = false;
                    self.viewport_preview_active_key = None;
                    self.viewport_preview_cancel = None;
                    self.viewport_preview_failed = Some(preview_key);
                    self.viewport_preview_worker_failed = true;
                    let _ = self
                        .ui
                        .set_binding("preview_worker_failed", Value::Bool(true));
                    break;
                }
            }
        }
        let current_k = (
            preview_key.revision,
            preview_key.width,
            preview_key.height,
            preview_key.frame,
            preview_key.media_revision,
        );
        let current_preview = self.preview_cache.get(&current_k).cloned();
        let preview = current_preview.clone().or_else(|| {
            // Fallback to closest cached frame of the same revision and dimensions
            self.preview_cache
                .iter()
                .filter(|(k, _)| {
                    k.0 == preview_key.revision
                        && k.1 == preview_key.width
                        && k.2 == preview_key.height
                        && k.4 == preview_key.media_revision
                })
                .min_by_key(|(k, _)| (k.3 as i64 - preview_key.frame as i64).abs())
                .map(|(_, img)| img.clone())
        });
        if (current_preview.is_none() || (!self.is_playing && !self.viewport_preview_inflight))
            && !self.viewport_preview_worker_failed
        {
            if self.viewport_preview_inflight {
                // Let the active render finish; the next frame will request the latest playhead.
            } else {
                let target_key = if current_preview.is_none()
                    && self.viewport_preview_failed != Some(preview_key)
                {
                    Some(preview_key)
                } else if !self.is_playing && composition.duration_frames > 1 {
                    let (max_entries, max_bytes) =
                        viewport_preview_cache_limits(self.memory_limit_gb);
                    let cached_bytes = self
                        .preview_cache
                        .keys()
                        .map(|key| viewport_preview_cache_entry_bytes(key.1, key.2))
                        .fold(0u64, u64::saturating_add);
                    let entry_bytes =
                        viewport_preview_cache_entry_bytes(preview_width, preview_height);
                    if self.preview_cache.len() < max_entries
                        && cached_bytes.saturating_add(entry_bytes) <= max_bytes
                    {
                        let total = composition.duration_frames;
                        (1..=16.min(total - 1)).find_map(|offset| {
                            let candidate_frame = (frame_index + offset) % total;
                            let candidate_media_rev =
                                kagari_vfx::core::media_revision::composition_media_revision(
                                    composition,
                                    candidate_frame,
                                );
                            let candidate_k = (
                                preview_key.revision,
                                preview_width,
                                preview_height,
                                candidate_frame,
                                candidate_media_rev,
                            );
                            let candidate_key = ViewportPreviewKey {
                                revision: preview_key.revision,
                                media_revision: candidate_media_rev,
                                frame: candidate_frame,
                                width: preview_width,
                                height: preview_height,
                            };
                            (!self.preview_cache.contains_key(&candidate_k)
                                && self.viewport_preview_failed != Some(candidate_key))
                            .then_some(candidate_key)
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(job_key) = target_key {
                    let job = ViewportPreviewJob {
                        key: job_key,
                        composition: Box::new(composition.clone()),
                        cancel: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    };
                    let cancel = job.cancel.clone();
                    match self.viewport_preview_request_tx.try_send(job) {
                        Ok(()) => {
                            self.viewport_preview_inflight = true;
                            self.viewport_preview_active_key = Some(job_key);
                            self.viewport_preview_cancel = Some(cancel);
                        }
                        Err(std::sync::mpsc::TrySendError::Full(_)) => {}
                        Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                            self.viewport_preview_failed = Some(job_key);
                            self.viewport_preview_worker_failed = true;
                            let _ = self
                                .ui
                                .set_binding("preview_worker_failed", Value::Bool(true));
                        }
                    }
                }
            }
        }
        scene.push_clip_layer(Fill::NonZero, Affine::IDENTITY, &viewport);
        let composition_frame;
        if let Some(image) = preview {
            let zoom_factor = self
                .zoom
                .trim()
                .trim_end_matches('%')
                .parse::<f64>()
                .unwrap_or(100.0)
                .clamp(10.0, 800.0)
                / 100.0;
            let fit = (width - 32.0).max(1.0).min(
                (height - 32.0).max(1.0) * f64::from(preview_width) * pixel_aspect
                    / f64::from(preview_height),
            ) / (f64::from(preview_width) * pixel_aspect)
                * zoom_factor;
            let draw_width = f64::from(preview_width) * pixel_aspect * fit;
            let draw_height = f64::from(preview_height) * fit;
            let left = x + (width - draw_width) * 0.5 + self.viewport_pan[0];
            let top = y + (height - draw_height) * 0.5 + self.viewport_pan[1];
            let filtered_image =
                if self.viewport_channel_mode != "RGBA" || self.viewport_exposure_ev != 0.0 {
                    use rayon::prelude::*;
                    let gain = 2f32.powf(self.viewport_exposure_ev as f32);
                    let src = image.data.data();
                    let mut dst = vec![0u8; src.len()];
                    let mode = self.viewport_channel_mode.as_str();
                    let apply_gain = |c: u8| -> u8 {
                        ((f32::from(c) * gain).round().clamp(0.0, 255.0)) as u8
                    };
                    dst.par_chunks_exact_mut(4)
                        .zip(src.par_chunks_exact(4))
                        .for_each(|(out, px)| match mode {
                            "R" => {
                                let v = apply_gain(px[0]);
                                out.copy_from_slice(&[v, v, v, 255]);
                            }
                            "G" => {
                                let v = apply_gain(px[1]);
                                out.copy_from_slice(&[v, v, v, 255]);
                            }
                            "B" => {
                                let v = apply_gain(px[2]);
                                out.copy_from_slice(&[v, v, v, 255]);
                            }
                            "A" => {
                                let v = px[3];
                                out.copy_from_slice(&[v, v, v, 255]);
                            }
                            "Luma" => {
                                let luma = (0.2126 * f32::from(px[0])
                                    + 0.7152 * f32::from(px[1])
                                    + 0.0722 * f32::from(px[2]))
                                    * gain;
                                let v = luma.round().clamp(0.0, 255.0) as u8;
                                out.copy_from_slice(&[v, v, v, 255]);
                            }
                            "Zebra" => {
                                let r = apply_gain(px[0]);
                                let g = apply_gain(px[1]);
                                let b = apply_gain(px[2]);
                                if r >= 254 || g >= 254 || b >= 254 {
                                    out.copy_from_slice(&[255, 48, 48, 255]);
                                } else if r <= 1 && g <= 1 && b <= 1 {
                                    out.copy_from_slice(&[40, 110, 255, 255]);
                                } else {
                                    out.copy_from_slice(&[r, g, b, px[3]]);
                                }
                            }
                            _ => {
                                out.copy_from_slice(&[
                                    apply_gain(px[0]),
                                    apply_gain(px[1]),
                                    apply_gain(px[2]),
                                    px[3],
                                ]);
                            }
                        });
                    Some(vello::peniko::ImageData {
                        data: vello::peniko::Blob::from(dst),
                        format: image.format,
                        alpha_type: image.alpha_type,
                        width: image.width,
                        height: image.height,
                    })
                } else {
                    None
                };
            let draw_img = filtered_image.as_ref().unwrap_or(&image);
            if self.viewport_wipe_enabled {
                let split_x =
                    (left + draw_width * self.viewport_wipe_split.clamp(0.05, 0.95)).round() + 0.5;
                let transform = Affine::translate((left, top))
                    * Affine::scale_non_uniform(fit * pixel_aspect, fit);
                let left_clip = Rect::new(left, top, split_x, top + draw_height);
                scene.push_clip_layer(Fill::NonZero, Affine::IDENTITY, &left_clip);
                scene.draw_image(&image, transform);
                let hud_border = Color::from_rgba8(255, 255, 255, 24);
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            hud_border,
            None,
            &Rect::new(x + 0.5, y + 0.5, x + width - 0.5, y + height - 0.5),
        );
        let corner = 10.0;
        let bracket_color = Color::from_rgba8(255, 255, 255, 55);
        for (cx, cy, dx, dy) in [
            (x + 8.5, y + 8.5, 1.0, 1.0),
            (x + width - 8.5, y + 8.5, -1.0, 1.0),
            (x + 8.5, y + height - 8.5, 1.0, -1.0),
            (x + width - 8.5, y + height - 8.5, -1.0, -1.0),
        ] {
            scene.stroke(
                &Stroke::new(1.2),
                Affine::IDENTITY,
                bracket_color,
                None,
                &Line::new((cx, cy), (cx + corner * dx, cy)),
            );
            scene.stroke(
                &Stroke::new(1.2),
                Affine::IDENTITY,
                bracket_color,
                None,
                &Line::new((cx, cy), (cx, cy + corner * dy)),
            );
        }

        scene.pop_layer();

                let right_clip = Rect::new(split_x, top, left + draw_width, top + draw_height);
                scene.push_clip_layer(Fill::NonZero, Affine::IDENTITY, &right_clip);
                scene.draw_image(draw_img, transform);
                let hud_border = Color::from_rgba8(255, 255, 255, 24);
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            hud_border,
            None,
            &Rect::new(x + 0.5, y + 0.5, x + width - 0.5, y + height - 0.5),
        );
        let corner = 10.0;
        let bracket_color = Color::from_rgba8(255, 255, 255, 55);
        for (cx, cy, dx, dy) in [
            (x + 8.5, y + 8.5, 1.0, 1.0),
            (x + width - 8.5, y + 8.5, -1.0, 1.0),
            (x + 8.5, y + height - 8.5, 1.0, -1.0),
            (x + width - 8.5, y + height - 8.5, -1.0, -1.0),
        ] {
            scene.stroke(
                &Stroke::new(1.2),
                Affine::IDENTITY,
                bracket_color,
                None,
                &Line::new((cx, cy), (cx + corner * dx, cy)),
            );
            scene.stroke(
                &Stroke::new(1.2),
                Affine::IDENTITY,
                bracket_color,
                None,
                &Line::new((cx, cy), (cx, cy + corner * dy)),
            );
        }

        scene.pop_layer();

                scene.stroke(
                    &Stroke::new(1.5),
                    Affine::IDENTITY,
                    Color::from_rgba8(255, 255, 255, 210),
                    None,
                    &Line::new((split_x, top), (split_x, top + draw_height)),
                );
            } else {
                scene.draw_image(
                    draw_img,
                    Affine::translate((left, top))
                        * Affine::scale_non_uniform(fit * pixel_aspect, fit),
                );
            }
            let frame = Rect::new(left, top, left + draw_width, top + draw_height);
            if self.viewport_wipe_enabled {
                let split_x =
                    (left + draw_width * self.viewport_wipe_split.clamp(0.05, 0.95)).round() + 0.5;
                let mid_y = top + draw_height * 0.5;
                scene.stroke(
                    &Stroke::new(1.5),
                    Affine::IDENTITY,
                    Color::from_rgb8(245, 140, 75),
                    None,
                    &Line::new((split_x, top), (split_x, top + draw_height)),
                );
                let grip = RoundedRect::new(split_x - 4.0, mid_y - 14.0, split_x + 4.0, mid_y + 14.0, 4.0);
                scene.fill(Fill::NonZero, Affine::IDENTITY, Color::from_rgb8(245, 140, 75), None, &grip);
                scene.stroke(
                    &Stroke::new(1.0),
                    Affine::IDENTITY,
                    Color::from_rgb8(20, 22, 26),
                    None,
                    &grip,
                );
            }
            scene.stroke(
                &Stroke::new(1.0),
                Affine::IDENTITY,
                Color::from_rgb8(42, 48, 58),
                None,
                &frame,
            );
            composition_frame = Some((frame, draw_width, draw_height));
        } else {
            let frame = Rect::new(x + 16.0, y + 16.0, x + width - 16.0, y + height - 16.0);
            scene.stroke(
                &Stroke::new(1.0),
                Affine::IDENTITY,
                Color::from_rgb8(65, 70, 76),
                None,
                &frame,
            );
            composition_frame = Some((frame, frame.width(), frame.height()));
        }
        if let Some((frame, draw_width, draw_height)) = composition_frame {
            if self.safe_area_visible {
                for (scale, color) in [
                    (0.9, Color::from_rgba8(255, 255, 255, 70)),
                    (0.8, Color::from_rgba8(255, 190, 90, 80)),
                ] {
                    let inset_x = draw_width * (1.0 - scale) * 0.5;
                    let inset_y = draw_height * (1.0 - scale) * 0.5;
                    scene.stroke(
                        &Stroke::new(1.0),
                        Affine::IDENTITY,
                        color,
                        None,
                        &Rect::new(
                            frame.x0 + inset_x,
                            frame.y0 + inset_y,
                            frame.x1 - inset_x,
                            frame.y1 - inset_y,
                        ),
                    );
                }
                let cx = (frame.x0 + frame.x1) * 0.5;
                let cy = (frame.y0 + frame.y1) * 0.5;
                let cross_color = Color::from_rgba8(255, 255, 255, 95);
                scene.stroke(
                    &Stroke::new(1.0),
                    Affine::IDENTITY,
                    cross_color,
                    None,
                    &Line::new((cx - 10.0, cy), (cx + 10.0, cy)),
                );
                scene.stroke(
                    &Stroke::new(1.0),
                    Affine::IDENTITY,
                    cross_color,
                    None,
                    &Line::new((cx, cy - 10.0), (cx, cy + 10.0)),
                );
            }
            if self.grid_visible && draw_width > 8.0 && draw_height > 8.0 {
                let spacing = self.grid_spacing.clamp(8.0, 512.0);
                let step_x = (spacing * draw_width / f64::from(source_width)).max(4.0);
                let step_y = (spacing * draw_height / f64::from(source_height)).max(4.0);
                let color = Color::from_rgba8(255, 255, 255, 34);
                let mut grid_x = frame.x0 + step_x;
                while grid_x < frame.x1 - 0.5 {
                    scene.stroke(
                        &Stroke::new(1.0),
                        Affine::IDENTITY,
                        color,
                        None,
                        &Line {
                            p0: Point::new(grid_x, frame.y0),
                            p1: Point::new(grid_x, frame.y1),
                        },
                    );
                    grid_x += step_x;
                }
                let mut grid_y = frame.y0 + step_y;
                while grid_y < frame.y1 - 0.5 {
                    scene.stroke(
                        &Stroke::new(1.0),
                        Affine::IDENTITY,
                        color,
                        None,
                        &Line {
                            p0: Point::new(frame.x0, grid_y),
                            p1: Point::new(frame.x1, grid_y),
                        },
                    );
                    grid_y += step_y;
                }
            }
        }
        self.draw_viewport_overlays(
            scene,
            super::ame_studio_viewport_overlays::ViewportOverlayContext {
                x,
                y,
                width,
                height,
                source_width,
                source_height,
                frame_index,
            },
        );

        let hud_border = Color::from_rgba8(255, 255, 255, 24);
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            hud_border,
            None,
            &Rect::new(x + 0.5, y + 0.5, x + width - 0.5, y + height - 0.5),
        );
        let corner = 10.0;
        let bracket_color = Color::from_rgba8(255, 255, 255, 55);
        for (cx, cy, dx, dy) in [
            (x + 8.5, y + 8.5, 1.0, 1.0),
            (x + width - 8.5, y + 8.5, -1.0, 1.0),
            (x + 8.5, y + height - 8.5, 1.0, -1.0),
            (x + width - 8.5, y + height - 8.5, -1.0, -1.0),
        ] {
            scene.stroke(
                &Stroke::new(1.2),
                Affine::IDENTITY,
                bracket_color,
                None,
                &Line::new((cx, cy), (cx + corner * dx, cy)),
            );
            scene.stroke(
                &Stroke::new(1.2),
                Affine::IDENTITY,
                bracket_color,
                None,
                &Line::new((cx, cy), (cx, cy + corner * dy)),
            );
        }

        scene.pop_layer();
    }
}

#[cfg(test)]
mod preview_cancellation_tests {
    use super::*;

    fn key(revision: u64, frame: u32, width: u32, height: u32) -> ViewportPreviewKey {
        ViewportPreviewKey {
            revision,
            media_revision: frame as u64,
            frame,
            width,
            height,
        }
    }

    #[test]
    fn paused_scrubbing_cancels_an_obsolete_frame() {
        assert!(viewport_preview_job_should_cancel(
            key(4, 12, 640, 360),
            key(4, 13, 640, 360),
            false,
        ));
    }

    #[test]
    fn playback_does_not_cancel_every_frame_render() {
        assert!(!viewport_preview_job_should_cancel(
            key(4, 12, 640, 360),
            key(4, 13, 640, 360),
            true,
        ));
    }

    #[test]
    fn project_edits_and_preview_resize_cancel_even_during_playback() {
        assert!(viewport_preview_job_should_cancel(
            key(4, 12, 640, 360),
            key(5, 12, 640, 360),
            true,
        ));
        assert!(viewport_preview_job_should_cancel(
            key(4, 12, 640, 360),
            key(4, 12, 480, 270),
            true,
        ));
    }

    #[test]
    fn preview_cache_obeys_both_entry_and_byte_budgets() {
        assert!(viewport_preview_cache_needs_eviction(
            MAX_VIEWPORT_PREVIEW_CACHE_ENTRIES,
            0,
            4,
        ));
        assert!(viewport_preview_cache_needs_eviction(
            10,
            MAX_VIEWPORT_PREVIEW_CACHE_BYTES - 1,
            2,
        ));
        let square_preview_bytes = viewport_preview_cache_entry_bytes(960, 960);
        assert!(viewport_preview_cache_needs_eviction(
            40,
            square_preview_bytes * 39,
            square_preview_bytes,
        ));
        assert!(!viewport_preview_cache_needs_eviction(
            16,
            square_preview_bytes * 15,
            square_preview_bytes,
        ));
    }

    #[test]
    fn preview_cache_evicts_old_revisions_before_distant_frames() {
        let keys = [
            (4, 640, 360, 3, 7),
            (5, 640, 360, 20, 8),
            (5, 640, 360, 2, 9),
        ];
        assert_eq!(
            viewport_preview_cache_eviction_key(&keys, 5, 10),
            Some(keys[0])
        );

        let current_revision = &keys[1..];
        assert_eq!(
            viewport_preview_cache_eviction_key(current_revision, 5, 10),
            Some(keys[1])
        );
    }
}
