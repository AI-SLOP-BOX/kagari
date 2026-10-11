use super::common::*;
use kagari_vfx::core::keyframe::Keyframe;
use kagari_vfx::core::mask::{Mask, MaskMode, MaskPath};
use kagari_vfx::core::property::*;
use kagari_vfx::core::timeline::*;

pub fn add_glitch_text_layers(comp: &mut Composition) {
    // ---- 5. per-character text (fragmentation via layers) ----
    for (ci, spec) in char_specs().iter().enumerate() {
        let id = format!("ch{ci}");
        let sx = if hash_to_unit(ci as u64, 20) > 0.5 {
            1.0
        } else {
            -1.0
        };
        let scatter = [
            spec.home[0] + sx * (200.0 + hash_to_unit(ci as u64, 21) * 320.0),
            spec.home[1] + (hash_to_unit(ci as u64, 22) * 2.0 - 1.0) * 260.0,
        ];
        comp.add_layer(Layer::new(
            id.clone(),
            format!("char {}", spec.ch),
            LayerType::Text {
                text: spec.ch.to_string(),
                font_size: spec.size,
                color: spec.color,
                font_family: "Helvetica".into(),
                tracking: 0.0,
                leading: 1.2,
                align: 1,
                stroke_color: [0.0; 4],
                stroke_width: 0.0,
                text_on_path: false,
            },
            DUR,
        ));
        {
            let l = comp.layers.last_mut().unwrap();
            // slice-reveal mask: a slit that TRACKS the char (world coords),
            // then opens fully once it locks home
            let mut pos_keys = vec![key_vec2(0, scatter, hold())];
            let mut f = 3u32;
            while f < spec.land.saturating_sub(6) {
                let t = f as f32 / spec.land as f32;
                let amp = 1.0 - t;
                let jx = scatter[0]
                    + (spec.home[0] - scatter[0]) * t * 0.35
                    + (hash_to_unit(ci as u64, 100 + f as u64) * 2.0 - 1.0) * 60.0 * amp;
                let jy = scatter[1]
                    + (spec.home[1] - scatter[1]) * t * 0.35
                    + (hash_to_unit(ci as u64, 200 + f as u64) * 2.0 - 1.0) * 44.0 * amp;
                pos_keys.push(key_vec2(f, [jx, jy], hold()));
                f += 3;
            }
            pos_keys.push(key_vec2(spec.land, spec.home, ease_bezier(E_SMOOTH)));
            pos_keys.push(key_vec2(150, spec.home, lin()));
            // slice-reveal mask: rect corners follow the same keys as the
            // char (slit tracks it), slightly smaller so edges stay clipped
            // until the lock-open envelope releases it
            let slit = |cx: f32, cy: f32| -> Vec<[f32; 2]> {
                vec![
                    [cx - 40.0, cy - 65.0],
                    [cx + 40.0, cy - 65.0],
                    [cx + 40.0, cy + 65.0],
                    [cx - 40.0, cy + 65.0],
                ]
            };
            let mask_path_keys: Vec<Keyframe<Vec<[f32; 2]>>> = pos_keys
                .iter()
                .map(|k| Keyframe::new(k.frame, slit(k.value[0], k.value[1]), k.interpolation))
                .collect();
            l.transform.position = anim_vec2(pos_keys);
            // fragment flicker -> solid
            let mut op_keys = vec![key_scalar(0, 30.0, hold())];
            let mut f = 2u32;
            while f < 24 {
                op_keys.push(key_scalar(
                    f,
                    24.0 + hash_to_unit(ci as u64, 300 + f as u64) * 14.0,
                    hold(),
                ));
                f += 2;
            }
            let mut f = 24u32;
            while f < spec.land {
                let t = (f - 24) as f32 / (spec.land - 24).max(1) as f32;
                let v = 42.0 + t * 50.0 + (hash_to_unit(ci as u64, 400 + f as u64) - 0.5) * 26.0;
                op_keys.push(key_scalar(f, v.clamp(12.0, 96.0), hold()));
                f += 3;
            }
            op_keys.push(key_scalar(spec.land, 88.0, ease_bezier(E_SMOOTH)));
            op_keys.push(key_scalar(spec.land + 8, 100.0, lin()));
            op_keys.push(key_scalar(150, 100.0, lin()));
            l.transform.opacity = anim_scalar(op_keys);
            let mask = Mask {
                id: format!("m{ci}"),
                name: format!("mask {ci}"),
                enabled: true,
                mode: MaskMode::Add,
                path: MaskPath {
                    vertices: Animatable::new_animated(mask_path_keys),
                    tangents: None,
                    is_closed: true,
                },
                feather: const_scalar(2.0),
                opacity: const_scalar(100.0),
                expansion: anim_scalar(vec![
                    key_scalar(0, -10.0, lin()),
                    key_scalar(spec.land + 2, -10.0, lin()),
                    key_scalar(spec.land + 10, 25.0, ease_bezier(E_SMOOTH)),
                    key_scalar(150, 25.0, lin()),
                ]),
                inverted: false,
                wiggle: None,
            };
            l.masks.push(mask);
            // per-char RGB split: separated -> merged (staggered)
            let split = 10.0 + hash_to_unit(ci as u64, 23) * 10.0;
            l.effects.push(make_effect(
                &format!("ch{ci}_rgb"),
                "RGB Split",
                EffectType::RGBSplit {
                    red_offset: anim_vec2(vec![
                        key_vec2(0, [sx * split, 0.0], hold()),
                        key_vec2(spec.land.saturating_sub(8), [sx * split, 0.0], hold()),
                        key_vec2(spec.land + 4, [0.0, 0.0], ease_bezier(E_SMOOTH)),
                        key_vec2(150, [0.0, 0.0], lin()),
                    ]),
                    green_offset: const_vec2([0.0, 0.0]),
                    blue_offset: anim_vec2(vec![
                        key_vec2(0, [-sx * split, 0.0], hold()),
                        key_vec2(spec.land.saturating_sub(8), [-sx * split, 0.0], hold()),
                        key_vec2(spec.land + 4, [0.0, 0.0], ease_bezier(E_SMOOTH)),
                        key_vec2(150, [0.0, 0.0], lin()),
                    ]),
                },
            ));
            // glow follows formation with a delay, then settles low
            let glow_col = if ci < 6 {
                [1.0, 0.93, 0.8, 1.0]
            } else {
                [1.0, 0.8, 0.5, 1.0]
            };
            l.effects.push(make_effect(
                &format!("ch{ci}_glow"),
                "Glow",
                EffectType::Glow {
                    threshold: const_scalar(100.0),
                    radius: const_scalar(22.0),
                    intensity: anim_scalar(vec![
                        key_scalar(0, 0.0, lin()),
                        key_scalar(spec.land.saturating_sub(4), 0.0, lin()),
                        key_scalar(spec.land + 6, 75.0, ease_bezier(E_SNAP)),
                        key_scalar(126, 18.0, lin()),
                        key_scalar(150, 14.0, lin()),
                    ]),
                    color: Animatable::new_constant(glow_col),
                },
            ));
        }
    }
}
