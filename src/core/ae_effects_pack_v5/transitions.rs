#![allow(dead_code)]

// 151. CC Blur Wipe
pub fn apply_cc_blur_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack::apply_fast_box_blur(
        pixels,
        width,
        height,
        (completion * 0.2) as u32,
    );
    crate::core::ae_effects_pack::apply_linear_wipe(pixels, width, height, completion, 90.0);
}

// 152. CC Cross Blinds
pub fn apply_cc_cross_blinds(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack::apply_venetian_blinds(pixels, width, height, completion, 20);
}

// 153. CC Drizzle Wipe
pub fn apply_cc_drizzle_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v2::apply_ripple(pixels, width, height, completion, 30.0, 0.0);
    crate::core::ae_effects_pack_v2::apply_iris_wipe(pixels, width, height, completion);
}

// 154. CC Glass Edges Wipe
pub fn apply_cc_glass_edges(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v4::apply_cc_glass_wipe(pixels, width, height, completion);
}

// 155. CC Light Wipe
pub fn apply_cc_light_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v3::apply_cc_light_sweep(pixels, width, height, completion, 40);
    crate::core::ae_effects_pack::apply_linear_wipe(pixels, width, height, completion, 0.0);
}

// 156. CC Line Sweep
pub fn apply_cc_line_sweep(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack::apply_linear_wipe(pixels, width, height, completion, 45.0);
}

// 157. CC Radial Scale Wipe
pub fn apply_cc_radial_scale_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v2::apply_radial_wipe(pixels, width, height, completion);
}

// 158. CC Scale Wipe
pub fn apply_cc_scale_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v2::apply_cc_tiler(pixels, width, height, 100.0 + completion);
    crate::core::ae_effects_pack_v2::apply_iris_wipe(pixels, width, height, completion);
}

// 159. CC Twister Wipe
pub fn apply_cc_twister_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack::apply_twirl(
        pixels,
        width,
        height,
        completion * 3.6,
        width as f32 * 0.5,
    );
    crate::core::ae_effects_pack_v2::apply_iris_wipe(pixels, width, height, completion);
}

// 160. CC Vignette Wipe
pub fn apply_cc_vignette_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v2::apply_vignette(pixels, width, height, completion * 0.01);
    crate::core::ae_effects_pack_v2::apply_iris_wipe(pixels, width, height, completion);
}
