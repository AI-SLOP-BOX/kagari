use super::operators::*;
use super::tree::*;
use crate::core::mask::MaskVertex;

#[test]
fn test_pucker_bloat_displacement() {
    let vertices = vec![MaskVertex::new(-10.0, -10.0), MaskVertex::new(10.0, 10.0)];

    let options = PuckerBloatOptions { amount: 50.0 };
    let bloat = apply_pucker_bloat(&vertices, &options);

    assert_eq!(bloat.len(), 2);
    assert!(bloat[1].position[0] > 10.0);
}

#[test]
fn test_round_corners_generates_tangents() {
    let rect = vec![
        MaskVertex::new(0.0, 0.0),
        MaskVertex::new(100.0, 0.0),
        MaskVertex::new(100.0, 100.0),
        MaskVertex::new(0.0, 100.0),
    ];
    let rounded = apply_round_corners(&rect, 20.0, true);
    // Each corner splits into 2 vertices (incoming & outgoing tangent) => 8 vertices
    assert_eq!(rounded.len(), 8);
}

#[test]
fn test_evaluate_shape_tree_with_repeater() {
    let tree = vec![
        ShapeContentItem::Rectangle {
            name: "Rect".into(),
            size: [50.0, 50.0],
            position: [0.0, 0.0],
            roundness: 0.0,
        },
        ShapeContentItem::Repeater {
            name: "Repeater".into(),
            copies: 3,
            offset: 0.0,
            transform: ShapeTransform {
                position: [100.0, 0.0],
                ..Default::default()
            },
        },
        ShapeContentItem::Fill {
            name: "Fill".into(),
            color: [1.0, 0.0, 0.0, 1.0],
            opacity: 100.0,
        },
    ];
    let paths = evaluate_shape_tree(&tree);
    assert_eq!(paths.len(), 3);
    assert_eq!(paths[0].fill_color, Some([1.0, 0.0, 0.0, 1.0]));
    assert_eq!(paths[1].vertices[0].position[0], 75.0); // 1st copy shifted by 100
}

#[test]
fn test_shape_tree_zero_copy_repeater_removes_paths() {
    let tree = vec![
        ShapeContentItem::Rectangle {
            name: "Rect".into(),
            size: [20.0, 20.0],
            position: [0.0, 0.0],
            roundness: 0.0,
        },
        ShapeContentItem::Repeater {
            name: "Empty".into(),
            copies: 0,
            offset: 0.0,
            transform: ShapeTransform::default(),
        },
    ];
    assert!(evaluate_shape_tree(&tree).is_empty());
}

#[test]
fn test_shape_tree_repeater_sanitizes_nonfinite_transform() {
    let tree = vec![
        ShapeContentItem::Rectangle {
            name: "Rect".into(),
            size: [20.0, 20.0],
            position: [0.0, 0.0],
            roundness: 0.0,
        },
        ShapeContentItem::Repeater {
            name: "Safe".into(),
            copies: 2,
            offset: f32::NAN,
            transform: ShapeTransform {
                position: [f32::INFINITY, f32::NEG_INFINITY],
                rotation: f32::NAN,
                scale: [f32::INFINITY, f32::NAN],
                ..ShapeTransform::default()
            },
        },
    ];
    for path in evaluate_shape_tree(&tree) {
        for vertex in path.vertices {
            assert!(vertex.position.iter().all(|value| value.is_finite()));
        }
    }
}

#[test]
fn test_evaluate_shape_tree_with_merge_paths() {
    let tree = vec![
        ShapeContentItem::Rectangle {
            name: "RectA".into(),
            size: [100.0, 100.0],
            position: [0.0, 0.0],
            roundness: 0.0,
        },
        ShapeContentItem::Rectangle {
            name: "RectB".into(),
            size: [100.0, 100.0],
            position: [50.0, 50.0],
            roundness: 0.0,
        },
        ShapeContentItem::MergePaths {
            name: "Merge".into(),
            mode: MergePathsMode::Add,
        },
        ShapeContentItem::Fill {
            name: "Fill".into(),
            color: [0.0, 1.0, 0.0, 1.0],
            opacity: 100.0,
        },
    ];
    let paths = evaluate_shape_tree(&tree);
    assert!(!paths.is_empty());
    assert_eq!(paths[0].fill_color, Some([0.0, 1.0, 0.0, 1.0]));
}

#[test]
fn test_trim_path_vertices_shortens_polyline() {
    let raw = vec![MaskVertex::new(0.0, 0.0), MaskVertex::new(100.0, 0.0)];
    let trimmed = trim_path_vertices(&raw, 25.0, 75.0, 0.0, false);
    assert!(trimmed.len() >= 2);
    assert!((trimmed[0].position[0] - 25.0).abs() < 1e-3);
    assert!((trimmed[1].position[0] - 75.0).abs() < 1e-3);
}

#[test]
fn test_apply_offset_paths_expands_rect() {
    let rect = vec![
        MaskVertex::new(0.0, 0.0),
        MaskVertex::new(100.0, 0.0),
        MaskVertex::new(100.0, 100.0),
        MaskVertex::new(0.0, 100.0),
    ];
    let offset = apply_offset_paths(&rect, 10.0, OffsetPathsJoin::Miter, 4.0);
    assert_eq!(offset.len(), 4);
    assert!(offset[0].position[0] < 0.0 && offset[0].position[1] < 0.0);
}
