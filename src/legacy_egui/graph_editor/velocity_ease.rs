use super::channel_ops::axis_3d;
use crate::core::timeline::Layer;
use crate::ui::theme::colors;
use eframe::egui;

pub fn track_velocity<T: Clone>(
    track: &mut crate::core::property::Animatable<T>,
    index: usize,
    fps: f32,
    component: impl Fn(&T) -> f32,
    replacement: Option<[f32; 4]>,
) -> Option<[f32; 4]> {
    use crate::core::keyframe::{
        compute_ae_bezier_control_points, BezierControlPoint, InterpolationType,
    };
    let keys = track.keyframes_mut()?;
    let key = keys.get(index)?;
    let next = keys.get(index + 1)?;
    let span = next.frame.checked_sub(key.frame)? as f32;
    let delta = component(&next.value) - component(&key.value);
    let values = replacement.unwrap_or(match key.interpolation {
        InterpolationType::Bezier {
            incoming, outgoing, ..
        } => [
            incoming.influence * 100.0,
            outgoing.influence * 100.0,
            incoming.speed,
            outgoing.speed,
        ],
        _ => [33.3, 33.3, 0.0, 0.0],
    });
    if replacement.is_some() {
        let incoming = BezierControlPoint {
            influence: values[0] / 100.0,
            speed: values[2],
        };
        let outgoing = BezierControlPoint {
            influence: values[1] / 100.0,
            speed: values[3],
        };
        let control = compute_ae_bezier_control_points(&outgoing, &incoming, span, delta, fps);
        keys[index].interpolation = InterpolationType::Bezier {
            incoming,
            outgoing,
            custom_bezier: Some(control),
        };
    }
    Some(values)
}

pub fn layer_velocity(
    layer: &mut Layer,
    property: &str,
    index: usize,
    fps: f32,
    replacement: Option<[f32; 4]>,
) -> Option<[f32; 4]> {
    let axis = axis_3d(property);
    match property {
        "Position X" | "Position Y" => track_velocity(
            &mut layer.transform.position,
            index,
            fps,
            |v| v[axis],
            replacement,
        ),
        "Scale X" | "Scale Y" => track_velocity(
            &mut layer.transform.scale,
            index,
            fps,
            |v| v[axis],
            replacement,
        ),
        "Rotation" => track_velocity(
            &mut layer.transform.rotation,
            index,
            fps,
            |v| *v,
            replacement,
        ),
        "Opacity" => track_velocity(
            &mut layer.transform.opacity,
            index,
            fps,
            |v| *v,
            replacement,
        ),
        p if p.starts_with("3D Position") => track_velocity(
            &mut layer.transform_3d.position,
            index,
            fps,
            |v| v[axis],
            replacement,
        ),
        p if p.starts_with("3D Rotation") => track_velocity(
            &mut layer.transform_3d.rotation,
            index,
            fps,
            |v| v[axis],
            replacement,
        ),
        p if p.starts_with("3D Scale") => track_velocity(
            &mut layer.transform_3d.scale,
            index,
            fps,
            |v| v[axis],
            replacement,
        ),
        _ => None,
    }
}

