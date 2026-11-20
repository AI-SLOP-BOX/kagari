use super::*;

impl AfterEffectsStudio {
    fn render_small_rgba_f32(&self) -> (u32, u32, Vec<f32>) {
        let comp = self.project_model.active_composition();
        let w = 64u32;
        let h = 36u32;
        let rgba8 = kagari_vfx::core::software_renderer::render_frame_to_pixels(
            comp,
            self.current_frame,
            w,
            h,
            0.0,
            0,
        );
        let rgba32: Vec<f32> = rgba8.into_iter().map(|b| f32::from(b) / 255.0).collect();
        (w, h, rgba32)
    }

    pub(crate) fn handle_engine_hub_action(&mut self, action_name: &str) -> bool {
        match action_name {
            "engine_analyze_color_scopes" => {
                let (w, h, rgba) = self.render_small_rgba_f32();
                let picked = kagari_vfx::core::color_scopes::pick_hdr_color_region(
                    &rgba,
                    w,
                    h,
                    w / 2,
                    h / 2,
                    2,
                );
                let wave = kagari_vfx::core::color_scopes::compute_waveform_monitor(&rgba, w, h, 16, 8);
                let vec_s = kagari_vfx::core::color_scopes::compute_vectorscope(&rgba, w, h, 17);
                let parade = kagari_vfx::core::color_scopes::compute_rgb_parade(&rgba, w, h, 16, 8);
                let wave_peak = wave.iter().copied().max().unwrap_or(0);
                let vec_peak = vec_s.iter().copied().max().unwrap_or(0);
                let r_peak = parade[0].iter().copied().max().unwrap_or(0);
                if let Some(p) = picked {
                    self.engine_hub_status = format!(
                        "スコープ計測完了: Luma={:.3} ({:+.2} stops) | 波形ピーク={} | ベクトル={} | Rパレード={}",
                        p.luma_709, p.stops_over_midgray, wave_peak, vec_peak, r_peak
                    );
                }
                true
            }
            "engine_analyze_film_grain" => {
                let (w, h, mut rgba) = self.render_small_rgba_f32();
                let prof = kagari_vfx::core::film_grain_pro::analyze_film_grain(&rgba, w, h)
                    .unwrap_or_default();
                kagari_vfx::core::film_grain_pro::synthesize_film_grain(
                    &mut rgba,
                    w,
                    h,
                    &prof,
                    self.current_frame,
                    1,
                );
                self.engine_hub_status = format!(
                    "フィルムグレイン解析/再生成: σR={:.4} σG={:.4} σB={:.4}",
                    prof.sigma_r, prof.sigma_g, prof.sigma_b
                );
                true
            }
            "engine_run_pro_keyer" => {
                let (_, _, mut rgba) = self.render_small_rgba_f32();
                kagari_vfx::core::pro_keyer::apply_pro_keyer_cpu(
                    &mut rgba,
                    &kagari_vfx::core::pro_keyer::ProKeyerParams::default(),
                );
                let opaque = rgba.chunks_exact(4).filter(|px| px[3] > 0.5).count();
                self.engine_hub_status = format!(
                    "ProKeyer 実行完了: 残存不透明ピクセル {opaque} / {}",
                    rgba.len() / 4
                );
                true
            }
            "engine_run_deep_and_node_graph" => {
                let (w, h, rgba) = self.render_small_rgba_f32();
                let bbox = kagari_vfx::core::roi_scanline::compute_nonzero_bbox(&rgba, w, h, 1e-3);
                let mut tagged = kagari_vfx::core::premult_guard::TaggedRgbaBuffer::new(
                    rgba.clone(),
                    kagari_vfx::core::premult_guard::AlphaMode::Premultiplied,
                );
                tagged.with_safe_color_grade(|px| {
                    for ch in px.chunks_exact_mut(4) {
                        ch[0] = (ch[0] * 1.05).min(1.0);
                    }
                });
                let mut g = kagari_vfx::core::compositor_node_graph::CompositorNodeGraph::new();
                g.add_node(1, "In", kagari_vfx::core::compositor_node_graph::NodeKind::MediaIn);
                g.add_node(
                    2,
                    "Wheel",
                    kagari_vfx::core::compositor_node_graph::NodeKind::ColorWheelGrade(
                        Default::default(),
                    ),
                );
                g.add_node(
                    3,
                    "Keyer",
                    kagari_vfx::core::compositor_node_graph::NodeKind::ProKeyer(Default::default()),
                );
                g.add_node(4, "Out", kagari_vfx::core::compositor_node_graph::NodeKind::MediaOut);
                let _ = g.connect(1, 2, 0);
                let _ = g.connect(2, 3, 0);
                let _ = g.connect(3, 4, 0);
                let mut inputs = std::collections::HashMap::new();
                if let Ok(fb) = kagari_vfx::core::compositor_node_graph::FrameBufferF32::new(
                    w,
                    h,
                    tagged.rgba,
                    None,
                ) {
                    inputs.insert(1, fb);
                }
                #[cfg(feature = "wgpu")]
                let res = g.evaluate(&inputs, 4, None);
                #[cfg(not(feature = "wgpu"))]
                let res = g.evaluate(&inputs, 4);
                let ok_str = if res.is_ok() { "OK" } else { "Err" };
                self.engine_hub_status = format!(
                    "DAG/ROI 実行({ok_str}): DataWindow={}x{}@({},{})",
                    bbox.width, bbox.height, bbox.x, bbox.y
                );
                true
            }
            "engine_run_roto_refine" => {
                let (w, h, rgba) = self.render_small_rgba_f32();
                let matte: Vec<f32> = rgba.chunks_exact(4).map(|px| px[3]).collect();
                let refined = kagari_vfx::core::roto_refine::refine_roto_matte(
                    &rgba,
                    &matte,
                    w,
                    h,
                    &Default::default(),
                );
                self.engine_hub_status = format!(
                    "Roto Refine 完了: {}px 境界処理",
                    refined.map_or(0, |(_, a)| a.len())
                );
                true
            }
            "engine_run_mograph_cloner" => {
                let clones = kagari_vfx::core::mograph_cloner::evaluate_cloner(
                    &kagari_vfx::core::mograph_cloner::ClonerMode::Radial {
                        count: 6,
                        radius: 120.0,
                        plane: kagari_vfx::core::mograph_cloner::RadialPlane::XZ,
                        start_angle_deg: 0.0,
                        end_angle_deg: 360.0,
                        align_to_circle: true,
                    },
                    &[kagari_vfx::core::mograph_cloner::Effector::Step {
                        strength: 1.0,
                        scale_end: [1.5, 1.5, 1.5],
                        rotation_end_deg: [0.0, 45.0, 0.0],
                    }],
                );
                let mesh = kagari_vfx::core::extruded_mesh_3d::extrude_polygon_beveled(
                    &[[-10.0, -10.0], [10.0, -10.0], [10.0, 10.0], [-10.0, 10.0]],
                    16.0,
                    2.0,
                );
                #[cfg(feature = "wgpu")]
                let sh_verts = mesh
                    .as_ref()
                    .map_or(0, |m| m.to_shadow_vertices([1.0; 4]).len());
                #[cfg(not(feature = "wgpu"))]
                let sh_verts = mesh.as_ref().map_or(0, |m| m.vertices.len());
                self.engine_hub_status = format!(
                    "MoGraph & 3D 押し出し生成: {} クローン / {} 影対応頂点",
                    clones.len(),
                    sh_verts
                );
                true
            }
            "engine_run_sfm_and_lens" => {
                let intr = kagari_vfx::core::sfm_camera_tracker::CameraIntrinsics {
                    focal_px: 1000.0,
                    cx: 960.0,
                    cy: 540.0,
                };
                let pose_a = kagari_vfx::core::sfm_camera_tracker::CameraPose3D {
                    position: [0.0, 0.0, 0.0],
                    rotation_deg: [0.0, 0.0, 0.0],
                };
                let pose_b = kagari_vfx::core::sfm_camera_tracker::CameraPose3D {
                    position: [80.0, 0.0, 0.0],
                    rotation_deg: [0.0, -4.0, 0.0],
                };
                let pts = [
                    [-100.0, -50.0, 800.0],
                    [120.0, 60.0, 900.0],
                    [0.0, 100.0, 750.0],
                    [80.0, -90.0, 850.0],
                ];
                let corr: Vec<_> = pts
                    .iter()
                    .filter_map(|&wpt| {
                        let pa = pose_a.project(wpt, &intr)?;
                        let pb = pose_b.project(wpt, &intr)?;
                        let tri = kagari_vfx::core::sfm_camera_tracker::triangulate_two_view(
                            pa, &pose_a, pb, &pose_b, &intr,
                        )?;
                        Some((tri, pb))
                    })
                    .collect();
                let (_, rmse) = kagari_vfx::core::sfm_camera_tracker::refine_camera_pose(
                    &corr,
                    kagari_vfx::core::sfm_camera_tracker::CameraPose3D {
                        position: [72.0, 2.0, -4.0],
                        rotation_deg: [0.5, -3.0, 0.0],
                    },
                    &intr,
                );
                let lens = kagari_vfx::core::warp_morph_lens::estimate_lens_distortion_k1_k2(&[
                    vec![[0.1, 0.2], [0.5, 0.19], [0.9, 0.2]],
                ]);
                self.engine_hub_status = format!(
                    "SfM 3Dカメラ解析 RMSE={rmse:.2}px | レンズ歪み k1={:.3}",
                    lens.k1
                );
                true
            }
            "engine_run_temporal_cleanplate" => {
                let (w, h, rgba) = self.render_small_rgba_f32();
                let mask = vec![0.0_f32; (w * h) as usize];
                let clean = kagari_vfx::core::temporal_cleanplate::remove_object_temporal_f32(
                    &rgba,
                    &mask,
                    &[(&rgba, &mask)],
                    w,
                    h,
                );
                self.engine_hub_status = format!(
                    "時間軸クリーンプレート解析完了: {} floats",
                    clean.map_or(0, |v| v.len())
                );
                true
            }
            "engine_apply_composer_preset" => {
                let rig = kagari_vfx::core::motion_composer::TransitionRig {
                    in_preset: Some("ac_overshoot_pop"),
                    out_preset: Some("ac_whip_slide_left"),
                    in_duration_frames: 15,
                    out_duration_frames: 10,
                };
                let delta = rig.evaluate(self.current_frame as f32, 0.0, 100.0);
                let mut applied_layer = false;
                if let Some(layer_id) = self.selected_layer_id.clone() {
                    let f0 = self.current_frame;
                    if let Some(layer) = self
                        .project_model
                        .active_composition_mut()
                        .layers
                        .iter_mut()
                        .find(|l| l.id == layer_id && !l.locked)
                    {
                        let ease = kagari_vfx::core::keyframe::InterpolationType::Bezier {
                            outgoing: Default::default(),
                            incoming: Default::default(),
                            custom_bezier: Some([0.22, 1.0, 0.36, 1.0]),
                        };
                        layer.transform.scale.add_keyframe(
                            kagari_vfx::core::keyframe::Keyframe::new(f0, [0.0, 0.0], ease.clone()),
                        );
                        layer.transform.scale.add_keyframe(
                            kagari_vfx::core::keyframe::Keyframe::new(
                                f0 + 10,
                                [112.0, 112.0],
                                ease.clone(),
                            ),
                        );
                        layer.transform.scale.add_keyframe(
                            kagari_vfx::core::keyframe::Keyframe::new(f0 + 15, [100.0, 100.0], ease),
                        );
                        applied_layer = true;
                    }
                    if applied_layer {
                        self.mark_project_changed();
                    }
                }
                self.engine_hub_status = format!(
                    "Animation Composer (Overshoot): scale=({:.2},{:.2}) opacity={:.2}{}",
                    delta.scale_mul[0],
                    delta.scale_mul[1],
                    delta.opacity_mul,
                    if applied_layer { " [レイヤーへキーフレーム適用済]" } else { "" }
                );
                true
            }
            "engine_apply_shape_pucker_twist" => {
                let sq = [[-20.0, -20.0], [20.0, -20.0], [20.0, 20.0], [-20.0, 20.0]];
                let bloated = kagari_vfx::core::ae_shape_modifiers::apply_pucker_bloat(&sq, 45.0, true);
                let twisted =
                    kagari_vfx::core::ae_shape_modifiers::apply_twist(&bloated, 35.0, [0.0, 0.0], 25.0);
                let wig = kagari_vfx::core::ae_shape_modifiers::evaluate_wiggle_transform(
                    0,
                    self.current_frame as f32 / 30.0,
                    &Default::default(),
                );
                self.engine_hub_status = format!(
                    "AEシェイプ変形: {}頂点 (Pucker+Twist) / Wiggle回転={:.1}°",
                    twisted.len(),
                    wig.rotation_deg
                );
                true
            }
            "engine_auto_proxy_step" => {
                let mut ctrl = kagari_vfx::core::auto_proxy_cache::AutoProxyController::new(60.0);
                let tier = ctrl.record_frame_time(22.0);
                let layout_ok =
                    kagari_vfx::core::studio_ux::WorkspaceLayout::preset_vfx_compositing()
                        .to_json()
                        .is_ok();
                self.engine_hub_status = format!(
                    "自動プロキシ={}x | レイアウトJSON={layout_ok}",
                    tier.scale()
                );
                true
            }
            "engine_run_cryptomatte" => {
                let id_a = kagari_vfx::core::cryptomatte::cryptomatte_id_float("Hero");
                let rank0 = [id_a, 0.75, 0.0, 0.25];
                let matte = kagari_vfx::core::cryptomatte::extract_cryptomatte_matte(
                    &[&rank0],
                    1,
                    1,
                    &[id_a],
                );
                self.engine_hub_status = format!(
                    "Cryptomatte 抽出完了: cov={:.2}",
                    matte.and_then(|m| m.first().copied()).unwrap_or(0.0)
                );
                true
            }
            "engine_run_aces_check" => {
                let cg = kagari_vfx::core::aces::lin_srgb_to_aces_cg([0.9, 0.25, 0.1]);
                let comp = kagari_vfx::core::aces::aces_gamut_compress(cg, 0.8, 1.2);
                let odt = kagari_vfx::core::aces::aces_preview_transform(comp);
                self.engine_hub_status = format!(
                    "ACES 1.3 ODT変換: ({:.2}, {:.2}, {:.2})",
                    odt[0], odt[1], odt[2]
                );
                true
            }
            "node_graph_add_blur" => {
                self.node_graph_insert_before_out(
                    "GaussianBlur",
                    kagari_vfx::core::compositor_node_graph::NodeKind::GaussianBlur { radius: 4.0 },
                );
                true
            }
            "node_graph_add_color_wheel" => {
                self.node_graph_insert_before_out(
                    "ColorWheel",
                    kagari_vfx::core::compositor_node_graph::NodeKind::ColorWheelGrade(
                        Default::default(),
                    ),
                );
                true
            }
            "node_graph_add_pro_keyer" => {
                self.node_graph_insert_before_out(
                    "ProKeyer",
                    kagari_vfx::core::compositor_node_graph::NodeKind::ProKeyer(Default::default()),
                );
                true
            }
            "node_graph_add_transform" => {
                self.node_graph_insert_before_out(
                    "Transform2D",
                    kagari_vfx::core::compositor_node_graph::NodeKind::Transform2D {
                        translate: [12.0, -8.0],
                        scale: [1.05, 1.05],
                        rotate_deg: 2.0,
                    },
                );
                true
            }
            "node_graph_remove_selected" => {
                self.node_graph_remove_selected();
                true
            }
            "node_graph_evaluate" => {
                self.node_graph_evaluate_current_frame();
                true
            }
            "add_generator_smpte" => {
                self.add_ae_generator_layer(0, "SMPTE カラーバー");
                true
            }
            "add_generator_grid" => {
                self.add_ae_generator_layer(1, "Pro Grid");
                true
            }
            "add_generator_gradient" => {
                self.add_ae_generator_layer(2, "4色グラデーション");
                true
            }
            "add_generator_cells" => {
                self.add_ae_generator_layer(3, "セルパターン");
                true
            }
            "time_remap_optical_flow" => {
                self.run_time_remap_optical_flow();
                true
            }
            "add_saber_beam_layer" => {
                self.add_saber_beam_layer();
                true
            }
            "mograph_clone_selected_layer" => {
                self.mograph_clone_selected_layer();
                true
            }
            "add_3d_extruded_shape" => {
                self.add_3d_extruded_shape();
                true
            }
            "apply_matte_choker" => {
                self.apply_matte_choker();
                true
            }
            "add_tracked_3d_camera" => {
                self.add_tracked_3d_camera();
                true
            }
            "node_graph_add_sapphire" => {
                #[cfg(feature = "wgpu")]
                self.node_graph_insert_before_out(
                    "SapphireGlow",
                    kagari_vfx::core::compositor_node_graph::NodeKind::SapphireOptical(
                        Default::default(),
                    ),
                );
                #[cfg(not(feature = "wgpu"))]
                {
                    self.node_graph_status =
                        "SapphireOptical は wgpu ビルドで有効になります。".to_owned();
                }
                true
            }
            "node_graph_add_saber" => {
                #[cfg(feature = "wgpu")]
                self.node_graph_insert_before_out(
                    "SaberBeam",
                    kagari_vfx::core::compositor_node_graph::NodeKind::SaberBeam {
                        width: 64,
                        height: 36,
                        points: vec![[8.0, 28.0], [56.0, 8.0]],
                        params: Default::default(),
                    },
                );
                #[cfg(not(feature = "wgpu"))]
                {
                    self.node_graph_status = "SaberBeam は wgpu ビルドで有効になります。".to_owned();
                }
                true
            }
            "node_graph_add_deep_merge" => {
                self.node_graph_insert_before_out(
                    "DeepMerge",
                    kagari_vfx::core::compositor_node_graph::NodeKind::DeepMerge { z_epsilon: 0.01 },
                );
                true
            }
            "add_3d_studio_rig" => {
                self.add_3d_extruded_shape();
                self.add_scene_light();
                self.add_tracked_3d_camera();
                self.engine_hub_status =
                    "3Dスタジオセット構築完了 (3D押し出し + 3Dライト + SfMカメラ)".to_owned();
                true
            }
            "apply_wiggle_keyframes" => {
                self.apply_wiggle_keyframes();
                true
            }
            "node_graph_param_up" => {
                self.node_graph_adjust_selected(1.0);
                true
            }
            "node_graph_param_down" => {
                self.node_graph_adjust_selected(-1.0);
                true
            }
            "add_mesh_warp" => {
                self.add_mesh_warp_effect();
                true
            }
            "node_graph_bake_to_layer" => {
                self.node_graph_bake_to_layer();
                true
            }
            "bake_cleanplate_layer" => {
                self.bake_cleanplate_layer();
                true
            }
            "effect_cat_all" => {
                self.effect_category = "all";
                true
            }
            "effect_cat_color" => {
                self.effect_category = "color";
                true
            }
            "effect_cat_blur" => {
                self.effect_category = "blur";
                true
            }
            "effect_cat_distort" => {
                self.effect_category = "distort";
                true
            }
            "effect_cat_key" => {
                self.effect_category = "key";
                true
            }
            "node_graph_reset" => {
                let mut g = kagari_vfx::core::compositor_node_graph::CompositorNodeGraph::new();
                g.add_node(
                    1,
                    "MediaIn",
                    kagari_vfx::core::compositor_node_graph::NodeKind::MediaIn,
                );
                g.add_node(
                    2,
                    "ColorWheel",
                    kagari_vfx::core::compositor_node_graph::NodeKind::ColorWheelGrade(
                        Default::default(),
                    ),
                );
                g.add_node(
                    999,
                    "MediaOut",
                    kagari_vfx::core::compositor_node_graph::NodeKind::MediaOut,
                );
                let _ = g.connect(1, 2, 0);
                let _ = g.connect(2, 999, 0);
                self.compositor_graph = g;
                self.node_graph_selected_id = Some(2);
                self.node_graph_next_id = 3;
                self.node_graph_status =
                    "DAGリセット完了 (3ノード: MediaIn → ColorWheel → MediaOut)".to_owned();
                true
            }
            "node_graph_move_left" => {
                self.node_graph_swap_selected(false);
                true
            }
            "node_graph_move_right" => {
                self.node_graph_swap_selected(true);
                true
            }
            "node_graph_add_openfx" => {
                let plugin_name = self
                    .openfx_plugins
                    .first()
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "Kagari.Gain".to_owned());
                self.node_graph_insert_before_out(
                    &format!("OFX:{plugin_name}"),
                    kagari_vfx::core::compositor_node_graph::NodeKind::OpenFx {
                        plugin_name,
                        frame: self.current_frame as f64,
                    },
                );
                true
            }
            "export_run_qc" => {
                self.run_export_qc_check();
                true
            }
            "node_graph_add_over_merge" => {
                self.node_graph_insert_before_out(
                    "OverMerge",
                    kagari_vfx::core::compositor_node_graph::NodeKind::OverMerge,
                );
                true
            }
            "node_graph_auto_layout" => {
                self.node_graph_positions.clear();
                self.node_graph_status = format!(
                    "DAGノードを自動整列しました ({}ノード)",
                    self.compositor_graph.nodes.len()
                );
                true
            }
            "toggle_node_graph_preview" => {
                self.node_graph_live_preview = !self.node_graph_live_preview;
                self.preview_revision = self.preview_revision.wrapping_add(1);
                self.preview_cache.clear();
                self.node_graph_status = if self.node_graph_live_preview {
                    "DAGライブ合成モード: ON (ビューポート/QCへリアルタイム適用)".to_owned()
                } else {
                    "DAGライブ合成モード: OFF".to_owned()
                };
                true
            }
            "engine_hub_toggle" => {
                self.engine_hub_expanded = !self.engine_hub_expanded;
                true
            }
            "load_demo_vfx_scene" => {
                self.load_demo_vfx_scene();
                true
            }
            "tutorial_next" => {
                self.tutorial_step = if self.tutorial_step >= 4 {
                    0
                } else {
                    self.tutorial_step + 1
                };
                true
            }
            "tutorial_close" => {
                self.tutorial_step = 0;
                true
            }
            _ => false,
        }
    }

    fn load_demo_vfx_scene(&mut self) {
        self.screen = Screen::Editor;
        self.render_queue = false;
        self.tutorial_step = 1;
        self.add_ae_generator_layer(2, "VFX 背景グラデーション");
        self.add_saber_beam_layer();
        self.add_3d_extruded_shape();
        self.add_tracked_3d_camera();
        #[cfg(feature = "wgpu")]
        self.node_graph_insert_before_out(
            "SapphireGlow",
            kagari_vfx::core::compositor_node_graph::NodeKind::SapphireOptical(Default::default()),
        );
        self.node_graph_evaluate_current_frame();
        self.engine_hub_status =
            "VFX サンプルシーン構築完了 (非破壊グラデ + Saber + 3D押し出し + SfMカメラ + DAG)"
                .to_owned();
    }

    fn run_export_qc_check(&mut self) {
        let (w, h, mut rgba) = self.render_small_rgba_f32();
        if self.node_graph_live_preview {
            let mut inputs = std::collections::HashMap::new();
            if let Ok(fb) =
                kagari_vfx::core::compositor_node_graph::FrameBufferF32::new(w, h, rgba.clone(), None)
            {
                inputs.insert(1u64, fb);
            }
            #[cfg(feature = "wgpu")]
            let eval = self.compositor_graph.evaluate(&inputs, 999, None);
            #[cfg(not(feature = "wgpu"))]
            let eval = self.compositor_graph.evaluate(&inputs, 999);
            if let Ok(out) = eval {
                rgba = out.rgba;
            }
        }
        let bbox = kagari_vfx::core::roi_scanline::compute_nonzero_bbox(&rgba, w, h, 1e-3);
        let picked = kagari_vfx::core::color_scopes::pick_hdr_color_region(
            &rgba,
            w,
            h,
            w / 2,
            h / 2,
            (w / 2).min(h / 2).max(1),
        );
        let luma = picked.map_or(0.0, |p| p.luma_709);
        let black_flag = if bbox.width == 0 || bbox.height == 0 || luma < 0.005 {
            "黒フレーム警告"
        } else {
            "QC合格"
        };
        let msg = format!(
            "{black_flag}: DataWindow={}x{} / 平均Luma={:.3}",
            bbox.width, bbox.height, luma
        );
        self.render_status = msg.clone();
        self.engine_hub_status = msg;
    }

    fn node_graph_swap_selected(&mut self, forward: bool) {
        let Some(target_id) = self.node_graph_selected_id else {
            self.node_graph_status = "移動する中間ノードを選択してください。".to_owned();
            return;
        };
        if target_id == 1 || target_id == 999 {
            self.node_graph_status = "MediaIn / MediaOut は移動できません。".to_owned();
            return;
        }
        let Ok(mut order) = self.compositor_graph.topological_order() else {
            return;
        };
        let Some(pos) = order.iter().position(|&id| id == target_id) else {
            return;
        };
        let swap_idx = if forward {
            if pos + 1 < order.len() && order[pos + 1] != 999 {
                pos + 1
            } else {
                self.node_graph_status = "これ以上右へ移動できません。".to_owned();
                return;
            }
        } else if pos > 0 && order[pos - 1] != 1 {
            pos - 1
        } else {
            self.node_graph_status = "これ以上左へ移動できません。".to_owned();
            return;
        };
        order.swap(pos, swap_idx);
        self.compositor_graph.edges.clear();
        for w in order.windows(2) {
            let _ = self.compositor_graph.connect(w[0], w[1], 0);
        }
        self.node_graph_status = format!("ノード #{target_id} の順序を移動しました");
    }

    fn bake_cleanplate_layer(&mut self) {
        let (w, h, center, max_f) = {
            let comp = self.project_model.active_composition();
            (
                comp.width.clamp(64, 320),
                comp.height.clamp(36, 180),
                [comp.width as f64 * 0.5, comp.height as f64 * 0.5],
                comp.duration_frames.saturating_sub(1),
            )
        };
        let render_f32 = |frame: u32| -> Vec<f32> {
            kagari_vfx::core::software_renderer::render_frame_to_pixels(
                self.project_model.active_composition(),
                frame,
                w,
                h,
                0.0,
                0,
            )
            .into_iter()
            .map(|b| f32::from(b) / 255.0)
            .collect()
        };
        let cur = render_f32(self.current_frame);
        let prev = render_f32(self.current_frame.saturating_sub(2));
        let next = render_f32((self.current_frame + 2).min(max_f));
        let mut center_mask = vec![0.0_f32; (w * h) as usize];
        for y in (h / 4)..(3 * h / 4) {
            for x in (w / 4)..(3 * w / 4) {
                center_mask[(y * w + x) as usize] = 1.0;
            }
        }
        let empty_mask = vec![0.0_f32; (w * h) as usize];
        let Some(clean_f32) = kagari_vfx::core::temporal_cleanplate::remove_object_temporal_f32(
            &cur,
            &center_mask,
            &[(&prev, &empty_mask), (&next, &empty_mask)],
            w,
            h,
        ) else {
            return;
        };
        let rgba8: Vec<u8> = clean_f32
            .into_iter()
            .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
            .collect();
        let Some(img) = image::RgbaImage::from_raw(w, h, rgba8) else {
            return;
        };
        let cache_dir = std::env::temp_dir().join("kagari_cleanplate");
        let _ = std::fs::create_dir_all(&cache_dir);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let out_png = cache_dir.join(format!("clean_{stamp}.png"));
        if img.save(&out_png).is_err() {
            return;
        }
        let source_path = out_png.display().to_string();
        let label = "クリーンプレート".to_owned();
        self.register_project_asset(
            label.clone(),
            kagari_vfx::core::timeline::ProjectItemType::Image {
                path: source_path.clone(),
                width: w,
                height: h,
            },
        );
        self.add_canvas_layer(
            label,
            kagari_vfx::core::timeline::LayerType::Image { path: source_path },
            center,
        );
        self.engine_hub_status = format!("クリーンプレート生成・配置完了 ({w}x{h})");
    }

    fn node_graph_bake_to_layer(&mut self) {
        let (w, h, center) = {
            let comp = self.project_model.active_composition();
            (
                comp.width.clamp(64, 320),
                comp.height.clamp(36, 180),
                [comp.width as f64 * 0.5, comp.height as f64 * 0.5],
            )
        };
        let rgba8_in = kagari_vfx::core::software_renderer::render_frame_to_pixels(
            self.project_model.active_composition(),
            self.current_frame,
            w,
            h,
            0.0,
            0,
        );
        let rgba_f32: Vec<f32> = rgba8_in.into_iter().map(|b| f32::from(b) / 255.0).collect();
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

        let Ok(out) = res else {
            self.node_graph_status = "DAGベイク失敗: 評価エラー".to_owned();
            return;
        };
        let rgba8_out: Vec<u8> = out
            .rgba
            .into_iter()
            .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
            .collect();
        let Some(img) = image::RgbaImage::from_raw(out.width, out.height, rgba8_out) else {
            return;
        };
        let cache_dir = std::env::temp_dir().join("kagari_node_bake");
        let _ = std::fs::create_dir_all(&cache_dir);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let out_png = cache_dir.join(format!("dag_{stamp}.png"));
        if img.save(&out_png).is_err() {
            return;
        }
        let source_path = out_png.display().to_string();
        let label = format!("DAG出力 ({}ノード)", self.compositor_graph.nodes.len());
        self.register_project_asset(
            label.clone(),
            kagari_vfx::core::timeline::ProjectItemType::Image {
                path: source_path.clone(),
                width: out.width,
                height: out.height,
            },
        );
        self.add_canvas_layer(
            label,
            kagari_vfx::core::timeline::LayerType::Image { path: source_path },
            center,
        );
        self.node_graph_status = format!(
            "DAG結果をレイヤーへ書き出し完了: {}x{}",
            out.width, out.height
        );
    }

    fn node_graph_adjust_selected(&mut self, dir: f32) {
        let Some(target_id) = self.node_graph_selected_id else {
            self.node_graph_status = "調整するノードを選択してください。".to_owned();
            return;
        };
        let Some(node) = self.compositor_graph.nodes.get_mut(&target_id) else {
            return;
        };
        let detail = match &mut node.kind {
            kagari_vfx::core::compositor_node_graph::NodeKind::GaussianBlur { radius } => {
                *radius = (*radius + dir * 2.0).clamp(0.5, 64.0);
                format!("radius={radius:.1}px")
            }
            kagari_vfx::core::compositor_node_graph::NodeKind::ColorWheelGrade(p) => {
                for ch in &mut p.gain[..3] {
                    *ch = (*ch + dir * 0.1).clamp(0.2, 4.0);
                }
                format!("gain={:.2}", p.gain[0])
            }
            kagari_vfx::core::compositor_node_graph::NodeKind::ProKeyer(p) => {
                p.gain = (p.gain + dir * 0.1).clamp(0.2, 3.0);
                format!("key_gain={:.2}", p.gain)
            }
            kagari_vfx::core::compositor_node_graph::NodeKind::Transform2D { rotate_deg, .. } => {
                *rotate_deg += dir * 5.0;
                format!("rotate={rotate_deg:.1}°")
            }
            #[cfg(feature = "wgpu")]
            kagari_vfx::core::compositor_node_graph::NodeKind::SapphireOptical(p) => {
                p.glow_intensity = (p.glow_intensity + dir * 0.25).clamp(0.0, 5.0);
                format!("glow={:.2}", p.glow_intensity)
            }
            #[cfg(feature = "wgpu")]
            kagari_vfx::core::compositor_node_graph::NodeKind::SaberBeam { params, .. } => {
                params.glow_intensity = (params.glow_intensity + dir * 0.25).clamp(0.1, 6.0);
                format!("beam_glow={:.2}", params.glow_intensity)
            }
            kagari_vfx::core::compositor_node_graph::NodeKind::DeepMerge { z_epsilon } => {
                *z_epsilon = (*z_epsilon + dir * 0.005).clamp(0.001, 0.5);
                format!("z_eps={z_epsilon:.3}")
            }
            _ => "固定IOノード".to_owned(),
        };
        self.node_graph_status = format!("ノード #{target_id} ({}) 更新: {detail}", node.label);
    }

    fn add_mesh_warp_effect(&mut self) {
        let Some(layer_id) = self.selected_layer_id.clone() else {
            self.engine_hub_status = "MeshWarp を適用するレイヤーを選択してください。".to_owned();
            return;
        };
        let id = format!(
            "ameui_effect_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        );
        if let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|l| l.id == layer_id && !l.locked)
        {
            layer.effects.push(kagari_vfx::core::timeline::Effect {
                id: id.clone(),
                name: "Mesh Warp".to_owned(),
                effect_type: kagari_vfx::core::timeline::EffectType::MeshWarp {
                    top_left: kagari_vfx::core::timeline::Animatable::new_constant([0.0, 0.0]),
                    top_right: kagari_vfx::core::timeline::Animatable::new_constant([1.0, 0.0]),
                    bottom_left: kagari_vfx::core::timeline::Animatable::new_constant([0.0, 1.0]),
                    bottom_right: kagari_vfx::core::timeline::Animatable::new_constant([1.0, 1.0]),
                },
                enabled: true,
            });
            self.selected_effect_id = Some(id);
            self.mark_project_changed();
            self.engine_hub_status = "Mesh Warp (4隅ワープ) をレイヤーに追加しました".to_owned();
        }
    }

    fn apply_wiggle_keyframes(&mut self) {
        let Some(layer_id) = self.selected_layer_id.clone() else {
            self.engine_hub_status = "Wiggle を適用するレイヤーを選択してください。".to_owned();
            return;
        };
        let f0 = self.current_frame;
        let mut applied = false;
        if let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|l| l.id == layer_id && !l.locked)
        {
            let base_pos = layer.transform.position.value_at(f0);
            let base_rot = layer.transform.rotation.value_at(f0);
            for step in 0..=6u32 {
                let frame = f0 + step * 5;
                let t = frame as f32 / 30.0;
                let wig = kagari_vfx::core::ae_shape_modifiers::evaluate_wiggle_transform(
                    0,
                    t,
                    &Default::default(),
                );
                layer.transform.position.add_keyframe(
                    kagari_vfx::core::keyframe::Keyframe::new(
                        frame,
                        [
                            base_pos[0] + wig.position[0],
                            base_pos[1] + wig.position[1],
                        ],
                        kagari_vfx::core::keyframe::InterpolationType::Linear,
                    ),
                );
                layer.transform.rotation.add_keyframe(
                    kagari_vfx::core::keyframe::Keyframe::new(
                        frame,
                        base_rot + wig.rotation_deg,
                        kagari_vfx::core::keyframe::InterpolationType::Linear,
                    ),
                );
            }
            applied = true;
        }
        if applied {
            self.mark_project_changed();
            self.engine_hub_status =
                "Wiggle 揺れキーフレーム (7点: position + rotation) を適用しました".to_owned();
        } else {
            self.engine_hub_status =
                "選択レイヤーがロックされているか見つかりません。".to_owned();
        }
    }

    fn add_tracked_3d_camera(&mut self) {
        let intr = kagari_vfx::core::sfm_camera_tracker::CameraIntrinsics {
            focal_px: 1000.0,
            cx: 960.0,
            cy: 540.0,
        };
        let pose_a = kagari_vfx::core::sfm_camera_tracker::CameraPose3D {
            position: [0.0, 0.0, 0.0],
            rotation_deg: [0.0, 0.0, 0.0],
        };
        let pose_b = kagari_vfx::core::sfm_camera_tracker::CameraPose3D {
            position: [120.0, -15.0, -40.0],
            rotation_deg: [1.5, -5.0, 0.0],
        };
        let pts = [
            [-100.0, -50.0, 800.0],
            [120.0, 60.0, 900.0],
            [0.0, 100.0, 750.0],
            [80.0, -90.0, 850.0],
        ];
        let corr: Vec<_> = pts
            .iter()
            .filter_map(|&wpt| {
                let pa = pose_a.project(wpt, &intr)?;
                let pb = pose_b.project(wpt, &intr)?;
                let tri = kagari_vfx::core::sfm_camera_tracker::triangulate_two_view(
                    pa, &pose_a, pb, &pose_b, &intr,
                )?;
                Some((tri, pb))
            })
            .collect();
        let (solved, rmse) = kagari_vfx::core::sfm_camera_tracker::refine_camera_pose(
            &corr,
            kagari_vfx::core::sfm_camera_tracker::CameraPose3D {
                position: [108.0, -10.0, -32.0],
                rotation_deg: [1.0, -4.0, 0.0],
            },
            &intr,
        );
        self.add_scene_camera();
        if let Some(cam_layer_id) = self.selected_layer_id.clone() {
            if let Some(layer) = self
                .project_model
                .active_composition_mut()
                .layers
                .iter_mut()
                .find(|l| l.id == cam_layer_id)
            {
                layer.name = "SfM トラックカメラ".to_owned();
                layer.transform_3d.position.add_keyframe(
                    kagari_vfx::core::keyframe::Keyframe::new(
                        0,
                        [960.0, 540.0, -1000.0],
                        kagari_vfx::core::keyframe::InterpolationType::Linear,
                    ),
                );
                layer.transform_3d.position.add_keyframe(
                    kagari_vfx::core::keyframe::Keyframe::new(
                        30,
                        [
                            960.0 + solved.position[0],
                            540.0 + solved.position[1],
                            -1000.0 + solved.position[2],
                        ],
                        kagari_vfx::core::keyframe::InterpolationType::Linear,
                    ),
                );
                layer.transform_3d.rotation.add_keyframe(
                    kagari_vfx::core::keyframe::Keyframe::new(
                        0,
                        [0.0, 0.0, 0.0],
                        kagari_vfx::core::keyframe::InterpolationType::Linear,
                    ),
                );
                layer.transform_3d.rotation.add_keyframe(
                    kagari_vfx::core::keyframe::Keyframe::new(
                        30,
                        solved.rotation_deg,
                        kagari_vfx::core::keyframe::InterpolationType::Linear,
                    ),
                );
            }
            self.mark_project_changed();
        }
        self.engine_hub_status = format!(
            "SfM 3Dカメラ生成 & キーフレーム焼き込み完了 (RMSE={rmse:.2}px)"
        );
    }

    fn add_3d_extruded_shape(&mut self) {
        use kagari_vfx::core::timeline::{Animatable, LayerType, ShapeFillType, ShapeType};
        let center = {
            let comp = self.project_model.active_composition();
            [comp.width as f64 * 0.5, comp.height as f64 * 0.5]
        };
        self.add_canvas_layer(
            "3D押し出しシェイプ".to_owned(),
            LayerType::Shape {
                shape_type: ShapeType::Rectangle {
                    width: Animatable::new_constant(180.0),
                    height: Animatable::new_constant(180.0),
                    corner_radius: Animatable::new_constant(16.0),
                },
                color: [0.22, 0.68, 0.98, 1.0],
                stroke_color: [1.0, 1.0, 1.0, 1.0],
                stroke_width: 2.0,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 36.0,
                bevel_depth: 4.0,
            },
            center,
        );
        if let Some(layer_id) = self.selected_layer_id.clone() {
            if let Some(layer) = self
                .project_model
                .active_composition_mut()
                .layers
                .iter_mut()
                .find(|l| l.id == layer_id)
            {
                layer.is_3d = true;
                layer.transform_3d.rotation = Animatable::new_constant([15.0, 25.0, 0.0]);
            }
            self.mark_project_changed();
        }
        self.engine_hub_status = "3D押し出しベベルシェイプ作成 (depth=36, bevel=4)".to_owned();
    }

    fn apply_matte_choker(&mut self) {
        let Some(layer_id) = self.selected_layer_id.clone() else {
            self.engine_hub_status = "Matte Choker を適用するレイヤーを選択してください。".to_owned();
            return;
        };
        let (w, h, rgba) = self.render_small_rgba_f32();
        let matte: Vec<f32> = rgba.chunks_exact(4).map(|px| px[3]).collect();
        let refined = kagari_vfx::core::roto_refine::refine_roto_matte(
            &rgba,
            &matte,
            w,
            h,
            &Default::default(),
        );
        let id = format!(
            "ameui_effect_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        );
        if let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|l| l.id == layer_id && !l.locked)
        {
            layer.effects.push(kagari_vfx::core::timeline::Effect {
                id: id.clone(),
                name: "Matte Choker".to_owned(),
                effect_type: kagari_vfx::core::timeline::EffectType::MatteChoker {
                    choke_amount: kagari_vfx::core::timeline::Animatable::new_constant(2.0),
                    gray_level: kagari_vfx::core::timeline::Animatable::new_constant(0.5),
                },
                enabled: true,
            });
            self.selected_effect_id = Some(id);
            self.mark_project_changed();
        }
        self.engine_hub_status = format!(
            "Matte Choker 適用 & Rotoリファイン ({}px)",
            refined.map_or(0, |(_, a)| a.len())
        );
    }

    fn add_saber_beam_layer(&mut self) {
        use kagari_vfx::core::timeline::{
            Animatable, Effect, EffectType, LayerType, ShapeFillType, ShapeType,
        };
        let center = {
            let comp = self.project_model.active_composition();
            [comp.width as f64 * 0.5, comp.height as f64 * 0.5]
        };
        self.add_canvas_layer(
            "Saber ビーム (非破壊)".to_owned(),
            LayerType::Shape {
                shape_type: ShapeType::Rectangle {
                    width: Animatable::new_constant(640.0),
                    height: Animatable::new_constant(12.0),
                    corner_radius: Animatable::new_constant(6.0),
                },
                color: [0.65, 0.96, 1.0, 1.0],
                stroke_color: [0.15, 0.82, 1.0, 1.0],
                stroke_width: 2.0,
                fill_type: ShapeFillType::Solid,
                extrusion_depth: 0.0,
                bevel_depth: 0.0,
            },
            center,
        );
        if let Some(layer_id) = self.selected_layer_id.clone() {
            if let Some(layer) = self
                .project_model
                .active_composition_mut()
                .layers
                .iter_mut()
                .find(|l| l.id == layer_id)
            {
                layer.transform.rotation = Animatable::new_constant(-18.0);
                layer.effects.push(Effect {
                    id: "saber_glow_pro".to_owned(),
                    name: "Glow Pro (Saber Core)".to_owned(),
                    effect_type: EffectType::GlowPro {
                        threshold: Animatable::new_constant(10.0),
                        radius: Animatable::new_constant(32.0),
                        intensity: Animatable::new_constant(140.0),
                    },
                    enabled: true,
                });
                layer.effects.push(Effect {
                    id: "saber_turb".to_owned(),
                    name: "Energy Flicker".to_owned(),
                    effect_type: EffectType::TurbulentDisplace {
                        amount: Animatable::new_constant(8.0),
                        size: Animatable::new_constant(28.0),
                        evolution: Animatable::new_constant(0.0),
                        complexity: Animatable::new_constant(2.0),
                    },
                    enabled: true,
                });
            }
            self.mark_project_changed();
        }
        self.engine_hub_status =
            "非破壊 Saber ビーム生成 (Shape + GlowPro + TurbulentDisplace)".to_owned();
    }

    fn mograph_clone_selected_layer(&mut self) {
        let Some(selected_id) = self.selected_layer_id.clone() else {
            self.engine_hub_status = "複製元のレイヤーを選択してください。".to_owned();
            return;
        };
        let clones = kagari_vfx::core::mograph_cloner::evaluate_cloner(
            &kagari_vfx::core::mograph_cloner::ClonerMode::Radial {
                count: 6,
                radius: 160.0,
                plane: kagari_vfx::core::mograph_cloner::RadialPlane::XY,
                start_angle_deg: 0.0,
                end_angle_deg: 360.0,
                align_to_circle: true,
            },
            &[kagari_vfx::core::mograph_cloner::Effector::Step {
                strength: 1.0,
                scale_end: [1.3, 1.3, 1.3],
                rotation_end_deg: [0.0, 0.0, 30.0],
            }],
        );
        let base_layer = {
            let comp = self.project_model.active_composition();
            comp.layers.iter().find(|l| l.id == selected_id).cloned()
        };
        let Some(base) = base_layer else {
            return;
        };
        if base.locked {
            self.engine_hub_status = "ロックされたレイヤーは MoGraph 複製できません。".to_owned();
            return;
        }
        let base_pos = base.transform.position.value_at(self.current_frame);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let mut created = 0usize;
        {
            let comp = self.project_model.active_composition_mut();
            for (idx, tr) in clones.into_iter().enumerate() {
                let mut cloned = base.clone();
                cloned.id = format!("mograph_clone_{stamp}_{idx}");
                cloned.name = format!("{} [Clone {}]", base.name, idx + 1);
                cloned.transform.position = kagari_vfx::core::timeline::Animatable::new_constant([
                    base_pos[0] + tr.position[0],
                    base_pos[1] + tr.position[1],
                ]);
                cloned.transform.scale = kagari_vfx::core::timeline::Animatable::new_constant([
                    100.0 * tr.scale[0],
                    100.0 * tr.scale[1],
                ]);
                cloned.transform.rotation =
                    kagari_vfx::core::timeline::Animatable::new_constant(tr.rotation_deg[2]);
                comp.add_layer(cloned);
                created += 1;
            }
        }
        self.mark_project_changed();
        self.engine_hub_status = format!("MoGraph クローン生成: {created} レイヤー配置");
    }

    fn add_ae_generator_layer(&mut self, gen_idx: u8, label: &str) {
        use kagari_vfx::core::timeline::{Animatable, Effect, EffectType, LayerType};
        if gen_idx == 2 || gen_idx == 3 {
            let center = {
                let comp = self.project_model.active_composition();
                [comp.width as f64 * 0.5, comp.height as f64 * 0.5]
            };
            self.add_canvas_layer(
                label.to_owned(),
                LayerType::Solid {
                    color: [0.12, 0.16, 0.24, 1.0],
                },
                center,
            );
            if let Some(layer_id) = self.selected_layer_id.clone() {
                if let Some(layer) = self
                    .project_model
                    .active_composition_mut()
                    .layers
                    .iter_mut()
                    .find(|l| l.id == layer_id)
                {
                    if gen_idx == 2 {
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
                    } else {
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
                }
                self.mark_project_changed();
            }
            self.engine_hub_status = format!("非破壊ジェネレーター追加: {label} (スライダー/キーフレーム対応)");
            return;
        }
        let (w, h, center) = {
            let comp = self.project_model.active_composition();
            (
                comp.width.clamp(128, 640),
                comp.height.clamp(72, 360),
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

    fn run_time_remap_optical_flow(&mut self) {
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

    fn node_graph_insert_before_out(
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

    fn node_graph_remove_selected(&mut self) {
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

    fn node_graph_evaluate_current_frame(&mut self) {
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

    fn node_graph_layout_rects(
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

#[cfg(test)]
mod engine_hub_tests {
    use super::*;

    #[test]
    fn engine_hub_actions_exercise_all_subsystems() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        for act in [
            "engine_analyze_color_scopes",
            "engine_analyze_film_grain",
            "engine_run_pro_keyer",
            "engine_run_deep_and_node_graph",
            "engine_run_roto_refine",
            "engine_run_mograph_cloner",
            "engine_run_sfm_and_lens",
            "engine_run_temporal_cleanplate",
            "engine_apply_composer_preset",
            "engine_apply_shape_pucker_twist",
            "engine_auto_proxy_step",
            "engine_run_cryptomatte",
            "engine_run_aces_check",
        ] {
            app.handle_action(HostAction {
                name: act.to_owned(),
                value: None,
                target: None,
            })
            .unwrap();
            assert!(
                !app.engine_hub_status.is_empty()
                    && app.engine_hub_status != "エンジン統合ハブ待機中",
                "action {act} did not update engine_hub_status"
            );
        }
    }

    #[test]
    fn node_graph_add_evaluate_and_remove_works() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        let base_nodes = app.compositor_graph.nodes.len();
        app.handle_action(HostAction {
            name: "node_graph_add_blur".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(app.compositor_graph.nodes.len(), base_nodes + 1);
        app.handle_action(HostAction {
            name: "node_graph_evaluate".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.node_graph_status.contains("DAG評価完了"));
        app.handle_action(HostAction {
            name: "node_graph_remove_selected".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(app.compositor_graph.nodes.len(), base_nodes);
    }

    #[test]
    fn generator_layers_and_optical_flow_time_remap_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        let base_layers = app.project_model.active_composition().layers.len();
        for act in [
            "add_generator_smpte",
            "add_generator_grid",
            "add_generator_gradient",
            "add_generator_cells",
        ] {
            app.handle_action(HostAction {
                name: act.to_owned(),
                value: None,
                target: None,
            })
            .unwrap();
        }
        assert_eq!(
            app.project_model.active_composition().layers.len(),
            base_layers + 4
        );
        app.handle_action(HostAction {
            name: "time_remap_optical_flow".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.engine_hub_status.contains("Optical Flow"));
    }

    #[test]
    fn saber_beam_and_mograph_clone_layers_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        let base_layers = app.project_model.active_composition().layers.len();
        app.handle_action(HostAction {
            name: "add_saber_beam_layer".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(
            app.project_model.active_composition().layers.len(),
            base_layers + 1
        );
        app.handle_action(HostAction {
            name: "mograph_clone_selected_layer".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(
            app.project_model.active_composition().layers.len(),
            base_layers + 7
        );
    }

    #[test]
    fn extruded_3d_shape_and_matte_choker_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        app.handle_action(HostAction {
            name: "add_3d_extruded_shape".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let lid = app.selected_layer_id.clone().unwrap();
        let layer = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == lid)
            .unwrap();
        assert!(layer.is_3d);
        let eff_before = layer.effects.len();
        app.handle_action(HostAction {
            name: "apply_matte_choker".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let layer_after = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == lid)
            .unwrap();
        assert_eq!(layer_after.effects.len(), eff_before + 1);
    }

    #[test]
    fn tracked_3d_camera_and_sapphire_saber_nodes_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        let base_cams = app.project_model.active_composition().cameras.len();
        app.handle_action(HostAction {
            name: "add_tracked_3d_camera".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(
            app.project_model.active_composition().cameras.len(),
            base_cams + 1
        );
        let cam_id = app.selected_layer_id.clone().unwrap();
        let cam_layer = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == cam_id)
            .unwrap();
        assert_eq!(cam_layer.name, "SfM トラックカメラ");
        assert!(cam_layer.transform_3d.position.keyframes().is_some());

        let base_nodes = app.compositor_graph.nodes.len();
        app.handle_action(HostAction {
            name: "node_graph_add_sapphire".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        app.handle_action(HostAction {
            name: "node_graph_add_saber".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        #[cfg(feature = "wgpu")]
        assert_eq!(app.compositor_graph.nodes.len(), base_nodes + 2);
    }

    #[test]
    fn studio_rig_deep_merge_and_wiggle_keyframes_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        let base_layers = app.project_model.active_composition().layers.len();
        app.handle_action(HostAction {
            name: "add_3d_studio_rig".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.project_model.active_composition().layers.len() >= base_layers + 3);

        app.handle_action(HostAction {
            name: "add_solid_layer".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        app.handle_action(HostAction {
            name: "apply_wiggle_keyframes".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let lid = app.selected_layer_id.clone().unwrap();
        let layer = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == lid)
            .unwrap();
        assert!(layer.transform.position.keyframes().is_some());
        assert!(layer.transform.rotation.keyframes().is_some());

        let base_nodes = app.compositor_graph.nodes.len();
        app.handle_action(HostAction {
            name: "node_graph_add_deep_merge".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(app.compositor_graph.nodes.len(), base_nodes + 1);
    }

    #[test]
    fn node_graph_param_tuning_and_mesh_warp_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        app.handle_action(HostAction {
            name: "node_graph_add_blur".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        app.handle_action(HostAction {
            name: "node_graph_param_up".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.node_graph_status.contains("radius=6.0px"));

        app.handle_action(HostAction {
            name: "add_solid_layer".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let lid = app.selected_layer_id.clone().unwrap();
        let before = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == lid)
            .unwrap()
            .effects
            .len();
        app.handle_action(HostAction {
            name: "add_mesh_warp".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let after = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == lid)
            .unwrap()
            .effects
            .len();
        assert_eq!(after, before + 1);
    }

    #[test]
    fn node_graph_bake_and_extra_effects_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        let base_layers = app.project_model.active_composition().layers.len();
        app.handle_action(HostAction {
            name: "node_graph_bake_to_layer".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(
            app.project_model.active_composition().layers.len(),
            base_layers + 1
        );
        assert!(app.node_graph_status.contains("書き出し完了"));

        for act in ["add_displacement_map", "add_compound_blur"] {
            app.handle_action(HostAction {
                name: act.to_owned(),
                value: None,
                target: None,
            })
            .unwrap();
        }
        let lid = app.selected_layer_id.clone().unwrap();
        let eff_len = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == lid)
            .unwrap()
            .effects
            .len();
        assert!(eff_len >= 2);
    }

    #[test]
    fn cleanplate_bake_and_eight_effects_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        let base_layers = app.project_model.active_composition().layers.len();
        app.handle_action(HostAction {
            name: "bake_cleanplate_layer".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(
            app.project_model.active_composition().layers.len(),
            base_layers + 1
        );
        for act in [
            "add_crt_scanlines",
            "add_polar_coordinates",
            "add_wave_warp",
            "add_mosaic",
            "add_emboss",
            "add_bevel_alpha",
            "add_median_filter",
            "add_sobel_edges",
        ] {
            app.handle_action(HostAction {
                name: act.to_owned(),
                value: None,
                target: None,
            })
            .unwrap();
        }
        let lid = app.selected_layer_id.clone().unwrap();
        let eff_len = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == lid)
            .unwrap()
            .effects
            .len();
        assert_eq!(eff_len, 8);
    }

    #[test]
    fn effect_category_tabs_switch_cleanly() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        for (act, expected) in [
            ("effect_cat_color", "color"),
            ("effect_cat_blur", "blur"),
            ("effect_cat_distort", "distort"),
            ("effect_cat_key", "key"),
            ("effect_cat_all", "all"),
        ] {
            app.handle_action(HostAction {
                name: act.to_owned(),
                value: None,
                target: None,
            })
            .unwrap();
            assert_eq!(app.effect_category, expected);
        }
        app.handle_action(HostAction {
            name: "node_graph_add_blur".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(app.compositor_graph.nodes.len(), 4);
        app.handle_action(HostAction {
            name: "node_graph_reset".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(app.compositor_graph.nodes.len(), 3);
    }

    #[test]
    fn node_graph_reorder_works() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        app.handle_action(HostAction {
            name: "node_graph_add_blur".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let before = app.compositor_graph.topological_order().unwrap();
        assert_eq!(before, vec![1, 2, 3, 999]);
        app.handle_action(HostAction {
            name: "node_graph_move_left".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let after = app.compositor_graph.topological_order().unwrap();
        assert_eq!(after, vec![1, 3, 2, 999]);
        app.handle_action(HostAction {
            name: "node_graph_move_right".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let back = app.compositor_graph.topological_order().unwrap();
        assert_eq!(back, vec![1, 2, 3, 999]);
    }

    #[test]
    fn node_graph_openfx_node_works() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        let base_nodes = app.compositor_graph.nodes.len();
        app.handle_action(HostAction {
            name: "node_graph_add_openfx".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(app.compositor_graph.nodes.len(), base_nodes + 1);
        assert!(app.node_graph_status.contains("OFX:"));
    }

    #[test]
    fn export_qc_and_corner_pin_colorama_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        app.handle_action(HostAction {
            name: "add_solid_layer".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let lid = app.selected_layer_id.clone().unwrap();
        app.handle_action(HostAction {
            name: "add_corner_pin".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        app.handle_action(HostAction {
            name: "add_colorama".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        let eff_len = app
            .project_model
            .active_composition()
            .layers
            .iter()
            .find(|l| l.id == lid)
            .unwrap()
            .effects
            .len();
        assert_eq!(eff_len, 2);

        app.handle_action(HostAction {
            name: "export_run_qc".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.render_status.contains("DataWindow="));
    }

    #[test]
    fn node_graph_drag_wire_over_merge_and_live_preview_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        let make_event = |phase, x, y| ame_ui::CanvasPointerEvent {
            phase,
            kind: ame_ui::CanvasPointerKind::Mouse,
            pointer_id: None,
            click_count: 1,
            x,
            y,
            button: Some(ame_ui::event::PointerButton::Primary),
            modifiers: ame_ui::event::KeyModifiers::default(),
            pressure: None,
            tilt: None,
            coalesced_samples: Vec::new(),
            space_pressed: false,
        };
        app.node_graph_bounds = Some((0.0, 0.0, 600.0, 120.0));
        let rects = app.node_graph_layout_rects(0.0, 0.0, 600.0, 120.0);
        let (id1, r1) = rects[0];
        let (id2, r2) = rects[1];
        // Drag node 2 freely
        assert!(app.handle_node_graph_canvas_pointer(make_event(
            CanvasPointerPhase::Down,
            r2.x0 + 20.0,
            r2.y0 + 20.0,
        )));
        assert!(app.handle_node_graph_canvas_pointer(make_event(
            CanvasPointerPhase::Moved,
            260.0,
            70.0,
        )));
        assert!(app.handle_node_graph_canvas_pointer(make_event(
            CanvasPointerPhase::Up,
            260.0,
            70.0,
        )));
        assert!(app.node_graph_positions.contains_key(&id2));

        // Wire drag from node 1 output to node 2 input
        let cy1 = (r1.y0 + r1.y1) * 0.5;
        assert!(app.handle_node_graph_canvas_pointer(make_event(
            CanvasPointerPhase::Down,
            r1.x1,
            cy1,
        )));
        assert!(app.node_graph_drag_wire.is_some());
        assert!(app.handle_node_graph_canvas_pointer(make_event(
            CanvasPointerPhase::Up,
            245.0,
            60.0,
        )));
        assert!(app.node_graph_status.contains("結線") || app.node_graph_status.contains("接続"));

        // Auto layout, OverMerge, and Live Preview toggle
        app.handle_action(HostAction {
            name: "node_graph_auto_layout".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.node_graph_positions.is_empty());
        app.handle_action(HostAction {
            name: "node_graph_add_over_merge".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        app.handle_action(HostAction {
            name: "toggle_node_graph_preview".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.node_graph_live_preview);
        app.handle_action(HostAction {
            name: "export_run_qc".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.render_status.contains("DataWindow="));
        let _ = id1;
    }

    #[test]
    fn demo_vfx_scene_and_hub_toggle_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        assert!(!app.engine_hub_expanded);
        app.handle_action(HostAction {
            name: "engine_hub_toggle".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert!(app.engine_hub_expanded);

        let base_layers = app.project_model.active_composition().layers.len();
        app.handle_action(HostAction {
            name: "load_demo_vfx_scene".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(app.screen, Screen::Editor);
        assert!(app.project_model.active_composition().layers.len() >= base_layers + 4);
        assert!(app.engine_hub_status.contains("VFX サンプルシーン構築完了"));
    }

    #[test]
    fn corner_pin_viewport_gizmo_drag_and_tutorial_work() {
        let mut app = AfterEffectsStudio::new().expect("studio app");
        app.handle_action(HostAction {
            name: "load_demo_vfx_scene".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        assert_eq!(app.tutorial_step, 1);
        for expected in [2usize, 3, 4, 0] {
            app.handle_action(HostAction {
                name: "tutorial_next".to_owned(),
                value: None,
                target: None,
            })
            .unwrap();
            assert_eq!(app.tutorial_step, expected);
        }

        app.handle_action(HostAction {
            name: "add_solid_layer".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        app.handle_action(HostAction {
            name: "add_corner_pin".to_owned(),
            value: None,
            target: None,
        })
        .unwrap();
        app.viewport_mapping = Some((0.0, 0.0, 1.0, 1.0));
        let make_event = |phase, x, y| ame_ui::CanvasPointerEvent {
            phase,
            kind: ame_ui::CanvasPointerKind::Mouse,
            pointer_id: None,
            click_count: 1,
            x,
            y,
            button: Some(ame_ui::event::PointerButton::Primary),
            modifiers: ame_ui::event::KeyModifiers::default(),
            pressure: None,
            tilt: None,
            coalesced_samples: Vec::new(),
            space_pressed: false,
        };
        // Grab top-left corner at (0, 0) and drag to (192, 108)
        assert!(app.handle_viewport_canvas_pointer(make_event(
            CanvasPointerPhase::Down,
            0.0,
            0.0,
        )));
        assert_eq!(app.corner_pin_drag, Some(0));
        assert!(app.handle_viewport_canvas_pointer(make_event(
            CanvasPointerPhase::Moved,
            192.0,
            108.0,
        )));
        assert!(app.handle_viewport_canvas_pointer(make_event(
            CanvasPointerPhase::Up,
            192.0,
            108.0,
        )));
        assert_eq!(app.corner_pin_drag, None);

        // Live compositor viewport helper check
        let sample = vec![128u8; 8 * 8 * 4];
        let out = app.evaluate_compositor_graph_rgba8(8, 8, sample.clone());
        assert_eq!(out.len(), sample.len());
    }
}
