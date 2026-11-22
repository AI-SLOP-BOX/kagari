use super::*;

impl AfterEffectsStudio {
    pub(super) fn load_demo_vfx_scene(&mut self) {
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

    pub(super) fn run_export_qc_check(&mut self) {
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

    pub(super) fn node_graph_swap_selected(&mut self, forward: bool) {
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

    pub(super) fn bake_cleanplate_layer(&mut self) {
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

    pub(super) fn node_graph_bake_to_layer(&mut self) {
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

    pub(super) fn node_graph_adjust_selected(&mut self, dir: f32) {
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

    pub(super) fn add_mesh_warp_effect(&mut self) {
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

    pub(super) fn apply_wiggle_keyframes(&mut self) {
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

    pub(super) fn add_tracked_3d_camera(&mut self) {
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

    pub(super) fn add_3d_extruded_shape(&mut self) {
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

    pub(super) fn apply_matte_choker(&mut self) {
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

    pub(super) fn add_saber_beam_layer(&mut self) {
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

    pub(super) fn mograph_clone_selected_layer(&mut self) {
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
}
