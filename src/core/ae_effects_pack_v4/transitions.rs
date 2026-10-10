// Transitions (Block Dissolve, Card Dance, Shatter, Iris, Wipes)

// 101. Block Dissolve
pub fn apply_block_dissolve(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v2::apply_mosaic_average(
        pixels,
        width,
        height,
        (completion * 0.5) as u32 + 1,
        (completion * 0.5) as u32 + 1,
    );
}

// 102. Card Wipe
pub fn apply_card_dance(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack::apply_venetian_blinds(pixels, width, height, completion, 20);
}

pub fn apply_shatter_v4(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack::apply_linear_wipe(pixels, width, height, completion, 45.0);
}

// 104. Iris Shape Wipe
pub fn apply_iris_shape_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v2::apply_iris_wipe(pixels, width, height, completion);
}

// 105. Radial Blur Zoom
pub fn apply_radial_blur_zoom(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    crate::core::ae_effects_pack::apply_radial_blur(pixels, width, height, amount);
}

// 106. CC Glass Wipe
pub fn apply_cc_glass_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v2::apply_cc_glass(pixels, width, height, completion * 0.5);
}

// 107. CC Grid Wipe
pub fn apply_cc_grid_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack::apply_venetian_blinds(pixels, width, height, completion, 10);
}

// 108. CC Image Wipe
pub fn apply_cc_image_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack::apply_linear_wipe(pixels, width, height, completion, 90.0);
}

// 109. CC Jaws
pub fn apply_cc_jaws(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack::apply_venetian_blinds(pixels, width, height, completion, 30);
}

// 110. CC Radial Scale Wipe
pub fn apply_cc_radial_scale_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v2::apply_iris_wipe(pixels, width, height, completion);
}
