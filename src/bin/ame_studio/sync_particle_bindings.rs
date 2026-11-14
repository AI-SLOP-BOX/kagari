use super::*;

impl AfterEffectsStudio {
    pub(crate) fn sync_particle_layer_bindings(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let selected_particle = self
            .selected_layer_id
            .as_deref()
            .and_then(|id| {
                self.project_model
                    .active_composition()
                    .layers
                    .iter()
                    .find(|layer| layer.id == id)
            })
            .and_then(|layer| match &layer.layer_type {
                kagari_vfx::core::timeline::LayerType::Particle { emitter } => Some(emitter),
                _ => None,
            });
        self.ui.set_binding(
            "selected_is_particle",
            Value::Bool(selected_particle.is_some()),
        )?;
        self.ui.set_binding(
            "emitter_shape_circle",
            Value::Bool(selected_particle.is_some_and(|emitter| {
                emitter.shape == kagari_vfx::core::particle_system::EmitterShape::Circle
            })),
        )?;
        for (name, value) in [
            ("emitter_rate", selected_particle.map(|e| e.rate)),
            (
                "emitter_radius",
                selected_particle.map(|e| e.emitter_size[0] * 0.5),
            ),
            (
                "emitter_spread",
                selected_particle.map(|e| e.spread_degrees),
            ),
            ("velocity", selected_particle.map(|e| e.speed)),
            (
                "velocity_random",
                selected_particle.map(|e| e.speed_variance),
            ),
            ("lifetime", selected_particle.map(|e| e.lifetime)),
            ("particle_size", selected_particle.map(|e| e.size_start)),
            ("size_random", selected_particle.map(|e| e.size_end)),
            (
                "emitter_opacity_start",
                selected_particle.map(|e| e.opacity_start),
            ),
            (
                "emitter_opacity_end",
                selected_particle.map(|e| e.opacity_end),
            ),
            ("emitter_gravity_y", selected_particle.map(|e| e.gravity[1])),
            ("emitter_wind_x", selected_particle.map(|e| e.wind[0])),
            (
                "emitter_turbulence",
                selected_particle.map(|e| e.turbulence),
            ),
            ("emitter_drag", selected_particle.map(|e| e.drag)),
        ] {
            self.ui.set_binding(
                name,
                Value::String(value.map(|n| format!("{n:.2}")).unwrap_or_default()),
            )?;
        }

        self.sync_layer_styles_bindings()?;
        self.sync_composite_bindings()?;
        Ok(())
    }
}
