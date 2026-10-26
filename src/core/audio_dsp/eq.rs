/// Parametric EQ band: bell, high shelf, low shelf, or pass filter.
#[derive(Debug, Clone)]
pub struct EqBand {
    pub freq: f32,
    pub gain_db: f32,
    pub q: f32,
    pub band_type: EqBandType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EqBandType {
    Bell,
    HighShelf,
    LowShelf,
    HighPass,
    LowPass,
}

impl Default for EqBand {
    fn default() -> Self {
        Self {
            freq: 1000.0,
            gain_db: 0.0,
            q: 1.0,
            band_type: EqBandType::Bell,
        }
    }
}

/// Biquad filter state (one second-order section).
#[derive(Debug, Clone)]
pub(crate) struct Biquad {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
    pub x1_l: f64,
    pub x2_l: f64,
    pub y1_l: f64,
    pub y2_l: f64,
    pub x1_r: f64,
    pub x2_r: f64,
    pub y1_r: f64,
    pub y2_r: f64,
}

impl Biquad {
    pub fn new() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1_l: 0.0,
            x2_l: 0.0,
            y1_l: 0.0,
            y2_l: 0.0,
            x1_r: 0.0,
            x2_r: 0.0,
            y1_r: 0.0,
            y2_r: 0.0,
        }
    }

    pub fn process_stereo(&mut self, buf: &mut [f32]) {
        for sample in buf.as_chunks_mut::<2>().0 {
            let x_l = if sample[0].is_finite() {
                f64::from(sample[0])
            } else {
                0.0
            };
            let x_r = if sample[1].is_finite() {
                f64::from(sample[1])
            } else {
                0.0
            };

            let y_l = self.b0 * x_l + self.b1 * self.x1_l + self.b2 * self.x2_l
                - self.a1 * self.y1_l
                - self.a2 * self.y2_l;
            let y_r = self.b0 * x_r + self.b1 * self.x1_r + self.b2 * self.x2_r
                - self.a1 * self.y1_r
                - self.a2 * self.y2_r;

            self.x2_l = self.x1_l;
            self.x1_l = x_l;
            self.y2_l = self.y1_l;
            self.y1_l = y_l;

            self.x2_r = self.x1_r;
            self.x1_r = x_r;
            self.y2_r = self.y1_r;
            self.y1_r = y_r;

            sample[0] = y_l as f32;
            sample[1] = y_r as f32;
        }
    }
}

/// Design a biquad filter for the given band type.
pub(crate) fn design_biquad(band: &EqBand, sample_rate: f64) -> Biquad {
    let mut bq = Biquad::new();
    let f0 = band.freq as f64;
    let gain = band.gain_db as f64;
    let q = band.q as f64;
    let fs = sample_rate;

    let a = if gain.abs() > 0.001 {
        10.0_f64.powf(gain / 40.0)
    } else {
        1.0
    };
    let w0 = 2.0 * std::f64::consts::PI * f0 / fs;
    let sin_w0 = w0.sin();
    let cos_w0 = w0.cos();
    let alpha = sin_w0 / (2.0 * q);

    match band.band_type {
        EqBandType::Bell => {
            let a_sqrt = a.sqrt();
            let b0 = 1.0 + alpha * a_sqrt;
            let b1 = -2.0 * cos_w0;
            let b2 = 1.0 - alpha * a_sqrt;
            let a0 = 1.0 + alpha / a_sqrt;
            let a1 = -2.0 * cos_w0;
            let a2 = 1.0 - alpha / a_sqrt;
            bq.b0 = b0 / a0;
            bq.b1 = b1 / a0;
            bq.b2 = b2 / a0;
            bq.a1 = a1 / a0;
            bq.a2 = a2 / a0;
        }
        EqBandType::HighShelf => {
            let a_sqrt = a.sqrt();
            let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a_sqrt * alpha);
            let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
            let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a_sqrt * alpha);
            let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a_sqrt * alpha;
            let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
            let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a_sqrt * alpha;
            bq.b0 = b0 / a0;
            bq.b1 = b1 / a0;
            bq.b2 = b2 / a0;
            bq.a1 = a1 / a0;
            bq.a2 = a2 / a0;
        }
        EqBandType::LowShelf => {
            let a_sqrt = a.sqrt();
            let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a_sqrt * alpha);
            let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
            let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a_sqrt * alpha);
            let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a_sqrt * alpha;
            let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
            let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a_sqrt * alpha;
            bq.b0 = b0 / a0;
            bq.b1 = b1 / a0;
            bq.b2 = b2 / a0;
            bq.a1 = a1 / a0;
            bq.a2 = a2 / a0;
        }
        EqBandType::HighPass => {
            let b0 = (1.0 + cos_w0) / 2.0;
            let b1 = -(1.0 + cos_w0);
            let b2 = (1.0 + cos_w0) / 2.0;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_w0;
            let a2 = 1.0 - alpha;
            bq.b0 = b0 / a0;
            bq.b1 = b1 / a0;
            bq.b2 = b2 / a0;
            bq.a1 = a1 / a0;
            bq.a2 = a2 / a0;
        }
        EqBandType::LowPass => {
            let b0 = (1.0 - cos_w0) / 2.0;
            let b1 = 1.0 - cos_w0;
            let b2 = (1.0 - cos_w0) / 2.0;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_w0;
            let a2 = 1.0 - alpha;
            bq.b0 = b0 / a0;
            bq.b1 = b1 / a0;
            bq.b2 = b2 / a0;
            bq.a1 = a1 / a0;
            bq.a2 = a2 / a0;
        }
    }
    bq
}

/// Apply parametric EQ with multiple bands to a stereo interleaved buffer.
pub fn apply_eq(buf: &mut [f32], bands: &[EqBand], sample_rate: u32) {
    if bands.is_empty() || buf.is_empty() || sample_rate == 0 {
        return;
    }
    let fs = sample_rate as f64;
    let nyquist = fs * 0.5;
    let filters: Vec<Biquad> = bands
        .iter()
        .filter(|b| {
            if !b.freq.is_finite()
                || !b.gain_db.is_finite()
                || !b.q.is_finite()
                || b.q < 1.0e-6
                || b.gain_db.abs() > 120.0
            {
                return false;
            }
            match b.band_type {
                EqBandType::HighPass => b.freq > 1.0 && (b.freq as f64) < nyquist,
                EqBandType::LowPass => b.freq > 100.0 && (b.freq as f64) < nyquist,
                _ => b.gain_db.abs() > 0.01,
            }
        })
        .map(|b| design_biquad(b, fs))
        .collect();

    for mut bq in filters {
        bq.process_stereo(buf);
    }
}
