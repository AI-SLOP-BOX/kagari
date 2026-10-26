#![allow(dead_code)]
use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::mask::MaskPath;
use crate::core::mask::{Mask, MaskMode};
use crate::core::timeline::{
    BlendMode, Composition, Layer, LayerType, Project, ShapeType, TrackMatteMode,
};
use serde_json::{json, Value};

mod importer;
mod layer;
use layer::serialize_layer;
mod validation;
pub use importer::*;
use validation::{collect_image_assets, validate_lottie_comp};

fn color_to_lottie(c: &[f32; 4]) -> Value {
    json!([c[0], c[1], c[2], c[3]])
}

fn hex_color(c: &[f32; 4]) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        (c[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (c[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (c[2].clamp(0.0, 1.0) * 255.0).round() as u8
    )
}
use crate::core::property::Animatable;

/// Exporter for converting Aura Composition into industry-standard Lottie / Bodymovin JSON format.
pub struct LottieExporter;

/// Serializes an Animatable scalar/vec as a Lottie transform property.
/// Constant → {"a":0,"k":v}; Animated → {"a":1,"k":[{t,s}...]}.
fn anim_property_f32(prop: &Animatable<f32>, scale: f32) -> Value {
    match prop {
        Animatable::Constant(v) => json!({ "a": 0, "k": *v * scale }),
        Animatable::Animated(kfs) => {
            let keys: Vec<Value> = kfs
                .iter()
                .map(|kf| {
                    json!({
                        "t": kf.frame,
                        "s": [kf.value * scale],
                        "i": { "x": [0.667], "y": [1.0] },
                        "o": { "x": [0.333], "y": [0.0] },
                    })
                })
                .collect();
            json!({ "a": 1, "k": keys })
        }
    }
}

fn anim_property_v2(prop: &Animatable<[f32; 2]>, scale: f32) -> Value {
    match prop {
        Animatable::Constant(v) => json!({ "a": 0, "k": [v[0] * scale, v[1] * scale] }),
        Animatable::Animated(kfs) => {
            let keys: Vec<Value> = kfs
                .iter()
                .map(|kf| {
                    json!({
                        "t": kf.frame,
                        "s": [kf.value[0] * scale, kf.value[1] * scale],
                        "i": { "x": [0.667, 0.667], "y": [1.0, 1.0] },
                        "o": { "x": [0.333, 0.333], "y": [0.0, 0.0] },
                    })
                })
                .collect();
            json!({ "a": 1, "k": keys })
        }
    }
}

fn blend_mode_code(bm: &BlendMode) -> i32 {
    match bm {
        BlendMode::Normal => 0,
        BlendMode::Multiply => 1,
        BlendMode::Screen => 2,
        BlendMode::Overlay => 3,
        BlendMode::Darken => 4,
        BlendMode::Lighten => 5,
        BlendMode::ColorDodge => 6,
        BlendMode::ColorBurn => 7,
        BlendMode::HardLight => 8,
        BlendMode::SoftLight => 9,
        BlendMode::Difference => 10,
        BlendMode::Exclusion => 11,
        BlendMode::Hue => 12,
        BlendMode::Saturation => 13,
        BlendMode::Color => 14,
        BlendMode::Luminosity => 15,
        BlendMode::Add => 16,
        BlendMode::Divide
        | BlendMode::Subtract
        | BlendMode::LinearBurn
        | BlendMode::VividLight
        | BlendMode::LinearDodge
        | BlendMode::StencilAlpha
        | BlendMode::StencilLuma
        | BlendMode::SilhouetteAlpha
        | BlendMode::SilhouetteLuma
        | BlendMode::Behind
        | BlendMode::AlphaAdd
        | BlendMode::LinearLight => 0,
    }
}

/// Serializes a ShapeType into the Lottie shape item ("el"/"rc"/"sr"/"sr").
fn shape_geometry(st: &ShapeType) -> Value {
    match st {
        ShapeType::Rectangle {
            width,
            height,
            corner_radius,
        } => json!({
            "ty": "rc",
            "d": 1,
            "s": anim_property_v2(&merge_dims(width, height), 1.0),
            "p": { "a": 0, "k": [0.0, 0.0] },
            "r": anim_property_f32(corner_radius, 1.0),
        }),
        ShapeType::Ellipse { width, height } => json!({
            "ty": "el",
            "d": 1,
            "s": anim_property_v2(&merge_dims(width, height), 1.0),
            "p": { "a": 0, "k": [0.0, 0.0] },
        }),
        ShapeType::Star {
            points,
            inner_radius,
            outer_radius,
        } => json!({
            "ty": "sr",
            "sy": 2,
            "d": 1,
            "pt": anim_property_f32(points, 1.0),
            "p": { "a": 0, "k": [0.0, 0.0] },
            "or": anim_property_f32(outer_radius, 1.0),
            "ir": anim_property_f32(inner_radius, 1.0),
            "os": { "a": 0, "k": 0 },
            "is": { "a": 0, "k": 0 },
            "r": { "a": 0, "k": 0 },
        }),
        ShapeType::Polygon { sides, radius } => json!({
            "ty": "sr",
            "sy": 1,
            "d": 1,
            "pt": anim_property_f32(sides, 1.0),
            "p": { "a": 0, "k": [0.0, 0.0] },
            "or": anim_property_f32(radius, 1.0),
            "os": { "a": 0, "k": 0 },
            "r": { "a": 0, "k": 0 },
        }),
        ShapeType::FreeformBezier { points, .. } => {
            // Convert points to Lottie shape vertices
            let verts: Vec<Value> = points.iter().map(|p| json!([p[0], p[1]])).collect();
            json!({
                "ty": "sh",
                "d": 1,
                "ks": { "a": 0, "k": { "i": Vec::<Value>::new(), "o": Vec::<Value>::new(), "v": verts, "c": true } },
            })
        }
    }
}

/// Combines separately-animated width/height into a single animated [w,h] property.
fn merge_dims(w: &Animatable<f32>, h: &Animatable<f32>) -> Animatable<[f32; 2]> {
    match (w, h) {
        (Animatable::Constant(wv), Animatable::Constant(hv)) => Animatable::Constant([*wv, *hv]),
        _ => {
            let mut frames = std::collections::BTreeSet::new();
            if let Some(keyframes) = w.keyframes() {
                frames.extend(keyframes.iter().map(|keyframe| keyframe.frame));
            }
            if let Some(keyframes) = h.keyframes() {
                frames.extend(keyframes.iter().map(|keyframe| keyframe.frame));
            }
            if frames.is_empty() {
                return Animatable::Constant([w.evaluate(0), h.evaluate(0)]);
            }
            Animatable::Animated(
                frames
                    .into_iter()
                    .map(|frame| {
                        Keyframe::new(
                            frame,
                            [w.evaluate(frame), h.evaluate(frame)],
                            InterpolationType::Linear,
                        )
                    })
                    .collect(),
            )
        }
    }
}

/// Builds the Lottie shapes array (a group with geometry + fill + stroke) for shape layers.
fn serialize_shapes(
    shape_type: &ShapeType,
    color: &[f32; 4],
    stroke_color: &[f32; 4],
    stroke_width: f32,
) -> Vec<Value> {
    let mut items = vec![shape_geometry(shape_type)];
    items.push(json!({
        "ty": "fl",
        "c": { "a": 0, "k": color_to_lottie(color) },
        "o": { "a": 0, "k": 100 },
        "nm": "Fill",
    }));
    if stroke_width > 0.0 {
        items.push(json!({
            "ty": "st",
            "c": { "a": 0, "k": color_to_lottie(stroke_color) },
            "o": { "a": 0, "k": 100 },
            "w": { "a": 0, "k": stroke_width },
            "lc": 2,
            "lj": 2,
            "nm": "Stroke",
        }));
    }
    items.push(json!({ "ty": "tr", "p": { "a": 0, "k": [0.0, 0.0] }, "a": { "a": 0, "k": [0.0, 0.0] }, "s": { "a": 0, "k": [100.0, 100.0] }, "r": { "a": 0, "k": 0 }, "o": { "a": 0, "k": 100 } }));
    vec![json!({ "ty": "gr", "it": items, "nm": "Shape", "np": items.len() })]
}

fn layer_type_code(
    lt: &LayerType,
    image_asset_ids: &std::collections::HashMap<String, String>,
) -> (i32, Value) {
    match lt {
        LayerType::Solid { color } => (
            1,
            json!({
                "sw": 512, "sh": 512,
                "sc": format!("#{:02x}{:02x}{:02x}",
                    (color[0] * 255.0) as u8, (color[1] * 255.0) as u8, (color[2] * 255.0) as u8),
            }),
        ),
        LayerType::Image { path } => (
            2,
            json!({ "refId": image_asset_ids.get(path).map(String::as_str).unwrap_or("missing_asset") }),
        ),
        LayerType::Null => (3, json!({})),
        LayerType::Text { .. } => (5, json!({})),
        LayerType::PreComp { comp_id } => (0, json!({ "refId": comp_id })),
        _ => (4, json!({})), // Shape / others default to shape
    }
}

/// Lottie matte type code from the app's TrackMatteMode.
/// 1=Alpha 2=AlphaInv 3=Luma 4=LumaInv (Lottie bodymovin spec).
fn track_matte_code(mode: &TrackMatteMode) -> i32 {
    match mode {
        TrackMatteMode::None => 0,
        TrackMatteMode::AlphaMatte => 1,
        TrackMatteMode::AlphaMatteInverted => 2,
        TrackMatteMode::LumaMatte => 3,
        TrackMatteMode::LumaMatteInverted => 4,
    }
}

/// Serializes a Text layer into the Lottie text document property ("t").
/// Without this the text content itself would be lost in export.
///
/// Key mapping follows the Bodymovin spec: "t" carries the TEXT STRING while
/// "tr" carries tracking — a duplicate "t" key would silently overwrite the
/// text with the tracking value.
fn text_document(layer_type: &LayerType) -> Option<Value> {
    if let LayerType::Text {
        text,
        font_size,
        color,
        font_family,
        tracking,
        leading,
        align,
        ..
    } = layer_type
    {
        // Alignment mapping: app 0=Left 1=Center 2=Right → Lottie j 0=Left 2=Center 1=Right.
        let justify = match align {
            1 => 2,
            2 => 1,
            _ => 0,
        };
        Some(json!({
            "d": { "k": [{ "s": {
                "t": text,
                "f": font_family,
                "s": font_size,
                "fc": [color[0], color[1], color[2]],
                "j": justify,
                "tr": tracking,
                "lh": *leading * (*font_size as f32),
            }, "t": 0 }] },
            "p": {},
            "m": { "g": 1, "a": { "a": 0, "k": [0.0, 0.0] } },
            "a": [],
        }))
    } else {
        None
    }
}

/// Lottie mask mode letter from the app's MaskMode.
fn mask_mode_code(mode: &MaskMode, enabled: bool) -> &'static str {
    if !enabled {
        return "n";
    }
    match mode {
        MaskMode::None => "n",
        MaskMode::Add => "a",
        MaskMode::Subtract => "s",
        MaskMode::Intersect => "i",
        MaskMode::Lighten => "l",
        MaskMode::Darken => "d",
        MaskMode::Difference => "f",
    }
}

/// Builds a Bodymovin bezier shape object (`{c,v,i,o}`) from position /
/// in-tangent / out-tangent triplets.
fn bezier_shape(triplets: &[([f32; 2], [f32; 2], [f32; 2])], is_closed: bool) -> Value {
    let mut v = Vec::with_capacity(triplets.len());
    let mut ti = Vec::with_capacity(triplets.len());
    let mut to = Vec::with_capacity(triplets.len());
    for (pos, tin, tout) in triplets {
        v.push(json!([pos[0], pos[1]]));
        ti.push(json!([tin[0], tin[1]]));
        to.push(json!([tout[0], tout[1]]));
    }
    json!({ "c": is_closed, "v": v, "i": ti, "o": to })
}

/// Zips mask path vertices with their tangent handles (zero handles when the
/// path has none).
fn mask_triplets(path: &MaskPath, verts: &[[f32; 2]]) -> Vec<([f32; 2], [f32; 2], [f32; 2])> {
    verts
        .iter()
        .enumerate()
        .map(|(i, pos)| match &path.tangents {
            Some(ts) if ts.len() > i => (*pos, ts[i].0, ts[i].1),
            _ => (*pos, [0.0, 0.0], [0.0, 0.0]),
        })
        .collect()
}

/// Serializes one scalar Animatable as a Lottie {"a","k"} property.
fn anim_property_scalar(prop: &Animatable<f32>) -> Value {
    match prop {
        Animatable::Constant(v) => json!({ "a": 0, "k": *v }),
        Animatable::Animated(kfs) => json!({
            "a": 1,
            "k": kfs.iter().map(|kf| json!({ "t": kf.frame, "s": [kf.value] })).collect::<Vec<_>>(),
        }),
    }
}

/// Serializes one mask into a bodymovin masksProperties entry. Animated
/// vertex tracks become shape keyframes ("pt"."a" = 1).
fn serialize_mask(mask: &Mask) -> Value {
    let pt = match &mask.path.vertices {
        Animatable::Constant(verts) => json!({
            "a": 0,
            "k": bezier_shape(&mask_triplets(&mask.path, verts), mask.path.is_closed),
        }),
        Animatable::Animated(kfs) => json!({
            "a": 1,
            "k": kfs.iter().map(|kf| json!({
                "t": kf.frame,
                "s": [bezier_shape(&mask_triplets(&mask.path, &kf.value), mask.path.is_closed)],
            })).collect::<Vec<_>>(),
        }),
    };
    json!({
        "inv": mask.inverted,
        "mode": mask_mode_code(&mask.mode, mask.enabled),
        "pt": pt,
        "o": anim_property_scalar(&mask.opacity),
        "x": anim_property_scalar(&mask.expansion),
        "nm": mask.name,
    })
}

/// `Some((hasMask, masksProperties))` payload when the layer carries masks.
fn masks_properties(masks: &[Mask]) -> Option<(Value, Value)> {
    if masks.is_empty() {
        return None;
    }
    let props: Vec<Value> = masks.iter().map(serialize_mask).collect();
    Some((json!(true), json!(props)))
}

impl LottieExporter {
    /// Serializes a Composition into a valid Lottie JSON string with animated transforms.
    pub fn export_to_json(comp: &Composition) -> Result<String, String> {
        validate_lottie_comp(comp, false)?;
        let (image_asset_ids, mut assets) = collect_image_assets(&[comp])?;
        let layers: Vec<Value> = comp
            .layers
            .iter()
            .enumerate()
            .map(|(i, layer)| {
                // A layer is a matte source when the NEXT layer consumes it.
                let is_source = comp
                    .layers
                    .get(i + 1)
                    .is_some_and(|next| !matches!(next.track_matte, TrackMatteMode::None));
                serialize_layer(layer, comp, i, is_source, &image_asset_ids)
            })
            .collect();

        let lottie_json = json!({
            "v": "5.7.4",
            "fr": comp.fps,
            "ip": 0,
            "op": comp.duration_frames,
            "w": comp.width,
            "h": comp.height,
            "nm": comp.name,
            "ddd": 0,
            "bg": hex_color(&comp.background_color),
            "assets": std::mem::take(&mut assets),
            "layers": layers,
        });

        serde_json::to_string_pretty(&lottie_json).map_err(|error| error.to_string())
    }
}

// ───────────────────── Project / Precomp Tree Export ─────────────────────

/// Flattens every composition reachable from the project (top-level list plus
/// nested `sub_compositions`, depth-first).
fn flatten_comps(project: &Project) -> Vec<&Composition> {
    let mut out = Vec::new();
    let mut stack: Vec<&Composition> = project.compositions.iter().collect();
    while let Some(c) = stack.pop() {
        for sub in &c.sub_compositions {
            stack.push(sub);
        }
        out.push(c);
    }
    out
}

fn serialize_comp_layers(
    comp: &Composition,
    image_asset_ids: &std::collections::HashMap<String, String>,
) -> Vec<Value> {
    comp.layers
        .iter()
        .enumerate()
        .map(|(i, layer)| {
            let is_source = comp
                .layers
                .get(i + 1)
                .is_some_and(|next| !matches!(next.track_matte, TrackMatteMode::None));
            serialize_layer(layer, comp, i, is_source, image_asset_ids)
        })
        .collect()
}

/// Exports the active composition together with every precomp it references
/// (transitively) as bodymovin assets. Precomp layers become `ty:0` slots
/// pointing at their asset id; unreferenced comps are omitted; missing
/// references keep their placeholder but emit no asset.
pub fn export_project_to_json(project: &Project) -> Result<String, String> {
    let Some(root) = project.compositions.get(project.active_composition_idx) else {
        return Err("No active composition to export".into());
    };

    let all = flatten_comps(project);
    let compositions_by_id = all
        .iter()
        .map(|composition| (composition.id.as_str(), *composition))
        .collect::<std::collections::HashMap<_, _>>();
    let mut reachable_compositions = Vec::new();
    let mut pending = vec![root];
    let mut visited = std::collections::HashSet::new();
    while let Some(composition) = pending.pop() {
        if !visited.insert(composition.id.as_str()) {
            continue;
        }
        reachable_compositions.push(composition);
        for layer in &composition.layers {
            if let LayerType::PreComp { comp_id } = &layer.layer_type {
                if let Some(child) = compositions_by_id.get(comp_id.as_str()) {
                    pending.push(*child);
                }
            }
        }
    }

    let (image_asset_ids, image_assets) = collect_image_assets(&reachable_compositions)?;
    for composition in &reachable_compositions {
        validate_lottie_comp(composition, true)?;
    }
    let precomp_assets = reachable_compositions
        .iter()
        .filter(|composition| composition.id.as_str() != root.id.as_str())
        .map(|composition| {
            json!({
                "id": composition.id,
                "nm": composition.name,
                "fr": composition.fps,
                "ip": 0,
                "op": composition.duration_frames,
                "w": composition.width,
                "h": composition.height,
                "layers": serialize_comp_layers(composition, &image_asset_ids),
            })
        })
        .collect::<Vec<_>>();

    // Root layers reuse the same serializer as assets.
    let layers = serialize_comp_layers(root, &image_asset_ids);
    let mut assets = image_assets;
    assets.extend(precomp_assets);
    let lottie_json = json!({
        "v": "5.7.4",
        "fr": root.fps,
        "ip": 0,
        "op": root.duration_frames,
        "w": root.width,
        "h": root.height,
        "nm": root.name,
        "ddd": 0,
        "bg": hex_color(&root.background_color),
        "assets": assets,
        "layers": layers,
    });
    serde_json::to_string_pretty(&lottie_json).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
