use super::*;

impl AfterEffectsStudio {
    pub(crate) fn sync_media_layer_bindings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let selected_layer = self.selected_layer_id.as_deref().and_then(|id| {
            self.project_model
                .active_composition()
                .layers
                .iter()
                .find(|layer| layer.id == id)
        });

        let posterize_time = selected_layer.and_then(|layer| {
            layer
                .posterize_time
                .map(|settings| (settings.enabled, settings.target_fps))
        });
        self.ui.set_binding(
            "video_posterize_present",
            Value::Bool(posterize_time.is_some()),
        )?;
        self.ui.set_binding(
            "video_posterize_enabled",
            Value::Bool(posterize_time.is_some_and(|settings| settings.0)),
        )?;
        self.ui.set_binding(
            "video_posterize_fps",
            Value::String(
                posterize_time
                    .map(|settings| format!("{:.2}", settings.1))
                    .unwrap_or_else(|| "12".to_owned()),
            ),
        )?;
        let layer_proxy = selected_layer.map(|layer| {
            use kagari_vfx::core::proxy::ProxyResolution;
            let resolution_label = match layer.proxy.resolution {
                ProxyResolution::Full => "Full",
                ProxyResolution::Half => "1/2",
                ProxyResolution::Quarter => "1/4",
                ProxyResolution::Eighth => "1/8",
            };
            let path_resolution_label = match layer.proxy.path_resolution {
                Some(ProxyResolution::Full) => "Full",
                Some(ProxyResolution::Half) => "1/2",
                Some(ProxyResolution::Quarter) => "1/4",
                Some(ProxyResolution::Eighth) => "1/8",
                None => "不明",
            };
            let can_generate = matches!(
                &layer.layer_type,
                kagari_vfx::core::timeline::LayerType::Image { .. }
                    | kagari_vfx::core::timeline::LayerType::Video { .. }
            ) && layer.proxy.resolution != ProxyResolution::Full;
            let has_file = layer.proxy.proxy_path.as_deref().is_some_and(|path| {
                let path = std::path::Path::new(path);
                path.is_file() || path.is_dir()
            });
            (
                layer.proxy.enabled && has_file,
                resolution_label,
                can_generate,
                has_file,
                layer.proxy.proxy_path.is_some(),
                path_resolution_label,
            )
        });
        self.ui.set_binding(
            "layer_proxy_enabled",
            Value::Bool(layer_proxy.is_some_and(|proxy| proxy.0)),
        )?;
        self.ui.set_binding(
            "layer_proxy_resolution_label",
            Value::String(
                layer_proxy
                    .map(|proxy| proxy.1.to_owned())
                    .unwrap_or_else(|| "Full".to_owned()),
            ),
        )?;
        self.ui.set_binding(
            "layer_proxy_path_resolution_label",
            Value::String(
                layer_proxy
                    .map(|proxy| proxy.5.to_owned())
                    .unwrap_or_else(|| "不明".to_owned()),
            ),
        )?;
        self.ui.set_binding(
            "layer_proxy_can_generate",
            Value::Bool(layer_proxy.is_some_and(|proxy| proxy.2)),
        )?;
        self.ui.set_binding(
            "layer_proxy_has_file",
            Value::Bool(layer_proxy.is_some_and(|proxy| proxy.3)),
        )?;
        self.ui.set_binding(
            "layer_proxy_path_set",
            Value::Bool(layer_proxy.is_some_and(|proxy| proxy.4)),
        )?;
        self.ui.set_binding(
            "proxy_generation_active",
            Value::Bool(self.proxy_generation.is_some()),
        )?;
        self.ui
            .set_binding("proxy_status", Value::String(self.proxy_status.clone()))?;
        self.ui
            .set_binding("proxy_progress", Value::Int(self.proxy_progress))?;
        let audio_volume_track = selected_layer.and_then(|layer| match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Audio { volume, .. } => Some(volume),
            kagari_vfx::core::timeline::LayerType::Video {
                audio_wav: Some(_),
                audio_volume,
                ..
            } => Some(audio_volume),
            _ => None,
        });
        let audio_volume = audio_volume_track.map(|track| track.evaluate(self.current_frame));
        let audio_volume_keyframe_here = audio_volume_track.is_some_and(|track| {
            track.keyframes().is_some_and(|keyframes| {
                keyframes.iter().any(|key| key.frame == self.current_frame)
            })
        });
        self.ui
            .set_binding("selected_is_audio", Value::Bool(audio_volume.is_some()))?;
        self.ui.set_binding(
            "audio_volume_keyframe_here",
            Value::Bool(audio_volume_keyframe_here),
        )?;
        self.ui.set_binding(
            "audio_volume",
            Value::String(
                audio_volume
                    .map(|value| format!("{value:.2}"))
                    .unwrap_or_else(|| "0.0".to_owned()),
            ),
        )?;
        let transform_3d = selected_layer.filter(|layer| layer.is_3d).map(|layer| {
            (
                layer.transform_3d.position.evaluate(self.current_frame),
                layer.transform_3d.rotation.evaluate(self.current_frame),
                layer.transform_3d.scale.evaluate(self.current_frame),
            )
        });
        self.ui
            .set_binding("selected_is_3d", Value::Bool(transform_3d.is_some()))?;
        let selected_scene_ref = selected_layer.and_then(|layer| layer.scene_object.as_ref());
        let camera = selected_scene_ref.and_then(|scene_ref| match scene_ref {
            kagari_vfx::core::timeline::SceneObjectRef::Camera { id } => self
                .project_model
                .active_composition()
                .cameras
                .iter()
                .find(|camera| camera.id == *id),
            _ => None,
        });
        let light = selected_scene_ref.and_then(|scene_ref| match scene_ref {
            kagari_vfx::core::timeline::SceneObjectRef::Light { id } => self
                .project_model
                .active_composition()
                .lights
                .iter()
                .find(|light| light.id == *id),
            _ => None,
        });
        self.ui
            .set_binding("selected_is_camera", Value::Bool(camera.is_some()))?;
        self.ui.set_binding(
            "selected_is_active_camera",
            Value::Bool(camera.is_some_and(|camera| camera.active)),
        )?;
        self.ui.set_binding(
            "selected_camera_fov",
            Value::Float(
                camera
                    .map(|camera| f64::from(camera.fov_at(self.current_frame)))
                    .unwrap_or(50.0),
            ),
        )?;
        self.ui.set_binding(
            "selected_camera_focus",
            Value::Float(
                camera
                    .map(|camera| f64::from(camera.focus_distance_at(self.current_frame)))
                    .unwrap_or(1000.0),
            ),
        )?;
        self.ui.set_binding(
            "selected_camera_aperture",
            Value::Float(
                camera
                    .map(|camera| f64::from(camera.aperture_at(self.current_frame)))
                    .unwrap_or(2.8),
            ),
        )?;
        self.ui.set_binding(
            "selected_camera_dof_enabled",
            Value::Bool(camera.is_some_and(|camera| camera.dof_enabled_at(self.current_frame))),
        )?;
        self.ui.set_binding(
            "selected_camera_dof_max_blur",
            Value::Float(
                camera
                    .map(|camera| f64::from(camera.dof_max_blur_at(self.current_frame)))
                    .unwrap_or(16.0),
            ),
        )?;
        self.ui
            .set_binding("selected_is_light", Value::Bool(light.is_some()))?;
        self.ui.set_binding(
            "selected_light_intensity",
            Value::Float(
                light
                    .map(|light| f64::from(light.intensity_at(self.current_frame)))
                    .unwrap_or(100.0),
            ),
        )?;
        self.ui.set_binding(
            "selected_light_casts_shadows",
            Value::Bool(light.is_none_or(|light| light.casts_shadows)),
        )?;
        let (
            light_type_label,
            light_color,
            light_is_spot,
            cone_angle,
            cone_feather,
            falloff,
            shadow_darkness,
        ) = light
            .map(|light| {
                use kagari_vfx::core::timeline::LightType;
                let (label, cone_angle, cone_feather) = match &light.light_type {
                    LightType::Ambient => ("環境光", 90.0, 50.0),
                    LightType::Point => ("点光源", 90.0, 50.0),
                    LightType::Spot {
                        cone_angle_deg,
                        cone_feather_pct,
                    } => ("スポット", *cone_angle_deg, *cone_feather_pct),
                    LightType::Parallel => ("平行光", 90.0, 50.0),
                };
                let [r, g, b, _] = light.color;
                (
                    label,
                    format!(
                        "#{:02X}{:02X}{:02X}",
                        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
                        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
                        (b.clamp(0.0, 1.0) * 255.0).round() as u8
                    ),
                    matches!(&light.light_type, LightType::Spot { .. }),
                    cone_angle,
                    cone_feather,
                    light.falloff,
                    light.shadow_darkness,
                )
            })
            .unwrap_or(("点光源", "#FFFFFF".to_owned(), false, 90.0, 50.0, 1.0, 60.0));
        self.ui.set_binding(
            "selected_light_type_label",
            Value::String(light_type_label.to_owned()),
        )?;
        self.ui
            .set_binding("selected_light_color", Value::String(light_color))?;
        self.ui
            .set_binding("selected_light_is_spot", Value::Bool(light_is_spot))?;
        self.ui.set_binding(
            "selected_light_cone_angle",
            Value::Float(f64::from(cone_angle)),
        )?;
        self.ui.set_binding(
            "selected_light_cone_feather",
            Value::Float(f64::from(cone_feather)),
        )?;
        self.ui
            .set_binding("selected_light_falloff", Value::Float(f64::from(falloff)))?;
        self.ui.set_binding(
            "selected_light_shadow_darkness",
            Value::Float(f64::from(shadow_darkness)),
        )?;
        for (name, value) in [
            ("position_3d_x", transform_3d.map(|values| values.0[0])),
            ("position_3d_y", transform_3d.map(|values| values.0[1])),
            ("position_3d_z", transform_3d.map(|values| values.0[2])),
            ("rotation_3d_x", transform_3d.map(|values| values.1[0])),
            ("rotation_3d_y", transform_3d.map(|values| values.1[1])),
            ("rotation_3d_z", transform_3d.map(|values| values.1[2])),
        ] {
            self.ui
                .set_binding(name, Value::Float(value.map(f64::from).unwrap_or(0.0)))?;
        }
        for (name, value) in [
            ("scale_3d_x", transform_3d.map(|values| values.2[0])),
            ("scale_3d_y", transform_3d.map(|values| values.2[1])),
            ("scale_3d_z", transform_3d.map(|values| values.2[2])),
        ] {
            self.ui
                .set_binding(name, Value::Float(value.map(f64::from).unwrap_or(100.0)))?;
        }
        let material = selected_layer
            .filter(|layer| layer.is_3d)
            .map(|layer| &layer.material);
        for (name, value) in [
            ("material_ambient", material.map(|value| value.ambient)),
            ("material_diffuse", material.map(|value| value.diffuse)),
            ("material_specular", material.map(|value| value.specular)),
            ("material_emission", material.map(|value| value.emission)),
            ("material_metalness", material.map(|value| value.metalness)),
            (
                "material_light_transmission",
                material.map(|value| value.light_transmission),
            ),
        ] {
            self.ui
                .set_binding(name, Value::Float(value.map(f64::from).unwrap_or_default()))?;
        }
        for (name, value) in [
            (
                "material_cast_shadows",
                material.is_some_and(|value| value.cast_shadows),
            ),
            (
                "material_accepts_shadows",
                material.is_some_and(|value| value.accepts_shadows),
            ),
            (
                "material_accepts_lights",
                material.is_some_and(|value| value.accepts_lights),
            ),
        ] {
            self.ui.set_binding(name, Value::Bool(value))?;
        }
        let video_properties = selected_layer.and_then(|layer| match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Video {
                source,
                frame_count,
                audio_wav,
                speed,
                ..
            } => Some((
                *speed,
                format!(
                    "{} f · {:.1}s · 音声{} · {}",
                    frame_count,
                    *frame_count as f32 / self.project_fps.max(1) as f32 / speed.max(0.05),
                    if audio_wav.is_some() {
                        "あり"
                    } else {
                        "なし"
                    },
                    media_file_status(source),
                ),
            )),
            _ => None,
        });
        self.ui
            .set_binding("selected_is_video", Value::Bool(video_properties.is_some()))?;
        let selected_precomp = selected_layer.and_then(|layer| match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::PreComp { .. } => Some(layer.is_collapsed),
            _ => None,
        });
        self.ui.set_binding(
            "selected_is_precomp",
            Value::Bool(selected_precomp.is_some()),
        )?;
        self.ui.set_binding(
            "precomp_collapse_transforms",
            Value::Bool(selected_precomp.unwrap_or(false)),
        )?;
        let precomp_source_summary = selected_layer
            .and_then(|layer| match &layer.layer_type {
                kagari_vfx::core::timeline::LayerType::PreComp { comp_id } => self
                    .project_model
                    .active_composition()
                    .find_sub_comp(comp_id)
                    .map(|composition| {
                        format!(
                            "{} · {} × {} px · {} fps · {} 秒 · {} レイヤー",
                            composition.name,
                            composition.width,
                            composition.height,
                            composition.fps,
                            format_duration_seconds(composition.duration_frames, composition.fps,),
                            composition.layers.len(),
                        )
                    }),
                _ => None,
            })
            .unwrap_or_else(|| "参照先コンポジションが見つかりません。".to_owned());
        self.ui.set_binding(
            "precomp_source_summary",
            Value::String(precomp_source_summary),
        )?;
        self.ui.set_binding(
            "video_speed",
            Value::String(
                video_properties
                    .as_ref()
                    .map(|properties| format!("{:.2}", properties.0))
                    .unwrap_or_else(|| "1.0".to_owned()),
            ),
        )?;
        self.ui.set_binding(
            "video_info",
            Value::String(
                video_properties
                    .as_ref()
                    .map(|properties| properties.1.clone())
                    .unwrap_or_default(),
            ),
        )?;
        let image_info = selected_layer.and_then(|layer| match &layer.layer_type {
            kagari_vfx::core::timeline::LayerType::Image { path } => Some(media_file_status(path)),
            _ => None,
        });
        self.ui
            .set_binding("selected_is_image", Value::Bool(image_info.is_some()))?;
        self.ui
            .set_binding("image_info", Value::String(image_info.unwrap_or_default()))?;
        Ok(())
    }
}
