use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ColorScopeMode {
    Parade,
    Waveform,
    Vectorscope,
    Histogram,
}

impl ColorScopeMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Parade => "parade",
            Self::Waveform => "waveform",
            Self::Vectorscope => "vectorscope",
            Self::Histogram => "histogram",
        }
    }

    pub(crate) fn next(self) -> Self {
        match self {
            Self::Parade => Self::Waveform,
            Self::Waveform => Self::Vectorscope,
            Self::Vectorscope => Self::Histogram,
            Self::Histogram => Self::Parade,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ColorWheelDrag {
    pub(crate) wheel_index: usize,
    pub(crate) start_x: f64,
    pub(crate) start_y: f64,
    pub(crate) initial_value: f32,
    pub(crate) initial_hue_shift: f32,
    pub(crate) changed: bool,
}

impl AfterEffectsStudio {
    fn latest_preview_image(&self) -> Option<&vello::peniko::ImageData> {
        let rev = self.preview_revision;
        let frame = self.current_frame;
        self.preview_cache
            .iter()
            .filter(|(k, _)| k.0 == rev)
            .min_by_key(|(k, _)| (k.3 as i64 - frame as i64).abs())
            .or_else(|| self.preview_cache.iter().max_by_key(|(k, _)| k.0))
            .map(|(_, img)| img)
    }

    pub(crate) fn current_color_wheel_values(&self) -> (f32, f32, f32, f32) {
        let comp = self.project_model.active_composition();
        let Some(layer_id) = self.selected_layer_id.as_deref() else {
            return (0.0, 1.0, 1.0, 0.0);
        };
        let Some(layer) = comp.layers.iter().find(|layer| layer.id == layer_id) else {
            return (0.0, 1.0, 1.0, 0.0);
        };
        let frame = self.current_frame;
        for effect in layer.effects.iter().rev() {
            if let kagari_vfx::core::timeline::EffectType::FilmEmulation {
                lift,
                gamma,
                gain,
                hue_shift_deg,
                ..
            } = &effect.effect_type
            {
                return (
                    lift.evaluate(frame),
                    gamma.evaluate(frame),
                    gain.evaluate(frame),
                    hue_shift_deg.evaluate(frame),
                );
            }
        }
        (0.0, 1.0, 1.0, 0.0)
    }

    pub(crate) fn apply_color_wheel_values(
        &mut self,
        lift_val: f32,
        gamma_val: f32,
        gain_val: f32,
        hue_val: f32,
    ) -> bool {
        let Some(layer_id) = self.selected_layer_id.clone() else {
            return false;
        };
        let frame = self.current_frame;
        let comp = self.project_model.active_composition_mut();
        let Some(layer) = comp
            .layers
            .iter_mut()
            .find(|layer| layer.id == layer_id && !layer.locked)
        else {
            return false;
        };
        let existing_idx = layer.effects.iter().rposition(|effect| {
            matches!(
                effect.effect_type,
                kagari_vfx::core::timeline::EffectType::FilmEmulation { .. }
            )
        });
        let idx = if let Some(idx) = existing_idx {
            idx
        } else {
            let effect = kagari_vfx::core::timeline::Effect {
                id: format!("color-grade-{}", layer.effects.len() + 1),
                name: "カラーグレーディング".to_owned(),
                enabled: true,
                effect_type: kagari_vfx::core::timeline::EffectType::FilmEmulation {
                    lift: kagari_vfx::core::timeline::Animatable::new_constant(0.0),
                    gamma: kagari_vfx::core::timeline::Animatable::new_constant(1.0),
                    gain: kagari_vfx::core::timeline::Animatable::new_constant(1.0),
                    hue_shift_deg: kagari_vfx::core::timeline::Animatable::new_constant(0.0),
                },
            };
            layer.effects.push(effect);
            layer.effects.len() - 1
        };
        if let kagari_vfx::core::timeline::EffectType::FilmEmulation {
            lift,
            gamma,
            gain,
            hue_shift_deg,
            ..
        } = &mut layer.effects[idx].effect_type
        {
            set_animatable_at_frame(lift, frame, lift_val.clamp(-0.5, 0.5));
            set_animatable_at_frame(gamma, frame, gamma_val.clamp(0.2, 2.2));
            set_animatable_at_frame(gain, frame, gain_val.clamp(0.2, 2.2));
            set_animatable_at_frame(hue_shift_deg, frame, hue_val.clamp(-45.0, 45.0));
            return true;
        }
        false
    }

    pub(crate) fn reset_color_wheels(&mut self) {
        if self.apply_color_wheel_values(0.0, 1.0, 1.0, 0.0) {
            self.mark_project_changed();
        }
    }

    pub(crate) fn sync_color_grading_bindings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (lift, gamma, gain, _) = self.current_color_wheel_values();
        self.ui.set_binding(
            "color_scope_mode",
            Value::String(self.color_scope_mode.as_str().to_owned()),
        )?;
        self.ui
            .set_binding("color_lift_label", Value::String(format!("{lift:+.2}")))?;
        self.ui
            .set_binding("color_gamma_label", Value::String(format!("{gamma:.2}")))?;
        self.ui
            .set_binding("color_gain_label", Value::String(format!("{gain:.2}")))?;
        Ok(())
    }

    pub(crate) fn draw_scopes_canvas(
        &self,
        scene: &mut Scene,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) {
        self.draw_scopes_canvas_for_mode(self.color_scope_mode, scene, x, y, width, height);
    }

    pub(crate) fn draw_vectorscope_canvas(
        &self,
        scene: &mut Scene,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) {
        self.draw_scopes_canvas_for_mode(ColorScopeMode::Vectorscope, scene, x, y, width, height);
    }

    fn draw_scopes_canvas_for_mode(
        &self,
        mode: ColorScopeMode,
        scene: &mut Scene,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) {
        let bg = vello::kurbo::RoundedRect::new(x, y, x + width, y + height, 6.0);
        scene.fill(
            vello::peniko::Fill::NonZero,
            vello::kurbo::Affine::IDENTITY,
            vello::peniko::Color::from_rgb8(11, 12, 14),
            None,
            &bg,
        );
        scene.stroke(
            &vello::kurbo::Stroke::new(1.0),
            vello::kurbo::Affine::IDENTITY,
            vello::peniko::Color::from_rgb8(34, 38, 46),
            None,
            &bg,
        );

        let pad = 6.0;
        let inner_x = x + pad;
        let inner_y = y + pad;
        let inner_w = (width - pad * 2.0).max(1.0);
        let inner_h = (height - pad * 2.0).max(1.0);

        for i in 0..=4 {
            let gy = inner_y + inner_h * (i as f64 / 4.0);
            let line = vello::kurbo::Line::new((inner_x, gy), (inner_x + inner_w, gy));
            scene.stroke(
                &vello::kurbo::Stroke::new(1.0),
                vello::kurbo::Affine::IDENTITY,
                vello::peniko::Color::from_rgba8(38, 43, 54, 180),
                None,
                &line,
            );
        }

        let Some(image) = self.latest_preview_image() else {
            return;
        };
        let img_w = image.width as usize;
        let img_h = image.height as usize;
        let pixels: &[u8] = image.data.as_ref();
        if img_w == 0 || img_h == 0 || pixels.len() < img_w * img_h * 4 {
            return;
        }

        match mode {
            ColorScopeMode::Parade => {
                let lane_w = (inner_w - 8.0).max(3.0) / 3.0;
                let cols = (lane_w as usize).clamp(16, 64);
                let step_y = (img_h / 24).max(1);
                let colors = [
                    vello::peniko::Color::from_rgba8(245, 86, 86, 95),
                    vello::peniko::Color::from_rgba8(82, 224, 128, 95),
                    vello::peniko::Color::from_rgba8(88, 166, 255, 110),
                ];
                for (ch, color) in colors.into_iter().enumerate() {
                    let lane_x = inner_x + ch as f64 * (lane_w + 4.0);
                    for col in 0..cols {
                        let px_x = (col * img_w / cols).min(img_w - 1);
                        let dx = lane_x + (col as f64 / cols as f64) * lane_w;
                        for px_y in (0..img_h).step_by(step_y) {
                            let idx = (px_y * img_w + px_x) * 4 + ch;
                            let val = f64::from(pixels[idx]) / 255.0;
                            let dy = inner_y + (1.0 - val) * (inner_h - 2.0);
                            let dot = vello::kurbo::Rect::new(dx, dy, dx + 2.0, dy + 2.0);
                            scene.fill(
                                vello::peniko::Fill::NonZero,
                                vello::kurbo::Affine::IDENTITY,
                                color,
                                None,
                                &dot,
                            );
                        }
                    }
                }
            }
            ColorScopeMode::Waveform => {
                let cols = (inner_w as usize).clamp(24, 96);
                let step_y = (img_h / 28).max(1);
                let color = vello::peniko::Color::from_rgba8(168, 224, 255, 100);
                for col in 0..cols {
                    let px_x = (col * img_w / cols).min(img_w - 1);
                    let dx = inner_x + (col as f64 / cols as f64) * inner_w;
                    for px_y in (0..img_h).step_by(step_y) {
                        let base = (px_y * img_w + px_x) * 4;
                        let r = f64::from(pixels[base]) / 255.0;
                        let g = f64::from(pixels[base + 1]) / 255.0;
                        let b = f64::from(pixels[base + 2]) / 255.0;
                        let luma = (0.2126 * r + 0.7152 * g + 0.0722 * b).clamp(0.0, 1.0);
                        let dy = inner_y + (1.0 - luma) * (inner_h - 2.0);
                        let dot = vello::kurbo::Rect::new(dx, dy, dx + 2.0, dy + 2.0);
                        scene.fill(
                            vello::peniko::Fill::NonZero,
                            vello::kurbo::Affine::IDENTITY,
                            color,
                            None,
                            &dot,
                        );
                    }
                }
            }
            ColorScopeMode::Histogram => {
                let mut bins = [[0u32; 48]; 3];
                let step_x = (img_w / 64).max(1);
                let step_y = (img_h / 48).max(1);
                for py in (0..img_h).step_by(step_y) {
                    for px in (0..img_w).step_by(step_x) {
                        let base = (py * img_w + px) * 4;
                        for ch in 0..3 {
                            let bin = (usize::from(pixels[base + ch]) * 47) / 255;
                            bins[ch][bin] += 1;
                        }
                    }
                }
                let peak = bins
                    .iter()
                    .flat_map(|row| row.iter().copied())
                    .max()
                    .unwrap_or(1)
                    .max(1) as f64;
                let bar_w = inner_w / 48.0;
                let colors = [
                    vello::peniko::Color::from_rgba8(245, 86, 86, 120),
                    vello::peniko::Color::from_rgba8(82, 224, 128, 120),
                    vello::peniko::Color::from_rgba8(88, 166, 255, 130),
                ];
                for (ch, color) in colors.into_iter().enumerate() {
                    for (bin, &count) in bins[ch].iter().enumerate() {
                        let h = ((count as f64) / peak).sqrt() * inner_h;
                        let bx = inner_x + bin as f64 * bar_w;
                        let rect =
                            vello::kurbo::Rect::new(bx, inner_y + inner_h - h, bx + bar_w, inner_y + inner_h);
                        scene.fill(
                            vello::peniko::Fill::NonZero,
                            vello::kurbo::Affine::IDENTITY,
                            color,
                            None,
                            &rect,
                        );
                    }
                }
            }
            ColorScopeMode::Vectorscope => {
                let cx = inner_x + inner_w * 0.5;
                let cy = inner_y + inner_h * 0.5;
                let radius = (inner_w.min(inner_h) * 0.45).max(12.0);
                let reticle_col = vello::peniko::Color::from_rgba8(54, 62, 76, 200);
                for scale in [1.0, 0.5] {
                    let ring = vello::kurbo::Circle::new((cx, cy), radius * scale);
                    scene.stroke(
                        &vello::kurbo::Stroke::new(1.0),
                        vello::kurbo::Affine::IDENTITY,
                        reticle_col,
                        None,
                        &ring,
                    );
                }
                let hline = vello::kurbo::Line::new((cx - radius, cy), (cx + radius, cy));
                let vline = vello::kurbo::Line::new((cx, cy - radius), (cx, cy + radius));
                // Standard I-line (skin-tone reference ~123 deg in Cb/Cr plane)
                let skin_angle = -57.0_f64.to_radians();
                let skin_line = vello::kurbo::Line::new(
                    (cx, cy),
                    (cx + radius * skin_angle.cos(), cy + radius * skin_angle.sin()),
                );
                for line in [hline, vline] {
                    scene.stroke(
                        &vello::kurbo::Stroke::new(1.0),
                        vello::kurbo::Affine::IDENTITY,
                        reticle_col,
                        None,
                        &line,
                    );
                }
                scene.stroke(
                    &vello::kurbo::Stroke::new(1.0),
                    vello::kurbo::Affine::IDENTITY,
                    vello::peniko::Color::from_rgba8(198, 138, 86, 150),
                    None,
                    &skin_line,
                );
                let step_x = (img_w / 48).max(1);
                let step_y = (img_h / 36).max(1);
                for py in (0..img_h).step_by(step_y) {
                    for px in (0..img_w).step_by(step_x) {
                        let base = (py * img_w + px) * 4;
                        let r = f64::from(pixels[base]) / 255.0;
                        let g = f64::from(pixels[base + 1]) / 255.0;
                        let b = f64::from(pixels[base + 2]) / 255.0;
                        // BT.709 chroma coordinates in [-0.5, 0.5]
                        let cb = -0.1146 * r - 0.3854 * g + 0.5 * b;
                        let cr = 0.5 * r - 0.4542 * g - 0.0458 * b;
                        let dx = cx + cb * 2.0 * radius;
                        let dy = cy - cr * 2.0 * radius;
                        let dot = vello::kurbo::Rect::new(dx - 1.0, dy - 1.0, dx + 1.0, dy + 1.0);
                        scene.fill(
                            vello::peniko::Fill::NonZero,
                            vello::kurbo::Affine::IDENTITY,
                            vello::peniko::Color::from_rgba8(
                                pixels[base].max(48),
                                pixels[base + 1].max(48),
                                pixels[base + 2].max(48),
                                130,
                            ),
                            None,
                            &dot,
                        );
                    }
                }
            }
        }
    }

    pub(crate) fn draw_color_wheels_canvas(
        &mut self,
        scene: &mut Scene,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) {
        self.color_wheels_bounds = Some((x, y, width, height));
        let bg = vello::kurbo::RoundedRect::new(x, y, x + width, y + height, 6.0);
        scene.fill(
            vello::peniko::Fill::NonZero,
            vello::kurbo::Affine::IDENTITY,
            vello::peniko::Color::from_rgba8(18, 20, 26, 255),
            None,
            &bg,
        );

        let (lift, gamma, gain, hue_shift) = self.current_color_wheel_values();
        let slot_w = width / 3.0;
        let wheel_r = ((slot_w * 0.36).min((height - 26.0) * 0.42)).max(16.0);
        let norms = [
            ((lift + 0.5) / 1.0).clamp(0.0, 1.0),
            ((gamma - 0.2) / 2.0).clamp(0.0, 1.0),
            ((gain - 0.2) / 2.0).clamp(0.0, 1.0),
        ];
        let hue_norm = (hue_shift / 45.0).clamp(-1.0, 1.0);
        let ring_colors = [
            vello::peniko::Color::from_rgba8(245, 92, 92, 220),
            vello::peniko::Color::from_rgba8(245, 168, 72, 220),
            vello::peniko::Color::from_rgba8(236, 224, 82, 220),
            vello::peniko::Color::from_rgba8(96, 224, 128, 220),
            vello::peniko::Color::from_rgba8(78, 206, 232, 220),
            vello::peniko::Color::from_rgba8(98, 134, 248, 220),
            vello::peniko::Color::from_rgba8(192, 108, 245, 220),
            vello::peniko::Color::from_rgba8(242, 96, 178, 220),
        ];

        for i in 0..3 {
            let cx = x + slot_w * (i as f64 + 0.5);
            let cy = y + wheel_r + 12.0;
            let active = self
                .color_wheel_drag
                .is_some_and(|drag| drag.wheel_index == i);

            let disk = vello::kurbo::Circle::new((cx, cy), wheel_r);
            scene.fill(
                vello::peniko::Fill::NonZero,
                vello::kurbo::Affine::IDENTITY,
                vello::peniko::Color::from_rgba8(26, 29, 38, 255),
                None,
                &disk,
            );
            for (seg, color) in ring_colors.iter().enumerate() {
                let a0 = (seg as f64 / ring_colors.len() as f64) * std::f64::consts::TAU;
                let a1 = ((seg + 1) as f64 / ring_colors.len() as f64) * std::f64::consts::TAU;
                let arc = vello::kurbo::Arc::new(
                    (cx, cy),
                    (wheel_r, wheel_r),
                    a0,
                    a1 - a0,
                    0.0,
                );
                scene.stroke(
                    &vello::kurbo::Stroke::new(3.0),
                    vello::kurbo::Affine::IDENTITY,
                    *color,
                    None,
                    &arc,
                );
            }
            let hline = vello::kurbo::Line::new((cx - wheel_r + 4.0, cy), (cx + wheel_r - 4.0, cy));
            let vline = vello::kurbo::Line::new((cx, cy - wheel_r + 4.0), (cx, cy + wheel_r - 4.0));
            let cross_col = vello::peniko::Color::from_rgba8(72, 78, 92, 160);
            scene.stroke(
                &vello::kurbo::Stroke::new(1.0),
                vello::kurbo::Affine::IDENTITY,
                cross_col,
                None,
                &hline,
            );
            scene.stroke(
                &vello::kurbo::Stroke::new(1.0),
                vello::kurbo::Affine::IDENTITY,
                cross_col,
                None,
                &vline,
            );

            // Handle dot: horizontal = hue shift, vertical = lift/gamma/gain offset
            let dy_offset = -(f64::from(norms[i]) - 0.5) * 2.0 * (wheel_r - 7.0);
            let dx_offset = f64::from(hue_norm) * (wheel_r - 7.0) * 0.65;
            let handle = vello::kurbo::Circle::new((cx + dx_offset, cy + dy_offset), 4.5);
            scene.fill(
                vello::peniko::Fill::NonZero,
                vello::kurbo::Affine::IDENTITY,
                if active {
                    vello::peniko::Color::from_rgba8(255, 164, 64, 255)
                } else {
                    vello::peniko::Color::from_rgba8(242, 245, 250, 255)
                },
                None,
                &handle,
            );

            // Bottom master bar per wheel
            let bar_w = slot_w * 0.72;
            let bar_x = cx - bar_w * 0.5;
            let bar_y = y + height - 14.0;
            let track = vello::kurbo::RoundedRect::new(bar_x, bar_y, bar_x + bar_w, bar_y + 5.0, 2.5);
            scene.fill(
                vello::peniko::Fill::NonZero,
                vello::kurbo::Affine::IDENTITY,
                vello::peniko::Color::from_rgba8(40, 44, 54, 255),
                None,
                &track,
            );
            let fill_w = (bar_w * f64::from(norms[i])).max(3.0);
            let fill = vello::kurbo::RoundedRect::new(bar_x, bar_y, bar_x + fill_w, bar_y + 5.0, 2.5);
            scene.fill(
                vello::peniko::Fill::NonZero,
                vello::kurbo::Affine::IDENTITY,
                vello::peniko::Color::from_rgba8(243, 156, 41, 235),
                None,
                &fill,
            );
        }
    }

    pub(crate) fn handle_scopes_canvas_pointer(&mut self, event: CanvasPointerEvent) -> bool {
        if matches!(
            event.phase,
            CanvasPointerPhase::Down | CanvasPointerPhase::Up
        ) && event.button == Some(ame_ui::event::PointerButton::Primary)
        {
            if event.phase == CanvasPointerPhase::Up {
                self.color_scope_mode = self.color_scope_mode.next();
                self.sync_bindings_logged();
                self.request_ui_redraw();
            }
            return true;
        }
        false
    }

    pub(crate) fn handle_color_wheels_canvas_pointer(&mut self, event: CanvasPointerEvent) -> bool {
        match event.phase {
            CanvasPointerPhase::Down if event.button == Some(ame_ui::event::PointerButton::Primary) => {
                let width = self.color_wheels_bounds.map_or(300.0, |b| b.2.max(1.0));
                let local_x = if let Some(b) = self.color_wheels_bounds {
                    if event.x >= b.0 && event.x <= b.0 + b.2 {
                        event.x - b.0
                    } else {
                        event.x
                    }
                } else {
                    event.x
                };
                let wheel_index = ((local_x / (width / 3.0)).floor() as usize).min(2);
                let (lift, gamma, gain, hue) = self.current_color_wheel_values();
                let initial_value = match wheel_index {
                    0 => lift,
                    1 => gamma,
                    _ => gain,
                };
                self.color_wheel_drag = Some(ColorWheelDrag {
                    wheel_index,
                    start_x: event.x,
                    start_y: event.y,
                    initial_value,
                    initial_hue_shift: hue,
                    changed: false,
                });
                true
            }
            CanvasPointerPhase::Moved => {
                let Some(mut drag) = self.color_wheel_drag else {
                    return false;
                };
                let (mut lift, mut gamma, mut gain, _) = self.current_color_wheel_values();
                let dy = drag.start_y - event.y;
                let dx = event.x - drag.start_x;
                let next_hue = (drag.initial_hue_shift + dx as f32 * 0.45).clamp(-45.0, 45.0);
                match drag.wheel_index {
                    0 => lift = (drag.initial_value + dy as f32 * 0.006).clamp(-0.5, 0.5),
                    1 => gamma = (drag.initial_value + dy as f32 * 0.012).clamp(0.2, 2.2),
                    _ => gain = (drag.initial_value + dy as f32 * 0.012).clamp(0.2, 2.2),
                }
                if self.apply_color_wheel_values(lift, gamma, gain, next_hue) {
                    drag.changed = true;
                    self.color_wheel_drag = Some(drag);
                    self.mark_project_changed();
                    self.sync_bindings_logged();
                    self.request_ui_redraw();
                }
                true
            }
            CanvasPointerPhase::Up | CanvasPointerPhase::Cancelled => {
                if let Some(drag) = self.color_wheel_drag.take() {
                    if drag.changed {
                        self.mark_project_changed();
                        self.sync_bindings_logged();
                    }
                    self.request_ui_redraw();
                    return true;
                }
                false
            }
            _ => false,
        }
    }
}
