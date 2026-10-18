use super::*;

#[test]
fn test_ae_effects_pack_filters() {
    let mut pixels = vec![100u8; 64]; // 4x4
    apply_fast_box_blur(&mut pixels, 4, 4, 1);
    assert_eq!(pixels.len(), 64);

    apply_invert(&mut pixels, false);
    assert_eq!(pixels[0], 155);
}
