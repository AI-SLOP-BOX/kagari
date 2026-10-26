//! CPU pixel-effect pipeline.
//!
//! This module is the bridge that finally wires the large library of orphaned
//! `ae_effects_pack*` image kernels into the render pipeline. The GPU path
//! (`effect_plugin` -> `EffectParams` -> WGSL) handles the live viewport; this
//! CPU path applies the same logical effects to a layer's rasterized RGBA
//! buffer so effects are also visible in the software renderer used for frame
//! export and CPU preview, and so the effect kernels are actually used.
//!
//! Each `EffectType` maps to one or more `ae_effects_pack::apply_*` kernels.

use crate::core::timeline::{Effect, EffectType};

mod advanced_effects;
mod audio_effects;
mod compositing_effects;
mod distortion_effects;
mod ext_effects;
mod gpu_mirrored;
mod layer_effects;
mod procedural_effects;
mod standalone_effects;
mod utility_effects;

/// Convert a color (0..1 floats) into an `[u8; 4]` (0..255).
fn color_to_u8(c: [f32; 4]) -> [u8; 4] {
    [
        (c[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (c[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (c[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        (c[3].clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

/// Convert a color (0..1 floats) into a 3-channel `[u8; 3]`.
fn color3_to_u8(c: [f32; 4]) -> [u8; 3] {
    let c = color_to_u8(c);
    [c[0], c[1], c[2]]
}

/// Apply every enabled effect on a layer to its already-rasterized RGBA buffer.
///
/// `pixels` is a straight (non-premultiplied) RGBA8 buffer of size
/// `width*height*4`. Effects are applied in order, mirroring After Effects'
/// top-to-bottom effect stack within a layer.
#[allow(clippy::too_many_arguments)]
pub fn apply_layer_effects(
    comp: Option<&crate::core::timeline::Composition>,
    layer_idx: Option<usize>,
    pixels: &mut [u8],
    width: u32,
    height: u32,
    effects: &[Effect],
    frame: u32,
    fps: u32,
) {
    apply_layer_effects_with_context(
        pixels,
        effects,
        LayerEffectRenderContext {
            comp,
            layer_idx,
            width,
            height,
            frame,
            fps,
            light_screen: None,
        },
    );
}

/// Like [`apply_layer_effects`], with an optional pre-projected light position
/// (normalized 0..1) for lens flares that link to a comp light.
#[allow(clippy::too_many_arguments)]
pub fn apply_layer_effects_ctx(
    comp: Option<&crate::core::timeline::Composition>,
    layer_idx: Option<usize>,
    pixels: &mut [u8],
    width: u32,
    height: u32,
    effects: &[Effect],
    frame: u32,
    fps: u32,
    light_screen: Option<[f32; 2]>,
) {
    apply_layer_effects_with_context(
        pixels,
        effects,
        LayerEffectRenderContext {
            comp,
            layer_idx,
            width,
            height,
            frame,
            fps,
            light_screen,
        },
    );
}

#[derive(Clone, Copy)]
pub struct LayerEffectRenderContext<'a> {
    pub comp: Option<&'a crate::core::timeline::Composition>,
    pub layer_idx: Option<usize>,
    pub width: u32,
    pub height: u32,
    pub frame: u32,
    pub fps: u32,
    pub light_screen: Option<[f32; 2]>,
}

pub fn apply_layer_effects_with_context(
    pixels: &mut [u8],
    effects: &[Effect],
    context: LayerEffectRenderContext<'_>,
) {
    let LayerEffectRenderContext { width, height, .. } = context;
    let Some(expected_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
    else {
        return;
    };
    if pixels.len() != expected_len {
        return;
    }

    for effect in effects {
        if !effect.enabled {
            continue;
        }
        apply_one_ctx(pixels, &effect.effect_type, context);
    }
}

fn apply_one_ctx(
    pixels: &mut [u8],
    effect_type: &EffectType,
    context: LayerEffectRenderContext<'_>,
) {
    if gpu_mirrored::apply(pixels, effect_type, context)
        || audio_effects::apply(pixels, effect_type, context)
        || standalone_effects::apply(pixels, effect_type, context)
        || compositing_effects::apply(pixels, effect_type, context)
        || distortion_effects::apply(pixels, effect_type, context)
        || ext_effects::apply(pixels, effect_type, context)
        || utility_effects::apply(pixels, effect_type, context)
        || procedural_effects::apply(pixels, effect_type, context)
        || layer_effects::apply(pixels, effect_type, context)
        || advanced_effects::apply(pixels, effect_type, context)
    {
        return;
    }

    let LayerEffectRenderContext {
        comp,
        layer_idx,
        width,
        height,
        frame,
        fps: _,
        light_screen: _,
    } = context;
    use crate::core::ae_effects_pack as pack;

    match effect_type {
        // New CPU-only kernels (no GPU equivalent yet).
        EffectType::Twirl { angle, radius } => {
            let a = angle.evaluate(frame).clamp(-7200.0, 7200.0);
            let r = radius.evaluate(frame).clamp(1.0, 10000.0);
            let cx = width as f32 * 0.5;
            let cy = height as f32 * 0.5;
            if !crate::core::compute_pipeline::try_gpu_twirl(pixels, width, height, a, cx, cy) {
                pack::apply_twirl(pixels, width, height, a, r);
            }
        }
        EffectType::Bulge { amount, radius } => {
            let amt = amount.evaluate(frame).clamp(-1000.0, 1000.0);
            let r = radius.evaluate(frame).clamp(1.0, 10000.0);
            let cx = width as f32 * 0.5;
            let cy = height as f32 * 0.5;
            if !crate::core::compute_pipeline::try_gpu_bulge(pixels, width, height, amt, cx, cy) {
                pack::apply_bulge(pixels, width, height, amt, r);
            }
        }
        EffectType::Posterize { levels } => {
            let lv = levels.evaluate(frame).clamp(2.0, 256.0) as u32;
            if !crate::core::compute_pipeline::try_gpu_posterize(pixels, width, height, lv as f32) {
                pack::apply_posterize(pixels, lv);
            }
        }
        EffectType::Invert { invert_alpha } => {
            if !crate::core::compute_pipeline::try_gpu_invert(pixels, width, height) {
                pack::apply_invert(pixels, *invert_alpha);
            }
        }
        EffectType::Offset { shift_x, shift_y } => {
            let sx = shift_x.evaluate(frame) as i32;
            let sy = shift_y.evaluate(frame) as i32;
            if !crate::core::compute_pipeline::try_gpu_offset(pixels, width, height, sx, sy) {
                pack::apply_offset(pixels, width, height, sx, sy);
            }
        }
        EffectType::DirectionalBlur { angle, length } => {
            let a = angle.evaluate(frame);
            let len = length.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_directional_blur(
                pixels,
                width,
                height,
                len.round().clamp(2.0, 256.0) as u32,
                a,
            ) {
                pack::apply_directional_blur(pixels, width, height, a, len);
            }
        }
        EffectType::RadialBlur { amount } => {
            let amt = amount.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_radial_blur(
                pixels,
                width,
                height,
                (amt * 2.0).round().clamp(2.0, 256.0) as u32,
            ) {
                pack::apply_radial_blur(pixels, width, height, amt);
            }
        }
        EffectType::Sharpen { amount } => {
            let a = amount.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_sharpen(pixels, width, height, a) {
                pack::apply_sharpen(pixels, width, height, a);
            }
        }
        EffectType::Threshold { threshold } => {
            let t = threshold.evaluate(frame).clamp(0.0, 255.0);
            if !crate::core::compute_pipeline::try_gpu_threshold(pixels, width, height, t / 255.0) {
                pack::apply_threshold(pixels, t as u8);
            }
        }
        EffectType::LinearWipe { completion, angle } => {
            let c = completion.evaluate(frame).clamp(0.0, 100.0) / 100.0;
            let a = angle.evaluate(frame).to_radians();
            if !crate::core::compute_pipeline::try_gpu_linear_wipe(
                pixels, width, height, c, a, 0.01,
            ) {
                pack::apply_linear_wipe(pixels, width, height, c * 100.0, a);
            }
        }
        EffectType::SimpleChoker { choke_amount } => {
            pack::apply_simple_choker(pixels, choke_amount.evaluate(frame));
        }

        // Effects with CPU kernels: dispatch to cpu_effects_new
        EffectType::ChromaticAberration {
            shift_r,
            shift_b,
            edge_falloff,
            iris_linked,
        } => {
            let mut sr = shift_r.evaluate(frame);
            let mut sb = shift_b.evaluate(frame);
            if *iris_linked {
                let (aperture, iris_sides) = comp
                    .map(|c| {
                        let cam = c.resolve_camera_at();
                        (cam.aperture, cam.dof_iris_sides)
                    })
                    .unwrap_or((2.8, 8));
                let coc = (2.8 / aperture.max(0.5)).clamp(0.1, 4.0);
                let blade_scale = if iris_sides > 0 {
                    (iris_sides as f32 / 8.0).clamp(0.3, 1.0)
                } else {
                    0.625
                };
                let mut lens_scale = coc * blade_scale;
                let amp = crate::core::expression_engine::get_audio_amplitude();
                let bass = crate::core::expression_engine::get_audio_band(0);
                let treble = crate::core::expression_engine::get_audio_band(4);
                let audio_scale = (1.0 + amp * 0.6 + bass * 0.4).clamp(0.2, 3.0);
                lens_scale *= audio_scale;
                let blade_jitter = if treble > 0.3 {
                    1.0 + (treble - 0.3) * 0.5
                } else {
                    1.0
                };
                lens_scale *= blade_jitter;
                sr *= lens_scale;
                sb *= lens_scale;
            }
            let avg_amount = (sr.abs() + sb.abs()) * 0.5;
            if !crate::core::compute_pipeline::try_gpu_chromatic_aberration(
                pixels, width, height, avg_amount,
            ) {
                crate::core::cpu_effects_new::apply_chromatic_aberration(
                    pixels,
                    width,
                    height,
                    sr,
                    sb,
                    edge_falloff.evaluate(frame),
                );
            }
        }
        EffectType::Vignette {
            intensity,
            roundness,
            feather,
            color,
        } => {
            // Slider units are 0..100 like Glow; the kernel works in 0..1.
            let i = (intensity.evaluate(frame) / 100.0).clamp(0.0, 1.0);
            let r = roundness.evaluate(frame);
            let f = (feather.evaluate(frame) / 100.0).clamp(0.0, 1.0);
            if !crate::core::compute_pipeline::try_gpu_vignette(pixels, width, height, r, f) {
                crate::core::cpu_effects_new::apply_vignette(
                    pixels,
                    width,
                    height,
                    i,
                    r,
                    f,
                    color.evaluate(frame),
                );
            }
        }
        EffectType::Levels {
            input_black,
            input_white,
            gamma,
            output_black,
            output_white,
        } => {
            let ib = input_black.evaluate(frame);
            let iw = input_white.evaluate(frame);
            let g = gamma.evaluate(frame);
            let ob = output_black.evaluate(frame);
            let ow = output_white.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_levels(
                pixels, width, height, ib, iw, g, ob, ow,
            ) {
                crate::core::cpu_effects_new::apply_levels(
                    pixels, width, height, ib, iw, g, ob, ow,
                );
            }
        }
        EffectType::HueSaturation {
            hue_shift,
            saturation,
            lightness,
        } => {
            let h = hue_shift.evaluate(frame);
            let s = saturation.evaluate(frame);
            let l = lightness.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_hue_saturation(
                pixels, width, height, h, s, l,
            ) {
                crate::core::cpu_effects_new::apply_hue_saturation(pixels, width, height, h, s, l);
            }
        }
        EffectType::MotionBlur {
            shutter_angle,
            samples,
        } => {
            crate::core::cpu_effects_new::apply_motion_blur(
                pixels,
                width,
                height,
                shutter_angle.evaluate(frame),
                *samples as f32,
            );
        }
        EffectType::MeshWarp {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        } => {
            crate::core::cpu_effects_new::apply_mesh_warp(
                pixels,
                width,
                height,
                top_left.evaluate(frame),
                top_right.evaluate(frame),
                bottom_left.evaluate(frame),
                bottom_right.evaluate(frame),
            );
        }
        EffectType::CornerPin {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        } => {
            let quad = crate::core::corner_pin::CornerPinQuad {
                top_left: top_left.evaluate(frame),
                top_right: top_right.evaluate(frame),
                bottom_right: bottom_right.evaluate(frame),
                bottom_left: bottom_left.evaluate(frame),
            };
            let mut out = vec![0u8; pixels.len()];
            crate::core::corner_pin::apply_corner_pin_warp(
                pixels, width, height, &mut out, width, height, &quad,
            );
            pixels.copy_from_slice(&out);
        }
        EffectType::ColorGradeLUT {
            lut_path,
            intensity,
        } => {
            let intensity = intensity.evaluate(frame).clamp(0.0, 1.0);
            if !lut_path.is_empty() && intensity > 0.0 {
                use std::cell::RefCell;
                thread_local! { static LUT_CACHE: RefCell<crate::core::lut_cache::LutCache> = RefCell::new(crate::core::lut_cache::LutCache::new(512)); }
                if let Ok(text) = std::fs::read_to_string(lut_path) {
                    if let Ok(lut) = crate::core::ocio_color::Lut3D::parse_cube(&text) {
                        for p in pixels.as_chunks_mut::<4>().0 {
                            let r = p[0] as f32 / 255.0;
                            let g = p[1] as f32 / 255.0;
                            let b = p[2] as f32 / 255.0;
                            let (lr, lg, lb) = LUT_CACHE.with(|cache| {
                                let mut c = cache.borrow_mut();
                                c.get_or_insert(r, g, b, lut.size, |r, g, b| lut.apply(r, g, b))
                            });
                            let t = intensity;
                            p[0] = ((r * (1.0 - t) + lr * t).clamp(0.0, 1.0) * 255.0).round() as u8;
                            p[1] = ((g * (1.0 - t) + lg * t).clamp(0.0, 1.0) * 255.0).round() as u8;
                            p[2] = ((b * (1.0 - t) + lb * t).clamp(0.0, 1.0) * 255.0).round() as u8;
                        }
                    } else {
                        log::warn!("ColorGradeLUT: failed to parse LUT from {}", lut_path);
                    }
                } else {
                    log::warn!("ColorGradeLUT: cannot read LUT file {}", lut_path);
                }
            }
        }
        EffectType::ColorSpaceConvert { mode } => {
            let m = *mode as u32;
            if !crate::core::compute_pipeline::try_gpu_color_space_convert(pixels, width, height, m)
            {
                crate::core::cpu_effects_new::apply_color_space_convert(pixels, width, height, m);
            }
        }
        EffectType::FilmGrain {
            intensity,
            grain_size,
            color_film: _,
        } => {
            let inten = intensity.evaluate(frame);
            let seed = frame as f32;
            if !crate::core::compute_pipeline::try_gpu_film_grain(
                pixels, width, height, inten, seed,
            ) {
                crate::core::cpu_effects_new::apply_film_grain(
                    pixels,
                    width,
                    height,
                    inten,
                    *grain_size as u32,
                    frame,
                );
            }
        }
        EffectType::FractalNoise {
            fractal_type,
            contrast,
            brightness,
            complexity,
            evolution,
        } => {
            let br = brightness.evaluate(frame);
            let evo = evolution.evaluate(frame);
            let blend = contrast.evaluate(frame);
            if !crate::core::compute_pipeline::try_gpu_fractal_noise(
                pixels, width, height, br, evo, blend,
            ) {
                crate::core::cpu_effects_new::apply_fractal_noise(
                    pixels,
                    width,
                    height,
                    fractal_type.evaluate(frame),
                    contrast.evaluate(frame),
                    br,
                    complexity.evaluate(frame),
                    evo,
                );
            }
        }
        EffectType::Curves { channel } => {
            crate::core::cpu_effects_new::apply_curves(
                pixels,
                width,
                height,
                channel.evaluate(frame),
            );
        }
        EffectType::DisplacementMap {
            source_layer,
            max_horizontal,
            max_vertical,
        } => {
            let source_idx = source_layer.evaluate(frame).round();
            let source = comp.and_then(|composition| {
                if !source_idx.is_finite() || source_idx < 0.0 {
                    return None;
                }
                let index = source_idx as usize;
                (index < composition.layers.len() && Some(index) != layer_idx).then(|| {
                    crate::core::software_renderer::render_single_layer_pixels(
                        composition,
                        index,
                        frame,
                        width,
                        height,
                    )
                })
            });
            if let Some(source_pixels) = source {
                let options = crate::core::displacement_map::DisplacementMapOptions {
                    max_horizontal_displacement: max_horizontal.evaluate(frame),
                    max_vertical_displacement: max_vertical.evaluate(frame),
                    horizontal_channel:
                        crate::core::displacement_map::DisplacementChannel::Luminance,
                    vertical_channel: crate::core::displacement_map::DisplacementChannel::Luminance,
                    wrap_pixels: false,
                };
                let displaced = crate::core::displacement_map::apply_displacement_map(
                    pixels,
                    &source_pixels,
                    width,
                    height,
                    &options,
                );
                pixels.copy_from_slice(&displaced);
            }
        }
        EffectType::CompoundBlur {
            source_layer,
            max_blur,
        } => {
            let source_idx = source_layer.evaluate(frame).round();
            let source = comp.and_then(|composition| {
                if !source_idx.is_finite() || source_idx < 0.0 {
                    return None;
                }
                let index = source_idx as usize;
                (index < composition.layers.len() && Some(index) != layer_idx).then(|| {
                    crate::core::software_renderer::render_single_layer_pixels(
                        composition,
                        index,
                        frame,
                        width,
                        height,
                    )
                })
            });
            if let Some(source_pixels) = source {
                crate::core::cpu_effects_new::apply_compound_blur_with_map(
                    pixels,
                    width,
                    height,
                    Some(&source_pixels),
                    max_blur.evaluate(frame),
                );
            }
        }
        EffectType::Minimax { operation, radius } => {
            let op = operation.evaluate(frame);
            let r = radius.evaluate(frame).clamp(0.0, 100.0) as u32;
            let maximize = op > 0.5;
            if !crate::core::compute_pipeline::try_gpu_minimax(pixels, width, height, r, maximize) {
                crate::core::cpu_effects_new::apply_minimax(pixels, width, height, op, r as f32);
            }
        }
        EffectType::ShiftChannels {
            take_red,
            take_green,
            take_blue,
            take_alpha,
        } => {
            let r = take_red.evaluate(frame).clamp(0.0, 3.0) as u32;
            let g = take_green.evaluate(frame).clamp(0.0, 3.0) as u32;
            let b = take_blue.evaluate(frame).clamp(0.0, 3.0) as u32;
            let a = take_alpha.evaluate(frame) as u32;
            if !crate::core::compute_pipeline::try_gpu_shift_channels(
                pixels, width, height, r, g, b, a,
            ) {
                crate::core::cpu_effects_new::apply_shift_channels(
                    pixels, width, height, r as f32, g as f32, b as f32, a as f32,
                );
            }
        }

        // ── Effects migrated from ExtEffect ──
        _ => unreachable!("GPU and audio effects are handled before CPU-only dispatch"),
    }
}

#[cfg(test)]
mod tests;
