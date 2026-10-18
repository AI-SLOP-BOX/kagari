use crate::core::timeline::Composition;
use rayon::prelude::*;

pub(super) fn apply_output_transform(
    comp: &Composition,
    buffer: &mut [u8],
    exposure_ev: f32,
    lut_mode: u32,
) {
    let exposure_multiplier = 2.0f32.powf(exposure_ev);
    let active_lut = (lut_mode == 3)
        .then(crate::core::ocio_color::active_lut)
        .flatten();
    buffer
        .par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(pixel_index, pixel)| {
            let mut red = pixel[0] as f32 / 255.0 * exposure_multiplier;
            let mut green = pixel[1] as f32 / 255.0 * exposure_multiplier;
            let mut blue = pixel[2] as f32 / 255.0 * exposure_multiplier;

            if comp.working_color_space != crate::core::color_science::WorkingColorSpace::Rec709 {
                let converted = crate::core::color_science::convert_color_space(
                    [red, green, blue],
                    comp.working_color_space,
                    crate::core::color_science::WorkingColorSpace::Rec709,
                );
                red = converted[0];
                green = converted[1];
                blue = converted[2];
            }

            if lut_mode == 1 {
                red = red.powf(2.2);
                green = green.powf(2.2);
                blue = blue.powf(2.2);
            } else if lut_mode == 3 {
                if let Some(lut) = active_lut.as_ref() {
                    let (r, g, b) = lut.apply(red, green, blue);
                    red = r.clamp(0.0, 1.0);
                    green = g.clamp(0.0, 1.0);
                    blue = b.clamp(0.0, 1.0);
                }
            } else if lut_mode == 2 {
                let linear = [
                    crate::core::color::srgb_to_linear_piecewise(red),
                    crate::core::color::srgb_to_linear_piecewise(green),
                    crate::core::color::srgb_to_linear_piecewise(blue),
                ];
                let output = crate::core::aces::aces_preview_transform(linear);
                red = output[0];
                green = output[1];
                blue = output[2];
            }

            let dither_seed = if comp.dither_output {
                pixel_index as f32 * 0.618_034
            } else {
                f32::NAN
            };
            let noise = (fract(dither_seed * 7.13) - fract(dither_seed * 3.71)) / 255.0;
            let quantize = |value: f32| -> u8 {
                let value = if dither_seed.is_nan() {
                    value
                } else {
                    value + noise
                };
                (value.clamp(0.0, 1.0) * 255.0).round() as u8
            };
            pixel[0] = quantize(red);
            pixel[1] = quantize(green);
            pixel[2] = quantize(blue);
        });
}

#[inline]
fn fract(value: f32) -> f32 {
    value - value.floor()
}
