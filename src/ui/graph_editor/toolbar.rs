use super::*;

pub fn apply_preset_to_layer(
    layer: &mut Layer,
    prop: &str,
    preset: crate::core::keyframe::EasePreset,
    selected_frame: Option<u32>,
) {
    use crate::core::keyframe::{BezierControlPoint, InterpolationType};
    use crate::core::property::Animatable;
    let pts = preset.control_points();
    fn apply<T>(
        kfs: &mut [crate::core::keyframe::Keyframe<T>],
        pts: [f32; 4],
        selected_frame: Option<u32>,
    ) {
        for kf in kfs.iter_mut() {
            if selected_frame.is_some_and(|frame| frame != kf.frame) {
                continue;
            }
            kf.interpolation = InterpolationType::Bezier {
                outgoing: BezierControlPoint {
                    influence: 0.333,
                    speed: 0.0,
                },
                incoming: BezierControlPoint {
                    influence: 0.333,
                    speed: 0.0,
                },
                custom_bezier: Some(pts),
            };
        }
    }

    match prop {
        "Position X" | "Position Y" => {
            if let Animatable::Animated(ref mut kfs) = layer.transform.position {
                apply(kfs, pts, selected_frame);
            }
        }
        "Scale X" | "Scale Y" => {
            if let Animatable::Animated(ref mut kfs) = layer.transform.scale {
                apply(kfs, pts, selected_frame);
            }
        }
        "Rotation" => {
            if let Animatable::Animated(ref mut kfs) = layer.transform.rotation {
                apply(kfs, pts, selected_frame);
            }
        }
        "Opacity" => {
            if let Animatable::Animated(ref mut kfs) = layer.transform.opacity {
                apply(kfs, pts, selected_frame);
            }
        }
        p if p.starts_with("3D Position") => {
            if let Animatable::Animated(ref mut kfs) = layer.transform_3d.position {
                apply(kfs, pts, selected_frame);
            }
        }
        p if p.starts_with("3D Rotation") => {
            if let Animatable::Animated(ref mut kfs) = layer.transform_3d.rotation {
                apply(kfs, pts, selected_frame);
            }
        }
        p if p.starts_with("3D Scale") => {
            if let Animatable::Animated(ref mut kfs) = layer.transform_3d.scale {
                apply(kfs, pts, selected_frame);
            }
        }
        p if p.starts_with("Pin") => {
            if let Some(Animatable::Animated(ref mut kfs)) = pin_anim_mut(layer, p) {
                apply(kfs, pts, selected_frame);
            }
        }
        p if is_effect_property(p) => {
            if let Some((effect_id, parameter_name, _)) = parse_effect_property(p) {
                if let Some(effect) = layer
                    .effects
                    .iter_mut()
                    .find(|effect| effect.id == effect_id)
                {
                    let interpolation = InterpolationType::Bezier {
                        outgoing: BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        incoming: BezierControlPoint {
                            influence: 0.333,
                            speed: 0.0,
                        },
                        custom_bezier: Some(pts),
                    };
                    if let Some(frame) = selected_frame {
                        effect
                            .effect_type
                            .set_parameter_keyframe_interpolation_at_frame(
                                parameter_name,
                                frame,
                                interpolation,
                            );
                    } else {
                        effect.effect_type.set_parameter_keyframe_interpolation(
                            Some(parameter_name),
                            interpolation,
                        );
                    }
                }
            }
        }
        _ => {}
    }
}

