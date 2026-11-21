//! Discrete Fourier frequency analysis and multi-band analyzer state.

use super::types::AudioSpectrumOptions;

/// Generates raw audio spectrum frequency bands (normalized 0.0..1.0).
pub fn generate_audio_spectrum_bands(
    samples: &[f32],
    sample_rate: u32,
    options: &AudioSpectrumOptions,
) -> Vec<f32> {
    let bands = options.frequency_bands.clamp(1, 512);
    let mut magnitudes = vec![0.0f32; bands];
    if samples.is_empty() || sample_rate == 0 {
        return magnitudes;
    }

    let fft_size = options.fft_size.clamp(64, 8192).next_power_of_two();
    let mono = if samples.len() >= 2 && samples.len().is_multiple_of(2) {
        samples
            .as_chunks::<2>()
            .0
            .iter()
            .map(|stereo| {
                let left = if stereo[0].is_finite() {
                    stereo[0]
                } else {
                    0.0
                };
                let right = if stereo[1].is_finite() {
                    stereo[1]
                } else {
                    0.0
                };
                (left + right) * 0.5
            })
            .collect::<Vec<_>>()
    } else {
        samples
            .iter()
            .map(|sample| if sample.is_finite() { *sample } else { 0.0 })
            .collect::<Vec<_>>()
    };
    let window_len = mono.len().min(fft_size);
    let mut real = vec![0.0f32; fft_size];
    let mut imag = vec![0.0f32; fft_size];
    let mut window_sum = 0.0f32;
    for index in 0..window_len {
        let window = if window_len > 1 {
            0.5 - 0.5 * (std::f32::consts::TAU * index as f32 / (window_len - 1) as f32).cos()
        } else {
            1.0
        };
        real[index] = mono[index] * window;
        window_sum += window;
    }
    fft_in_place(&mut real, &mut imag);

    let nyquist = sample_rate as f32 * 0.5;
    let low = options.start_frequency.max(1.0).min(nyquist);
    let high = options.end_frequency.max(low).min(nyquist);
    if high <= low || window_sum <= f32::EPSILON {
        return magnitudes;
    }
    let ratio = high / low;
    for (band, magnitude) in magnitudes.iter_mut().enumerate() {
        let band_low = low * ratio.powf(band as f32 / bands as f32);
        let band_high = low * ratio.powf((band + 1) as f32 / bands as f32);
        let first_bin = ((band_low * fft_size as f32 / sample_rate as f32).floor() as usize)
            .max(1)
            .min(fft_size / 2);
        let end_bin = ((band_high * fft_size as f32 / sample_rate as f32).ceil() as usize)
            .max(first_bin + 1)
            .min(fft_size / 2 + 1);
        if first_bin >= fft_size / 2 || end_bin <= first_bin {
            continue;
        }
        let mut power = 0.0f32;
        for bin in first_bin..end_bin {
            let amplitude = real[bin].hypot(imag[bin]) * 2.0 / window_sum;
            power += amplitude * amplitude;
        }
        *magnitude = ((power / (end_bin - first_bin) as f32).sqrt() * 2.5).clamp(0.0, 1.0);
    }
    magnitudes
}

pub fn fft_in_place(real: &mut [f32], imag: &mut [f32]) {
    let size = real.len();
    debug_assert!(size.is_power_of_two() && imag.len() == size);
    let mut reversed = 0usize;
    for index in 1..size {
        let mut bit = size >> 1;
        while reversed & bit != 0 {
            reversed ^= bit;
            bit >>= 1;
        }
        reversed ^= bit;
        if index < reversed {
            real.swap(index, reversed);
            imag.swap(index, reversed);
        }
    }
    let mut block = 2usize;
    while block <= size {
        let angle = -std::f32::consts::TAU / block as f32;
        let step_real = angle.cos();
        let step_imag = angle.sin();
        for start in (0..size).step_by(block) {
            let mut twiddle_real = 1.0f32;
            let mut twiddle_imag = 0.0f32;
            for offset in 0..block / 2 {
                let even = start + offset;
                let odd = even + block / 2;
                let odd_real = real[odd] * twiddle_real - imag[odd] * twiddle_imag;
                let odd_imag = real[odd] * twiddle_imag + imag[odd] * twiddle_real;
                real[odd] = real[even] - odd_real;
                imag[odd] = imag[even] - odd_imag;
                real[even] += odd_real;
                imag[even] += odd_imag;
                let next_real = twiddle_real * step_real - twiddle_imag * step_imag;
                twiddle_imag = twiddle_real * step_imag + twiddle_imag * step_real;
                twiddle_real = next_real;
            }
        }
        block <<= 1;
    }
}

/// Multi-band spectrum analyzer with smoothing and peak tracking state.
#[derive(Debug, Clone)]
pub struct SpectrumAnalyzer {
    pub band_count: usize,
    prev_bands: Vec<f32>,
    peaks: Vec<f32>,
}

impl SpectrumAnalyzer {
    pub fn new(band_count: usize) -> Self {
        Self {
            band_count,
            prev_bands: vec![0.0; band_count],
            peaks: vec![0.0; band_count],
        }
    }

    pub fn analyze(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        options: &AudioSpectrumOptions,
    ) -> Vec<f32> {
        let current = generate_audio_spectrum_bands(samples, sample_rate, options);
        if self.prev_bands.len() != current.len() {
            self.prev_bands = vec![0.0; current.len()];
            self.peaks = vec![0.0; current.len()];
        }
        let smooth = options.smoothing.clamp(0.0, 0.95);
        for (i, &v) in current.iter().enumerate() {
            self.prev_bands[i] = self.prev_bands[i] * smooth + v * (1.0 - smooth);
            self.peaks[i] = self.peaks[i].max(self.prev_bands[i]) - options.peak_decay;
            if self.peaks[i] < 0.0 {
                self.peaks[i] = 0.0;
            }
        }
        self.prev_bands.clone()
    }

    pub fn peaks(&self) -> &[f32] {
        &self.peaks
    }
}
