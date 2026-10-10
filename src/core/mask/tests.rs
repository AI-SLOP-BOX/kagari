use super::*;

#[test]
fn test_point_in_polygon_square() {
    let square = vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
    assert!(
        point_in_polygon(50.0, 50.0, &square),
        "center should be inside"
    );
    assert!(!point_in_polygon(150.0, 50.0, &square), "outside right");
    assert!(!point_in_polygon(-10.0, 50.0, &square), "outside left");
}

#[test]
fn test_mask_rect_path() {
    let path = MaskPath::new_rect(0.0, 0.0, 100.0, 100.0);
    let poly = path.to_polygon(0, 4);
    assert!(!poly.is_empty(), "polygon should have points");
}

#[test]
fn test_mask_mode_default() {
    let mask = Mask::new_rect("m0".into(), "Mask 1".into(), 0.0, 0.0, 100.0, 100.0);
    assert_eq!(mask.mode, MaskMode::Add);
    assert!(mask.enabled);
    assert!(!mask.inverted);
}

#[test]
fn test_eval_cubic_bezier() {
    let p0 = [0.0, 0.0];
    let c0 = [0.0, 50.0];
    let c1 = [100.0, 50.0];
    let p1 = [100.0, 100.0];
    let mid = eval_cubic_bezier(p0, c0, c1, p1, 0.5);
    assert_eq!(mid, [50.0, 50.0]);
}

#[test]
fn test_set_vertex_at_frame_preserves_tangents() {
    let mut path = MaskPath::new_ellipse(100.0, 100.0, 50.0, 50.0);
    let tangents_before = path.tangents.clone();
    path.set_vertex_at_frame(0, 0, [120.0, 80.0]);
    let verts = path.vertices_at_frame(0);
    assert_eq!(verts[0], [120.0, 80.0]);
    assert_eq!(path.tangents, tangents_before);
}

#[test]
fn test_mask_ellipse_bezier_sampling() {
    let path = MaskPath::new_ellipse(100.0, 100.0, 50.0, 50.0);
    assert!(
        path.tangents.is_some(),
        "ellipse mask should have tangent handles"
    );
    let poly = path.to_polygon(0, 8);
    assert_eq!(
        poly.len(),
        4 * 8 + 1,
        "sampled polygon should contain curve segments"
    );
}

#[test]
fn test_insert_and_remove_vertex_maintains_topology() {
    let mut path = MaskPath::new_rect(0.0, 0.0, 100.0, 100.0);
    assert_eq!(path.vertices_at_frame(0).len(), 4);

    // Insert point in segment 0 (top edge) at t = 0.5
    let new_idx = path
        .insert_vertex_at_frame(0, 0, 0.5)
        .expect("insert succeeds");
    assert_eq!(new_idx, 1);
    let verts = path.vertices_at_frame(0);
    assert_eq!(verts.len(), 5);
    assert_eq!(verts[1], [50.0, 0.0]); // Midpoint of [0,0] -> [100,0]

    // Remove inserted vertex
    assert!(path.remove_vertex_at_frame(1));
    assert_eq!(path.vertices_at_frame(0).len(), 4);
}

#[test]
fn test_set_tangents_collinear_link() {
    let mut path = MaskPath::new_rect(0.0, 0.0, 100.0, 100.0);
    path.set_tangents_at_vertex(0, [-10.0, 0.0], [20.0, 0.0], true);
    let tangents = path.tangents.as_ref().unwrap();
    let (t_in, t_out) = tangents[0];
    assert_eq!(t_in, [-10.0, 0.0]);
    assert_eq!(t_out, [20.0, 0.0]); // Collinear opposite direction
}

#[test]
fn test_transform_selected_vertices_scale_and_translate() {
    let mut path = MaskPath::new_rect(0.0, 0.0, 100.0, 100.0);
    // Select all 4 vertices, scale by 2.0 around centroid [50, 50], and translate by [10, 20]
    path.transform_selected_vertices(0, &[0, 1, 2, 3], [2.0, 2.0], 0.0, [10.0, 20.0], None);
    let verts = path.vertices_at_frame(0);
    // Top-left [0, 0] scaled around [50, 50] -> [-50, -50] + [10, 20] = [-40, -30]
    assert_eq!(verts[0], [-40.0, -30.0]);
    // Bottom-right [100, 100] -> [150, 150] + [10, 20] = [160, 170]
    assert_eq!(verts[2], [160.0, 170.0]);
}
