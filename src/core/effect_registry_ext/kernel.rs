use crate::core::ae_effects_pack_v27::{
    apply_cc_lens_pro, apply_optics_compensation, apply_polar_coordinates_pro, apply_wave_warp_pro,
    CcLensParams, OpticsCompensationParams, PinKind, PolarMode, WaveType, WaveWarpParams,
};
use crate::core::ae_effects_pack_v28::{
    apply_cc_bend_it_pro, apply_cc_tiler_pro, apply_glow_pro, apply_light_sweep,
    apply_radial_fast_blur, LightSweepParams, TileEdgeMode,
};
use crate::core::color_correction::{
    apply_channel_mixer, apply_color_balance, apply_curves, apply_hsl_adjust, apply_vibrance,
    apply_white_balance, ChannelCurves, ChannelMixer, ColorBalance, ToneCurve,
};

use super::types::ExtEffect;

impl ExtEffect {
    /// Apply this effect to a packed RGBA8 buffer. `time` drives animated
    /// parameters (currently Wave Warp travel phase).
    pub fn apply(&self, pixels: &mut [u8], width: u32, height: u32, time: f32) {
        match self {
            ExtEffect::WaveWarp {
                wave_height,
                wave_width,
                speed,
                direction_deg,
                wave_type,
                pinning,
            } => {
                let params = WaveWarpParams {
                    wave_height: *wave_height,
                    wave_width: *wave_width,
                    speed: *speed,
                    time,
                    direction_deg: *direction_deg,
                    wave_type: match wave_type {
                        1 => WaveType::Triangle,
                        2 => WaveType::Square,
                        3 => WaveType::Sawtooth,
                        _ => WaveType::Sine,
                    },
                    pinning: match pinning {
                        1 => PinKind::LeftRight,
                        2 => PinKind::TopBottom,
                        3 => PinKind::None,
                        _ => PinKind::All,
                    },
                    ..Default::default()
                };
                apply_wave_warp_pro(pixels, width, height, &params);
            }
            ExtEffect::CcLens { convergence, zoom } => {
                apply_cc_lens_pro(
                    pixels,
                    width,
                    height,
                    &CcLensParams {
                        convergence: *convergence,
                        zoom: *zoom,
                    },
                );
            }
            ExtEffect::PolarCoordinates {
                to_polar,
                interpolation,
            } => {
                let mode = if *to_polar {
                    PolarMode::RectToPolar
                } else {
                    PolarMode::PolarToRect
                };
                apply_polar_coordinates_pro(pixels, width, height, mode, *interpolation);
            }
            ExtEffect::OpticsCompensation {
                field_of_view_deg,
                reverse,
                zoom,
            } => {
                apply_optics_compensation(
                    pixels,
                    width,
                    height,
                    &OpticsCompensationParams {
                        field_of_view_deg: *field_of_view_deg,
                        reverse: *reverse,
                        zoom: *zoom,
                    },
                );
            }
            ExtEffect::Curves {
                master,
                red,
                green,
                blue,
            } => {
                let curve = |pts: &Vec<[f32; 2]>| -> Option<ToneCurve> {
                    if pts.len() < 2 {
                        None
                    } else {
                        Some(ToneCurve::new(pts.clone()))
                    }
                };
                let cc = ChannelCurves {
                    master: curve(master),
                    red: curve(red),
                    green: curve(green),
                    blue: curve(blue),
                };
                apply_curves(pixels, &cc);
            }
            ExtEffect::ColorBalance {
                shadows,
                midtones,
                highlights,
                preserve_luminosity,
            } => {
                let bal = ColorBalance {
                    shadows: *shadows,
                    midtones: *midtones,
                    highlights: *highlights,
                    preserve_luminosity: *preserve_luminosity,
                };
                apply_color_balance(pixels, &bal);
            }
            ExtEffect::ChannelMixer { matrix, monochrome } => {
                let cm = ChannelMixer {
                    matrix: *matrix,
                    monochrome: *monochrome,
                };
                apply_channel_mixer(pixels, &cm);
            }
            ExtEffect::LightSweep {
                direction_deg,
                center,
                width: sweep_width,
                sweep_intensity,
                edge_intensity,
            } => {
                apply_light_sweep(
                    pixels,
                    width,
                    height,
                    &LightSweepParams {
                        direction_deg: *direction_deg,
                        center: *center,
                        width: *sweep_width,
                        sweep_intensity: *sweep_intensity,
                        edge_intensity: *edge_intensity,
                    },
                );
            }
            ExtEffect::RadialFastBlur { amount, samples } => {
                let cx = width as f32 * 0.5;
                let cy = height as f32 * 0.5;
                apply_radial_fast_blur(pixels, width, height, [cx, cy], *amount, *samples);
            }
            ExtEffect::BendIt {
                top_offset,
                bottom_offset,
            } => {
                apply_cc_bend_it_pro(pixels, width, height, *top_offset, *bottom_offset);
            }
            ExtEffect::Tiler {
                scale_percent,
                mirror,
            } => {
                let mode = if *mirror {
                    TileEdgeMode::Mirror
                } else {
                    TileEdgeMode::Repeat
                };
                apply_cc_tiler_pro(pixels, width, height, *scale_percent, mode);
            }
            ExtEffect::GlowPro {
                threshold,
                radius,
                intensity,
            } => {
                apply_glow_pro(pixels, width, height, *threshold, *radius, *intensity);
            }
            ExtEffect::Vibrance { amount } => {
                apply_vibrance(pixels, *amount);
            }
            ExtEffect::WhiteBalance { temperature, tint } => {
                apply_white_balance(
                    pixels,
                    &crate::core::color_correction::WhiteBalance {
                        temperature: *temperature,
                        tint: *tint,
                    },
                );
            }
            ExtEffect::HslAdjust {
                hue_deg,
                saturation,
                lightness,
            } => {
                apply_hsl_adjust(
                    pixels,
                    &crate::core::color_correction::HslAdjust {
                        hue_deg: *hue_deg,
                        saturation: *saturation,
                        lightness: *lightness,
                    },
                );
            }
        }
    }
}
