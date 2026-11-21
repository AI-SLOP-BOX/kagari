// ────────────────────── Cinema Log Color Profiles ──────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum CinemaLogProfile {
    #[default]
    ArriLogC3,
    ArriLogC4,
    SonySLog3,
    RedLog3G10,
    CanonCLog2,
    CanonCLog3,
    BmdFilmGen5,
}

/// Convert Linear floating-point scene light to Cinema Log value.
pub fn linear_to_cinema_log(x: f32, profile: CinemaLogProfile) -> f32 {
    let x = if x.is_nan() { 0.0 } else { x };
    match profile {
        CinemaLogProfile::ArriLogC3 => {
            // ARRI ALEXA LogC3 EI 800
            let cut = 0.010591f32;
            let a = 5.555556f32;
            let b = 0.052272f32;
            let c = 0.247190f32;
            let d = 0.385537f32;
            let e = 5.367655f32;
            let f = 0.092809f32;
            if x > cut {
                c * (a * x + b).log10() + d
            } else {
                e * x + f
            }
        }
        CinemaLogProfile::ArriLogC4 => {
            // ARRI ALEXA LogC4
            let a = 0.0003f32;
            let b = 0.2238f32;
            let c = 0.18f32;
            if x > a {
                b * (x / c + 1.0).ln() + 0.1
            } else {
                (x / a) * 0.1
            }
        }
        CinemaLogProfile::SonySLog3 => {
            // Sony S-Log3
            if x >= 0.011_25 {
                (420.0 + ((x + 0.01) / (0.18 + 0.01)).log10() * 261.5) / 1023.0
            } else {
                (x * (171.210_3 - 95.0) / 0.011_25 + 95.0) / 1023.0
            }
        }
        CinemaLogProfile::RedLog3G10 => {
            // RED Log3G10
            let a = 0.224282f32;
            let b = 155.975_33_f32;
            let sign = if x < 0.0 { -1.0 } else { 1.0 };
            sign * (a * (b * x.abs() + 1.0).log10())
        }
        CinemaLogProfile::CanonCLog2 => {
            // Canon C-Log2
            if x >= 0.00392157 {
                0.24136077 * (8.725661 * x + 1.0).log10() + 0.092864
            } else {
                3.98402 * x + 0.07727
            }
        }
        CinemaLogProfile::CanonCLog3 => {
            // Canon C-Log3
            if x >= 0.00392157 {
                0.1788 * (14.0 * x + 1.0).log10() + 0.125
            } else {
                4.2 * x + 0.06
            }
        }
        CinemaLogProfile::BmdFilmGen5 => {
            // Blackmagic Film Gen5
            let a = 0.08692876f32;
            let b = 0.005494143f32;
            let c = 0.5300135f32;
            let d = 8.283606f32;
            let e = 0.09246582f32;
            if x > -b {
                c * (a * x + b + 1.0).ln() + e
            } else {
                d * x
            }
        }
    }
}

/// Convert Cinema Log encoded value back to Linear light floating-point.
pub fn cinema_log_to_linear(y: f32, profile: CinemaLogProfile) -> f32 {
    let y = if y.is_nan() { 0.0 } else { y };
    match profile {
        CinemaLogProfile::ArriLogC3 => {
            let cut = 0.010591f32;
            let a = 5.555556f32;
            let b = 0.052272f32;
            let c = 0.247190f32;
            let d = 0.385537f32;
            let e = 5.367655f32;
            let f = 0.092809f32;
            let y_cut = e * cut + f;
            if y > y_cut {
                (10.0f32.powf((y - d) / c) - b) / a
            } else {
                (y - f) / e
            }
        }
        CinemaLogProfile::ArriLogC4 => {
            let a = 0.0003f32;
            let b = 0.2238f32;
            let c = 0.18f32;
            if y > 0.1 {
                (((y - 0.1) / b).exp() - 1.0) * c
            } else {
                (y / 0.1) * a
            }
        }
        CinemaLogProfile::SonySLog3 => {
            let y_cut = 171.210_3 / 1023.0;
            if y >= y_cut {
                10.0f32.powf((y * 1023.0 - 420.0) / 261.5) * (0.18 + 0.01) - 0.01
            } else {
                (y * 1023.0 - 95.0) * 0.011_25 / (171.210_3 - 95.0)
            }
        }
        CinemaLogProfile::RedLog3G10 => {
            let a = 0.224282f32;
            let b = 155.975_33_f32;
            let sign = if y < 0.0 { -1.0 } else { 1.0 };
            sign * ((10.0f32.powf(y.abs() / a) - 1.0) / b)
        }
        CinemaLogProfile::CanonCLog2 => {
            let y_cut = 0.24136077 * (8.725661 * 0.00392157 + 1.0f32).log10() + 0.092864;
            if y >= y_cut {
                (10.0f32.powf((y - 0.092864) / 0.24136077) - 1.0) / 8.725661
            } else {
                (y - 0.07727) / 3.98402
            }
        }
        CinemaLogProfile::CanonCLog3 => {
            let y_cut = 0.1788 * (14.0 * 0.00392157 + 1.0f32).log10() + 0.125;
            if y >= y_cut {
                (10.0f32.powf((y - 0.125) / 0.1788) - 1.0) / 14.0
            } else {
                (y - 0.06) / 4.2
            }
        }
        CinemaLogProfile::BmdFilmGen5 => {
            let a = 0.08692876f32;
            let b = 0.005494143f32;
            let c = 0.5300135f32;
            let d = 8.283606f32;
            let e = 0.09246582f32;
            if y > e {
                (((y - e) / c).exp() - 1.0 - b) / a
            } else {
                y / d
            }
        }
    }
}

/// Apply cinema log encoding or decoding to an [R, G, B] triple in-place.
pub fn apply_cinema_log_to_rgb(rgb: [f32; 3], profile: CinemaLogProfile, invert: bool) -> [f32; 3] {
    if invert {
        [
            cinema_log_to_linear(rgb[0], profile),
            cinema_log_to_linear(rgb[1], profile),
            cinema_log_to_linear(rgb[2], profile),
        ]
    } else {
        [
            linear_to_cinema_log(rgb[0], profile),
            linear_to_cinema_log(rgb[1], profile),
            linear_to_cinema_log(rgb[2], profile),
        ]
    }
}