pub fn draw_graph_toolbar(
    ui: &mut egui::Ui,
    selected_property: &mut Option<String>,
    linked_tangent: &mut bool,
    layer: &mut Layer,
    project_changed: &mut bool,
    fps: u32,
) {
    let prop_name = selected_property
        .clone()
        .unwrap_or_else(|| "Position X".to_string());

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Graph Editor").strong());
        egui::ComboBox::from_id_salt("graph_prop_select_module")
            .selected_text(&prop_name)
            .show_ui(ui, |ui| {
                let mut props: Vec<(String, String)> = [
                    ("Position X", "Position X"),
                    ("Position Y", "Position Y"),
                    ("Scale X", "Scale X"),
                    ("Scale Y", "Scale Y"),
                    ("Rotation", "Rotation"),
                    ("Opacity", "Opacity"),
                    ("3D Position X", "3D Position X"),
                    ("3D Position Y", "3D Position Y"),
                    ("3D Position Z", "3D Position Z"),
                    ("3D Rotation X", "3D Rotation X"),
                    ("3D Rotation Y", "3D Rotation Y"),
                    ("3D Rotation Z", "3D Rotation Z"),
                    ("3D Scale X", "3D Scale X"),
                    ("3D Scale Y", "3D Scale Y"),
                    ("3D Scale Z", "3D Scale Z"),
                ]
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect();
                for pin in &layer.puppet_pins {
                    props.push((
                        format!("PinX:{}", pin.id),
                        format!("\u{1f9f7} {} X", pin.name),
                    ));
                    props.push((
                        format!("PinY:{}", pin.id),
                        format!("\u{1f9f7} {} Y", pin.name),
                    ));
                }
                for effect in &layer.effects {
                    for (label, parameter) in effect.effect_type.animatable_params_ref() {
                        let channels: &[(&str, usize)] = match parameter {
                            crate::core::effect_params::ParamRefRef::Scalar(_) => &[("", 0)],
                            crate::core::effect_params::ParamRefRef::Vec2(_) => {
                                &[(" X", 0), (" Y", 1)]
                            }
                            crate::core::effect_params::ParamRefRef::Vec3(_) => {
                                &[(" X", 0), (" Y", 1), (" Z", 2)]
                            }
                            crate::core::effect_params::ParamRefRef::Vec4Color(_) => {
                                &[(" R", 0), (" G", 1), (" B", 2), (" A", 3)]
                            }
                        };
                        for (suffix, channel) in channels {
                            let key = if suffix.is_empty() {
                                format!("fxid:{}::{}", effect.id, label)
                            } else {
                                format!("fxid:{}::{}|{}", effect.id, label, channel)
                            };
                            props.push((
                                key,
                                format!("⚙ {} / {}{}", effect.name, label, suffix),
                            ));
                        }
                    }
                }
                let label_of = |key: &str| -> String {
                    props
                        .iter()
                        .find(|(k, _)| k == key)
                        .map(|(_, l)| l.clone())
                        .unwrap_or_else(|| key.to_string())
                };
                let sel_label = label_of(&prop_name);
                ui.label(egui::RichText::new(&sel_label).weak());
                for (key, lbl) in &props {
                    if ui.selectable_label(prop_name == *key, lbl).clicked() {
                        *selected_property = Some(GraphProperty::parse(key).key());
                    }
                }
            });

        ui.add_space(8.0);
        ui.checkbox(linked_tangent, "Link");

        let active_prop = selected_property
            .clone()
            .unwrap_or_else(|| "Position X".to_string());
        let ease_scope_id = egui::Id::new(("graph_ease_scope", &layer.id, &active_prop));
        let mut apply_all_keys = ui.ctx().data(|d| d.get_temp(ease_scope_id).unwrap_or(true));
        ui.horizontal(|ui| {
            ui.checkbox(&mut apply_all_keys, "All keys");
            ui.label(egui::RichText::new("off = hovered key only").small().weak());
        });
        ui.ctx()
            .data_mut(|d| d.insert_temp(ease_scope_id, apply_all_keys));
        let hovered_index: Option<usize> = ui.ctx().data(|d| {
            d.get_temp(egui::Id::new((
                "ae_graph_hovered_kf",
                &layer.id,
                &active_prop,
            )))
        });
        let ease_target_frame = if apply_all_keys {
            None
        } else {
            hovered_index.and_then(|index| graph_keyframe_frame(layer, &active_prop, index))
        };

        ui.horizontal_wrapped(|ui| {
            for (lbl, short, preset, tip) in [
                (
                    "Easy Ease (F9)",
                    "Easy Ease",
                    crate::core::keyframe::EasePreset::Standard,
                    "Standard symmetric ease [0.25, 0.1, 0.25, 1.0]",
                ),
                (
                    "In",
                    "In",
                    crate::core::keyframe::EasePreset::EaseIn,
                    "Ease In (slow start, fast end)",
                ),
                (
                    "Out",
                    "Out",
                    crate::core::keyframe::EasePreset::EaseOut,
                    "Ease Out (fast start, slow end)",
                ),
                (
                    "Sine",
                    "Sine",
                    crate::core::keyframe::EasePreset::Sine,
                    "Ultra smooth Sine ease",
                ),
                (
                    "Fast Out",
                    "Fast Out",
                    crate::core::keyframe::EasePreset::FastOut,
                    "Quick initial burst then smooth decelerate",
                ),
                (
                    "Overshoot",
                    "Overshoot",
                    crate::core::keyframe::EasePreset::Overshoot,
                    "Spring overshoot past target value",
                ),
                (
                    "Bounce",
                    "Bounce",
                    crate::core::keyframe::EasePreset::Bounce,
                    "Physical single bounce easing",
                ),
                (
                    "Elastic",
                    "Elastic",
                    crate::core::keyframe::EasePreset::Elastic,
                    "Elastic spring recoil easing",
                ),
            ] {
                ui.vertical(|ui| {
                    ui.set_min_width(52.0);
                    let thumb =
                        draw_ease_thumbnail(ui, preset).on_hover_text(format!("{lbl}\n{tip}"));
                    let label = ui
                        .small_button(short)
                        .on_hover_text(format!("{lbl}\n{tip}"));
                    if thumb.clicked() || label.clicked() {
                        apply_preset_to_layer(layer, &active_prop, preset, ease_target_frame);
                        *project_changed = true;
                    }
                });
            }
        });

        ui.add_space(4.0);
        let rove_supported = matches!(active_prop.as_str(), "Position X" | "Position Y")
            || active_prop.starts_with("3D Position")
            || active_prop.starts_with("PinX:")
            || active_prop.starts_with("PinY:");
        if ui
            .add_enabled(rove_supported, egui::Button::new("〰 Rove Across Time"))
            .on_hover_text(if rove_supported {
                "Redistribute position keyframes by cumulative spatial distance"
            } else {
                "Rove is available for position and puppet-pin tracks"
            })
            .clicked()
        {
            *project_changed |= rove_across_time(layer, &active_prop);
        }

        if ui
            .button("⇄ Reverse Keys")
            .on_hover_text("Reverse keyframe order in time (values stay, timing flips)")
            .clicked()
        {
            let changed = match selected_property
                .clone()
                .unwrap_or_else(|| "Position X".to_string())
                .as_str()
            {
                "Position X" | "Position Y" => reverse_keyframes(&mut layer.transform.position),
                "Scale X" | "Scale Y" => reverse_keyframes(&mut layer.transform.scale),
                "Rotation" => reverse_keyframes(&mut layer.transform.rotation),
                "Opacity" => reverse_keyframes(&mut layer.transform.opacity),
                p if p.starts_with("3D Position") => {
                    reverse_keyframes(&mut layer.transform_3d.position)
                }
                p if p.starts_with("3D Rotation") => {
                    reverse_keyframes(&mut layer.transform_3d.rotation)
                }
                p if p.starts_with("3D Scale") => {
                    reverse_keyframes(&mut layer.transform_3d.scale)
                }
                p if p.starts_with("Pin") => {
                    pin_anim_mut(layer, p).is_some_and(reverse_keyframes)
                }
                p if is_effect_property(p) => reverse_effect_property(layer, p),
                _ => false,
            };
            *project_changed |= changed;
        }

        ui.add_space(4.0);
        if ui
            .button("Mirror Ease")
            .on_hover_text("Symmetrically mirror Ease In / Ease Out handles")
            .clicked()
        {
            use crate::core::keyframe::InterpolationType;

            let mut mirror_custom_bezier = |interpolation: &mut InterpolationType| {
                if let InterpolationType::Bezier {
                    custom_bezier: Some(ref mut pts),
                    ..
                } = interpolation
                {
                    let mirrored = [1.0 - pts[2], 1.0 - pts[3], 1.0 - pts[0], 1.0 - pts[1]];
                    *pts = mirrored;
                }
            };
            *project_changed |= map_layer_interpolation(
                layer,
                &active_prop,
                ease_target_frame,
                &mut mirror_custom_bezier,
            );
        }

        ui.add_space(4.0);
        if ui
            .button("Ease In")
            .on_hover_text("Flatten incoming tangent — keyframe eases into its value")
            .clicked()
        {
            use crate::core::keyframe::InterpolationType;
            let mut ease_in = |interpolation: &mut InterpolationType| {
                if let InterpolationType::Bezier {
                    ref mut outgoing,
                    ref mut incoming,
                    ref mut custom_bezier,
                    ..
                } = *interpolation
                {
                    outgoing.influence = 0.0;
                    outgoing.speed = 0.0;
                    incoming.influence = 0.333;
                    incoming.speed = 0.0;
                    *custom_bezier = Some([0.0, 0.0, 0.33, 1.0]);
                }
            };
            *project_changed |=
                map_layer_interpolation(layer, &active_prop, ease_target_frame, &mut ease_in);
        }
        if ui
            .button("Ease Out")
            .on_hover_text("Flatten outgoing tangent — keyframe eases out of its value")
            .clicked()
        {
            use crate::core::keyframe::InterpolationType;
            let mut ease_out = |interpolation: &mut InterpolationType| {
                if let InterpolationType::Bezier {
                    ref mut outgoing,
                    ref mut incoming,
                    ref mut custom_bezier,
                    ..
                } = *interpolation
                {
                    outgoing.influence = 0.333;
                    outgoing.speed = 0.0;
                    incoming.influence = 0.0;
                    incoming.speed = 0.0;
                    *custom_bezier = Some([0.67, 0.0, 1.0, 1.0]);
                }
            };
            *project_changed |=
                map_layer_interpolation(layer, &active_prop, ease_target_frame, &mut ease_out);
        }

        ui.add_space(8.0);
        // ── Speed Graph vs Value Graph Mode Switcher ──
        let mode_id = egui::Id::new("ae_graph_mode_select");
        let mut current_mode = ui.ctx().data(|d| d.get_temp::<i32>(mode_id).unwrap_or(0));
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Mode:")
                    .small()
                    .color(colors::TEXT_SECONDARY),
            );
            if ui
                .selectable_label(current_mode == 0, "Speed Graph")
                .clicked()
            {
                current_mode = 0;
                ui.ctx().data_mut(|d| d.insert_temp(mode_id, 0));
            }
            if ui
                .selectable_label(current_mode == 1, "Value Graph")
                .clicked()
            {
                current_mode = 1;
                ui.ctx().data_mut(|d| d.insert_temp(mode_id, 1));
            }
        });

        ui.collapsing("Keyframe Velocity / Influence", |ui| {
            let prop = selected_property.as_deref().unwrap_or("Position X");
            let target_id = egui::Id::new(("ae_graph_hovered_kf", &layer.id, prop));
            let index: Option<usize> = ui.ctx().data(|d| d.get_temp(target_id));
            if let Some((index, mut values)) = index.and_then(|index| {
                layer_velocity(layer, prop, index, fps as f32, None)
                    .map(|values| (index, values))
            }) {
                let mut changed = false;
                ui.horizontal(|ui| {
                    ui.label("Incoming:");
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut values[0])
                                .range(0.1..=100.0)
                                .speed(0.5)
                                .prefix("Inf: ")
                                .suffix("%"),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut values[2])
                                .speed(1.0)
                                .prefix("Spd: ")
                                .suffix(" units/s"),
                        )
                        .changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Outgoing:");
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut values[1])
                                .range(0.1..=100.0)
                                .speed(0.5)
                                .prefix("Inf: ")
                                .suffix("%"),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut values[3])
                                .speed(1.0)
                                .prefix("Spd: ")
                                .suffix(" units/s"),
                        )
                        .changed();
                });
                if changed {
                    *project_changed |=
                        layer_velocity(layer, prop, index, fps as f32, Some(values)).is_some();
                }
            } else {
                ui.label("Hover a keyframe with a following segment to edit velocity.");
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .small_button("Linear")
                    .on_hover_text("Convert keyframes to linear interpolation")
                    .clicked()
                {
                    *project_changed |= set_layer_interpolation(
                        layer,
                        selected_property.as_deref().unwrap_or("Position X"),
                        crate::core::keyframe::InterpolationType::Linear,
                    );
                }
                if ui
                    .small_button("Auto Bezier")
                    .on_hover_text("Smooth keyframe tangents automatically")
                    .clicked()
                {
                    *project_changed |= set_layer_interpolation(
                        layer,
                        selected_property.as_deref().unwrap_or("Position X"),
                        crate::core::keyframe::InterpolationType::Bezier {
                            outgoing: crate::core::keyframe::BezierControlPoint::default(),
                            incoming: crate::core::keyframe::BezierControlPoint::default(),
                            custom_bezier: None,
                        },
                    );
                }
                if ui
                    .small_button("Hold")
                    .on_hover_text("Hold keyframe value until next keyframe")
                    .clicked()
                {
                    *project_changed |= set_layer_interpolation(
                        layer,
                        selected_property.as_deref().unwrap_or("Position X"),
                        crate::core::keyframe::InterpolationType::Hold,
                    );
                }
            });
        });
    });
}
