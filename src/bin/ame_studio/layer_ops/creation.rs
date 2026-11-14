use super::*;

impl AfterEffectsStudio {
    pub(crate) fn add_utility_layer(
        &mut self,
        name: &str,
        layer_type: kagari_vfx::core::timeline::LayerType,
    ) {
        let composition = self.project_model.active_composition_mut();
        let id = format!(
            "ameui_utility_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        );
        let mut layer = kagari_vfx::core::timeline::Layer::new(
            id.clone(),
            name.to_owned(),
            layer_type,
            composition.duration_frames,
        );
        layer.transform.position = kagari_vfx::core::timeline::Animatable::new_constant([
            composition.width as f32 * 0.5,
            composition.height as f32 * 0.5,
        ]);
        layer.is_adjustment_layer = matches!(
            &layer.layer_type,
            kagari_vfx::core::timeline::LayerType::AdjustmentLayer
        );
        composition.add_layer(layer);
        self.selected_layer_id = Some(id);
        self.sync_selected_layer_controls();
        self.mark_project_changed();
    }

    pub(crate) fn add_canvas_layer(
        &mut self,
        name: String,
        layer_type: kagari_vfx::core::timeline::LayerType,
        position: [f64; 2],
    ) {
        let composition = self.project_model.active_composition_mut();
        let id = format!(
            "ameui_canvas_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        );
        let mut layer = kagari_vfx::core::timeline::Layer::new(
            id.clone(),
            name,
            layer_type,
            composition.duration_frames,
        );
        layer.transform.position = kagari_vfx::core::timeline::Animatable::new_constant([
            position[0] as f32,
            position[1] as f32,
        ]);
        composition.add_layer(layer);
        self.selected_layer_id = Some(id);
        self.sync_selected_layer_controls();
        self.mark_project_changed();
    }

    pub(crate) fn add_text_at(&mut self, position: [f64; 2]) {
        use kagari_vfx::core::timeline::LayerType;
        self.add_canvas_layer(
            "テキスト".to_owned(),
            LayerType::Text {
                text: "新しいテキスト".to_owned(),
                font_size: 48,
                color: [1.0, 1.0, 1.0, 1.0],
                font_family: "Inter".to_owned(),
                tracking: 0.0,
                leading: 1.2,
                align: 0,
                stroke_color: [0.0, 0.0, 0.0, 1.0],
                stroke_width: 0.0,
                text_on_path: false,
            },
            position,
        );
        self.sync_bindings_logged();
    }

    pub(crate) fn cycle_layer_blend_mode(&mut self) {
        use kagari_vfx::core::timeline::BlendMode;
        const MODES: &[BlendMode] = &[
            BlendMode::Normal,
            BlendMode::Multiply,
            BlendMode::Screen,
            BlendMode::Overlay,
            BlendMode::Add,
            BlendMode::Darken,
            BlendMode::Lighten,
            BlendMode::SoftLight,
            BlendMode::HardLight,
            BlendMode::Difference,
            BlendMode::Exclusion,
            BlendMode::Divide,
            BlendMode::Subtract,
            BlendMode::ColorBurn,
            BlendMode::LinearBurn,
            BlendMode::VividLight,
            BlendMode::ColorDodge,
            BlendMode::LinearDodge,
            BlendMode::Color,
            BlendMode::Hue,
            BlendMode::Saturation,
            BlendMode::Luminosity,
            BlendMode::StencilAlpha,
            BlendMode::StencilLuma,
            BlendMode::SilhouetteAlpha,
            BlendMode::SilhouetteLuma,
            BlendMode::Behind,
            BlendMode::AlphaAdd,
            BlendMode::LinearLight,
        ];
        if let Some(id) = self.selected_layer_id.as_deref() {
            let selected = self
                .project_model
                .active_composition_mut()
                .layers
                .iter_mut()
                .find(|layer| layer.id == id && !layer.locked);
            if let Some(layer) = selected {
                let index = MODES
                    .iter()
                    .position(|mode| *mode == layer.blend_mode)
                    .unwrap_or(0);
                layer.blend_mode = MODES[(index + 1) % MODES.len()];
                self.mark_project_changed();
            }
        }
    }

    pub(crate) fn cycle_layer_track_matte(&mut self) {
        use kagari_vfx::core::timeline::TrackMatteMode as Matte;
        const MODES: &[Matte] = &[
            Matte::None,
            Matte::AlphaMatte,
            Matte::AlphaMatteInverted,
            Matte::LumaMatte,
            Matte::LumaMatteInverted,
        ];
        if let Some(id) = self.selected_layer_id.as_deref() {
            let selected = self
                .project_model
                .active_composition_mut()
                .layers
                .iter_mut()
                .find(|layer| layer.id == id && !layer.locked);
            if let Some(layer) = selected {
                let index = MODES
                    .iter()
                    .position(|mode| *mode == layer.track_matte)
                    .unwrap_or(0);
                layer.track_matte = MODES[(index + 1) % MODES.len()];
                if layer.track_matte == Matte::None {
                    layer.track_matte_layer_id = None;
                }
                self.mark_project_changed();
            }
        }
    }

    pub(crate) fn select_track_matte_source(&mut self, source_id_opt: Option<&str>) {
        if let (Some(layer_id), Some(source_id)) =
            (self.selected_layer_id.as_deref(), source_id_opt)
        {
            if layer_id != source_id {
                let selected = self
                    .project_model
                    .active_composition_mut()
                    .layers
                    .iter_mut()
                    .find(|layer| layer.id == layer_id && !layer.locked);
                if let Some(layer) = selected {
                    layer.track_matte_layer_id = Some(source_id.to_owned());
                    self.mark_project_changed();
                }
            }
        }
    }

    pub(crate) fn clear_track_matte_source(&mut self) {
        if let Some(layer_id) = self.selected_layer_id.as_deref() {
            let selected = self
                .project_model
                .active_composition_mut()
                .layers
                .iter_mut()
                .find(|layer| layer.id == layer_id && !layer.locked);
            if let Some(layer) = selected {
                if layer.track_matte_layer_id.take().is_some() {
                    self.mark_project_changed();
                }
            }
        }
    }
}
