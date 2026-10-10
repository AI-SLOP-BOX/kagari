use crate::core::keyframe::InterpolationType;
use crate::core::property::Animatable;
use crate::ui::graph_editor::property::{GraphProperty, Scalar2dTrack, Vector2Track};
use crate::ui::kui::{self, ButtonKind};
use crate::KagariApp;
use eframe::egui;

/// Applies a custom bezier derived from AE velocity/influence sliders to every
/// keyframe of the selected track. Influence maps to the x control points
/// (33.3% -> 0.25, matching Easy Ease); speed biases the y handles so higher
/// incoming speed sharpens the approach.
fn apply_velocity_bezier<T: Clone + crate::core::property::Interpolate>(
    anim: &mut Animatable<T>,
    in_influence: f32,
    in_speed: f32,
    out_influence: f32,
    out_speed: f32,
) {
    if let Some(kfs) = anim.keyframes_mut() {
        let x1 = (out_influence / 100.0 * 0.75).clamp(0.0, 0.9);
        let x2 = 1.0 - (in_influence / 100.0 * 0.75).clamp(0.0, 0.9);
        // Map speed (0..2000 px/s) to a normalized y bias
        let y1 = 0.1 + (out_speed / 2000.0).clamp(0.0, 1.0) * 0.8;
        let y2 = 0.9 - (in_speed / 2000.0).clamp(0.0, 1.0) * 0.8;
        for kf in kfs.iter_mut() {
            kf.interpolation = InterpolationType::Bezier {
                outgoing: crate::core::keyframe::BezierControlPoint {
                    influence: x1,
                    speed: out_speed,
                },
                incoming: crate::core::keyframe::BezierControlPoint {
                    influence: x2,
                    speed: in_speed,
                },
                custom_bezier: Some([x1, y1, x2, y2]),
            };
        }
    }
}

pub fn draw_speed_graph_options(app: &mut KagariApp, ui: &mut egui::Ui, _current_frame: u32) {
    kui::section(ui, "Graph Editor Options & Keyframe Velocity");
    let graph_mode_id = egui::Id::new("ae_graph_mode_select");
    let mut graph_mode = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(graph_mode_id, || 0));
    let graph_modes = [(0, "Edit Speed Graph"), (1, "Edit Value Graph")];
    let graph_grid = kui::PropertyGrid::new(ui, &["Graph Type"]);
    if graph_grid.row(ui, "Graph Type", |ui| {
        kui::combo(ui, graph_mode_id, &mut graph_mode, &graph_modes)
    }) {
        ui.ctx()
            .data_mut(|data| data.insert_temp(graph_mode_id, graph_mode));
    }

    ui.add_space(kui::space::SM);
    kui::separator(ui);
    kui::caption(
        ui,
        "Keyframe Velocity & Influence (applies to the selected layer's graph property):",
    );

    let vel_id = egui::Id::new("ae_velocity_sliders");
    let (mut in_influence, mut in_speed, mut out_influence, mut out_speed) =
        ui.ctx().data_mut(|d| {
            *d.get_temp_mut_or_insert_with(vel_id, || (33.3f32, 0.0f32, 33.3f32, 0.0f32))
        });

    let velocity_grid = kui::PropertyGrid::new(
        ui,
        &[
            "Incoming Speed",
            "Incoming Influence",
            "Outgoing Speed",
            "Outgoing Influence",
        ],
    );
    velocity_grid.row(ui, "Incoming Speed", |ui| {
        kui::slider(ui, &mut in_speed, 0.0..=2000.0, " px/s")
    });
    velocity_grid.row(ui, "Incoming Influence", |ui| {
        kui::slider(ui, &mut in_influence, 0.0..=100.0, "%")
    });
    velocity_grid.row(ui, "Outgoing Speed", |ui| {
        kui::slider(ui, &mut out_speed, 0.0..=2000.0, " px/s")
    });
    velocity_grid.row(ui, "Outgoing Influence", |ui| {
        kui::slider(ui, &mut out_influence, 0.0..=100.0, "%")
    });
    ui.ctx()
        .data_mut(|d| d.insert_temp(vel_id, (in_influence, in_speed, out_influence, out_speed)));

    ui.add_space(kui::space::SM);
    kui::separator(ui);
    ui.horizontal_wrapped(|ui| {
        if kui::button(ui, "Apply Velocity", ButtonKind::Accent)
            .on_hover_text(
                "Write the velocity/influence values into the selected track's keyframes",
            )
            .clicked()
        {
            let Some(layer_idx) = app.selection.selected_layer_idx else {
                app.toasts
                    .error(crate::ui::localization::tr("Select a layer first"));
                return;
            };
            let prop = app
                .selection
                .selected_property
                .clone()
                .unwrap_or_else(|| "Position X".into());
            let graph_property = GraphProperty::parse(&prop);
            app.modify_project(|p| {
                let comp = p.active_composition_mut();
                let Some(layer) = comp.layers.get_mut(layer_idx) else {
                    return;
                };
                let t = &mut layer.transform;
                match graph_property {
                    GraphProperty::Vector2 {
                        track: Vector2Track::Position,
                        ..
                    } => apply_velocity_bezier(
                        &mut t.position,
                        in_influence,
                        in_speed,
                        out_influence,
                        out_speed,
                    ),
                    GraphProperty::Vector2 {
                        track: Vector2Track::Scale,
                        ..
                    } => apply_velocity_bezier(
                        &mut t.scale,
                        in_influence,
                        in_speed,
                        out_influence,
                        out_speed,
                    ),
                    GraphProperty::Scalar2d(Scalar2dTrack::Rotation) => apply_velocity_bezier(
                        &mut t.rotation,
                        in_influence,
                        in_speed,
                        out_influence,
                        out_speed,
                    ),
                    GraphProperty::Scalar2d(Scalar2dTrack::Opacity) => apply_velocity_bezier(
                        &mut t.opacity,
                        in_influence,
                        in_speed,
                        out_influence,
                        out_speed,
                    ),
                    _ => {}
                }
            });
            app.toasts.info(crate::ui::localization::tr_format(
                "Velocity applied to {}",
                &[&prop],
            ));
        }
        if kui::button(ui, "Easy Ease (F9)", ButtonKind::Default)
            .on_hover_text("Symmetrical smooth ease on all keyframes")
            .clicked()
        {
            apply_preset_ease(app, 33.3, 0.0, 33.3, 0.0, |x1, _y1, x2, _y2| {
                [x1, 0.1, x2, 1.0]
            });
        }
        if kui::button(ui, "Easy Ease In", ButtonKind::Default)
            .on_hover_text("Ease into the next keyframe")
            .clicked()
        {
            apply_preset_ease(app, 33.3, 0.0, 33.3, 0.0, |_x1, _y1, x2, _y2| {
                [0.0, 0.0, x2, 1.0]
            });
        }
        if kui::button(ui, "Easy Ease Out", ButtonKind::Default)
            .on_hover_text("Ease out of the previous keyframe")
            .clicked()
        {
            apply_preset_ease(app, 33.3, 0.0, 33.3, 0.0, |x1, _y1, _x2, _y2| {
                [x1, 0.1, 1.0, 1.0]
            });
        }
    });
}

