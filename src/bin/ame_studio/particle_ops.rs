use super::*;

impl AfterEffectsStudio {
    pub(crate) fn add_particle_layer(&mut self) {
        let composition = self.project_model.active_composition_mut();
        let id = format!(
            "ameui_particle_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        );
        let name = format!(
            "パーティクル {}",
            composition
                .layers
                .iter()
                .filter(|layer| matches!(
                    &layer.layer_type,
                    kagari_vfx::core::timeline::LayerType::Particle { .. }
                ))
                .count()
                + 1
        );
        let mut layer = kagari_vfx::core::timeline::Layer::new(
            id.clone(),
            name,
            kagari_vfx::core::timeline::LayerType::Particle {
                emitter: Default::default(),
            },
            composition.duration_frames,
        );
        layer.transform.position = kagari_vfx::core::timeline::Animatable::new_constant([
            composition.width as f32 * 0.5,
            composition.height as f32 * 0.5,
        ]);
        composition.add_layer(layer);
        self.selected_layer_id = Some(id);
        self.sync_selected_layer_controls();
        self.mark_project_changed();
    }

    pub(crate) fn update_selected_emitter(&mut self, field: &str, input: &str) {
        let Ok(value) = input.trim().parse::<f32>() else {
            return;
        };
        if !value.is_finite() {
            return;
        }
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let layer = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked);
        let Some(layer) = layer else {
            return;
        };
        let kagari_vfx::core::timeline::LayerType::Particle { emitter } = &mut layer.layer_type
        else {
            return;
        };
        match field {
            "rate" => emitter.rate = value.clamp(0.0, 100_000.0),
            "radius" => {
                let diameter = value.clamp(0.0, 100_000.0) * 2.0;
                emitter.emitter_size = [diameter, diameter];
            }
            "speed" => emitter.speed = value.clamp(0.0, 100_000.0),
            "speed_variance" => emitter.speed_variance = value.clamp(0.0, 1.0),
            "spread" => emitter.spread_degrees = value.clamp(0.0, 360.0),
            "lifetime" => emitter.lifetime = value.clamp(0.01, 10_000.0),
            "size_start" => emitter.size_start = value.clamp(0.0, 100_000.0),
            "size_end" => emitter.size_end = value.clamp(0.0, 100_000.0),
            "gravity_y" => emitter.gravity[1] = value.clamp(-100_000.0, 100_000.0),
            "wind_x" => emitter.wind[0] = value.clamp(-100_000.0, 100_000.0),
            "turbulence" => emitter.turbulence = value.clamp(0.0, 100_000.0),
            "drag" => emitter.drag = value.clamp(0.0, 100.0),
            _ => return,
        }
        self.mark_project_changed();
    }

    pub(crate) fn set_selected_emitter_shape(
        &mut self,
        shape: kagari_vfx::core::particle_system::EmitterShape,
    ) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let layer = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked);
        if let Some(layer) = layer {
            if let kagari_vfx::core::timeline::LayerType::Particle { emitter } =
                &mut layer.layer_type
            {
                emitter.shape = shape;
                self.mark_project_changed();
            }
        }
    }
}
