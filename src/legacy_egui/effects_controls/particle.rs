use crate::KagariApp;
use eframe::egui;

pub fn draw_particle_emitter_controls(app: &mut KagariApp, ui: &mut egui::Ui) {
    use crate::core::particle_system::{EmitterShape, ParticleEmitter};
    use crate::core::timeline::LayerType;

    let Some(idx) = app.selection.selected_layer_idx else {
        return;
    };
    let is_particle = matches!(
        app.history.current().active_composition().layers.get(idx),
        Some(l) if matches!(l.layer_type, LayerType::Particle { .. })
    );
    if !is_particle {
        return;
    }

    let mut em: ParticleEmitter = match app.history.current().active_composition().layers.get(idx) {
        Some(l) => match &l.layer_type {
            LayerType::Particle { emitter } => emitter.clone(),
            _ => return,
        },
        None => return,
    };
    let mut changed = false;

    crate::ui::kui::section(ui, "Particle Emitter");
    crate::ui::kui::collapsing_section(ui, "particle-emission", "Emission", true, |ui| {
        let grid = crate::ui::kui::PropertyGrid::new(
            ui,
            &[
                "Rate",
                "Maximum Particles",
                "Lifetime",
                "Lifetime Variation",
                "Speed",
                "Speed Variation",
                "Spread",
            ],
        );
        changed |= row_f32(&grid, ui, "Rate", &mut em.rate, 0.0..=500.0, 1.0, " p/s");
        changed |= row_u32(
            &grid,
            ui,
            "Maximum Particles",
            &mut em.max_particles,
            1..=20000,
        );
        changed |= row_f32(
            &grid,
            ui,
            "Lifetime",
            &mut em.lifetime,
            0.1..=30.0,
            0.1,
            " s",
        );
        changed |= row_f32(
            &grid,
            ui,
            "Lifetime Variation",
            &mut em.lifetime_variance,
            0.0..=1.0,
            0.01,
            "",
        );
        changed |= row_f32(
            &grid,
            ui,
            "Speed",
            &mut em.speed,
            0.0..=1000.0,
            1.0,
            " px/s",
        );
        changed |= row_f32(
            &grid,
            ui,
            "Speed Variation",
            &mut em.speed_variance,
            0.0..=2.0,
            0.01,
            "",
        );
        changed |= row_f32(
            &grid,
            ui,
            "Spread",
            &mut em.spread_degrees,
            0.0..=360.0,
            1.0,
            "°",
        );
    });

    crate::ui::kui::collapsing_section(ui, "particle-emitter-shape", "Emitter Shape", true, |ui| {
        let grid = crate::ui::kui::PropertyGrid::new(ui, &["Shape", "Size", "Height"]);
        let shapes = [
            (EmitterShape::Point, "Point"),
            (EmitterShape::Box, "Box"),
            (EmitterShape::Circle, "Circle"),
            (EmitterShape::Line, "Line"),
            (EmitterShape::Ring, "Ring"),
        ];
        changed |= grid.row(ui, "Shape", |ui| {
            crate::ui::kui::combo(ui, "particle_emitter_shape", &mut em.shape, &shapes)
        });
        changed |= row_f32(
            &grid,
            ui,
            "Size",
            &mut em.emitter_size[0],
            1.0..=4000.0,
            1.0,
            " px",
        );
        if em.shape == EmitterShape::Box {
            changed |= row_f32(
                &grid,
                ui,
                "Height",
                &mut em.emitter_size[1],
                1.0..=4000.0,
                1.0,
                " px",
            );
        }
    });

    crate::ui::kui::collapsing_section(ui, "particle-forces", "Forces", true, |ui| {
        let grid = crate::ui::kui::PropertyGrid::new(
            ui,
            &[
                "Gravity",
                "Wind",
                "Gust Strength",
                "Gust Frequency",
                "Turbulence",
                "Air Drag",
            ],
        );
        changed |= grid
            .row(ui, "Gravity", |ui| {
                crate::ui::kui::vector_axes(
                    ui,
                    &mut em.gravity,
                    -2000.0..=2000.0,
                    1.0,
                    " px/s²",
                    ["X", "Y"],
                )
            })
            .changed();
        changed |= grid
            .row(ui, "Wind", |ui| {
                crate::ui::kui::vector_axes(
                    ui,
                    &mut em.wind,
                    -2000.0..=2000.0,
                    1.0,
                    " px/s",
                    ["X", "Y"],
                )
            })
            .changed();
        changed |= row_f32(
            &grid,
            ui,
            "Gust Strength",
            &mut em.wind_gust_strength,
            0.0..=300.0,
            1.0,
            "",
        );
        changed |= row_f32(
            &grid,
            ui,
            "Gust Frequency",
            &mut em.wind_gust_frequency,
            0.0..=10.0,
            0.05,
            " Hz",
        );
        changed |= row_f32(
            &grid,
            ui,
            "Turbulence",
            &mut em.turbulence,
            0.0..=300.0,
            1.0,
            "",
        );
        changed |= row_f32(&grid, ui, "Air Drag", &mut em.drag, 0.0..=20.0, 0.05, "");
    });

    crate::ui::kui::collapsing_section(ui, "particle-appearance", "Appearance", false, |ui| {
        let grid = crate::ui::kui::PropertyGrid::new(ui, &["Start Size", "End Size"]);
        changed |= row_f32(
            &grid,
            ui,
            "Start Size",
            &mut em.size_start,
            0.0..=200.0,
            0.5,
            " px",
        );
        changed |= row_f32(
            &grid,
            ui,
            "End Size",
            &mut em.size_end,
            0.0..=200.0,
            0.5,
            " px",
        );
    });

    crate::ui::kui::collapsing_section(ui, "particle-collisions", "Collisions", false, |ui| {
        let grid = crate::ui::kui::PropertyGrid::new(
            ui,
            &[
                "Boundary Collisions",
                "Bounds",
                "Restitution",
                "Friction",
                "Particle Collisions",
                "Contact Diameter",
            ],
        );
        changed |= grid
            .row(ui, "Boundary Collisions", |ui| {
                crate::ui::kui::toggle(ui, &mut em.collision_enabled, "Enabled")
            })
            .changed();
        if em.collision_enabled {
            changed |= grid
                .row(ui, "Bounds", |ui| {
                    crate::ui::kui::vector_axes(
                        ui,
                        &mut em.collision_bounds,
                        -10000.0..=10000.0,
                        1.0,
                        " px",
                        ["X0", "Y0", "X1", "Y1"],
                    )
                })
                .changed();
            changed |= row_f32(
                &grid,
                ui,
                "Restitution",
                &mut em.restitution,
                0.0..=1.0,
                0.01,
                "",
            );
            changed |= row_f32(
                &grid,
                ui,
                "Friction",
                &mut em.surface_friction,
                0.0..=1.0,
                0.01,
                "",
            );
        }
        changed |= grid
            .row(ui, "Particle Collisions", |ui| {
                crate::ui::kui::toggle(ui, &mut em.particle_collisions, "Enabled")
            })
            .changed();
        if em.particle_collisions {
            changed |= row_f32(
                &grid,
                ui,
                "Contact Diameter",
                &mut em.particle_diameter,
                0.5..=200.0,
                0.5,
                " px",
            );
        }
    });

    if changed {
        app.modify_project(move |p| {
            let comp = p.active_composition_mut();
            if let Some(layer) = comp.layers.get_mut(idx) {
                if let LayerType::Particle { emitter } = &mut layer.layer_type {
                    *emitter = em.clone();
                }
            }
        });
    }
}

fn row_f32(
    grid: &crate::ui::kui::PropertyGrid,
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    speed: f64,
    unit: &str,
) -> bool {
    grid.row(ui, label, |ui| {
        crate::ui::kui::number(ui, value, range, speed, unit)
    })
    .changed()
}

fn row_u32(
    grid: &crate::ui::kui::PropertyGrid,
    ui: &mut egui::Ui,
    label: &str,
    value: &mut u32,
    range: std::ops::RangeInclusive<u32>,
) -> bool {
    grid.row(ui, label, |ui| crate::ui::kui::integer(ui, value, range))
        .changed()
}
