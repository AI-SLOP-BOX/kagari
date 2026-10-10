//! MLT XML and OpenShot JSON exporters.

use super::xml_util::escape_xml;
use crate::core::timeline::{Composition, LayerType};

pub struct MltExporter;

impl MltExporter {
    /// Serializes supported layers into MLT XML and refuses lossy layer-type conversion.
    pub fn export_to_xml(comp: &Composition) -> Result<String, String> {
        for layer in &comp.layers {
            let unsupported_kind = match &layer.layer_type {
                LayerType::Null
                | LayerType::Image { .. }
                | LayerType::Video { .. }
                | LayerType::Audio { .. }
                | LayerType::Solid { .. } => None,
                LayerType::Model3D { .. } => Some("3D model"),
                LayerType::Text { .. } => Some("text"),
                LayerType::Shape { .. } => Some("shape"),
                LayerType::PreComp { .. } => Some("precomposition"),
                LayerType::AdjustmentLayer => Some("adjustment layer"),
                LayerType::Particle { .. } => Some("particle"),
            };
            if let Some(kind) = unsupported_kind {
                return Err(format!(
                    "MLT export cannot represent {kind} layer '{}'",
                    layer.name
                ));
            }
        }
        if comp.width == 0 || comp.height == 0 || comp.duration_frames == 0 || comp.fps == 0 {
            return Err("MLT export requires non-zero dimensions, duration, and frame rate".into());
        }
        let mut xml = String::with_capacity(4096);
        xml.push_str("<?xml version=\"1.0\" standalone=\"no\"?>\n");
        xml.push_str(
            "<mlt LC_NUMERIC=\"C\" version=\"7.0.0\" title=\"Exported from Kagari VFX\">\n",
        );
        xml.push_str("  <profile description=\"automatic\" ");
        xml.push_str(&format!(
            "width=\"{}\" height=\"{}\" progressive=\"1\" ",
            comp.width, comp.height
        ));
        xml.push_str(&format!(
            "sample_aspect_numeral=\"1\" sample_aspect_denominator=\"1\" display_aspect_numeral=\"{}\" display_aspect_denominator=\"{}\" frame_rate_num=\"{}\" frame_rate_den=\"1\" colorspace=\"709\"/>\n",
            comp.width, comp.height, comp.fps
        ));

        // Producers: one per media-backed layer
        xml.push_str(&format!(
            "  <producer id=\"black\" in=\"0\" out=\"{}\">\n",
            comp.duration_frames - 1
        ));
        xml.push_str(&format!(
            "    <property name=\"length\">{}</property>\n",
            comp.duration_frames
        ));
        xml.push_str("    <property name=\"resource\">0</property>\n");
        xml.push_str("    <property name=\"mlt_service\">color</property>\n");
        xml.push_str(&format!(
            "    <property name=\"background\">#{:02x}{:02x}{:02x}</property>\n",
            (comp.background_color[0] * 255.0) as u8,
            (comp.background_color[1] * 255.0) as u8,
            (comp.background_color[2] * 255.0) as u8
        ));
        xml.push_str("  </producer>\n");

        let mut exported_layers = Vec::new();
        for (idx, layer) in comp.layers.iter().enumerate() {
            if !layer.visible || layer.in_frame >= comp.duration_frames {
                continue;
            }
            let duration_frames = layer
                .out_frame
                .min(comp.duration_frames)
                .saturating_sub(layer.in_frame);
            if duration_frames == 0 {
                continue;
            }
            let resource = match &layer.layer_type {
                LayerType::Image { path } => Some(path.clone()),
                LayerType::Video { source, .. } => Some(source.clone()),
                LayerType::Audio { path, .. } => Some(path.clone()),
                LayerType::Solid { color } => {
                    // Solids become color producers so the grade survives roundtrip
                    let to_hex = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                    Some(format!(
                        "color:{:02x}{:02x}{:02x}",
                        to_hex(color[0]),
                        to_hex(color[1]),
                        to_hex(color[2]),
                    ))
                }
                _ => None,
            };
            let Some(resource) = resource else { continue };
            let pid = format!("producer{}", idx);
            let source_out = duration_frames - 1;
            xml.push_str(&format!(
                "  <producer id=\"{}\" in=\"0\" out=\"{}\">\n",
                pid, source_out
            ));
            xml.push_str(&format!(
                "    <property name=\"length\">{}</property>\n",
                duration_frames
            ));
            xml.push_str(&format!(
                "    <property name=\"resource\">{}</property>\n",
                escape_xml(&resource)
            ));
            let service = match &layer.layer_type {
                LayerType::Solid { .. } => "color".to_string(),
                LayerType::Audio { .. } => "avformat-novalidate".to_string(),
                _ => "avformat".to_string(),
            };
            xml.push_str(&format!(
                "    <property name=\"mlt_service\">{}</property>\n",
                service
            ));
            if let LayerType::Audio { volume, .. } = &layer.layer_type {
                xml.push_str(&format!(
                    "    <property name=\"volume\">{}</property>\n",
                    10.0f32.powf(volume.evaluate(0) / 20.0)
                ));
            }
            xml.push_str(&format!(
                "    <property name=\"name\">{}</property>\n",
                escape_xml(&layer.name)
            ));
            xml.push_str("  </producer>\n");
            exported_layers.push((idx, layer, duration_frames));
        }

        let timeline_out = comp.duration_frames - 1;
        xml.push_str(&format!(
            "  <playlist id=\"background\">\n    <entry producer=\"black\" in=\"0\" out=\"{}\"/>\n  </playlist>\n",
            timeline_out
        ));

        let mut visual_track_indices = Vec::new();
        let mut audio_track_indices = Vec::new();
        for (playlist_index, (idx, layer, duration_frames)) in exported_layers.iter().enumerate() {
            xml.push_str(&format!("  <playlist id=\"playlist{idx}\">\n"));
            if layer.in_frame > 0 {
                xml.push_str(&format!("    <blank length=\"{}\"/>\n", layer.in_frame));
            }
            let source_out = *duration_frames - 1;
            xml.push_str(&format!(
                "    <entry producer=\"producer{}\" in=\"0\" out=\"{}\">\n",
                idx, source_out
            ));
            xml.push_str(&format!(
                "      <property name=\"name\">{}</property>\n",
                escape_xml(&layer.name)
            ));
            xml.push_str("    </entry>\n  </playlist>\n");
            let mlt_track_index = playlist_index + 1;
            match &layer.layer_type {
                LayerType::Audio { .. } => audio_track_indices.push(mlt_track_index),
                LayerType::Video { .. } => {
                    visual_track_indices.push(mlt_track_index);
                    audio_track_indices.push(mlt_track_index);
                }
                _ => visual_track_indices.push(mlt_track_index),
            }
        }

        // MLT stores parallel edits as playlists in a multitrack tractor.
        xml.push_str(&format!(
            "  <tractor id=\"tractor0\" global_feed=\"1\" title=\"{}\" in=\"0\" out=\"{}\">\n",
            escape_xml(&comp.name),
            timeline_out
        ));
        xml.push_str(
            "    <multitrack id=\"multitrack0\">\n      <track producer=\"background\"/>\n",
        );
        for (idx, layer, _) in &exported_layers {
            let hide = if matches!(&layer.layer_type, LayerType::Audio { .. }) {
                " hide=\"video\""
            } else {
                ""
            };
            xml.push_str(&format!(
                "      <track producer=\"playlist{idx}\"{hide}/>\n"
            ));
        }
        xml.push_str("    </multitrack>\n");
        for track_index in visual_track_indices {
            xml.push_str(&format!(
                "    <transition id=\"composite{track_index}\">\n      <property name=\"a_track\">0</property>\n      <property name=\"b_track\">{track_index}</property>\n      <property name=\"mlt_service\">qtblend</property>\n      <property name=\"always_active\">1</property>\n    </transition>\n"
            ));
        }
        if let Some((&base_audio, other_audio)) = audio_track_indices.split_first() {
            for &track_index in other_audio {
                xml.push_str(&format!(
                    "    <transition id=\"mix{track_index}\">\n      <property name=\"a_track\">{base_audio}</property>\n      <property name=\"b_track\">{track_index}</property>\n      <property name=\"mlt_service\">mix</property>\n      <property name=\"always_active\">1</property>\n      <property name=\"sum\">1</property>\n    </transition>\n"
                ));
            }
        }
        xml.push_str("  </tractor>\n");
        xml.push_str("</mlt>\n");
        Ok(xml)
    }
}

