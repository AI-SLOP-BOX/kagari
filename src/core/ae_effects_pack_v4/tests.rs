#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::super::*;

    #[test]
    fn test_ae_effects_v4_filters() {
        let mut pixels = vec![100u8; 64];
        apply_exposure(&mut pixels, 1.0);
        assert_eq!(pixels.len(), 64);
        assert_eq!(pixels[0], 200);
    }
}
