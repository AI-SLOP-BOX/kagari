use std::f32::consts::{PI, TAU};
use crate::core::effect_utils;

/// Clamp-to-edge bilinear RGBA sample — delegates to shared `effect_utils`.
#[inline]
pub(crate) fn sample_bilinear(src: &[u8], w: u32, h: u32, fx: f32, fy: f32, out: &mut [u8; 4]) {
    effect_utils::sample_bilinear_into(src, w, h, fx, fy, out);
}

/// Wave shape family for Wave Warp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveType {
    Sine,
    Triangle,
    Square,
    Sawtooth,
}

impl WaveType {
    /// Periodic waveform in [-1, 1] for a phase in radians.
    pub fn waveform(self, phase: f32) -> f32 {
        match self {
            WaveType::Sine => phase.sin(),
            WaveType::Triangle => {
                let f = (phase / TAU).rem_euclid(1.0);
                1.0 - 4.0 * (f - 0.5).abs()
            }
            WaveType::Square => {
                if phase.rem_euclid(TAU) < PI {
                    1.0
                } else {
                    -1.0
                }
            }
            WaveType::Sawtooth => {
                let f = (phase / TAU).rem_euclid(1.0);
                2.0 * f - 1.0
            }
        }
    }
}

/// Which edges are pinned (displacement attenuated to zero), matching AE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinKind {
    All,
    LeftRight,
    TopBottom,
    None,
}

/// Parameters for [`apply_wave_warp_pro`].
#[derive(Debug, Clone, Copy)]
pub struct WaveWarpParams {
    /// Peak displacement in pixels.
    pub wave_height: f32,
    /// Spatial wavelength in pixels.
    pub wave_width: f32,
    /// Phase advance per second (wave travel speed).
    pub speed: f32,
    /// Absolute time in seconds (animation driver).
    pub time: f32,
    /// Static phase offset in radians.
    pub phase: f32,
    /// Displacement direction in degrees (90 = vertical, AE default).
    pub direction_deg: f32,
    pub wave_type: WaveType,
    pub pinning: PinKind,
}

impl Default for WaveWarpParams {
    fn default() -> Self {
        Self {
            wave_height: 50.0,
            wave_width: 100.0,
            speed: 1.0,
            time: 0.0,
            phase: 0.0,
            direction_deg: 90.0,
            wave_type: WaveType::Sine,
            pinning: PinKind::All,
        }
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Edge attenuation factor (0 at pinned edges, 1 in the interior).
fn pin_attenuation(kind: PinKind, x: u32, y: u32, w: u32, h: u32) -> f32 {
    let fw = w as f32;
    let fh = h as f32;
    let ramp = |pos: f32, size: f32| -> f32 {
        let e = (size * 0.12).clamp(2.0, 64.0);
        smoothstep(pos / e).min(smoothstep((size - 1.0 - pos) / e))
    };
    match kind {
        PinKind::None => 1.0,
        PinKind::LeftRight => ramp(x as f32, fw),
        PinKind::TopBottom => ramp(y as f32, fh),
        PinKind::All => ramp(x as f32, fw) * ramp(y as f32, fh),
    }
}

/// Production-quality Wave Warp with selectable wave shape, direction and
/// edge pinning. Displacement happens along `direction_deg`; the wave phase
/// varies along the perpendicular axis.
pub fn apply_wave_warp_pro(pixels: &mut [u8], width: u32, height: u32, p: &WaveWarpParams) {
    if width == 0 || height == 0 || pixels.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    let temp = pixels.to_vec();
    let dir = p.direction_deg.to_radians();
    let (dx, dy) = (dir.cos(), dir.sin());
    // Sampling axis is perpendicular to the displacement direction.
    let (px, py) = (-dy, dx);
    let ww = p.wave_width.max(1.0);

    for y in 0..height {
        for x in 0..width {
            let u = (x as f32 * px + y as f32 * py) / ww;
            let phase = u * TAU + p.phase + p.speed * p.time * TAU;
            let disp = p.wave_height * p.wave_type.waveform(phase);
            let att = pin_attenuation(p.pinning, x, y, width, height);
            let d = disp * att;
            let mut rgba = [0u8; 4];
            sample_bilinear(
                &temp,
                width,
                height,
                x as f32 + dx * d,
                y as f32 + dy * d,
                &mut rgba,
            );
            let idx = ((y * width + x) * 4) as usize;
            pixels[idx..idx + 4].copy_from_slice(&rgba);
        }
    }
}
