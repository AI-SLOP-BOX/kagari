use super::*;

mod actions_scene;
mod generators_and_graph;
#[cfg(test)]
mod tests;


impl AfterEffectsStudio {
    pub(super) fn render_small_rgba_f32(&self) -> (u32, u32, Vec<f32>) {
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
}