pub fn draw_ease_thumbnail(
    ui: &mut egui::Ui,
    preset: crate::core::keyframe::EasePreset,
) -> egui::Response {
    let want = egui::vec2(52.0, 30.0);
    let (rect, resp) = ui.allocate_exact_size(want, egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let pts = preset.control_points();
    let bg = if resp.hovered() {
        colors::BG_HOVER
    } else {
        colors::BG_SURFACE
    };
    ui.painter().rect_filled(rect, 6.0, bg);
    let border_stroke = if resp.hovered() {
        egui::Stroke::new(1.0_f32, colors::ACCENT_SELECTION)
    } else {
        egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM)
    };
    ui.painter().rect_stroke(rect, 6.0, border_stroke);
    let pad = 5.0;
    let inner = egui::Rect::from_min_max(
        rect.min + egui::vec2(pad, pad),
        rect.max - egui::vec2(pad, pad),
    );
    let w = inner.width().max(0.01);
    let h = inner.height().max(0.01);
    // Faint diagonal reference (linear)
    ui.painter().line_segment(
        [
            egui::pos2(inner.min.x, inner.max.y),
            egui::pos2(inner.max.x, inner.min.y),
        ],
        egui::Stroke::new(1.0_f32, colors::BORDER_SUBTLE),
    );
    let stroke_col = if resp.hovered() {
        colors::ACCENT_BRAND_HOVER
    } else {
        colors::ACCENT_BRAND
    };
    let mut prev: Option<egui::Pos2> = None;
    for step in 0..=16 {
        let x = step as f32 / 16.0;
        let t = crate::core::keyframe::solve_bezier_eased_time(x, pts[0], pts[1], pts[2], pts[3]);
        if !t.is_finite() {
            prev = None;
            continue;
        }
        let omt = 1.0 - t;
        let y = 3.0 * omt * omt * t * pts[1] + 3.0 * omt * t * t * pts[3] + t * t * t;
        if !y.is_finite() {
            prev = None;
            continue;
        }
        let y_clamped = y.clamp(-0.35, 1.35);
        let norm = (y_clamped + 0.35) / 1.7;
        let p = egui::pos2(inner.min.x + x * w, inner.max.y - norm * h);
        if let Some(pr) = prev {
            if resp.hovered() {
                ui.painter().line_segment(
                    [pr, p],
                    egui::Stroke::new(3.5_f32, stroke_col.gamma_multiply(0.25)),
                );
            }
            ui.painter()
                .line_segment([pr, p], egui::Stroke::new(1.8_f32, stroke_col));
        }
        prev = Some(p);
    }
    resp
}

pub fn remove_camera_key<T>(
    track: &mut Option<crate::core::property::Animatable<T>>,
    frame: u32,
) -> bool {
    if let Some(crate::core::property::Animatable::Animated(keys)) = track {
        let original_len = keys.len();
        keys.retain(|key| key.frame != frame);
        let removed = keys.len() != original_len;
        let became_empty = keys.is_empty();
        if became_empty {
            *track = None;
        }
        return removed;
    }
    false
}

pub fn set_camera_key_interpolation<T>(
    track: &mut Option<crate::core::property::Animatable<T>>,
    frame: u32,
    interpolation: crate::core::keyframe::InterpolationType,
) -> bool {
    if let Some(crate::core::property::Animatable::Animated(keys)) = track {
        if let Some(key) = keys.iter_mut().find(|key| key.frame == frame) {
            if key.interpolation != interpolation {
                key.interpolation = interpolation;
                return true;
            }
        }
    }
    false
}

pub fn set_camera_key_ease<T>(
    track: &mut Option<crate::core::property::Animatable<T>>,
    frame: u32,
    preset: crate::core::keyframe::EasePreset,
) -> bool {
    use crate::core::keyframe::{BezierControlPoint, InterpolationType};

    if let Some(crate::core::property::Animatable::Animated(keys)) = track {
        if let Some(key) = keys.iter_mut().find(|key| key.frame == frame) {
            let next = InterpolationType::Bezier {
                outgoing: BezierControlPoint {
                    influence: 0.333,
                    speed: 0.0,
                },
                incoming: BezierControlPoint {
                    influence: 0.333,
                    speed: 0.0,
                },
                custom_bezier: Some(preset.control_points()),
            };
            if key.interpolation != next {
                key.interpolation = next;
                return true;
            }
        }
    }
    false
}

pub fn compute_velocity_curve(keyframes: &[(u32, f32)], fps: u32) -> Vec<(f32, f32)> {
    if keyframes.len() < 2 {
        return keyframes
            .iter()
            .map(|&(frame, _)| (frame as f32, 0.0))
            .collect();
    }
    let mut velocities = Vec::with_capacity(keyframes.len());
    for i in 0..keyframes.len() {
        let vel = if i == 0 {
            let dt = (keyframes[1].0 as f32 - keyframes[0].0 as f32) / fps as f32;
            if dt > 0.0 {
                (keyframes[1].1 - keyframes[0].1) / dt
            } else {
                0.0
            }
        } else if i == keyframes.len() - 1 {
            let dt = (keyframes[i].0 as f32 - keyframes[i - 1].0 as f32) / fps as f32;
            if dt > 0.0 {
                (keyframes[i].1 - keyframes[i - 1].1) / dt
            } else {
                0.0
            }
        } else {
            let dt = (keyframes[i + 1].0 as f32 - keyframes[i - 1].0 as f32) / fps as f32;
            if dt > 0.0 {
                (keyframes[i + 1].1 - keyframes[i - 1].1) / dt
            } else {
                0.0
            }
        };
        velocities.push((keyframes[i].0 as f32, vel));
    }
    velocities
}
