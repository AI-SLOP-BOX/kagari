use super::*;

pub(super) fn serialize_layer(
    layer: &Layer,
    comp: &Composition,
    index: usize,
    is_matte_source: bool,
    image_asset_ids: &std::collections::HashMap<String, String>,
) -> Value {
    let (ty, extra) = layer_type_code(&layer.layer_type, image_asset_ids);
    let in_frame = layer.in_frame;
    let out_frame = layer
        .out_frame
        .max(comp.duration_frames.min(layer.out_frame));

    let mut l = json!({
        "ddd": 0,
        "ind": index + 1,
        "ty": ty,
        "nm": layer.name,
        "sr": 1,
        "ks": {
            "o": anim_property_f32(&layer.transform.opacity, 1.0),
            "r": anim_property_f32(&layer.transform.rotation, 1.0),
            "p": anim_property_v2(&layer.transform.position, 1.0),
            "a": anim_property_v2(&layer.transform.anchor_point, 1.0),
            "s": anim_property_v2(&layer.transform.scale, 1.0),
        },
        "ao": 0,
        "shapes": [],
        "ip": in_frame,
        "op": out_frame.max(in_frame + 1),
        "st": 0,
        "bm": blend_mode_code(&layer.blend_mode),
    });
    if let Some(obj) = l.as_object_mut() {
        if let Some(parent_id) = &layer.parent_id {
            // Map parent id to its layer index (Lottie uses indices)
            if let Some(pidx) = comp.layers.iter().position(|l| &l.id == parent_id) {
                obj.insert("parent".into(), json!(pidx + 1));
            }
        }
        for (k, v) in extra.as_object().cloned().unwrap_or_default() {
            obj.insert(k, v);
        }
        // Precomp placeholders carry the containing comp's dimensions so
        // players can size the referenced composition slot.
        if matches!(layer.layer_type, LayerType::PreComp { .. }) {
            obj.insert("w".into(), json!(comp.width));
            obj.insert("h".into(), json!(comp.height));
        }
        if layer.motion_blur {
            obj.insert("mb".into(), json!(1));
        }
        if let LayerType::Shape {
            shape_type,
            color,
            stroke_color,
            stroke_width,
            ..
        } = &layer.layer_type
        {
            obj.insert(
                "shapes".into(),
                json!(serialize_shapes(
                    shape_type,
                    color,
                    stroke_color,
                    *stroke_width
                )),
            );
        }
        if let Some(doc) = text_document(&layer.layer_type) {
            obj.insert("t".into(), doc);
        }
        // Track mattes: this layer consumes the layer above it as its matte.
        if !matches!(layer.track_matte, TrackMatteMode::None) {
            obj.insert("tt".into(), json!(track_matte_code(&layer.track_matte)));
        }
        // The layer directly above a matte consumer becomes the matte source.
        if is_matte_source {
            obj.insert("td".into(), json!(1));
        }
        if let Some((has_mask, props)) = masks_properties(&layer.masks) {
            obj.insert("hasMask".into(), has_mask);
            obj.insert("masksProperties".into(), props);
        }
    }
    l
}
