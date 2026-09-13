//! One-click demo composition so first-time users instantly see the app
//! rendering animated content instead of an empty void.
use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::particle_system::ParticleEmitter;
use crate::core::property::Animatable;
use crate::core::timeline::{Composition, Effect, EffectType, Expression, Layer, LayerType, ShapeType};

fn kf(frame: u32, v: f32) -> Keyframe<f32> {
    Keyframe::new(frame, v, InterpolationType::Linear)
}
fn kfv2(frame: u32, v: [f32; 2]) -> Keyframe<[f32; 2]> {
    Keyframe::new(frame, v, InterpolationType::Linear)
}

fn fx(id: &str, name: &str, effect_type: EffectType) -> Effect {
    Effect {
        id: id.into(),
        name: name.into(),
        effect_type,
        enabled: true,
    }
}

fn c(v: f32) -> Animatable<f32> {
    Animatable::new_constant(v)
}

pub fn build(app: &mut crate::KagariApp) {
    let count = app.history.current().compositions.len();
    let mut comp = Composition::new(
        format!("comp_demo_{}", count),
        "🎬 Demo Scene".to_string(),
        1280,
        720,
        30,
        150,
    );
    comp.blend_linear = false;
    comp.dither_output = true;

    // ── Background with vignette (positioned at comp center so the layer
    // buffer spans the full frame) ──
    let mut bg = Layer::new(
        "demo_bg".into(),
        "Background".into(),
        LayerType::Solid {
            color: [0.04, 0.05, 0.10, 1.0],
        },
        comp.duration_frames,
    );
    bg.transform.position = Animatable::new_constant([640.0, 360.0]);
    bg.effects.push(fx(
        "demo_vignette",
        "Vignette",
        EffectType::Vignette {
            intensity: c(45.0),
            roundness: c(0.55),
            feather: c(60.0),
            color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
        },
    ));
    comp.layers.push(bg);

    // ── Floating embers (additive sparks drifting upward) ──
    let mut embers = Layer::new(
        "demo_embers".into(),
        "Embers".into(),
        LayerType::Particle {
            emitter: ParticleEmitter {
                rate: 90.0,
                max_particles: 1200,
                lifetime: 3.0,
                speed: 90.0,
                speed_variance: 0.6,
                spread_degrees: 360.0,
                gravity: [0.0, -60.0],
                turbulence: 40.0,
                color_start: [1.0, 0.62, 0.15, 1.0],
                color_end: [1.0, 0.15, 0.0, 0.0],
                size_start: 7.0,
                size_end: 1.5,
                ..Default::default()
            },
        },
        comp.duration_frames,
    );
    embers.transform.position = Animatable::new_constant([640.0, 520.0]);
    comp.layers.push(embers);

    // ── Accent orb: scale bounce + drift + pulsing glow ──
    // NOTE: shape width/height are in units where 200 spans the full layer
    // buffer, so 28 ≈ a 180px orb on this 1280-wide comp.
    let mut circle = Layer::new(
        "demo_circle".into(),
        "Accent Orb".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(28.0),
                height: Animatable::new_constant(28.0),
            },
            color: [0.0, 0.64, 1.0, 1.0],
            stroke_color: [1.0, 1.0, 1.0, 1.0],
            stroke_width: 0.0,
            fill_type: Default::default(),
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        comp.duration_frames,
    );
    circle.transform.position =
        Animatable::new_animated(vec![kfv2(0, [320.0, 500.0]), kfv2(150, [960.0, 220.0])]);
    circle.transform.position.easy_ease();
    circle.transform.scale = Animatable::new_animated(vec![
        kfv2(0, [0.0, 0.0]),
        kfv2(20, [115.0, 115.0]),
        kfv2(35, [100.0, 100.0]),
    ]);
    circle.transform.rotation_expression = Some(Expression::Raw("time * 60".into()));
    circle.effects.push(fx(
        "demo_glow",
        "Glow",
        EffectType::Glow {
            threshold: c(30.0),
            radius: c(28.0),
            intensity: Animatable::new_animated(vec![kf(0, 35.0), kf(75, 70.0), kf(150, 35.0)]),
            color: Animatable::new_constant([0.45, 0.75, 1.0, 1.0]),
        },
    ));
    comp.layers.push(circle);

    // ── Counter-rotating stroke ring (≈300px: 47 units) ──
    let mut ring = Layer::new(
        "demo_ring".into(),
        "Orbit Ring".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(47.0),
                height: Animatable::new_constant(47.0),
            },
            color: [0.0, 0.0, 0.0, 0.0],
            stroke_color: [1.0, 1.0, 1.0, 1.0],
            stroke_width: 5.0,
            fill_type: Default::default(),
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        comp.duration_frames,
    );
    ring.transform.position =
        Animatable::new_animated(vec![kfv2(0, [320.0, 500.0]), kfv2(150, [960.0, 220.0])]);
    ring.transform.position.easy_ease();
    ring.transform.rotation_expression = Some(Expression::Raw("time * -30".into()));
    ring.transform.opacity = Animatable::new_animated(vec![kf(0, 0.0), kf(40, 80.0)]);
    comp.layers.push(ring);

    // ── Title: fade + rise + soft glow ──
    let mut title = Layer::new(
        "demo_title".into(),
        "Title".into(),
        LayerType::new_text("KAGARI VFX", 88, [0.95, 0.96, 1.0, 1.0]),
        comp.duration_frames,
    );
    title.transform.position =
        Animatable::new_animated(vec![kfv2(10, [640.0, 400.0]), kfv2(50, [640.0, 350.0])]);
    title.transform.opacity = Animatable::new_animated(vec![kf(0, 0.0), kf(35, 100.0)]);
    title.transform.opacity.easy_ease();
    title.effects.push(fx(
        "demo_title_glow",
        "Glow",
        EffectType::Glow {
            threshold: c(60.0),
            radius: c(12.0),
            intensity: c(25.0),
            color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
        },
    ));
    comp.layers.push(title);

    // ── Subtitle ──
    let mut sub = Layer::new(
        "demo_sub".into(),
        "Subtitle".into(),
        LayerType::new_text("Rust • GPU • Open Source", 34, [0.55, 0.75, 1.0, 1.0]),
        comp.duration_frames,
    );
    sub.transform.opacity = Animatable::new_animated(vec![kf(25, 0.0), kf(60, 90.0)]);
    sub.transform.opacity.easy_ease();
    sub.transform.position = Animatable::new_constant([640.0, 430.0]);
    comp.layers.push(sub);

    let proj = app.history.current_mut();
    proj.compositions.push(comp);
    proj.active_composition_idx = proj.compositions.len() - 1;
    crate::core::frame_cache::bump_version();
    app.toasts.info("Demo scene loaded — press Space to play!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_scene_renders_nontrivial_frame() {
        let mut app = crate::KagariApp::default();
        build(&mut app);
        let comp = app.history.current().active_composition().clone();
        assert_eq!(comp.layers.len(), 6);
        let pixels = crate::core::software_renderer::render_frame_to_pixels(
            &comp, 75, 320, 180, 0.0, 0,
        );
        assert_eq!(pixels.len(), 320 * 180 * 4);
        // Must be more than a flat background: orb, ring, particles, text.
        let distinct: std::collections::HashSet<[u8; 3]> = pixels
            .chunks_exact(4)
            .map(|p| [p[0], p[1], p[2]])
            .collect();
        assert!(
            distinct.len() > 16,
            "demo frame looks flat: {} colors",
            distinct.len()
        );
        // Deterministic: same input renders byte-identical pixels.
        let again = crate::core::software_renderer::render_frame_to_pixels(
            &comp, 75, 320, 180, 0.0, 0,
        );
        assert_eq!(pixels, again);
    }
}
