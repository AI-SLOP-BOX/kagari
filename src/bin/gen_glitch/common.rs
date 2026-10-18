use kagari_vfx::core::keyframe::*;
use kagari_vfx::core::property::*;
use kagari_vfx::core::timeline::*;

pub const W: f32 = 1920.0;
pub const H: f32 = 1080.0;
pub const CX: f32 = 960.0;
pub const CY: f32 = 540.0;
pub const FPS: u32 = 30;
pub const DUR: u32 = 150;

// ─── deterministic build-time hash (splitmix64) ───
pub fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
pub fn hash_to_unit(a: u64, b: u64) -> f32 {
    (splitmix64(a.wrapping_mul(0x9E37_79B9).wrapping_add(b)) >> 11) as f32 / (1u64 << 53) as f32
}

pub fn lin() -> InterpolationType {
    InterpolationType::Linear
}
pub fn hold() -> InterpolationType {
    InterpolationType::Hold
}
pub fn ease_bezier(c: [f32; 4]) -> InterpolationType {
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
pub const E_SMOOTH: [f32; 4] = [0.0, 0.0, 0.4, 1.0];
pub const E_SNAP: [f32; 4] = [0.3, 0.0, 0.2, 1.0];

pub fn key_scalar(f: u32, v: f32, e: InterpolationType) -> Keyframe<f32> {
    Keyframe::new(f, v, e)
}
pub fn key_vec2(f: u32, v: [f32; 2], e: InterpolationType) -> Keyframe<[f32; 2]> {
    Keyframe::new(f, v, e)
}
pub fn const_scalar(v: f32) -> Animatable<f32> {
    Animatable::new_constant(v)
}
pub fn const_vec2(v: [f32; 2]) -> Animatable<[f32; 2]> {
    Animatable::new_constant(v)
}
pub fn anim_scalar(keys: Vec<Keyframe<f32>>) -> Animatable<f32> {
    Animatable::new_animated(keys)
}
pub fn anim_vec2(keys: Vec<Keyframe<[f32; 2]>>) -> Animatable<[f32; 2]> {
    Animatable::new_animated(keys)
}
pub fn make_effect(id: &str, name: &str, et: EffectType) -> Effect {
    Effect {
        id: id.into(),
        name: name.into(),
        effect_type: et,
        enabled: true,
    }
}

// ─── title layout ───
pub struct CharSpec {
    pub ch: char,
    pub home: [f32; 2],
    pub color: [f32; 4],
    pub size: u32,
    pub land: u32, // frame the char locks home
}

pub fn char_specs() -> Vec<CharSpec> {
    let mut out = Vec::new();
    let word1: Vec<char> = "KAGARI".chars().collect();
    for (i, ch) in word1.iter().enumerate() {
        out.push(CharSpec {
            ch: *ch,
            home: [CX + (i as f32 - 2.5) * 128.0 + 46.0, 482.0],
            color: [1.0, 1.0, 1.0, 1.0],
            size: 168,
            land: 48 + i as u32 * 4,
        });
    }
    let word2: Vec<char> = "VFX".chars().collect();
    for (j, ch) in word2.iter().enumerate() {
        out.push(CharSpec {
            ch: *ch,
            home: [CX + (j as f32 - 1.0) * 104.0 + 8.0, 667.0],
            color: [1.0, 0.72, 0.25, 1.0],
            size: 132,
            land: 64 + j as u32 * 4,
        });
    }
    out
}
