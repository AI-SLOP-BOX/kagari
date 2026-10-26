use super::*;

#[test]
fn test_ae_effects_v5_filters() {
    let mut samples = vec![1.0f32; 100];
    apply_low_pass_filter(&mut samples);
    assert_eq!(samples.len(), 100);
}

#[test]
fn test_bass_treble() {
    let mut s = vec![0.5f32; 256];
    apply_bass_treble(&mut s, 44100.0, 6.0, -3.0, 300.0);
    assert_eq!(s.len(), 256);
}

#[test]
fn test_flanger() {
    let mut s = vec![0.5f32; 1024];
    apply_flanger(&mut s, 44100.0, 5.0, 0.5, 0.5, 0.5);
    assert_eq!(s.len(), 1024);
}

#[test]
fn test_chorus() {
    let mut s = vec![0.5f32; 1024];
    apply_chorus(&mut s, 44100.0, 15.0, 5.0, 1.0, 3.0, 0.3);
    assert_eq!(s.len(), 1024);
}

#[test]
fn test_parametric_eq_bell() {
    let mut s = vec![0.5f32; 256];
    apply_parametric_eq_bell(&mut s, 44100.0, 1000.0, 6.0, 1.0);
    assert_eq!(s.len(), 256);
}
