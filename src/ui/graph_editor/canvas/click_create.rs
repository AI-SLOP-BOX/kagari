use eframe::egui;
use crate::core::keyframe::{InterpolationType as GInterp, Keyframe as GKeyframe};
use crate::core::timeline::Layer;
use crate::ui::graph_editor::channel_ops::{
    axis_3d, is_effect_property, pin_anim_mut, pin_component,
};
use crate::ui::graph_editor::interpolation_ops::set_effect_channel_at_frame;

pub fn handle_canvas_click_create(
    graph_response: &egui::Response,
    rect: egui::Rect,
    total_f: u32,
    min_val: f32,
    val_range: f32,
    graph_prop: &str,
    layer: &mut Layer,
    project_changed: &mut bool,
) {
    let x_of = |f: u32| rect.left() + (f as f32 / total_f as f32) * rect.width();
    let y_of = |v: f32| rect.bottom() - 4.0 - ((v - min_val) / val_range) * (rect.height() - 8.0);

    if !graph_response.clicked() {
        return;
    }
    let Some(pos) = graph_response.interact_pointer_pos() else {
        return;
    };
    if !rect.contains(pos) {
        return;
    }

    // Existing anchor proximity guard: clicks on anchors belong to the anchor drag
    let chan_kfs = |p: &crate::core::property::Animatable<[f32; 2]>,
                    ci: usize|
     -> Vec<(u32, f32)> {
        p.keyframes()
            .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value[ci])).collect())
            .unwrap_or_default()
    };
    let scalar_kfs = |p: &crate::core::property::Animatable<f32>| -> Vec<(u32, f32)> {
        p.keyframes()
            .map(|kfs| kfs.iter().map(|kf| (kf.frame, kf.value)).collect())
            .unwrap_or_default()
    };
    let anchor_pts: Vec<(u32, f32)> = match graph_prop {
        "Position X" => chan_kfs(&layer.transform.position, 0),
        "Position Y" => chan_kfs(&layer.transform.position, 1),
        "Scale X" => chan_kfs(&layer.transform.scale, 0),
        "Scale Y" => chan_kfs(&layer.transform.scale, 1),
        "Rotation" => scalar_kfs(&layer.transform.rotation),
        "Opacity" => scalar_kfs(&layer.transform.opacity),
        p if p.starts_with("3D Position") => layer
            .transform_3d
            .position
            .keyframes()
            .map(|kfs| {
                kfs.iter()
                    .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                    .collect()
            })
            .unwrap_or_default(),
        p if p.starts_with("3D Rotation") => layer
            .transform_3d
            .rotation
            .keyframes()
            .map(|kfs| {
                kfs.iter()
                    .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                    .collect()
            })
            .unwrap_or_default(),
        p if p.starts_with("3D Scale") => layer
            .transform_3d
            .scale
            .keyframes()
            .map(|kfs| {
                kfs.iter()
                    .map(|kf| (kf.frame, kf.value[axis_3d(p)]))
                    .collect()
            })
            .unwrap_or_default(),
        p if is_effect_property(p) => {
            crate::ui::graph_editor::interpolation_ops::effect_parameter_channel_keyframes(
                layer, p,
            )
        }
        _ => Vec::new(),
    };

    let near_existing = anchor_pts.iter().any(|&(kf_f, kf_v)| {
        let ax = x_of(kf_f);
        let ay = y_of(kf_v);
        (pos.x - ax).abs() < 10.0 && (pos.y - ay).abs() < 10.0
    });

    if !near_existing {
        // Map pointer position to frame and value
        let click_t = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
        let new_frame = (click_t * total_f as f32).round() as u32;
        let norm_y =
            (1.0 - (pos.y - (rect.bottom() - 4.0 - (rect.height() - 8.0))) / (rect.height() - 8.0))
                .clamp(0.0, 1.0);
        let new_val = min_val + norm_y * val_range;

        match graph_prop {
            "Position X" | "Position Y" => {
                let ci = if graph_prop.ends_with('Y') { 1 } else { 0 };
                let cur = layer.transform.position.value_at(new_frame);
                let mut v = cur;
                v[ci] = new_val;
                let kf = GKeyframe {
                    frame: new_frame,
                    value: v,
                    interpolation: GInterp::Linear,
                };
                if let Some(kfs) = layer.transform.position.keyframes_mut() {
                    if let Some(existing) = kfs.iter_mut().find(|k| k.frame == new_frame) {
                        *existing = kf;
                    } else {
                        kfs.push(kf);
                        kfs.sort_by_key(|k| k.frame);
                    }
                } else {
                    layer.transform.position =
                        crate::core::property::Animatable::Animated(vec![kf]);
                }
                *project_changed = true;
            }
            "Scale X" | "Scale Y" => {
                let ci = if graph_prop.ends_with('Y') { 1 } else { 0 };
                let cur = layer.transform.scale.value_at(new_frame);
                let mut v = cur;
                v[ci] = new_val;
                let kf = GKeyframe {
                    frame: new_frame,
                    value: v,
                    interpolation: GInterp::Linear,
                };
                if let Some(kfs) = layer.transform.scale.keyframes_mut() {
                    if let Some(existing) = kfs.iter_mut().find(|k| k.frame == new_frame) {
                        *existing = kf;
                    } else {
                        kfs.push(kf);
                        kfs.sort_by_key(|k| k.frame);
                    }
                } else {
                    layer.transform.scale =
                        crate::core::property::Animatable::Animated(vec![kf]);
                }
                *project_changed = true;
            }
            "Rotation" => {
                let kf = GKeyframe {
                    frame: new_frame,
                    value: new_val,
                    interpolation: GInterp::Linear,
                };
                if let Some(kfs) = layer.transform.rotation.keyframes_mut() {
                    if let Some(existing) = kfs.iter_mut().find(|k| k.frame == new_frame) {
                        *existing = kf;
                    } else {
                        kfs.push(kf);
                        kfs.sort_by_key(|k| k.frame);
                    }
                } else {
                    layer.transform.rotation =
                        crate::core::property::Animatable::Animated(vec![kf]);
                }
                *project_changed = true;
            }
            "Opacity" => {
                let kf = GKeyframe {
                    frame: new_frame,
                    value: new_val.clamp(0.0, 100.0),
                    interpolation: GInterp::Linear,
                };
                if let Some(kfs) = layer.transform.opacity.keyframes_mut() {
                    if let Some(existing) = kfs.iter_mut().find(|k| k.frame == new_frame) {
                        *existing = kf;
                    } else {
                        kfs.push(kf);
                        kfs.sort_by_key(|k| k.frame);
                    }
                } else {
                    layer.transform.opacity =
                        crate::core::property::Animatable::Animated(vec![kf]);
                }
                *project_changed = true;
            }
            p if p.starts_with("3D Position") => {
                let ci = axis_3d(p);
                let cur = layer.transform_3d.position.value_at(new_frame);
                let mut v = cur;
                v[ci] = new_val;
                let kf = GKeyframe {
                    frame: new_frame,
                    value: v,
                    interpolation: GInterp::Linear,
                };
                if let Some(kfs) = layer.transform_3d.position.keyframes_mut() {
                    if let Some(existing) = kfs.iter_mut().find(|k| k.frame == new_frame) {
                        *existing = kf;
                    } else {
                        kfs.push(kf);
                        kfs.sort_by_key(|k| k.frame);
                    }
                } else {
                    layer.transform_3d.position =
                        crate::core::property::Animatable::Animated(vec![kf]);
                }
                *project_changed = true;
            }
            p if p.starts_with("3D Rotation") => {
                let ci = axis_3d(p);
                let cur = layer.transform_3d.rotation.value_at(new_frame);
                let mut v = cur;
                v[ci] = new_val;
                let kf = GKeyframe {
                    frame: new_frame,
                    value: v,
                    interpolation: GInterp::Linear,
                };
                if let Some(kfs) = layer.transform_3d.rotation.keyframes_mut() {
                    if let Some(existing) = kfs.iter_mut().find(|k| k.frame == new_frame) {
                        *existing = kf;
                    } else {
                        kfs.push(kf);
                        kfs.sort_by_key(|k| k.frame);
                    }
                } else {
                    layer.transform_3d.rotation =
                        crate::core::property::Animatable::Animated(vec![kf]);
                }
                *project_changed = true;
            }
            p if p.starts_with("3D Scale") => {
                let ci = axis_3d(p);
                let cur = layer.transform_3d.scale.value_at(new_frame);
                let mut v = cur;
                v[ci] = new_val;
                let kf = GKeyframe {
                    frame: new_frame,
                    value: v,
                    interpolation: GInterp::Linear,
                };
                if let Some(kfs) = layer.transform_3d.scale.keyframes_mut() {
                    if let Some(existing) = kfs.iter_mut().find(|k| k.frame == new_frame) {
                        *existing = kf;
                    } else {
                        kfs.push(kf);
                        kfs.sort_by_key(|k| k.frame);
                    }
                } else {
                    layer.transform_3d.scale =
                        crate::core::property::Animatable::Animated(vec![kf]);
                }
                *project_changed = true;
            }
            p if is_effect_property(p) => {
                if set_effect_channel_at_frame(layer, p, new_frame, new_val) {
                    *project_changed = true;
                }
            }
            p if p.starts_with("PinX:") || p.starts_with("PinY:") => {
                let ci = pin_component(p);
                if let Some(pin_anim) = pin_anim_mut(layer, p) {
                    let cur = pin_anim.value_at(new_frame);
                    let mut v = cur;
                    v[ci] = new_val;
                    let kf = GKeyframe {
                        frame: new_frame,
                        value: v,
                        interpolation: GInterp::Linear,
                    };
                    if let Some(kfs) = pin_anim.keyframes_mut() {
                        if let Some(existing) = kfs.iter_mut().find(|k| k.frame == new_frame) {
                            *existing = kf;
                        } else {
                            kfs.push(kf);
                            kfs.sort_by_key(|k| k.frame);
                        }
                    } else {
                        *pin_anim = crate::core::property::Animatable::Animated(vec![kf]);
                    }
                    *project_changed = true;
                }
            }
            _ => {}
        }
    }
}
