use super::super::render_frame_to_pixels;
use crate::core::property::Animatable;
use crate::core::timeline::{Composition, Layer, LayerType};

#[test]
fn model_layer_renders_an_obj_triangle() {
    let path = std::env::temp_dir().join(format!("kagari-model3d-test-{}.obj", std::process::id()));
    std::fs::write(&path, "v -0.5 -0.5 0\nv 0.5 -0.5 0\nv 0 0.5 0\nf 1 2 3\n")
        .expect("write test mesh");

    let mut comp = Composition::new("comp".into(), "Comp".into(), 64, 64, 24, 24);
    let mut layer = Layer::new(
        "mesh".into(),
        "Mesh".into(),
        LayerType::Model3D {
            path: path.to_string_lossy().into_owned(),
        },
        comp.duration_frames,
    );
    layer.is_3d = true;
    layer.transform.position = Animatable::new_constant([32.0, 32.0]);
    layer.transform_3d.position = Animatable::new_constant([0.0, 0.0, 600.0]);
    comp.layers.push(layer);

    let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    let model_pixels = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel[0] > 25 || pixel[1] > 25 || pixel[2] > 25)
        .count();
    assert!(
        model_pixels > 0,
        "OBJ triangle did not reach the compositor"
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn model_layer_renders_inside_a_precomposition() {
    let path = std::env::temp_dir().join(format!(
        "kagari-model3d-precomp-test-{}.obj",
        std::process::id()
    ));
    std::fs::write(&path, "v -0.5 -0.5 0\nv 0.5 -0.5 0\nv 0 0.5 0\nf 1 2 3\n")
        .expect("write test mesh");

    let mut nested = Composition::new("nested".into(), "Nested".into(), 64, 64, 24, 24);
    let mut model = Layer::new(
        "mesh".into(),
        "Mesh".into(),
        LayerType::Model3D {
            path: path.to_string_lossy().into_owned(),
        },
        nested.duration_frames,
    );
    model.is_3d = true;
    model.transform.position = Animatable::new_constant([32.0, 32.0]);
    model.transform_3d.position = Animatable::new_constant([0.0, 0.0, 600.0]);
    nested.layers.push(model);

    let mut root = Composition::new("root".into(), "Root".into(), 64, 64, 24, 24);
    root.sub_compositions.push(nested);
    let mut precomp = Layer::new(
        "precomp".into(),
        "Nested".into(),
        LayerType::PreComp {
            comp_id: "nested".into(),
        },
        root.duration_frames,
    );
    precomp.transform.position = Animatable::new_constant([32.0, 32.0]);
    root.layers.push(precomp);

    let pixels = render_frame_to_pixels(&root, 0, 64, 64, 0.0, 0);
    let model_pixels = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel[0] > 25 || pixel[1] > 25 || pixel[2] > 25)
        .count();
    assert!(
        model_pixels > 0,
        "nested OBJ triangle did not reach the compositor"
    );

    let _ = std::fs::remove_file(path);
}
