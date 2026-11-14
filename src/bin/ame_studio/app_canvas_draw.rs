use super::*;

fn composition_asset_preview_cache_key(
    asset_id: &str,
    preview_revision: u64,
    composition: Option<&kagari_vfx::core::timeline::Composition>,
) -> String {
    let media_revision = composition.map_or(0, |composition| {
        kagari_vfx::core::media_revision::composition_media_revision(composition, 0)
    });
    format!("composition:{asset_id}:{preview_revision}:{media_revision}")
}

impl AfterEffectsStudio {
    pub(crate) fn draw_canvas_inner(
        &mut self,
        canvas_id: &str,
        scene: &mut Scene,
        bounds: CanvasBounds,
        _context: &mut CanvasContext<'_>,
        _viewport: Viewport,
        _tree: &ViewTree,
    ) {
        self.advance_playback();
        self.maybe_autosave();
        let x = f64::from(bounds.x);
        let y = f64::from(bounds.y);
        let width = f64::from(bounds.width.max(1.0));
        let height = f64::from(bounds.height.max(1.0));
        if canvas_id == "kagari.graph" {
            self.graph_canvas_width = width;
            self.graph_canvas_height = height;
            let duration = self
                .project_model
                .active_composition()
                .duration_frames
                .max(1);
            let (timeline_start, visible_frames) = self.timeline_window(duration);
            let composition = self.project_model.active_composition();
            let selected = self
                .selected_layer_id
                .as_deref()
                .and_then(|id| composition.layers.iter().find(|layer| layer.id == id));
            let camera = selected.and_then(|layer| camera_for_layer(composition, layer));
            let light = selected.and_then(|layer| light_for_layer(composition, layer));
            let keys = selected
                .map(|layer| {
                    graph_channel_keys(
                        layer,
                        camera,
                        light,
                        self.selected_mask_id.as_deref(),
                        self.selected_effect_id.as_deref(),
                        self.graph_effect_parameter_id.as_deref(),
                        self.graph_channel,
                    )
                })
                .unwrap_or_default();
            draw_value_graph(
                scene,
                x,
                y,
                width,
                height,
                self.current_frame,
                duration,
                timeline_start,
                visible_frames,
                self.graph_channel,
                selected,
                camera,
                light,
                self.selected_mask_id.as_deref(),
                self.selected_effect_id.as_deref(),
                self.graph_effect_parameter_id.as_deref(),
                &keys,
            );
            if let Some(snap_frame) = self.timeline_snap_frame {
                let guide_x = x
                    + 14.0
                    + (f64::from(snap_frame) - timeline_start) / visible_frames.max(1.0)
                        * (width - 28.0).max(1.0);
                if guide_x >= x + 14.0 && guide_x <= x + width - 14.0 {
                    scene.stroke(
                        &Stroke::new(1.5),
                        Affine::IDENTITY,
                        Color::from_rgb8(100, 238, 208),
                        None,
                        &Line::new((guide_x, y + 12.0), (guide_x, y + height - 12.0)),
                    );
                }
            }
            return;
        }
        if canvas_id == "kagari.flowchart" {
            let composition = self.project_model.active_composition();
            let drag_wire = self
                .flowchart_wire_drag
                .as_ref()
                .map(|d| FlowchartDragWireView {
                    start: d.start_pos,
                    current: d.current_pos,
                    is_matte: d.kind == FlowchartWireKind::Matte,
                });
            draw_flowchart(
                scene,
                x,
                y,
                width,
                height,
                composition,
                self.selected_layer_id.as_deref(),
                drag_wire,
            );
            return;
        }
        if canvas_id == "kagari.timeline" {
            self.timeline_width = width;
            self.timeline_canvas_height = height;
            let layers = self.timeline_layers();
            let max_scroll = (layers.len() as f64 * 28.0 - (height - 22.0).max(0.0)).max(0.0);
            self.timeline_vertical_scroll = self.timeline_vertical_scroll.clamp(0.0, max_scroll);
            let first_visible_row = (self.timeline_vertical_scroll / 28.0).floor() as usize;
            let last_visible_row =
                ((self.timeline_vertical_scroll + (height - 22.0).max(0.0)) / 28.0).ceil() as usize;
            let visible_end = last_visible_row
                .min(layers.len())
                .max(first_visible_row.min(layers.len()));
            let waveform_state_changed = self.update_audio_waveforms(
                layers
                    .get(first_visible_row.min(layers.len())..visible_end)
                    .unwrap_or_default(),
            );
            if waveform_state_changed {
                self.sync_bindings_logged();
            }
            let composition = self.project_model.active_composition();
            let fps = composition.fps.max(1);
            let (timeline_start, visible_frames) =
                self.timeline_window(composition.duration_frames);
            draw_timeline(
                scene,
                x,
                y,
                width,
                height,
                self.current_frame,
                self.playback_range,
                composition.duration_frames,
                timeline_start,
                visible_frames,
                self.timeline_vertical_scroll,
                &layers,
                self.selected_layer_id.as_deref(),
                self.selected_layer_marker_frame,
                &self.waveform_cache,
                fps,
            );
            for (row, layer) in layers
                .iter()
                .enumerate()
                .skip(first_visible_row)
                .take(visible_end.saturating_sub(first_visible_row))
            {
                let camera_id =
                    layer
                        .scene_object
                        .as_ref()
                        .and_then(|scene_object| match scene_object {
                            kagari_vfx::core::timeline::SceneObjectRef::Camera { id } => {
                                Some(id.as_str())
                            }
                            _ => None,
                        });
                let Some(camera) = camera_id
                    .and_then(|id| composition.cameras.iter().find(|camera| camera.id == id))
                else {
                    continue;
                };
                for keyframe in camera_keyframe_frames(camera) {
                    let marker_x = x + width * (f64::from(keyframe) - timeline_start)
                        / visible_frames.max(1.0);
                    if marker_x < x || marker_x > x + width {
                        continue;
                    }
                    let center_y =
                        y + 22.0 + row as f64 * 28.0 + 14.0 - self.timeline_vertical_scroll;
                    let mut diamond = BezPath::new();
                    diamond.move_to((marker_x, center_y - 4.0));
                    diamond.line_to((marker_x + 4.0, center_y));
                    diamond.line_to((marker_x, center_y + 4.0));
                    diamond.line_to((marker_x - 4.0, center_y));
                    diamond.close_path();
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        Color::from_rgb8(255, 218, 174),
                        None,
                        &diamond,
                    );
                }
            }
            if let Some(snap_frame) = self.timeline_snap_frame {
                let guide_x =
                    x + width * (f64::from(snap_frame) - timeline_start) / visible_frames.max(1.0);
                if guide_x >= x && guide_x <= x + width {
                    scene.stroke(
                        &Stroke::new(1.5),
                        Affine::IDENTITY,
                        Color::from_rgb8(100, 238, 208),
                        None,
                        &Line::new((guide_x, y), (guide_x, y + height)),
                    );
                }
            }
            return;
        }
        if canvas_id == "kagari.asset_preview" {
            if self.update_audio_waveforms(&[]) {
                self.sync_bindings_logged();
            }
            let bounds_rect = Rect::new(x, y, x + width, y + height);
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                Color::from_rgb8(10, 11, 13),
                None,
                &bounds_rect,
            );
            let selected = self.selected_project_asset_id.as_deref().and_then(|id| {
                self.project_model
                    .assets
                    .iter()
                    .find(|item| item.id == id)
                    .map(|item| (item.id.clone(), item.item_type.clone()))
            });
            let selected_render_key = selected.as_ref().and_then(|(asset_id, item_type)| {
                use kagari_vfx::core::timeline::ProjectItemType;
                match item_type {
                    ProjectItemType::Image { path, .. } => {
                        Some(asset_image_preview_cache_key(asset_id, path))
                    }
                    ProjectItemType::Video { path, .. } => find_video_preview_frame(
                        &self.project_model.compositions,
                        path,
                    )
                    .map(|frame| asset_image_preview_cache_key(asset_id, &frame.to_string_lossy())),
                    ProjectItemType::Model3D { path } => Some(model_preview_cache_key(path)),
                    ProjectItemType::Composition { comp_idx } => {
                        Some(composition_asset_preview_cache_key(
                            asset_id,
                            self.preview_revision,
                            self.project_model.compositions.get(*comp_idx),
                        ))
                    }
                    _ => None,
                }
            });
            self.asset_render_preview_pending
                .retain(|key, _| selected_render_key.as_ref() == Some(key));
            if self.update_asset_preview_results() {
                self.sync_bindings_logged();
            }
            if let Some((asset_id, item_type)) = selected {
                use kagari_vfx::core::timeline::ProjectItemType;
                match item_type {
                    ProjectItemType::Audio { path, .. } => {
                        self.asset_preview_image = None;
                        self.asset_preview_failed_asset_id = None;
                        if let Some(waveform) = self.waveform_cache.get(&path) {
                            draw_asset_waveform(scene, x, y, width, height, waveform);
                        }
                    }
                    ProjectItemType::Model3D { path } => {
                        let cache_key = model_preview_cache_key(&path);
                        match self.asset_render_preview_cache.get(&cache_key) {
                            Some(Some(preview)) => {
                                let image = vello::peniko::ImageData {
                                    data: vello::peniko::Blob::new(preview.pixels.clone()),
                                    format: vello::peniko::ImageFormat::Rgba8,
                                    alpha_type: vello::peniko::ImageAlphaType::Alpha,
                                    width: preview.width,
                                    height: preview.height,
                                };
                                self.asset_preview_image = Some((cache_key, Some(image)));
                                self.asset_preview_failed_asset_id = None;
                            }
                            Some(None) => {
                                self.asset_preview_image = Some((cache_key, None));
                                self.asset_preview_failed_asset_id = Some(asset_id.clone());
                            }
                            None => {
                                self.asset_preview_image = Some((cache_key.clone(), None));
                                self.asset_preview_failed_asset_id =
                                    self.asset_preview_worker_failed.then(|| asset_id.clone());
                                if self.asset_preview_worker_failed {
                                    self.asset_render_preview_cache.insert(cache_key, None);
                                } else if !self.asset_render_preview_requested.contains(&cache_key)
                                    && !self.asset_render_preview_pending.contains_key(&cache_key)
                                {
                                    let job = AssetPreviewJob::Model3D(path);
                                    if self
                                        .asset_render_preview_request_tx
                                        .try_send((cache_key.clone(), job.clone()))
                                        .is_ok()
                                    {
                                        self.asset_render_preview_requested.insert(cache_key);
                                    } else {
                                        self.asset_render_preview_pending.insert(cache_key, job);
                                    }
                                }
                            }
                        }
                    }
                    ProjectItemType::Solid { color } => {
                        let rgba = color
                            .map(|component| (component.clamp(0.0, 1.0) * 255.0).round() as u8);
                        let cache_key = format!(
                            "solid:{asset_id}:{}:{}:{}:{}",
                            rgba[0], rgba[1], rgba[2], rgba[3]
                        );
                        let needs_refresh = self
                            .asset_preview_image
                            .as_ref()
                            .is_none_or(|(key, _)| key != &cache_key);
                        if needs_refresh {
                            let pixels = rgba.repeat(160 * 90);
                            let image = vello::peniko::ImageData {
                                data: vello::peniko::Blob::new(std::sync::Arc::new(pixels)),
                                format: vello::peniko::ImageFormat::Rgba8,
                                alpha_type: vello::peniko::ImageAlphaType::Alpha,
                                width: 160,
                                height: 90,
                            };
                            self.asset_preview_image = Some((cache_key, Some(image)));
                            self.asset_preview_failed_asset_id = None;
                        }
                    }
                    ProjectItemType::Composition { comp_idx } => {
                        let cache_key = composition_asset_preview_cache_key(
                            &asset_id,
                            self.preview_revision,
                            self.project_model.compositions.get(comp_idx),
                        );
                        match self.asset_render_preview_cache.get(&cache_key) {
                            Some(Some(preview)) => {
                                let image = vello::peniko::ImageData {
                                    data: vello::peniko::Blob::new(preview.pixels.clone()),
                                    format: vello::peniko::ImageFormat::Rgba8,
                                    alpha_type: vello::peniko::ImageAlphaType::Alpha,
                                    width: preview.width,
                                    height: preview.height,
                                };
                                self.asset_preview_image = Some((cache_key, Some(image)));
                                self.asset_preview_failed_asset_id = None;
                            }
                            Some(None) => {
                                self.asset_preview_image = Some((cache_key, None));
                                self.asset_preview_failed_asset_id = Some(asset_id.clone());
                            }
                            None => {
                                self.asset_preview_image = Some((cache_key.clone(), None));
                                self.asset_preview_failed_asset_id =
                                    self.asset_preview_worker_failed.then(|| asset_id.clone());
                                if self.asset_preview_worker_failed {
                                    self.asset_render_preview_cache.insert(cache_key, None);
                                } else if !self.asset_render_preview_requested.contains(&cache_key)
                                    && !self.asset_render_preview_pending.contains_key(&cache_key)
                                {
                                    if let Some(composition) =
                                        self.project_model.compositions.get(comp_idx).cloned()
                                    {
                                        let job =
                                            AssetPreviewJob::Composition(Box::new(composition));
                                        if self
                                            .asset_render_preview_request_tx
                                            .try_send((cache_key.clone(), job.clone()))
                                            .is_ok()
                                        {
                                            self.asset_render_preview_requested.insert(cache_key);
                                        } else {
                                            self.asset_render_preview_pending
                                                .insert(cache_key, job);
                                        }
                                    } else {
                                        self.asset_preview_failed_asset_id = Some(asset_id.clone());
                                    }
                                }
                            }
                        }
                    }
                    item_type => {
                        let path = match item_type {
                            ProjectItemType::Image { path, .. } => {
                                Some(std::path::PathBuf::from(path))
                            }
                            ProjectItemType::Video { path, .. } => {
                                find_video_preview_frame(&self.project_model.compositions, &path)
                            }
                            _ => None,
                        };
                        if let Some(path) = path {
                            let path = path.to_string_lossy().into_owned();
                            let cache_key = asset_image_preview_cache_key(&asset_id, &path);
                            match self.asset_render_preview_cache.get(&cache_key) {
                                Some(Some(preview)) => {
                                    let image = vello::peniko::ImageData {
                                        data: vello::peniko::Blob::new(preview.pixels.clone()),
                                        format: vello::peniko::ImageFormat::Rgba8,
                                        alpha_type: vello::peniko::ImageAlphaType::Alpha,
                                        width: preview.width,
                                        height: preview.height,
                                    };
                                    self.asset_preview_image = Some((cache_key, Some(image)));
                                    self.asset_preview_failed_asset_id = None;
                                }
                                Some(None) => {
                                    self.asset_preview_image = Some((cache_key, None));
                                    self.asset_preview_failed_asset_id = Some(asset_id.clone());
                                }
                                None => {
                                    self.asset_preview_image = Some((cache_key.clone(), None));
                                    self.asset_preview_failed_asset_id =
                                        self.asset_preview_worker_failed.then(|| asset_id.clone());
                                    if self.asset_preview_worker_failed {
                                        self.asset_render_preview_cache.insert(cache_key, None);
                                    } else if !self
                                        .asset_render_preview_requested
                                        .contains(&cache_key)
                                        && !self
                                            .asset_render_preview_pending
                                            .contains_key(&cache_key)
                                    {
                                        let job = AssetPreviewJob::Image(path);
                                        if self
                                            .asset_render_preview_request_tx
                                            .try_send((cache_key.clone(), job.clone()))
                                            .is_ok()
                                        {
                                            self.asset_render_preview_requested.insert(cache_key);
                                        } else {
                                            self.asset_render_preview_pending
                                                .insert(cache_key, job);
                                        }
                                    }
                                }
                            }
                        } else {
                            self.asset_preview_image = None;
                            self.asset_preview_failed_asset_id = None;
                        }
                    }
                }
            } else {
                self.asset_preview_image = None;
                self.asset_preview_failed_asset_id = None;
            }
            if let Some((_, Some(image))) = &self.asset_preview_image {
                let fit = ((width - 24.0).max(1.0) / f64::from(image.width.max(1)))
                    .min((height - 24.0).max(1.0) / f64::from(image.height.max(1)));
                let draw_width = f64::from(image.width) * fit;
                let draw_height = f64::from(image.height) * fit;
                let left = x + (width - draw_width) * 0.5;
                let top = y + (height - draw_height) * 0.5;
                scene.push_clip_layer(Fill::NonZero, Affine::IDENTITY, &bounds_rect);
                scene.draw_image(image, Affine::translate((left, top)) * Affine::scale(fit));
                scene.pop_layer();
                scene.stroke(
                    &Stroke::new(1.0),
                    Affine::IDENTITY,
                    Color::from_rgb8(56, 62, 69),
                    None,
                    &Rect::new(left, top, left + draw_width, top + draw_height),
                );
            }
            return;
        }
        if canvas_id != "kagari.viewport" {
            return;
        }
        let composition = self.project_model.active_composition();
        let preview_limit = if self.adaptive_preview && self.is_playing {
            320.0
        } else {
            640.0
        };
        let source_width = f64::from(composition.width.max(1));
        let source_height = f64::from(composition.height.max(1));
        let pixel_aspect = f64::from(composition.pixel_aspect).clamp(0.1, 10.0);
        let proxy_scale = f64::from(kagari_vfx::core::proxy::composition_preview_scale(
            composition,
        ));
        let render_scale = (preview_limit / (source_width * pixel_aspect))
            .min(preview_limit / source_height)
            .min(1.0)
            * proxy_scale;
        let render_width = source_width * render_scale;
        let render_height = source_height * render_scale;
        let display_width = render_width * pixel_aspect;
        let zoom_factor = self
            .zoom
            .trim()
            .trim_end_matches('%')
            .parse::<f64>()
            .unwrap_or(100.0)
            .clamp(10.0, 800.0)
            / 100.0;
        let fit = (width - 32.0)
            .max(1.0)
            .min((height - 32.0).max(1.0) * display_width / render_height)
            / display_width
            * zoom_factor;
        let draw_width = display_width * fit;
        let draw_height = render_height * fit;
        self.viewport_mapping = Some((
            (width - draw_width) * 0.5 + self.viewport_pan[0],
            (height - draw_height) * 0.5 + self.viewport_pan[1],
            render_scale * fit * pixel_aspect,
            render_scale * fit,
        ));
        self.draw_viewport(scene, x, y, width, height);
    }
}
