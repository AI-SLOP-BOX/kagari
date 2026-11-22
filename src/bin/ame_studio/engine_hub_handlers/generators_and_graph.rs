use super::*;

impl AfterEffectsStudio {
    pub(super) fn add_ae_generator_layer(&mut self, gen_idx: u8, label: &str) {
        use kagari_vfx::core::timeline::{Animatable, BlendMode, Effect, EffectType, LayerType};
        if (2..=6).contains(&gen_idx) {
            let center = {
                let comp = self.project_model.active_composition();
                [comp.width as f64 * 0.5, comp.height as f64 * 0.5]
            };
            let layer_type = match gen_idx {
                5 => LayerType::Solid {
                    color: [0.0, 0.0, 0.0, 0.0],
                },
                4 => LayerType::Solid {
                    color: [0.06, 0.03, 0.02, 1.0],
                },
                6 => LayerType::Solid {
                    color: [0.01, 0.03, 0.06, 1.0],
                },
                _ => LayerType::Solid {
                    color: [0.12, 0.16, 0.24, 1.0],
                },
            };
            self.add_canvas_layer(label.to_owned(), layer_type, center);
            if let Some(layer_id) = self.selected_layer_id.clone() {
                if let Some(layer) = self
                    .project_model
                    .active_composition_mut()
                    .layers
                    .iter_mut()
                    .find(|l| l.id == layer_id)
                {
                    match gen_idx {
                        2 => {
                            layer.effects.push(Effect {
                                id: format!("proc_grad_{}", self.current_frame),
                                name: "Gradient Map (非破壊)".to_owned(),
                                effect_type: EffectType::GradientMap {
                                    low_color: Animatable::new_constant([0.08, 0.18, 0.48]),
                                    mid_color: Animatable::new_constant([0.32, 0.62, 0.96]),
                                    high_color: Animatable::new_constant([0.98, 0.68, 0.24]),
                                },
                                enabled: true,
                            });
                            layer.effects.push(Effect {
                                id: format!("proc_turb_{}", self.current_frame),
                                name: "Turbulent Flow".to_owned(),
                                effect_type: EffectType::TurbulentDisplace {
                                    amount: Animatable::new_constant(24.0),
                                    size: Animatable::new_constant(90.0),
                                    evolution: Animatable::new_constant(0.0),
                                    complexity: Animatable::new_constant(2.0),
                                },
                                enabled: true,
                            });
                        }
                        3 => {
                            layer.effects.push(Effect {
                                id: format!("proc_noise_{}", self.current_frame),
                                name: "Fractal Cells (非破壊)".to_owned(),
                                effect_type: EffectType::FractalNoise {
                                    fractal_type: Animatable::new_constant(1.0),
                                    contrast: Animatable::new_constant(135.0),
                                    brightness: Animatable::new_constant(5.0),
                                    complexity: Animatable::new_constant(3.0),
                                    evolution: Animatable::new_constant(0.0),
                                },
                                enabled: true,
                            });
                            layer.effects.push(Effect {
                                id: format!("proc_colorama_{}", self.current_frame),
                                name: "Colorama".to_owned(),
                                effect_type: EffectType::Colorama {
                                    preset_index: Animatable::new_constant(1.0),
                                    cycle_phase: Animatable::new_constant(0.25),
                                },
                                enabled: true,
                            });
                        }
                        4 => {
                            layer.blend_mode = BlendMode::Screen;
                            layer.effects.push(Effect {
                                id: format!("proc_sweep_{}", self.current_frame),
                                name: "Optical Light Sweep".to_owned(),
                                effect_type: EffectType::LightSweep {
                                    direction_deg: Animatable::new_constant(28.0),
                                    center: Animatable::new_constant(0.32),
                                    width: Animatable::new_constant(180.0),
                                    sweep_intensity: Animatable::new_constant(88.0),
                                    edge_intensity: Animatable::new_constant(28.0),
                                },
                                enabled: true,
                            });
                            layer.effects.push(Effect {
                                id: format!("proc_leak_grad_{}", self.current_frame),
                                name: "Light Leak Palette".to_owned(),
                                effect_type: EffectType::GradientMap {
                                    low_color: Animatable::new_constant([0.04, 0.01, 0.02]),
                                    mid_color: Animatable::new_constant([0.92, 0.36, 0.14]),
                                    high_color: Animatable::new_constant([1.0, 0.82, 0.44]),
                                },
                                enabled: true,
                            });
                            layer.effects.push(Effect {
                                id: format!("proc_leak_glow_{}", self.current_frame),
                                name: "Optical Bloom".to_owned(),
                                effect_type: EffectType::GlowPro {
                                    threshold: Animatable::new_constant(35.0),
                                    radius: Animatable::new_constant(42.0),
                                    intensity: Animatable::new_constant(1.4),
                                },
                                enabled: true,
                            });
                        }
                        5 => {
                            layer.is_adjustment_layer = true;
                            layer.effects.push(Effect {
                                id: format!("proc_grain_{}", self.current_frame),
                                name: "35mm Film Grain".to_owned(),
                                effect_type: EffectType::FilmGrain {
                                    intensity: Animatable::new_constant(18.0),
                                    grain_size: 1.4,
                                    color_film: true,
                                },
                                enabled: true,
                            });
                            layer.effects.push(Effect {
                                id: format!("proc_vignette_{}", self.current_frame),
                                name: "Cinema Vignette".to_owned(),
                                effect_type: EffectType::Vignette {
                                    intensity: Animatable::new_constant(32.0),
                                    roundness: Animatable::new_constant(50.0),
                                    feather: Animatable::new_constant(65.0),
                                    color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                                },
                                enabled: true,
                            });
                        }
                        _ => {
                            layer.blend_mode = BlendMode::Add;
                            layer.effects.push(Effect {
                                id: format!("proc_flare_{}", self.current_frame),
                                name: "Anamorphic Flare".to_owned(),
                                effect_type: EffectType::LensFlare {
                                    enabled: Animatable::new_constant(1.0),
                                    position_x: Animatable::new_constant(0.5),
                                    position_y: Animatable::new_constant(0.45),
                                    intensity: Animatable::new_constant(1.25),
                                    threshold: Animatable::new_constant(0.2),
                                    color: Animatable::new_constant([0.38, 0.72, 1.0, 1.0]),
                                    link_to_light: None,
                                },
                                enabled: true,
                            });
                            layer.effects.push(Effect {
                                id: format!("proc_flare_ca_{}", self.current_frame),
                                name: "Lens Fringing".to_owned(),
                                effect_type: EffectType::ChromaticAberration {
                                    shift_r: Animatable::new_constant(2.5),
                                    shift_b: Animatable::new_constant(-2.5),
                                    edge_falloff: Animatable::new_constant(0.6),
                                    iris_linked: true,
                                },
                                enabled: true,
                            });
                            layer.effects.push(Effect {
                                id: format!("proc_flare_glow_{}", self.current_frame),
                                name: "Streak Glow".to_owned(),
                                effect_type: EffectType::GlowPro {
                                    threshold: Animatable::new_constant(40.0),
                                    radius: Animatable::new_constant(36.0),
                                    intensity: Animatable::new_constant(1.2),
                                },
                                enabled: true,
                            });
                        }
                    }
                }
                self.mark_project_changed();
            }
            self.engine_hub_status = format!("非破壊ジェネレーター追加: {label} (スライダー/キーフレーム対応)");
            return;
        }
        let (w, h, center) = {
            let comp = self.project_model.active_composition();
            (
                comp.width.clamp(128, 1920),
                comp.height.clamp(72, 1080),
                [comp.width as f64 * 0.5, comp.height as f64 * 0.5],
            )
        };
        let mut rgba_f32: Option<Vec<f32>> = None;

        #[cfg(feature = "wgpu")]
        {
            use kagari_vfx::core::ae_generators_gpu::{
                render_ae_generator_gpu, AeGeneratorKind, AeGeneratorParams,
            };
            let kind = match gen_idx {
                0 => AeGeneratorKind::SmpteBars,
                1 => AeGeneratorKind::ProGrid,
                2 => AeGeneratorKind::FourColorGradient,
                _ => AeGeneratorKind::CellPattern,
            };
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
            if let Some(adapter) =
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                }))
            {
                if let Ok((device, queue)) = pollster::block_on(adapter.request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("kagari_generator"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::default(),
                        memory_hints: wgpu::MemoryHints::default(),
                    },
                    None,
                )) {
                    let params = AeGeneratorParams {
                        kind,
                        ..Default::default()
                    };
                    rgba_f32 = render_ae_generator_gpu(&device, &queue, None, w, h, &params);
                }
            }
        }

        let rgba8: Vec<u8> = if let Some(buf) = rgba_f32 {
            buf.into_iter()
                .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
                .collect()
        } else {
            let mut out = vec![0u8; (w * h * 4) as usize];
            for y in 0..h {
                for x in 0..w {
                    let u = x as f32 / w as f32;
                    let v = y as f32 / h as f32;
                    let (r, g, b) = match gen_idx {
                        0 => {
                            let idx = ((u * 7.0) as usize).min(6);
                            const BARS: [[u8; 3]; 7] = [
                                [191, 191, 191],
                                [191, 191, 0],
                                [0, 191, 191],
                                [0, 191, 0],
                                [191, 0, 191],
                                [191, 0, 0],
                                [0, 0, 191],
                            ];
                            (BARS[idx][0], BARS[idx][1], BARS[idx][2])
                        }
                        1 => {
                            let grid = x % 32 == 0 || y % 32 == 0;
                            if grid {
                                (90, 195, 255)
                            } else {
                                (20, 24, 32)
                            }
                        }
                        2 => (
                            ((1.0 - u) * 240.0 + v * 60.0).min(255.0) as u8,
                            (u * 180.0 + (1.0 - v) * 90.0).min(255.0) as u8,
                            (v * 240.0 + u * 50.0).min(255.0) as u8,
                        ),
                        _ => {
                            let cx = ((u * 8.0).fract() - 0.5).abs();
                            let cy = ((v * 6.0).fract() - 0.5).abs();
                            let d = ((cx * cx + cy * cy).sqrt() * 255.0).min(255.0) as u8;
                            (d / 2, d, 255 - d / 2)
                        }
                    };
                    let idx = ((y * w + x) * 4) as usize;
                    out[idx] = r;
                    out[idx + 1] = g;
                    out[idx + 2] = b;
                    out[idx + 3] = 255;
                }
            }
            out
        };

        let Some(img) = image::RgbaImage::from_raw(w, h, rgba8) else {
            return;
        };
        let cache_dir = std::env::temp_dir().join("kagari_generators");
        let _ = std::fs::create_dir_all(&cache_dir);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let out_png = cache_dir.join(format!("gen_{gen_idx}_{stamp}.png"));
        if img.save(&out_png).is_err() {
            return;
        }
        let source_path = out_png.display().to_string();
        self.register_project_asset(
            label.to_owned(),
            kagari_vfx::core::timeline::ProjectItemType::Image {
                path: source_path.clone(),
                width: w,
                height: h,
            },
        );
        self.add_canvas_layer(
            label.to_owned(),
            kagari_vfx::core::timeline::LayerType::Image { path: source_path },
            center,
        );
        self.engine_hub_status = format!("生成レイヤー追加: {label} ({w}x{h})");
    }

    pub(super) fn run_time_remap_optical_flow(&mut self) {
        let (w, h, f0) = self.render_small_rgba_f32();
        let comp = self.project_model.active_composition();
        let next_frame = (self.current_frame + 1).min(comp.duration_frames.saturating_sub(1));
        let f1_u8 = kagari_vfx::core::software_renderer::render_frame_to_pixels(
            comp, next_frame, w, h, 0.0, 0,
        );
        let f0_u8: Vec<u8> = f0
            .iter()
            .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
            .collect();
        let flow_ab = kagari_vfx::core::optical_flow_timewarp::compute_dense_optical_flow(
            &f0_u8, &f1_u8, w, h, 2, 4,
        );
        let flow_ba = kagari_vfx::core::optical_flow_timewarp::compute_dense_optical_flow(
            &f1_u8, &f0_u8, w, h, 2, 4,
        );
        let mut interp = vec![0u8; (w * h * 4) as usize];
        kagari_vfx::core::optical_flow_timewarp::interpolate_timewarp_frame(
            &f0_u8, &f1_u8, w, h, &flow_ab, &flow_ba, 0.5, &mut interp,
        );
        if let Some(layer_id) = self.selected_layer_id.clone() {
            if let Some(layer) = self
                .project_model
                .active_composition_mut()
                .layers
                .iter_mut()
                .find(|l| l.id == layer_id && !l.locked)
            {
                layer.frame_blending = true;
                self.mark_project_changed();
            }
        }
        self.engine_hub_status = format!(
            "Optical Flow 時間補間 (t=0.5): {} bytes 生成・フレームブレンド有効化",
            interp.len()
        );
    }

    pub(crate) fn evaluate_compositor_graph_rgba8(
        &self,
        w: u32,
        h: u32,
        rgba8: Vec<u8>,
    ) -> Vec<u8> {
        let rgba_f32: Vec<f32> = rgba8.iter().map(|&b| f32::from(b) / 255.0).collect();
        let mut inputs = std::collections::HashMap::new();
        if let Ok(fb) =
            kagari_vfx::core::compositor_node_graph::FrameBufferF32::new(w, h, rgba_f32, None)
        {
            inputs.insert(1u64, fb);
        }
        #[cfg(feature = "wgpu")]
        let res = self.compositor_graph.evaluate(&inputs, 999, None);
        #[cfg(not(feature = "wgpu"))]
        let res = self.compositor_graph.evaluate(&inputs, 999);
        match res {
            Ok(out) => out
                .rgba
                .into_iter()
                .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
                .collect(),
            Err(_) => rgba8,
        }
    }

    pub(super) fn node_graph_insert_before_out(
        &mut self,
        label: &str,
        kind: kagari_vfx::core::compositor_node_graph::NodeKind,
    ) {
        let new_id = self.node_graph_next_id;
        self.node_graph_next_id += 1;
        let out_id = 999u64;
        let prev_to_out = self
            .compositor_graph
            .edges
            .iter()
            .find(|e| e.to == out_id && e.to_slot == 0)
            .map(|e| e.from)
            .unwrap_or(1);
        self.compositor_graph
            .edges
            .retain(|e| !(e.from == prev_to_out && e.to == out_id && e.to_slot == 0));
        self.compositor_graph.add_node(new_id, label, kind);
        let _ = self.compositor_graph.connect(prev_to_out, new_id, 0);
        let _ = self.compositor_graph.connect(new_id, out_id, 0);
        self.node_graph_selected_id = Some(new_id);
        self.node_graph_status = format!(
            "ノード追加: #{new_id} {label} ({}ノード / {}接続)",
            self.compositor_graph.nodes.len(),
            self.compositor_graph.edges.len()
        );
    }

    pub(super) fn node_graph_remove_selected(&mut self) {
        let Some(target_id) = self.node_graph_selected_id else {
            self.node_graph_status = "削除する中間ノードを選択してください。".to_owned();
            return;
        };
        if target_id == 1 || target_id == 999 {
            self.node_graph_status = "MediaIn / MediaOut ノードは削除できません。".to_owned();
            return;
        }
        let upstream = self
            .compositor_graph
            .edges
            .iter()
            .find(|e| e.to == target_id)
            .map(|e| e.from);
        let downstream = self
            .compositor_graph
            .edges
            .iter()
            .find(|e| e.from == target_id)
            .map(|e| (e.to, e.to_slot));
        self.compositor_graph.nodes.remove(&target_id);
        self.compositor_graph
            .edges
            .retain(|e| e.from != target_id && e.to != target_id);
        if let (Some(u), Some((d, slot))) = (upstream, downstream) {
            let _ = self.compositor_graph.connect(u, d, slot);
        }
        self.node_graph_selected_id = upstream;
        self.node_graph_status = format!(
            "ノード #{target_id} を削除 ({}ノード / {}接続)",
            self.compositor_graph.nodes.len(),
            self.compositor_graph.edges.len()
        );
    }

    pub(super) fn node_graph_evaluate_current_frame(&mut self) {
        let (w, h, rgba) = self.render_small_rgba_f32();
        let mut inputs = std::collections::HashMap::new();
        if let Ok(fb) =
            kagari_vfx::core::compositor_node_graph::FrameBufferF32::new(w, h, rgba, None)
        {
            inputs.insert(1u64, fb);
        }
        let t0 = std::time::Instant::now();
        #[cfg(feature = "wgpu")]
        let res = self.compositor_graph.evaluate(&inputs, 999, None);
        #[cfg(not(feature = "wgpu"))]
        let res = self.compositor_graph.evaluate(&inputs, 999);
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        self.node_graph_status = match res {
            Ok(out) => format!(
                "DAG評価完了: {}x{} RGBA32F ({}ノード, {:.2}ms)",
                out.width,
                out.height,
                self.compositor_graph.nodes.len(),
                ms
            ),
            Err(err) => format!("DAG評価エラー: {err}"),
        };
    }

    pub(super) fn node_graph_layout_rects(
        &self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Vec<(u64, Rect)> {
        let order = self.compositor_graph.topological_order().unwrap_or_else(|_| {
            let mut ids: Vec<u64> = self.compositor_graph.nodes.keys().copied().collect();
            ids.sort_unstable();
            ids
        });
        let count = order.len().max(1) as f64;
        let node_w = 104.0f64.min((width - 24.0) / count - 16.0).max(56.0);
        let node_h = 44.0f64.min(height - 24.0).max(28.0);
        let gap = if order.len() > 1 {
            ((width - 24.0 - node_w * count) / (count - 1.0)).clamp(12.0, 48.0)
        } else {
            0.0
        };
        let start_x = x + 12.0;
        let cy = y + (height - node_h) * 0.5;
        order
            .into_iter()
            .enumerate()
            .map(|(i, id)| {
                let (nx, ny) = self
                    .node_graph_positions
                    .get(&id)
                    .map(|&(rx, ry)| {
                        (
                            (x + rx).clamp(x + 6.0, x + width - node_w - 6.0),
                            (y + ry).clamp(y + 6.0, y + height - node_h - 6.0),
                        )
                    })
                    .unwrap_or_else(|| (start_x + i as f64 * (node_w + gap), cy));
                (id, Rect::new(nx, ny, nx + node_w, ny + node_h))
            })
            .collect()
    }

    pub(crate) fn draw_node_graph_canvas(
        &self,
        scene: &mut Scene,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) {
        let bg = Rect::new(x, y, x + width, y + height);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(11, 13, 18),
            None,
            &bg,
        );
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            Color::from_rgb8(36, 42, 54),
            None,
            &bg,
        );

        let rects = self.node_graph_layout_rects(x, y, width, height);
        let rect_map: std::collections::HashMap<u64, Rect> = rects.iter().copied().collect();

        for edge in &self.compositor_graph.edges {
            if let (Some(r0), Some(r1)) = (rect_map.get(&edge.from), rect_map.get(&edge.to)) {
                let p0 = (r0.x1, (r0.y0 + r0.y1) * 0.5);
                let p1 = (r1.x0, (r1.y0 + r1.y1) * 0.5);
                let dx = ((p1.0 - p0.0) * 0.45).max(16.0);
                let mut wire = BezPath::new();
                wire.move_to(p0);
                wire.curve_to((p0.0 + dx, p0.1), (p1.0 - dx, p1.1), p1);
                scene.stroke(
                    &Stroke::new(2.0),
                    Affine::IDENTITY,
                    Color::from_rgb8(96, 188, 255),
                    None,
                    &wire,
                );
            }
        }

        for (id, rect) in &rects {
            let Some(node) = self.compositor_graph.nodes.get(id) else {
                continue;
            };
            let selected = self.node_graph_selected_id == Some(*id);
            let accent = match node.kind {
                kagari_vfx::core::compositor_node_graph::NodeKind::MediaIn => {
                    Color::from_rgb8(45, 195, 220)
                }
                kagari_vfx::core::compositor_node_graph::NodeKind::ColorWheelGrade(_) => {
                    Color::from_rgb8(245, 168, 52)
                }
                kagari_vfx::core::compositor_node_graph::NodeKind::ProKeyer(_) => {
                    Color::from_rgb8(64, 212, 128)
                }
                kagari_vfx::core::compositor_node_graph::NodeKind::GaussianBlur { .. } => {
                    Color::from_rgb8(90, 148, 250)
                }
                kagari_vfx::core::compositor_node_graph::NodeKind::Transform2D { .. } => {
                    Color::from_rgb8(178, 118, 250)
                }
                #[cfg(feature = "wgpu")]
                kagari_vfx::core::compositor_node_graph::NodeKind::SapphireOptical(_) => {
                    Color::from_rgb8(255, 198, 64)
                }
                #[cfg(feature = "wgpu")]
                kagari_vfx::core::compositor_node_graph::NodeKind::SaberBeam { .. } => {
                    Color::from_rgb8(56, 232, 255)
                }
                kagari_vfx::core::compositor_node_graph::NodeKind::DeepMerge { .. } => {
                    Color::from_rgb8(92, 225, 198)
                }
                kagari_vfx::core::compositor_node_graph::NodeKind::OpenFx { .. } => {
                    Color::from_rgb8(255, 132, 86)
                }
                kagari_vfx::core::compositor_node_graph::NodeKind::MediaOut => {
                    Color::from_rgb8(236, 86, 118)
                }
                _ => Color::from_rgb8(140, 150, 168),
            };
            let card = RoundedRect::from_rect(*rect, 6.0);
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                Color::from_rgb8(24, 28, 36),
                None,
                &card,
            );
            let header = RoundedRect::from_rect(
                Rect::new(rect.x0, rect.y0, rect.x1, (rect.y0 + 10.0).min(rect.y1)),
                6.0,
            );
            scene.fill(Fill::NonZero, Affine::IDENTITY, accent, None, &header);
            scene.stroke(
                &Stroke::new(if selected { 2.0 } else { 1.0 }),
                Affine::IDENTITY,
                if selected {
                    Color::from_rgb8(255, 224, 102)
                } else {
                    Color::from_rgb8(58, 66, 82)
                },
                None,
                &card,
            );
            let cy = (rect.y0 + rect.y1) * 0.5;
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                accent,
                None,
                &Circle::new((rect.x0, cy), 3.5),
            );
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                accent,
                None,
                &Circle::new((rect.x1, cy), 3.5),
            );

            // Draw crisp node-type vector glyph inside the node card
            let cx = (rect.x0 + rect.x1) * 0.5;
            let gy = cy + 4.0;
            match node.kind {
                kagari_vfx::core::compositor_node_graph::NodeKind::MediaIn => {
                    let mut p = BezPath::new();
                    p.move_to((cx - 5.0, gy - 6.0));
                    p.line_to((cx + 6.0, gy));
                    p.line_to((cx - 5.0, gy + 6.0));
                    p.close_path();
                    scene.fill(Fill::NonZero, Affine::IDENTITY, accent, None, &p);
                }
                kagari_vfx::core::compositor_node_graph::NodeKind::ColorWheelGrade(_) => {
                    scene.stroke(&Stroke::new(1.8), Affine::IDENTITY, accent, None, &Circle::new((cx, gy), 7.0));
                    scene.fill(Fill::NonZero, Affine::IDENTITY, accent, None, &Circle::new((cx + 2.5, gy - 1.5), 2.2));
                }
                kagari_vfx::core::compositor_node_graph::NodeKind::GaussianBlur { .. } => {
                    scene.stroke(&Stroke::new(1.4), Affine::IDENTITY, accent, None, &Circle::new((cx, gy), 7.5));
                    scene.stroke(&Stroke::new(1.2), Affine::IDENTITY, accent, None, &Circle::new((cx, gy), 4.0));
                }
                kagari_vfx::core::compositor_node_graph::NodeKind::ProKeyer(_) => {
                    let mut p = BezPath::new();
                    p.move_to((cx, gy - 7.0));
                    p.line_to((cx + 7.0, gy));
                    p.line_to((cx, gy + 7.0));
                    p.line_to((cx - 7.0, gy));
                    p.close_path();
                    scene.stroke(&Stroke::new(1.6), Affine::IDENTITY, accent, None, &p);
                }
                kagari_vfx::core::compositor_node_graph::NodeKind::MediaOut => {
                    let mon = RoundedRect::from_rect(Rect::new(cx - 8.0, gy - 5.5, cx + 8.0, gy + 5.5), 2.0);
                    scene.stroke(&Stroke::new(1.6), Affine::IDENTITY, accent, None, &mon);
                }
                _ => {
                    let mut p = BezPath::new();
                    p.move_to((cx - 7.0, gy));
                    p.line_to((cx + 7.0, gy));
                    p.move_to((cx, gy - 6.0));
                    p.line_to((cx, gy + 6.0));
                    scene.stroke(&Stroke::new(1.6), Affine::IDENTITY, accent, None, &p);
                }
            }
        }
        if let Some((_, p0, p1)) = self.node_graph_drag_wire {
            let dx = ((p1.0 - p0.0).abs() * 0.45).max(16.0);
            let mut drag_bez = BezPath::new();
            drag_bez.move_to(p0);
            drag_bez.curve_to((p0.0 + dx, p0.1), (p1.0 - dx, p1.1), p1);
            scene.stroke(
                &Stroke::new(2.5),
                Affine::IDENTITY,
                Color::from_rgb8(255, 168, 56),
                None,
                &drag_bez,
            );
        }
    }

    pub(crate) fn handle_node_graph_canvas_pointer(&mut self, event: CanvasPointerEvent) -> bool {
        let Some((x, y, width, height)) = self.node_graph_bounds else {
            return false;
        };
        let px = f64::from(event.x);
        let py = f64::from(event.y);
        let rects = self.node_graph_layout_rects(x, y, width, height);

        match event.phase {
            CanvasPointerPhase::Down => {
                for &(id, rect) in &rects {
                    let cy = (rect.y0 + rect.y1) * 0.5;
                    let dx = px - rect.x1;
                    let dy = py - cy;
                    if dx * dx + dy * dy <= 100.0 {
                        self.node_graph_drag_wire = Some((id, (rect.x1, cy), (px, py)));
                        self.node_graph_selected_id = Some(id);
                        self.node_graph_status = format!("ノード #{id} から結線ドラッグ中...");
                        let _ = self.sync_bindings();
                        return true;
                    }
                }
                for &(id, rect) in &rects {
                    if rect.contains((px, py)) {
                        self.node_graph_selected_id = Some(id);
                        self.node_graph_drag_node = Some((id, px - rect.x0, py - rect.y0));
                        if let Some(node) = self.compositor_graph.nodes.get(&id) {
                            self.node_graph_status = format!(
                                "ノード選択: #{id} {} ({}ノード)",
                                node.label,
                                self.compositor_graph.nodes.len()
                            );
                        }
                        let _ = self.sync_bindings();
                        return true;
                    }
                }
                false
            }
            CanvasPointerPhase::Moved => {
                if let Some((from_id, start, _)) = self.node_graph_drag_wire {
                    self.node_graph_drag_wire = Some((from_id, start, (px, py)));
                    return true;
                }
                if let Some((drag_id, off_x, off_y)) = self.node_graph_drag_node {
                    self.node_graph_positions
                        .insert(drag_id, (px - off_x - x, py - off_y - y));
                    return true;
                }
                false
            }
            CanvasPointerPhase::Up => {
                if let Some((from_id, _, _)) = self.node_graph_drag_wire.take() {
                    for &(to_id, rect) in &rects {
                        if to_id == from_id {
                            continue;
                        }
                        let cy = (rect.y0 + rect.y1) * 0.5;
                        let dx = px - rect.x0;
                        let dy = py - cy;
                        if dx * dx + dy * dy <= 144.0 || rect.contains((px, py)) {
                            let _ = self.compositor_graph.connect(from_id, to_id, 0);
                            if self.compositor_graph.topological_order().is_err() {
                                self.compositor_graph
                                    .edges
                                    .retain(|e| !(e.from == from_id && e.to == to_id));
                                self.node_graph_status =
                                    "循環接続は禁止されているためキャンセルしました".to_owned();
                            } else {
                                self.node_graph_status = format!("結線接続: #{from_id} → #{to_id}");
                            }
                            let _ = self.sync_bindings();
                            return true;
                        }
                    }
                    self.node_graph_status = "結線をキャンセルしました".to_owned();
                    let _ = self.sync_bindings();
                    return true;
                }
                if self.node_graph_drag_node.take().is_some() {
                    return true;
                }
                false
            }
            CanvasPointerPhase::Cancelled => {
                self.node_graph_drag_wire = None;
                self.node_graph_drag_node = None;
                false
            }
        }
    }
}
