use super::*;

impl AfterEffectsStudio {
    pub fn draw_viewport(&mut self, scene: &mut Scene, x: f64, y: f64, width: f64, height: f64) {
        let viewport = Rect::new(x, y, x + width, y + height);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(10, 11, 13),
            None,
            &viewport,
        );
        let composition = self.project_model.active_composition();
        let source_width = composition.width.max(1);
        let source_height = composition.height.max(1);
        let pixel_aspect = f64::from(composition.pixel_aspect).clamp(0.1, 10.0);
        let proxy_scale = f64::from(kagari_vfx::core::proxy::composition_preview_scale(composition));
        let preview_limit = if self.adaptive_preview && self.is_playing {
            320.0
        } else {
            640.0
        };
        let render_scale = (preview_limit / (f64::from(source_width) * pixel_aspect))
            .min(preview_limit / f64::from(source_height))
            .min(1.0)
            * proxy_scale;
        let preview_width = (f64::from(source_width) * render_scale).round().max(1.0) as u32;
        let preview_height = (f64::from(source_height) * render_scale).round().max(1.0) as u32;
        let frame_index = self
            .current_frame
            .min(composition.duration_frames.saturating_sub(1));
        let preview_key = ViewportPreviewKey {
            revision: self.preview_revision,
            frame: frame_index,
            width: preview_width,
            height: preview_height,
        };
        loop {
            match self.viewport_preview_result_rx.try_recv() {
                Ok(result) => {
                    self.viewport_preview_inflight = false;
                    if result.key == preview_key {
                        if let Some(pixels) = result.pixels.filter(|pixels| {
                            pixels.len()
                                == (preview_width as usize)
                                    .saturating_mul(preview_height as usize)
                                    .saturating_mul(4)
                        }) {
                            let image = vello::peniko::ImageData {
                                data: vello::peniko::Blob::new(pixels),
                                format: vello::peniko::ImageFormat::Rgba8,
                                alpha_type: vello::peniko::ImageAlphaType::Alpha,
                                width: preview_width,
                                height: preview_height,
                            };
                            let cache_k = (result.key.revision, result.key.width, result.key.height, result.key.frame);
                            if self.preview_cache.len() >= 120 {
                                // Evict oldest or non-matching revision frames
                                if let Some(evict_key) = self.preview_cache.keys()
                                    .find(|k| k.0 != preview_key.revision)
                                    .or_else(|| self.preview_cache.keys().next())
                                    .cloned()
                                {
                                    self.preview_cache.remove(&evict_key);
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
                    self.viewport_preview_failed = Some(preview_key);
                    break;
                }
            }
        }
        let current_k = (preview_key.revision, preview_key.width, preview_key.height, preview_key.frame);
        let current_preview = self.preview_cache.get(&current_k).cloned();
        let preview = current_preview.clone().or_else(|| {
            // Fallback to closest cached frame of the same revision and dimensions
            self.preview_cache.iter()
                .filter(|(k, _)| k.0 == preview_key.revision && k.1 == preview_key.width && k.2 == preview_key.height)
                .min_by_key(|(k, _)| (k.3 as i64 - preview_key.frame as i64).abs())
                .map(|(_, img)| img.clone())
        });
        if current_preview.is_none() && self.viewport_preview_failed != Some(preview_key) {
            if self.viewport_preview_inflight {
                // Let the active render finish; the next frame will request the latest playhead.
            } else {
                let job = ViewportPreviewJob {
                    key: preview_key,
                    composition: Box::new(composition.clone()),
                };
                match self.viewport_preview_request_tx.try_send(job) {
                    Ok(()) => self.viewport_preview_inflight = true,
                    Err(std::sync::mpsc::TrySendError::Full(_)) => {}
                    Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                        self.viewport_preview_failed = Some(preview_key)
                    }
                }
            }
        }
        scene.push_clip_layer(Fill::NonZero, Affine::IDENTITY, &viewport);
        if let Some(image) = preview {
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
                .min((height - 32.0).max(1.0) * f64::from(preview_width) * pixel_aspect / f64::from(preview_height))
                / (f64::from(preview_width) * pixel_aspect)
                * zoom_factor;
            let draw_width = f64::from(preview_width) * pixel_aspect * fit;
            let draw_height = f64::from(preview_height) * fit;
            let left = x + (width - draw_width) * 0.5 + self.viewport_pan[0];
            let top = y + (height - draw_height) * 0.5 + self.viewport_pan[1];
            scene.draw_image(
                &image,
                Affine::translate((left, top)) * Affine::scale_non_uniform(fit * pixel_aspect, fit),
            );
            let frame = Rect::new(left, top, left + draw_width, top + draw_height);
            scene.stroke(
                &Stroke::new(1.0),
                Affine::IDENTITY,
                Color::from_rgb8(65, 70, 76),
                None,
                &frame,
            );
        } else {
            let frame = Rect::new(x + 16.0, y + 16.0, x + width - 16.0, y + height - 16.0);
            scene.stroke(
                &Stroke::new(1.0),
                Affine::IDENTITY,
                Color::from_rgb8(65, 70, 76),
                None,
                &frame,
            );
        }
        // AE-style layer bounding box and anchor point gizmo for the selected layer
        if let (Some((map_left, map_top, scale_x, scale_y)), Some(layer_id)) = (
            self.viewport_mapping,
            self.selected_layer_id.as_deref(),
        ) {
            if let Some(layer) = composition.layers.iter().find(|l| l.id == layer_id) {
                let pos = layer.transform.position.evaluate(frame_index);
                let anchor = layer.transform.anchor_point.evaluate(frame_index);
                let rotation = layer.transform.rotation.evaluate(frame_index) as f64;
                let scale = layer.transform.scale.evaluate(frame_index);

                let layer_w = match &layer.layer_type {
                    kagari_vfx::core::timeline::LayerType::Solid { .. } => f64::from(composition.width),
                    kagari_vfx::core::timeline::LayerType::Text { text, font_size, .. } => {
                        (text.len() as f64 * (*font_size as f64 * 0.6)).max(40.0)
                    }
                    _ => 200.0,
                };
                let layer_h = match &layer.layer_type {
                    kagari_vfx::core::timeline::LayerType::Solid { .. } => f64::from(composition.height),
                    kagari_vfx::core::timeline::LayerType::Text { font_size, .. } => f64::from(*font_size).max(20.0),
                    _ => 200.0,
                };

                // Screen position of the anchor point
                let anchor_screen_x = x + map_left + f64::from(pos[0]) * scale_x;
                let anchor_screen_y = y + map_top + f64::from(pos[1]) * scale_y;

                let rad = rotation.to_radians();
                let cos_r = rad.cos();
                let sin_r = rad.sin();
                let sx = f64::from(scale[0]) / 100.0;
                let sy = f64::from(scale[1]) / 100.0;

                // 4 corners relative to anchor point
                let corners = [
                    (-f64::from(anchor[0]), -f64::from(anchor[1])),
                    (layer_w - f64::from(anchor[0]), -f64::from(anchor[1])),
                    (layer_w - f64::from(anchor[0]), layer_h - f64::from(anchor[1])),
                    (-f64::from(anchor[0]), layer_h - f64::from(anchor[1])),
                ];

                let screen_corners: Vec<(f64, f64)> = corners.iter().map(|&(lx, ly)| {
                    let scaled_x = lx * sx;
                    let scaled_y = ly * sy;
                    let rot_x = scaled_x * cos_r - scaled_y * sin_r;
                    let rot_y = scaled_x * sin_r + scaled_y * cos_r;
                    (anchor_screen_x + rot_x * scale_x, anchor_screen_y + rot_y * scale_y)
                }).collect();

                // Draw bounding box outline (AE primary accent color)
                let mut box_path = BezPath::new();
                for (i, pt) in screen_corners.iter().enumerate() {
                    if i == 0 {
                        box_path.move_to(*pt);
                    } else {
                        box_path.line_to(*pt);
                    }
                }
                box_path.close_path();
                scene.stroke(
                    &Stroke::new(1.2),
                    Affine::IDENTITY,
                    Color::from_rgb8(80, 160, 255),
                    None,
                    &box_path,
                );

                // Draw 4 corner handles
                for pt in &screen_corners {
                    let handle = Rect::new(pt.0 - 3.5, pt.1 - 3.5, pt.0 + 3.5, pt.1 + 3.5);
                    scene.fill(Fill::NonZero, Affine::IDENTITY, Color::WHITE, None, &handle);
                    scene.stroke(&Stroke::new(1.0), Affine::IDENTITY, Color::from_rgb8(30, 90, 200), None, &handle);
                }

                // Draw AE Anchor Point crosshair circle
                let anchor_circle = vello::kurbo::Circle::new((anchor_screen_x, anchor_screen_y), 5.0);
                scene.stroke(&Stroke::new(1.2), Affine::IDENTITY, Color::from_rgb8(255, 70, 70), None, &anchor_circle);
                let cross_h = Line::new((anchor_screen_x - 7.0, anchor_screen_y), (anchor_screen_x + 7.0, anchor_screen_y));
                let cross_v = Line::new((anchor_screen_x, anchor_screen_y - 7.0), (anchor_screen_x, anchor_screen_y + 7.0));
                scene.stroke(&Stroke::new(1.2), Affine::IDENTITY, Color::from_rgb8(255, 70, 70), None, &cross_h);
                scene.stroke(&Stroke::new(1.2), Affine::IDENTITY, Color::from_rgb8(255, 70, 70), None, &cross_v);
            }
        }
        if let (Some((map_left, map_top, scale_x, scale_y)), Some(layer_id), Some(mask_id)) = (
            self.viewport_mapping,
            self.selected_layer_id.as_deref(),
            self.selected_mask_id.as_deref(),
        ) {
            if let Some(mask) = composition
                .layers
                .iter()
                .find(|layer| layer.id == layer_id)
                .and_then(|layer| layer.masks.iter().find(|mask| mask.id == mask_id))
            {
                let mut polygon = Vec::new();
                mask.path.to_polygon_into(frame_index, 8, &mut polygon);
                if polygon.len() >= 2 {
                    let mut outline = BezPath::new();
                    for (index, point) in polygon.iter().enumerate() {
                        let screen = (
                            x + map_left + f64::from(point[0]) * scale_x,
                            y + map_top + f64::from(point[1]) * scale_y,
                        );
                        if index == 0 {
                            outline.move_to(screen);
                        } else {
                            outline.line_to(screen);
                        }
                    }
                    scene.stroke(
                        &Stroke::new(1.6),
                        Affine::IDENTITY,
                        if mask.enabled {
                            Color::from_rgb8(255, 139, 62)
                        } else {
                            Color::from_rgb8(114, 121, 128)
                        },
                        None,
                        &outline,
                    );
                }
                for point in mask.path.vertices_at_frame(frame_index) {
                    let px = x + map_left + f64::from(point[0]) * scale_x;
                    let py = y + map_top + f64::from(point[1]) * scale_y;
                    let handle = RoundedRect::new(px - 4.0, py - 4.0, px + 4.0, py + 4.0, 2.0);
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        Color::from_rgb8(255, 221, 194),
                        None,
                        &handle,
                    );
                    scene.stroke(
                        &Stroke::new(1.0),
                        Affine::IDENTITY,
                        Color::from_rgb8(34, 27, 23),
                        None,
                        &handle,
                    );
                }
            }
        }

        // Live Pen Drawing Guide (interactive rubber-band stroke while dragging pen)
        if let (Some((map_left, map_top, scale_x, scale_y)), Some(start), Some(current)) = (
            self.viewport_mapping,
            self.viewport_pen_start,
            self.viewport_pen_current,
        ) {
            let s_x = x + map_left + start[0] * scale_x;
            let s_y = y + map_top + start[1] * scale_y;
            let c_x = x + map_left + current[0] * scale_x;
            let c_y = y + map_top + current[1] * scale_y;

            let mut pen_path = BezPath::new();
            pen_path.move_to((s_x, s_y));
            let mid_x = (s_x + c_x) * 0.5;
            let mid_y = (s_y + c_y) * 0.5;
            pen_path.quad_to((mid_x, mid_y), (c_x, c_y));

            // Outer glow stroke
            scene.stroke(
                &Stroke::new(3.5),
                Affine::IDENTITY,
                Color::from_rgba8(255, 120, 40, 80),
                None,
                &pen_path,
            );
            // Core stroke
            scene.stroke(
                &Stroke::new(1.8),
                Affine::IDENTITY,
                Color::from_rgb8(255, 140, 50),
                None,
                &pen_path,
            );

            // Handle points at start and cursor
            let start_circle = vello::kurbo::Circle::new((s_x, s_y), 4.0);
            scene.fill(Fill::NonZero, Affine::IDENTITY, Color::from_rgb8(255, 200, 100), None, &start_circle);
            scene.stroke(&Stroke::new(1.0), Affine::IDENTITY, Color::from_rgb8(30, 20, 15), None, &start_circle);

            let cur_circle = vello::kurbo::Circle::new((c_x, c_y), 4.0);
            scene.fill(Fill::NonZero, Affine::IDENTITY, Color::from_rgb8(255, 230, 150), None, &cur_circle);
            scene.stroke(&Stroke::new(1.0), Affine::IDENTITY, Color::from_rgb8(30, 20, 15), None, &cur_circle);
        }

        // 3D Light Gizmos (Point light radiating sun rays, Spot light cone)
        if let Some((map_left, map_top, scale_x, scale_y)) = self.viewport_mapping {
            let active_camera = composition.resolve_camera();
            for light in &composition.lights {
                let light_pos = light.position.evaluate(frame_index);
                // Project 3D light position to 2D composition coordinates
                let (lx, ly) = if let Some(norm_proj) = kagari_vfx::core::timeline::project_point_to_screen_at_frame(
                    active_camera,
                    light_pos,
                    source_width as f32,
                    source_height as f32,
                    frame_index,
                ) {
                    (
                        f64::from(norm_proj[0]) * f64::from(source_width),
                        f64::from(norm_proj[1]) * f64::from(source_height),
                    )
                } else {
                    (f64::from(light_pos[0]), f64::from(light_pos[1]))
                };

                let screen_x = x + map_left + lx * scale_x;
                let screen_y = y + map_top + ly * scale_y;

                if screen_x >= x - 20.0 && screen_x <= x + width + 20.0 && screen_y >= y - 20.0 && screen_y <= y + height + 20.0 {
                    let [lr, lg, lb, _] = light.color;
                    let light_color = Color::from_rgb8(
                        (lr.clamp(0.0, 1.0) * 255.0) as u8,
                        (lg.clamp(0.0, 1.0) * 255.0) as u8,
                        (lb.clamp(0.0, 1.0) * 255.0) as u8,
                    );

                    // Central light bulb circle
                    let bulb = Circle::new((screen_x, screen_y), 5.5);
                    scene.fill(Fill::NonZero, Affine::IDENTITY, light_color, None, &bulb);
                    scene.stroke(&Stroke::new(1.5), Affine::IDENTITY, Color::from_rgb8(15, 17, 20), None, &bulb);

                    match &light.light_type {
                        kagari_vfx::core::timeline::LightType::Point | kagari_vfx::core::timeline::LightType::Ambient => {
                            // 8 radiating sun rays for Point / Ambient Light
                            let ray_len = 11.0;
                            for angle_deg in [0.0, 45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0] {
                                let rad = angle_deg * std::f64::consts::PI / 180.0;
                                let r_start = (screen_x + rad.cos() * 7.5, screen_y + rad.sin() * 7.5);
                                let r_end = (screen_x + rad.cos() * (7.5 + ray_len), screen_y + rad.sin() * (7.5 + ray_len));
                                scene.stroke(
                                    &Stroke::new(1.2),
                                    Affine::IDENTITY,
                                    Color::from_rgba8((lr * 255.0) as u8, (lg * 255.0) as u8, (lb * 255.0) as u8, 180),
                                    None,
                                    &Line::new(r_start, r_end),
                                );
                            }
                        }
                        kagari_vfx::core::timeline::LightType::Spot { cone_angle_deg, .. } => {
                            // Spot Light Cone Wireframe
                            let half_cone_rad = f64::from(cone_angle_deg.clamp(5.0, 170.0) * 0.5) * std::f64::consts::PI / 180.0;
                            let cone_len = 50.0;
                            // Target direction down +Z (or downward on screen by default)
                            let dir_rad = std::f64::consts::FRAC_PI_2;
                            let left_rad = dir_rad - half_cone_rad;
                            let right_rad = dir_rad + half_cone_rad;

                            let end_left = (screen_x + left_rad.cos() * cone_len, screen_y + left_rad.sin() * cone_len);
                            let end_right = (screen_x + right_rad.cos() * cone_len, screen_y + right_rad.sin() * cone_len);

                            let mut cone = BezPath::new();
                            cone.move_to((screen_x, screen_y));
                            cone.line_to(end_left);
                            cone.line_to(end_right);
                            cone.close_path();

                            scene.stroke(
                                &Stroke::new(1.2),
                                Affine::IDENTITY,
                                Color::from_rgba8((lr * 255.0) as u8, (lg * 255.0) as u8, (lb * 255.0) as u8, 160),
                                None,
                                &cone,
                            );
                        }
                        kagari_vfx::core::timeline::LightType::Parallel => {
                            // 3 parallel directional arrows
                            let arrow_len = 24.0;
                            for offset in [-10.0, 0.0, 10.0] {
                                let start_pt = (screen_x + offset, screen_y);
                                let end_pt = (screen_x + offset, screen_y + arrow_len);
                                scene.stroke(
                                    &Stroke::new(1.2),
                                    Affine::IDENTITY,
                                    Color::from_rgba8((lr * 255.0) as u8, (lg * 255.0) as u8, (lb * 255.0) as u8, 180),
                                    None,
                                    &Line::new(start_pt, end_pt),
                                );
                            }
                        }
                    }
                }
            }

            // 3D Camera Frustum & Target Wireframe Overlay
            for camera in &composition.cameras {
                let _cam_pos = camera.transform.position.evaluate(frame_index);
                let fov_deg = camera.fov_at(frame_index);
                let focus_dist = camera.focus_distance_at(frame_index);

                // Viewport center / camera target point on screen
                let center_x = x + map_left + (f64::from(source_width) * 0.5) * scale_x;
                let center_y = y + map_top + (f64::from(source_height) * 0.5) * scale_y;

                // Outer focal distance frame corners (representing focal plane in 3D camera)
                let fov_rad = f64::from(fov_deg) * std::f64::consts::PI / 180.0;
                let frustum_half_w = (fov_rad * 0.5).tan() * (f64::from(focus_dist) * 0.1).clamp(20.0, 180.0);
                let frustum_half_h = frustum_half_w * (f64::from(source_height) / f64::from(source_width));

                let focal_rect = RoundedRect::new(
                    center_x - frustum_half_w * scale_x,
                    center_y - frustum_half_h * scale_y,
                    center_x + frustum_half_w * scale_x,
                    center_y + frustum_half_h * scale_y,
                    3.0,
                );

                // Subtle Cyan/Teal wireframe of Camera Focal Plane
                scene.stroke(
                    &Stroke::new(1.0),
                    Affine::IDENTITY,
                    Color::from_rgba8(0, 210, 230, 90),
                    None,
                    &focal_rect,
                );

                // Corner brackets on focal rect
                let bracket_len = 10.0;
                let corners = [
                    (center_x - frustum_half_w * scale_x, center_y - frustum_half_h * scale_y, 1.0, 1.0),
                    (center_x + frustum_half_w * scale_x, center_y - frustum_half_h * scale_y, -1.0, 1.0),
                    (center_x + frustum_half_w * scale_x, center_y + frustum_half_h * scale_y, -1.0, -1.0),
                    (center_x - frustum_half_w * scale_x, center_y + frustum_half_h * scale_y, 1.0, -1.0),
                ];
                for (cx, cy, dx, dy) in corners {
                    scene.stroke(
                        &Stroke::new(1.8),
                        Affine::IDENTITY,
                        Color::from_rgb8(0, 230, 255),
                        None,
                        &Line::new((cx, cy), (cx + dx * bracket_len, cy)),
                    );
                    scene.stroke(
                        &Stroke::new(1.8),
                        Affine::IDENTITY,
                        Color::from_rgb8(0, 230, 255),
                        None,
                        &Line::new((cx, cy), (cx, cy + dy * bracket_len)),
                    );
                }

                // Camera icon marker in bottom-left corner of viewport
                let icon_x = x + 16.0;
                let icon_y = y + height - 24.0;
                let cam_body = RoundedRect::new(icon_x, icon_y, icon_x + 18.0, icon_y + 12.0, 2.0);
                scene.fill(Fill::NonZero, Affine::IDENTITY, Color::from_rgba8(0, 180, 210, 180), None, &cam_body);
                scene.stroke(&Stroke::new(1.0), Affine::IDENTITY, Color::from_rgb8(150, 240, 255), None, &cam_body);

                let mut lens = BezPath::new();
                lens.move_to((icon_x + 18.0, icon_y + 3.0));
                lens.line_to((icon_x + 24.0, icon_y));
                lens.line_to((icon_x + 24.0, icon_y + 12.0));
                lens.line_to((icon_x + 18.0, icon_y + 9.0));
                lens.close_path();
                scene.fill(Fill::NonZero, Affine::IDENTITY, Color::from_rgb8(0, 210, 240), None, &lens);

                // Small lens center dot
                let cam_dot = Circle::new((icon_x + 7.0, icon_y + 6.0), 2.2);
                scene.fill(Fill::NonZero, Affine::IDENTITY, Color::WHITE, None, &cam_dot);

                // Camera Navigation Mode active HUD badge
                if let Some(nav) = &self.camera_nav_drag {
                    let nav_badge_x = icon_x + 32.0;
                    let nav_badge_y = icon_y - 2.0;
                    let nav_bg = RoundedRect::new(nav_badge_x, nav_badge_y, nav_badge_x + 76.0, nav_badge_y + 16.0, 3.0);
                    let (badge_color, border_color) = match nav.mode {
                        CameraNavMode::Orbit => (Color::from_rgba8(0, 180, 255, 140), Color::from_rgb8(0, 220, 255)),
                        CameraNavMode::Pan => (Color::from_rgba8(50, 220, 120, 140), Color::from_rgb8(100, 255, 160)),
                        CameraNavMode::Dolly => (Color::from_rgba8(255, 160, 40, 140), Color::from_rgb8(255, 200, 80)),
                    };
                    scene.fill(Fill::NonZero, Affine::IDENTITY, badge_color, None, &nav_bg);
                    scene.stroke(&Stroke::new(1.0), Affine::IDENTITY, border_color, None, &nav_bg);
                }
            }
        }

        scene.pop_layer();
    }
}

