use super::detail::asset_kind_matches;
use crate::core::timeline::{ProjectItem, ProjectItemType};

#[test]
fn asset_kind_filter_separates_models_from_footage() {
    let model = ProjectItem::new(
        "model",
        "fire",
        ProjectItemType::Model3D {
            path: "fire.glb".to_owned(),
        },
    );
    let footage = ProjectItem::new(
        "video",
        "shot",
        ProjectItemType::Video {
            path: "shot.mov".to_owned(),
            duration_sec: 2.0,
        },
    );

    assert!(asset_kind_matches(&model, "model3d"));
    assert!(!asset_kind_matches(&footage, "model3d"));
    assert!(asset_kind_matches(&footage, "video"));
    assert!(asset_kind_matches(&model, "all"));
}
