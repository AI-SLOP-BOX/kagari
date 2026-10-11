use super::*;

// ─────────────────────── Lottie / Bodymovin Import ───────────────────────

#[path = "importer_parsing.rs"]
mod parsing;
use parsing::*;

/// Imports Bodymovin/Lottie JSON into a [`Composition`]. Unknown constructs
/// degrade gracefully to null layers; invalid JSON yields None.
pub struct LottieImporter;

impl LottieImporter {
    pub fn import_from_file(path: &str) -> Option<Composition> {
        let json = std::fs::read_to_string(path).ok()?;
        Self::import_from_str(&json)
    }

    pub fn import_from_str(json: &str) -> Option<Composition> {
        let v: Value = serde_json::from_str(json).ok()?;
        Self::from_value(&v)
    }

    fn from_value(v: &Value) -> Option<Composition> {
        Self::parse_comp(v, None)
    }

    /// Builds one Composition from a bodymovin comp/asset object.
    /// `id_override` lets asset entries keep their published bodymovin id.
    fn parse_comp(v: &Value, id_override: Option<&str>) -> Option<Composition> {
        let layers_src = v.get("layers")?.as_array()?;
        let fps = v
            .get("fr")
            .and_then(Value::as_f64)
            .unwrap_or(30.0)
            .round()
            .max(1.0) as u32;
        let ip = v.get("ip").and_then(Value::as_f64).unwrap_or(0.0).max(0.0);
        let op = v.get("op").and_then(Value::as_f64).unwrap_or(ip + 1.0);
        let duration = ((op - ip).round() as u32).max(1);
        let width = v
            .get("w")
            .and_then(Value::as_u64)
            .unwrap_or(1920)
            .clamp(1, 16384) as u32;
        let height = v
            .get("h")
            .and_then(Value::as_u64)
            .unwrap_or(1080)
            .clamp(1, 16384) as u32;
        let name = v
            .get("nm")
            .and_then(Value::as_str)
            .unwrap_or("Lottie Import");
        let comp_id = id_override
            .map(str::to_string)
            .unwrap_or_else(|| format!("lottie_{}", name.replace(' ', "_")));

        let mut comp = Composition::new(comp_id, name.to_string(), width, height, fps, duration);
        comp.background_color =
            hex_to_rgba(v.get("bg").and_then(Value::as_str).unwrap_or("#000000"));

        let mut inds: Vec<i64> = Vec::with_capacity(layers_src.len());
        for (i, lj) in layers_src.iter().enumerate() {
            let ty = lj.get("ty").and_then(Value::as_i64).unwrap_or(3);
            let lname = lj
                .get("nm")
                .and_then(Value::as_str)
                .unwrap_or("Layer")
                .to_string();
            let id = format!("imp_l{i}");

            let mut layer = match ty {
                1 => Layer::new(
                    id,
                    lname,
                    LayerType::Solid {
                        color: hex_to_rgba(
                            lj.get("sc").and_then(Value::as_str).unwrap_or("#ffffff"),
                        ),
                    },
                    duration,
                ),
                2 => Layer::new(
                    id,
                    lname,
                    LayerType::Image {
                        path: lj
                            .get("refId")
                            .and_then(Value::as_str)
                            .unwrap_or("missing_asset")
                            .to_string(),
                    },
                    duration,
                ),
                5 => {
                    let doc = lj
                        .get("t")
                        .and_then(|t| t.get("d"))
                        .and_then(|d| d.get("k"))
                        .and_then(Value::as_array)
                        .and_then(|k| k.first());
                    match doc {
                        Some(d) if d.get("s").is_some() => {
                            Layer::new(id, lname, text_layer_type(d), duration)
                        }
                        _ => Layer::new(id, lname, LayerType::Null, duration),
                    }
                }
                4 => match lj.get("shapes").and_then(parse_shape_group) {
                    Some((shape_type, color, stroke_color, stroke_width)) => Layer::new(
                        id,
                        lname,
                        LayerType::Shape {
                            shape_type,
                            color,
                            stroke_color,
                            stroke_width,
                            fill_type: Default::default(),
                            extrusion_depth: 0.0,
                            bevel_depth: 0.0,
                        },
                        duration,
                    ),
                    None => Layer::new(id, lname, LayerType::Null, duration),
                },
                0 => {
                    let ref_id = lj
                        .get("refId")
                        .and_then(Value::as_str)
                        .unwrap_or("missing_precomp")
                        .to_string();
                    Layer::new(id, lname, LayerType::PreComp { comp_id: ref_id }, duration)
                }
                _ => Layer::new(id, lname, LayerType::Null, duration),
            };

            if let Some(ks) = lj.get("ks") {
                layer.transform = parse_transform(ks);
            }
            let ipf = lj.get("ip").and_then(Value::as_f64).unwrap_or(0.0).max(0.0) as u32;
            let opf = lj
                .get("op")
                .and_then(Value::as_f64)
                .unwrap_or(ipf as f64 + 1.0);
            layer.in_frame = ipf;
            layer.out_frame = (opf as u32).max(ipf + 1);
            layer.blend_mode = blend_from_code(lj.get("bm").and_then(Value::as_i64).unwrap_or(0));
            if lj.get("mb").and_then(Value::as_i64) == Some(1) {
                layer.motion_blur = true;
            }
            if let Some(tt) = lj.get("tt").and_then(Value::as_i64) {
                layer.track_matte = matte_from_code(tt);
            }
            if let Some(mps) = lj.get("masksProperties").and_then(Value::as_array) {
                layer.masks = mps
                    .iter()
                    .enumerate()
                    .map(|(mi, m)| import_mask(m, mi))
                    .collect();
            }

            inds.push(
                lj.get("ind")
                    .and_then(Value::as_i64)
                    .unwrap_or(i as i64 + 1),
            );
            comp.layers.push(layer);
        }

        // Resolve parenting: bodymovin "parent" references another layer's ind.
        for (i, lj) in layers_src.iter().enumerate() {
            if let Some(p) = lj.get("parent").and_then(Value::as_i64) {
                if let Some(pos) = inds.iter().position(|&x| x == p) {
                    if pos != i {
                        comp.layers[i].parent_id = Some(comp.layers[pos].id.clone());
                    }
                }
            }
        }

        Some(comp)
    }

    /// Imports a full bodymovin project: the root composition plus every
    /// precomp asset, preserving `ty:0` references as [`LayerType::PreComp`].
    pub fn import_project_from_str(json: &str) -> Option<crate::core::timeline::Project> {
        let v: Value = serde_json::from_str(json).ok()?;
        let mut comps = vec![Self::parse_comp(&v, None)?];
        if let Some(assets) = v.get("assets").and_then(Value::as_array) {
            for a in assets {
                if a.get("layers").and_then(Value::as_array).is_some() {
                    let id = a.get("id").and_then(Value::as_str);
                    if let Some(c) = Self::parse_comp(a, id) {
                        comps.push(c);
                    }
                }
            }
        }
        Some(crate::core::timeline::Project {
            compositions: comps,
            active_composition_idx: 0,
            assets: Vec::new(),
            use_gpu_compute: false,
        })
    }
}
