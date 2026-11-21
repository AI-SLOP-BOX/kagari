use super::modifiers::*;
use super::ops::*;
use crate::core::mask::point_in_polygon;

#[test]
fn test_polygon_intersection_overlapping_squares() {
    let sq_a = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let sq_b = vec![[5.0, 5.0], [15.0, 5.0], [15.0, 15.0], [5.0, 15.0]];

    let inter = polygon_intersect(&sq_a, &sq_b);
    assert_eq!(inter.len(), 1);
    let poly = &inter[0];
    assert!(poly.len() >= 4);

    // Intersected box should contain (7.5, 7.5)
    assert!(point_in_polygon(7.5, 7.5, poly));
    // But not (2.0, 2.0)
    assert!(!point_in_polygon(2.0, 2.0, poly));
}

#[test]
fn test_polygon_difference_subtract_geometry() {
    let sq_a = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
    let sq_b = vec![[5.0, 5.0], [15.0, 5.0], [15.0, 15.0], [5.0, 15.0]];

    let diff = polygon_subtract(&sq_a, &sq_b);
    assert_eq!(diff.len(), 1);
    let bridged_poly = &diff[0];
    assert!(point_in_polygon(2.0, 2.0, bridged_poly));
    assert!(!point_in_polygon(10.0, 10.0, bridged_poly));
}

#[test]
fn test_subtract_preserves_outer_region_but_removes_inner_region() {
    let outer = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
    let cutout = vec![[5.0, 5.0], [15.0, 5.0], [15.0, 15.0], [5.0, 15.0]];
    let result = polygon_subtract(&outer, &cutout);
    let compound = CompoundShape2D {
        contours: result,
        fill_rule: FillRule::NonZero,
    };

    assert!(compound.contains_point(2.0, 2.0));
    assert!(!compound.contains_point(10.0, 10.0));
}

#[test]
fn test_offset_polygon_path_expansion() {
    let square = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let expanded = offset_polygon_path(&square, 5.0);
    assert_eq!(expanded.len(), 4);

    // Bounds of expanded polygon should be larger
    let min_x = expanded.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
    let max_x = expanded
        .iter()
        .map(|p| p[0])
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(min_x < 0.0);
    assert!(max_x > 10.0);
}

#[test]
fn test_compound_shape_with_hole() {
    let outer = vec![[0.0, 0.0], [30.0, 0.0], [30.0, 30.0], [0.0, 30.0]];
    let hole = vec![[10.0, 10.0], [20.0, 10.0], [20.0, 20.0], [10.0, 20.0]];
    let shape = CompoundShape2D {
        contours: vec![outer, hole],
        fill_rule: FillRule::NonZero,
    };

    // Outer area should be filled
    assert!(shape.contains_point(5.0, 5.0));
    // Inner hole should be empty
    assert!(!shape.contains_point(15.0, 15.0));
    // Outside bounds should be empty
    assert!(!shape.contains_point(35.0, 35.0));
}

#[test]
fn test_boolean_with_nan_coordinates_is_rejected_without_panicking() {
    let subject = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [f32::NAN, 10.0]];
    let clip = vec![[2.0, 2.0], [8.0, 2.0], [8.0, 8.0], [2.0, 8.0]];
    let result = std::panic::catch_unwind(|| polygon_subtract(&subject, &clip));
    assert!(
        result.is_ok(),
        "NaN geometry must be rejected or handled safely"
    );
}

#[test]
fn test_union_of_overlapping_rectangles_contains_only_union_geometry() {
    let a = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let b = vec![[8.0, 8.0], [18.0, 8.0], [18.0, 18.0], [8.0, 18.0]];
    let result = polygon_union(&a, &b);
    assert!(result.iter().any(|p| point_in_polygon(2.0, 2.0, p)));
    assert!(result.iter().any(|p| point_in_polygon(16.0, 16.0, p)));
    assert!(!result.iter().any(|p| point_in_polygon(15.0, 2.0, p)));
}

#[test]
fn test_wiggle_paths_deforms_geometry_deterministically() {
    let sq = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let wiggled1 = apply_wiggle_paths(&sq, 5.0, 2.0, 42, 1.0, true);
    let wiggled2 = apply_wiggle_paths(&sq, 5.0, 2.0, 42, 1.0, true);
    assert_eq!(wiggled1, wiggled2, "Wiggle must be strictly deterministic");
    assert_ne!(wiggled1[0], sq[0], "Wiggle must displace vertices");
}

#[test]
fn test_evaluate_shape_repeater_generates_correct_copy_count() {
    let sq = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let copies = evaluate_shape_repeater(&[sq], 3, 0.0, [20.0, 0.0], [1.0, 1.0], 0.0, 1.0, 0.5);
    assert_eq!(copies.len(), 3);
    assert_eq!(copies[0].1, 1.0);
    assert_eq!(copies[2].1, 0.5);
}
