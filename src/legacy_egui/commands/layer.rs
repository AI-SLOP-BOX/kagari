use crate::KagariApp;

#[derive(Clone, Copy)]
pub enum NewLayerKind {
    Solid,
    Text,
    Null,
    Adjustment,
}

pub fn create_new_layer(app: &mut KagariApp, kind: NewLayerKind) {
    let mut created_idx = None;
    app.modify_project(|project| {
        let comp = project.active_composition_mut();
        let index = comp.layers.len();
        let mut id_index = index;
        while comp
            .layers
            .iter()
            .any(|layer| layer.id == format!("layer_{id_index}"))
        {
            id_index += 1;
        }
        let layer_id = format!("layer_{id_index}");
        let total_frames = comp.duration_frames;
        let (name, layer) = match kind {
            NewLayerKind::Solid => (
                format!("Solid {index}"),
                crate::core::timeline::Layer::new(
                    layer_id.clone(),
                    format!("Solid {index}"),
                    crate::core::timeline::LayerType::Solid {
                        color: [0.2, 0.5, 0.9, 1.0],
                    },
                    total_frames,
                ),
            ),
            NewLayerKind::Text => (
                format!("Text {index}"),
                crate::core::timeline::Layer::new(
                    layer_id.clone(),
                    format!("Text {index}"),
                    crate::core::timeline::LayerType::new_text(
                        "Title Text",
                        72,
                        [1.0, 1.0, 1.0, 1.0],
                    ),
                    total_frames,
                ),
            ),
            NewLayerKind::Null => (
                format!("Null {index}"),
                crate::core::timeline::Layer::new_null(
                    layer_id.clone(),
                    format!("Null {index}"),
                    total_frames,
                ),
            ),
            NewLayerKind::Adjustment => (
                format!("Adjustment Layer {index}"),
                crate::core::timeline::Layer::new_adjustment(
                    layer_id,
                    format!("Adjustment Layer {index}"),
                    total_frames,
                ),
            ),
        };
        comp.add_layer(layer);
        created_idx = Some((index, name));
    });
    if let Some((index, name)) = created_idx {
        app.selection.select_single(index);
        app.toasts.info(format!("Created {name}"));
        crate::core::frame_cache::bump_version();
    }
}
