use super::*;

#[test]
fn test_subpixel_refine_finds_true_minimum() {
    // Synthetic SAD valley centered at x = 10.4, y = 7.6
    let true_x = 10.4f32;
    let true_y = 7.6f32;
    let sad = |x: i32, y: i32| -> f32 {
        let fx = x as f32 - true_x;
        let fy = y as f32 - true_y;
        fx * fx + fy * fy
    };
    // Integer search finds (10, 8)
    let [dx, dy] = subpixel_refine(&sad, 10, 8);
    let refined_x = 10.0 + dx;
    let refined_y = 8.0 + dy;
    assert!(
        (refined_x - true_x).abs() < 0.15,
        "x {} vs {}",
        refined_x,
        true_x
    );
    assert!(
        (refined_y - true_y).abs() < 0.15,
        "y {} vs {}",
        refined_y,
        true_y
    );
}

#[test]
fn test_confidence_distinguishes_sharp_vs_flat() {
    // Sharp peak: best is far below second-best → low ratio (high confidence)
    let sharp = match_confidence(10.0, 100.0);
    // Flat valley: nearly equal → high ratio (ambiguous)
    let flat = match_confidence(100.0, 101.0);
    assert!(sharp < 0.2 && flat > 0.99);
}