/// Applies an ease preset to the selected layer's graph property keyframes.
fn apply_preset_ease(
    app: &mut KagariApp,
    in_inf: f32,
    _in_spd: f32,
    out_inf: f32,
    _out_spd: f32,
    coords: impl Fn(f32, f32, f32, f32) -> [f32; 4],
) {
    let Some(layer_idx) = app.selection.selected_layer_idx else {
        app.toasts
            .error(crate::ui::localization::tr("Select a layer first"));
        return;
    };
    let prop = app
        .selection
        .selected_property
        .clone()
        .unwrap_or_else(|| "Position X".into());
    let x1 = out_inf / 100.0 * 0.75;
    let x2 = 1.0 - in_inf / 100.0 * 0.75;
    app.modify_project(|p| {
        let comp = p.active_composition_mut();
        let Some(layer) = comp.layers.get_mut(layer_idx) else {
            return;
        };
        let t = &mut layer.transform;
        fn apply_to<T: Clone + crate::core::property::Interpolate>(
            anim: &mut Animatable<T>,
            x1: f32,
            x2: f32,
            coords: &dyn Fn(f32, f32, f32, f32) -> [f32; 4],
        ) {
            if let Some(kfs) = anim.keyframes_mut() {
                for kf in kfs.iter_mut() {
                    kf.interpolation = InterpolationType::Bezier {
                        outgoing: crate::core::keyframe::BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        incoming: crate::core::keyframe::BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        custom_bezier: Some(coords(x1, 0.0, x2, 1.0)),
                    };
                }
            }
        }
        match prop.as_str() {
            "Position X" | "Position Y" => apply_to(&mut t.position, x1, x2, &coords),
            "Scale X" | "Scale Y" => apply_to(&mut t.scale, x1, x2, &coords),
            "Rotation" => apply_to(&mut t.rotation, x1, x2, &coords),
            "Opacity" => apply_to(&mut t.opacity, x1, x2, &coords),
            _ => {}
        }
    });
    app.toasts.info(crate::ui::localization::tr_format(
        "Ease applied to {}",
        &[&prop],
    ));
}
