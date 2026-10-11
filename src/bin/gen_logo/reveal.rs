use super::geometry::*;
use super::still::{apart, c1, c2, ring_layer};
use kagari_vfx::core::keyframe::*;
use kagari_vfx::core::particle_system::{EmitterShape, FadeCurve, ParticleEmitter};
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::timeline::*;

pub const R_CTR: [f32; 2] = [960.0, 430.0];
pub const R_RING: f32 = 303.0;
pub const RING_U: f32 = 140.0;

pub fn ez4(c: [f32; 4]) -> InterpolationType {
    InterpolationType::Bezier {
        outgoing: BezierControlPoint {
            influence: 0.33,
            speed: 0.0,
        },
        incoming: BezierControlPoint {
            influence: 0.33,
            speed: 0.0,
        },
        custom_bezier: Some(c),
    }
}

pub const E_FASTIN: [f32; 4] = [0.55, 0.0, 1.0, 1.0];
pub const E_SMOOTH: [f32; 4] = [0.0, 0.0, 0.4, 1.0];
pub const E_SNAP: [f32; 4] = [0.3, 0.0, 0.2, 1.0];
pub const E_HEAVY: [f32; 4] = [0.5, 0.0, 0.3, 1.0];
pub const E_DRIFT: [f32; 4] = [0.37, 0.0, 0.63, 1.0];

pub fn k1(f: u32, v: f32, e: InterpolationType) -> Keyframe<f32> {
    Keyframe::new(f, v, e)
}

pub fn k2(f: u32, v: [f32; 2], e: InterpolationType) -> Keyframe<[f32; 2]> {
    Keyframe::new(f, v, e)
}

/// Position on the ring for a trim percentage (energy comet follow-path).
pub fn comet_pos(trim_pct: f32) -> [f32; 2] {
    let th = trim_pct * 3.6 * std::f32::consts::PI / 180.0;
    [R_CTR[0] + R_RING * th.cos(), R_CTR[1] + R_RING * th.sin()]
}

/// Cheap glow: radial-gradient halo with its own opacity/scale envelopes.
#[allow(clippy::too_many_arguments)]
pub fn halo(
    comp: &mut Composition,
    id: &str,
    size: f32,
    radius: f32,
    core: [f32; 4],
    pos: Animatable<[f32; 2]>,
    scale: Animatable<[f32; 2]>,
    opacity: Animatable<f32>,
    dur: u32,
) {
    comp.add_layer(Layer::new(
        id.into(),
        id.into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(size),
                height: Animatable::new_constant(size),
            },
            color: [1.0; 4],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: ShapeFillType::RadialGradient {
                center: [0.0, 0.0],
                radius,
                colors: vec![core, [0.0, 0.0, 0.0, 0.0]],
                stops: vec![0.0, 1.0],
            },
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        dur,
    ));
    let l = comp.layers.last_mut().unwrap();
    l.transform.position = pos;
    l.transform.scale = scale;
    l.transform.opacity = opacity;
}

/// Bright traveling window on the ring (energy pulse): thin white-gold arc.
pub fn pulse_layer(
    comp: &mut Composition,
    id: &str,
    start: Animatable<f32>,
    end: Animatable<f32>,
    opacity: Animatable<f32>,
    dur: u32,
) {
    comp.add_layer(Layer::new(
        id.into(),
        id.into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(RING_U),
                height: Animatable::new_constant(RING_U),
            },
            color: [0.0; 4],
            stroke_color: [1.0, 0.92, 0.75, 1.0],
            stroke_width: 5.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        dur,
    ));
    let l = comp.layers.last_mut().unwrap();
    l.transform.position = c2(R_CTR);
    l.transform.opacity = opacity;
    l.trim_paths = Some(TrimPaths {
        start,
        end,
        offset: Animatable::new_constant(0.0),
    });
}

pub fn particle_layer(
    comp: &mut Composition,
    id: &str,
    emitter: ParticleEmitter,
    pos: [f32; 2],
    opacity: Animatable<f32>,
    dur: u32,
) {
    comp.add_layer(Layer::new(
        id.into(),
        id.into(),
        LayerType::Particle { emitter },
        dur,
    ));
    let l = comp.layers.last_mut().unwrap();
    l.transform.position = c2(pos);
    l.transform.opacity = opacity;
}

pub fn base_emitter() -> ParticleEmitter {
    ParticleEmitter {
        fade_curve: FadeCurve::EaseOut,
        blend_mode: 1,
        ..Default::default()
    }
}

pub type RingDraw = (&'static str, f32, f32, [f32; 4], u32, u32);

#[path = "reveal/atmosphere.rs"]
mod atmosphere;
#[path = "reveal/flame.rs"]
mod flame;
#[path = "reveal/rings.rs"]
mod rings;
#[path = "reveal/wings.rs"]
mod wings;

pub fn build_reveal() -> Composition {
    let mut comp = Composition::new(
        "LogoReveal".into(),
        "Kagari Logo Reveal".into(),
        1920,
        1080,
        30,
        150,
    );
    let ctr = R_CTR;
    let base = 45.0;
    let lin = InterpolationType::Linear;

    atmosphere::add(&mut comp, ctr, base, lin);
    wings::add(&mut comp, ctr, base, lin);
    rings::add(&mut comp, ctr, base, lin);
    flame::add(&mut comp, ctr, base, lin);

    comp
}