/// OpenShot Project (.osp) Exporter for OpenShot video editor interoperability.
pub struct OpenShotExporter;

impl OpenShotExporter {
    /// Serializes composition layers into OpenShot (.osp) project JSON.
    pub fn export_to_json(comp: &Composition) -> Result<String, String> {
        let mut files = Vec::new();
        let mut clips = Vec::new();

        for (idx, layer) in comp.layers.iter().enumerate() {
            let unsupported_kind = match &layer.layer_type {
                LayerType::Null
                | LayerType::Image { .. }
                | LayerType::Video { .. }
                | LayerType::Audio { .. } => None,
                LayerType::Solid { .. } => Some("solid"),
                LayerType::Model3D { .. } => Some("3D model"),
                LayerType::Text { .. } => Some("text"),
                LayerType::Shape { .. } => Some("shape"),
                LayerType::PreComp { .. } => Some("precomposition"),
                LayerType::AdjustmentLayer => Some("adjustment"),
                LayerType::Particle { .. } => Some("particle"),
            };
            if let Some(kind) = unsupported_kind {
                return Err(format!(
                    "OpenShot export cannot represent {kind} layer '{}'",
                    layer.name
                ));
            }
            let path = match &layer.layer_type {
                LayerType::Image { path } => Some(path.clone()),
                LayerType::Video { source, .. } => Some(source.clone()),
                LayerType::Audio { path, .. } => Some(path.clone()),
                _ => None,
            };

            if let Some(resource_path) = path {
                let file_id = format!("file_{}", idx);
                let duration_sec =
                    (layer.out_frame as f64 - layer.in_frame as f64) / comp.fps.max(1) as f64;
                let position_sec = layer.in_frame as f64 / comp.fps.max(1) as f64;

                files.push(serde_json::json!({
                    "id": file_id,
                    "path": resource_path,
                    "media_type": if matches!(layer.layer_type, LayerType::Audio { .. }) { "audio" } else { "video" },
                    "width": comp.width,
                    "height": comp.height,
                    "fps": {"num": comp.fps, "den": 1},
                }));

                clips.push(serde_json::json!({
                    "file_id": file_id,
                    "layer": idx + 1,
                    "position": position_sec,
                    "start": 0.0,
                    "end": duration_sec,
                    "scale": 1.0,
                    "alpha": {"Points": [{"co": {"X": 1.0, "Y": 1.0}}]},
                }));
            }
        }

        let osp = serde_json::json!({
            "width": comp.width,
            "height": comp.height,
            "fps": {"num": comp.fps, "den": 1},
            "sample_rate": 48000,
            "channels": 2,
            "channel_layout": 3,
            "files": files,
            "clips": clips,
            "layers": [{"number": 1, "y": 0, "label": "Track 1"}],
        });

        serde_json::to_string_pretty(&osp)
            .map_err(|e| format!("Failed to serialize OpenShot JSON: {}", e))
    }
}
