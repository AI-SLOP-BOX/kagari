/// Standard professional working color spaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum WorkingColorSpace {
    #[default]
    Rec709,
    DisplayP3,
    Rec2020,
    LinearSRGB,
    AcesCG,
}

/// Converts sRGB Gamma (2.2 / standard transfer) to Linear RGB.
pub fn srgb_to_linear(v: f32) -> f32 {
    let v = v.clamp(0.0, 1.0);
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// Converts Linear RGB to sRGB Gamma.
pub fn linear_to_srgb(v: f32) -> f32 {
    let v = v.clamp(0.0, 1.0);
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// Color transformation matrix multiply for Rec.709 <-> Display P3 <-> Rec.2020 <-> ACEScg
pub fn convert_color_space(
    rgb: [f32; 3],
    src: WorkingColorSpace,
    dst: WorkingColorSpace,
) -> [f32; 3] {
    if src == dst {
        return rgb;
    }
    // Convert source to Linear sRGB first
    let lin_src = match src {
        WorkingColorSpace::Rec709 => [
            srgb_to_linear(rgb[0]),
            srgb_to_linear(rgb[1]),
            srgb_to_linear(rgb[2]),
        ],
        WorkingColorSpace::LinearSRGB => rgb,
        WorkingColorSpace::DisplayP3 => {
            let lin_p3 = [
                srgb_to_linear(rgb[0]),
                srgb_to_linear(rgb[1]),
                srgb_to_linear(rgb[2]),
            ];
            [
                lin_p3[0] * 1.2249 - lin_p3[1] * 0.2247 + lin_p3[2] * 0.0,
                -lin_p3[0] * 0.0420 + lin_p3[1] * 1.0419 - lin_p3[2] * 0.0,
                -lin_p3[0] * 0.0196 - lin_p3[1] * 0.0786 + lin_p3[2] * 1.0982,
            ]
        }
        WorkingColorSpace::Rec2020 => {
            let lin_2020 = [
                srgb_to_linear(rgb[0]),
                srgb_to_linear(rgb[1]),
                srgb_to_linear(rgb[2]),
            ];
            [
                lin_2020[0] * 1.6605 - lin_2020[1] * 0.5876 - lin_2020[2] * 0.0728,
                -lin_2020[0] * 0.1246 + lin_2020[1] * 1.1329 - lin_2020[2] * 0.0083,
                -lin_2020[0] * 0.0182 - lin_2020[1] * 0.1006 + lin_2020[2] * 1.1187,
            ]
        }
        WorkingColorSpace::AcesCG => [
            rgb[0] * 1.7050 - rgb[1] * 0.6242 - rgb[2] * 0.0808,
            -rgb[0] * 0.1297 + rgb[1] * 1.1385 - rgb[2] * 0.0088,
            -rgb[0] * 0.0241 - rgb[1] * 0.1246 + rgb[2] * 1.1488,
        ],
    };

    // Convert Linear sRGB to destination
    match dst {
        WorkingColorSpace::Rec709 => [
            linear_to_srgb(lin_src[0]),
            linear_to_srgb(lin_src[1]),
            linear_to_srgb(lin_src[2]),
        ],
        WorkingColorSpace::LinearSRGB => lin_src,
        WorkingColorSpace::DisplayP3 => {
            let p3_r = lin_src[0] * 0.8225 + lin_src[1] * 0.1775 + lin_src[2] * 0.0;
            let p3_g = lin_src[0] * 0.0332 + lin_src[1] * 0.9668 + lin_src[2] * 0.0;
            let p3_b = lin_src[0] * 0.0171 + lin_src[1] * 0.0724 + lin_src[2] * 0.9105;
            [
                linear_to_srgb(p3_r),
                linear_to_srgb(p3_g),
                linear_to_srgb(p3_b),
            ]
        }
        WorkingColorSpace::Rec2020 => {
            let r2020_r = lin_src[0] * 0.6274 + lin_src[1] * 0.3293 + lin_src[2] * 0.0433;
            let r2020_g = lin_src[0] * 0.0691 + lin_src[1] * 0.9195 + lin_src[2] * 0.0114;
            let r2020_b = lin_src[0] * 0.0164 + lin_src[1] * 0.0880 + lin_src[2] * 0.8956;
            [
                linear_to_srgb(r2020_r),
                linear_to_srgb(r2020_g),
                linear_to_srgb(r2020_b),
            ]
        }
        WorkingColorSpace::AcesCG => [
            lin_src[0] * 0.6131 + lin_src[1] * 0.3395 + lin_src[2] * 0.0474,
            lin_src[0] * 0.0702 + lin_src[1] * 0.9164 + lin_src[2] * 0.0134,
            lin_src[0] * 0.0206 + lin_src[1] * 0.1096 + lin_src[2] * 0.8698,
        ],
    }
}
