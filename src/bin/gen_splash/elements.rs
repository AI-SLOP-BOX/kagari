use kagari_vfx::core::keyframe::*;
use kagari_vfx::core::property::*;
use kagari_vfx::core::timeline::*;

pub fn bez_in(inf: f32) -> InterpolationType {
    InterpolationType::Bezier {
        outgoing: BezierControlPoint {
            influence: 0.0,
            speed: 0.0,
        },
        incoming: BezierControlPoint {
            influence: inf,
            speed: 0.0,
        },
        custom_bezier: None,
    }
}
pub fn bez_out(inf: f32) -> InterpolationType {
    InterpolationType::Bezier {
        outgoing: BezierControlPoint {
            influence: inf,
            speed: 0.0,
        },
        incoming: BezierControlPoint {
            influence: 0.0,
            speed: 0.0,
        },
        custom_bezier: None,
    }
}
pub fn bez_both(a: f32, b: f32) -> InterpolationType {
    InterpolationType::Bezier {
        outgoing: BezierControlPoint {
            influence: a,
            speed: 0.0,
        },
        incoming: BezierControlPoint {
            influence: b,
            speed: 0.0,
        },
        custom_bezier: None,
    }
}

pub fn dot_layer(comp: &mut Composition, name: &str, color: [f32; 4], size: f32) {
    comp.add_layer(Layer::new(
        name.into(),
        name.into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(size),
                height: Animatable::new_constant(size),
            },
            color,
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        150,
    ));
}

/// Google式: 4つの残り火ドットが楕円軌道を回りながら中心へ吸い込まれる
#[allow(clippy::too_many_arguments)]
pub fn add_orbit_dot(
    comp: &mut Composition,
    name: &str,
    color: [f32; 4],
    size: f32,
    phase_deg: f32,
    r0: f32,
    r1: f32,
    f0: u32,
    f1: u32,
) {
    dot_layer(comp, name, color, size);
    let l = comp.layers.last_mut().unwrap();
    let mut pos_kf = vec![Keyframe::new(0, [960.0, 480.0], InterpolationType::Linear)];
    let mut t = f0;
    while t <= f1 {
        let k = (t - f0) as f32 / (f1 - f0).max(1) as f32;
        let ang = (phase_deg + k * 450.0) * std::f32::consts::PI / 180.0;
        let r = r0 + (r1 - r0) * k * k;
        let x = 960.0 + ang.cos() * r;
        let y = 480.0 + ang.sin() * r * 0.62;
        let interp = if t == f1 {
            bez_in(0.6)
        } else {
            InterpolationType::Linear
        };
        pos_kf.push(Keyframe::new(t, [x, y], interp));
        t += 4;
    }
    l.transform.position = Animatable::new_animated(pos_kf);
    l.transform.scale = Animatable::new_animated(vec![
        Keyframe::new(0, [0.0, 0.0], InterpolationType::Linear),
        Keyframe::new(f0, [60.0, 60.0], InterpolationType::Linear),
        Keyframe::new(f1, [130.0, 130.0], bez_in(0.5)),
        Keyframe::new(f1 + 4, [0.0, 0.0], InterpolationType::Linear),
    ]);
    l.transform.opacity = Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(f0, 0.0, InterpolationType::Linear),
        Keyframe::new(f0 + 3, 100.0, InterpolationType::Linear),
        Keyframe::new(f1, 100.0, InterpolationType::Linear),
        Keyframe::new(f1 + 4, 0.0, InterpolationType::Linear),
    ]);
}

/// Netflix式の縦光柱
pub fn add_pillar(comp: &mut Composition, appear: u32, peak: u32, fade: u32) {
    comp.add_layer(Layer::new(
        "pillar".into(),
        "Light Pillar".into(),
        LayerType::Shape {
            shape_type: ShapeType::FreeformBezier {
                points: vec![[-10.0, 10.0], [10.0, 10.0], [5.0, -760.0], [-5.0, -760.0]],
                tangents: vec![([0.0, 0.0], [0.0, 0.0]); 4],
                closed: true,
            },
            color: [1.0, 0.85, 0.45, 0.85],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        150,
    ));
    let l = comp.layers.last_mut().unwrap();
    l.transform.position = Animatable::new_constant([960.0, 480.0]);
    l.transform.scale = Animatable::new_animated(vec![
        Keyframe::new(0, [0.0, 20.0], InterpolationType::Linear),
        Keyframe::new(appear, [0.0, 20.0], InterpolationType::Linear),
        Keyframe::new(peak, [100.0, 100.0], bez_out(0.5)),
    ]);
    l.transform.opacity = Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(appear, 0.0, InterpolationType::Linear),
        Keyframe::new(peak, 45.0, InterpolationType::Linear),
        Keyframe::new(fade, 0.0, InterpolationType::Linear),
    ]);
}

/// Netflix式: ロゴの縁から光の筋が左右へ分解していく
pub fn add_edge_streak(comp: &mut Composition, name: &str, dir: f32, appear: u32, len: f32) {
    comp.add_layer(Layer::new(
        name.into(),
        name.into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(len),
                height: Animatable::new_constant(3.0),
            },
            color: [1.0, 0.8, 0.35, 0.8],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        150,
    ));
    let l = comp.layers.last_mut().unwrap();
    let ex = 960.0 + dir * 420.0;
    l.transform.position = Animatable::new_animated(vec![
        Keyframe::new(0, [960.0, 560.0], InterpolationType::Linear),
        Keyframe::new(appear, [960.0, 560.0], InterpolationType::Linear),
        Keyframe::new(appear + 12, [ex, 560.0], bez_out(0.4)),
    ]);
    l.transform.scale = Animatable::new_animated(vec![
        Keyframe::new(0, [0.0, 100.0], InterpolationType::Linear),
        Keyframe::new(appear, [0.0, 100.0], InterpolationType::Linear),
        Keyframe::new(appear + 6, [70.0, 100.0], bez_out(0.4)),
    ]);
    l.transform.opacity = Animatable::new_animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(appear, 0.0, InterpolationType::Linear),
        Keyframe::new(appear + 3, 60.0, InterpolationType::Linear),
        Keyframe::new(appear + 14, 0.0, InterpolationType::Linear),
    ]);
}

