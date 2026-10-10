use super::brush::*;
use super::clone::*;
use super::types::*;

fn blank(w: u32, h: u32) -> Vec<u8> {
    vec![0u8; (w * h * 4) as usize]
}

#[test]
fn test_draw_stroke_ignores_invalid_geometry_and_color() {
    let original = vec![12u8; 16];
    let mut buf = original.clone();
    draw_stroke(
        &mut buf,
        2,
        2,
        &[[f32::NAN, 1.0], [1.0, f32::INFINITY]],
        [f32::NAN, 0.0, 0.0, 1.0],
        f32::INFINITY,
    );
    assert_eq!(buf, original);
    draw_stroke(&mut buf, 2, 2, &[[1.0, 1.0]], [1.0; 4], 2.0);
    assert_ne!(buf, original);
}

#[test]
fn brush_controls_affect_alpha_and_edge_falloff() {
    let mut buf = blank(9, 9);
    draw_stroke_with_brush(
        &mut buf,
        9,
        9,
        &[[4.5, 4.5]],
        [1.0, 0.0, 0.0, 1.0],
        6.0,
        0.0,
        0.5,
        1.0,
    );
    let center_alpha = buf[((4 * 9 + 4) * 4) + 3];
    let edge_alpha = buf[((9 + 4) * 4) + 3];
    assert!((center_alpha as i32 - 128).abs() <= 1);
    assert!(edge_alpha < center_alpha);
}

#[test]
fn clone_brush_samples_from_source_points() {
    let mut source = blank(9, 9);
    let source_idx = ((4 * 9 + 2) * 4) as usize;
    source[source_idx + 1] = 255;
    source[source_idx + 3] = 255;
    let mut target = blank(9, 9);
    draw_clone_stroke_with_brush(
        &mut target,
        &source,
        9,
        9,
        &[[6.5, 4.5]],
        &[[1.5, 3.5]],
        2.0,
        1.0,
        1.0,
        1.0,
    );
    let target_idx = ((4 * 9 + 6) * 4) as usize;
    assert_eq!(target[target_idx + 1], 255);
    assert_eq!(target[target_idx + 3], 255);
}

#[test]
fn test_single_dot_paints_center_opaque() {
    let mut buf = blank(9, 9);
    draw_stroke(&mut buf, 9, 9, &[[4.5, 4.5]], [1.0, 0.0, 0.0, 1.0], 4.0);
    let center = ((4 * 9 + 4) * 4) as usize;
    assert_eq!(buf[center], 255);
    assert_eq!(buf[center + 3], 255);
    // Corner untouched.
    assert_eq!(buf[3], 0);
}

#[test]
fn test_line_is_contiguous() {
    let w = 40u32;
    let mut buf = blank(w, 7);
    draw_stroke(
        &mut buf,
        w,
        7,
        &[[2.0, 3.5], [37.0, 3.5]],
        [0.0, 1.0, 0.0, 1.0],
        3.0,
    );
    // Sample midpoints along the line — all should be green-ish opaque.
    for x in [5u32, 15, 25, 34] {
        let i = ((3 * w + x) * 4) as usize;
        assert!(buf[i + 1] > 200, "gap at x={}", x);
        assert_eq!(buf[i + 3], 255);
    }
}

#[test]
fn test_alpha_over_blends_existing() {
    let w = 5u32;
    let mut buf = blank(w, 1);
    // Pre-fill with opaque blue.
    for px in buf.as_chunks_mut::<4>().0 {
        px.copy_from_slice(&[0, 0, 255, 255]);
    }
    // Half-alpha red dot on top → purple-ish, still opaque.
    draw_stroke(&mut buf, w, 1, &[[2.5, 0.5]], [1.0, 0.0, 0.0, 0.5], 5.0);
    let i = (2 * 4) as usize;
    assert_eq!(buf[i + 3], 255, "stays opaque");
    assert!(
        buf[i] > 100 && buf[i + 2] > 100,
        "blend produces mixed color"
    );
}

#[test]
fn test_zero_size_noop() {
    let mut buf = blank(4, 4);
    draw_stroke(&mut buf, 4, 4, &[[2.0, 2.0]], [1.0, 1.0, 1.0, 1.0], 0.0);
    assert!(buf.iter().all(|&b| b == 0));
}

#[test]
fn test_paint_layer_add_and_clear() {
    let mut layer = PaintLayer::new();
    layer.add_stroke(PaintStroke {
        id: 1,
        color: [1.0; 4],
        size: 5.0,
        hardness: 0.5,
        opacity: 1.0,
        points: vec![StrokePoint {
            x: 0.0,
            y: 0.0,
            pressure: 1.0,
        }],
    });
    assert_eq!(layer.strokes.len(), 1);
    layer.clear();
    assert!(layer.strokes.is_empty());
}

#[test]
fn test_paint_layer_render() {
    let mut layer = PaintLayer::new();
    layer.add_stroke(PaintStroke {
        id: 1,
        color: [1.0, 0.0, 0.0, 1.0],
        size: 10.0,
        hardness: 0.5,
        opacity: 1.0,
        points: vec![
            StrokePoint {
                x: 50.0,
                y: 50.0,
                pressure: 1.0,
            },
            StrokePoint {
                x: 60.0,
                y: 50.0,
                pressure: 1.0,
            },
        ],
    });
    let mut buf = blank(100, 100);
    layer.render(&mut buf, 100, 100);
    let center = ((50 * 100 + 50) * 4) as usize;
    assert!(buf[center + 3] > 0, "center pixel should be painted");
}

#[test]
fn test_empty_stroke_noop() {
    let mut layer = PaintLayer::new();
    layer.add_stroke(PaintStroke {
        id: 1,
        color: [1.0; 4],
        size: 5.0,
        hardness: 0.5,
        opacity: 1.0,
        points: vec![],
    });
    let mut buf = blank(10, 10);
    layer.render(&mut buf, 10, 10);
    assert!(buf.iter().all(|&b| b == 0));
}

#[test]
fn test_clone_stamp_copies_source() {
    let w = 20u32;
    let h = 20u32;
    let mut dst = blank(w, h);
    let mut src = vec![0u8; (w * h * 4) as usize];
    // Draw red circle in source at (5,5)
    for py in 0..h {
        for px in 0..w {
            let dx = px as f32 - 5.0;
            let dy = py as f32 - 5.0;
            if dx * dx + dy * dy < 4.0 {
                let i = ((py * w + px) * 4) as usize;
                src[i] = 255;
                src[i + 3] = 255;
            }
        }
    }
    // Clone stamp at (10,10) with offset (0,0) — no transform
    let config = CloneStampConfig {
        src_offset: [0.0, 0.0],
        size: 4.0,
        opacity: 1.0,
        hardness: 1.0,
        rotation_deg: 0.0,
        flip_h: false,
        flip_v: false,
    };
    draw_clone_stamp(&mut dst, &src, w, h, &[[10.0, 10.0]], &config);
    let idx = ((10 * w + 10) * 4) as usize;
    assert!(dst[idx + 3] > 0, "cloned pixel should have alpha");
}

#[test]
fn test_clone_stamp_empty_points_noop() {
    let mut dst = blank(10, 10);
    let src = vec![128u8; 400];
    let config = CloneStampConfig {
        src_offset: [0.0; 2],
        size: 5.0,
        opacity: 1.0,
        hardness: 1.0,
        rotation_deg: 0.0,
        flip_h: false,
        flip_v: false,
    };
    draw_clone_stamp(&mut dst, &src, 10, 10, &[], &config);
    assert!(dst.iter().all(|&b| b == 0));
}
