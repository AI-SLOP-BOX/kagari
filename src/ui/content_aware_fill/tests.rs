use super::bake::*;
use crate::core::mask::Mask;
use crate::core::timeline::{Composition, Layer, LayerType};
use image::GenericImageView;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[test]
fn fill_source_isolated_to_selected_layer_and_ignores_the_selection_mask() {
    let mut comp = Composition::new("comp".into(), "Comp".into(), 16, 16, 30, 30);
    let mut selected = Layer::new(
        "selected".into(),
        "Selected".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    );
    selected.transform.position = crate::core::property::Animatable::new_constant([8.0, 8.0]);
    selected.masks.push(Mask::new_rect(
        "selection".into(),
        "Selection".into(),
        0.0,
        0.0,
        8.0,
        16.0,
    ));
    comp.layers.push(selected);
    comp.layers.push(Layer::new(
        "unrelated".into(),
        "Unrelated".into(),
        LayerType::Solid {
            color: [0.0, 0.0, 1.0, 1.0],
        },
        30,
    ));

    let source = prepare_caf_source(&comp, 0, 0).unwrap();
    let pixels = render_caf_source_frame(&source, 0, 0);
    let left = ((4 * 16 + 2) * 4) as usize;
    let right = ((4 * 16 + 12) * 4) as usize;
    assert_eq!(&pixels[left..left + 4], &[255, 0, 0, 255]);
    assert_eq!(&pixels[right..right + 4], &[255, 0, 0, 255]);
}

#[test]
fn selected_fill_ranges_are_clamped_to_the_layer_and_composition() {
    assert_eq!(caf_range(0, 100, Some(10), Some(80), 20, 90), Ok((20, 80)));
    assert_eq!(caf_range(1, 100, None, None, 20, 90), Ok((20, 90)));
    assert!(caf_range(0, 100, Some(0), Some(10), 20, 90).is_err());
    assert_eq!(caf_range(1, 10, None, None, 0, 99), Ok((0, 9)));
}

#[test]
fn range_bake_writes_decodable_webp_sequence_and_reports_progress() {
    let mut comp = Composition::new("comp".into(), "Comp".into(), 16, 16, 30, 2);
    let mut layer = Layer::new(
        "layer".into(),
        "Layer".into(),
        LayerType::Solid {
            color: [0.8, 0.2, 0.1, 1.0],
        },
        2,
    );
    layer.masks.push(Mask::new_rect(
        "mask".into(),
        "Mask".into(),
        4.0,
        4.0,
        12.0,
        12.0,
    ));
    comp.layers.push(layer);
    let source = prepare_caf_source(&comp, 0, 0).unwrap();
    let mask = &comp.layers[0].masks[0];
    let root = std::env::temp_dir().join(format!(
        "kagari-caf-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let output = caf_sequence_directory(&root).unwrap();
    let cancelled = AtomicBool::new(false);
    let progress = AtomicU32::new(0);
    bake_caf_sequence(CafBakeRequest {
        source: &source,
        layer_index: 0,
        mask,
        start_frame: 0,
        end_frame: 1,
        expansion: 2.0,
        method: crate::core::content_aware_engine::FillMethod::Object,
        asset_dir: &output,
        cancelled: &cancelled,
        progress: &progress,
    })
    .unwrap();

    for index in 0..2 {
        let path = output.join(format!("frame_{index:05}.webp"));
        let image = image::open(path).unwrap();
        assert_eq!(image.dimensions(), (16, 16));
    }
    assert_eq!(progress.load(Ordering::Relaxed), 2);
    std::fs::remove_dir_all(root).unwrap();
}
