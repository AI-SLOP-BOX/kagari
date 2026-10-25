use super::*;

mod standard;
use standard::{is_video_media_path, parse_standard_otio};

// --- 1. OpenTimelineIO (OTIO) / MLT Interoperability ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtioItem {
    pub name: String,
    pub source_range: [u32; 2], // [start_frame, duration]
    #[serde(default)]
    pub timeline_start_frame: Option<u32>,
    #[serde(default)]
    pub native_layer: Option<serde_json::Value>,
    pub media_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtioTrack {
    pub name: String,
    pub kind: String, // "video" or "audio"
    pub items: Vec<OtioItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtioTimeline {
    pub name: String,
    pub global_start_time: u32,
    pub fps: f64,
    /// Kagari's native dimensions. External OTIO files may omit these fields;
    /// the import path then falls back to a standard HD composition.
    #[serde(default = "default_otio_width")]
    pub width: u32,
    #[serde(default = "default_otio_height")]
    pub height: u32,
    #[serde(default = "default_otio_duration")]
    pub duration_frames: u32,
    pub tracks: Vec<OtioTrack>,
}

#[derive(Serialize, Deserialize)]
struct VideoMediaReference {
    source: String,
    frames_dir: String,
    frame_count: u32,
    audio_wav: Option<String>,
}

fn default_otio_width() -> u32 {
    1920
}

fn default_otio_height() -> u32 {
    1080
}

fn default_otio_duration() -> u32 {
    300
}

impl OtioTimeline {
    /// Convert our Composition model into an OpenTimelineIO-compatible data structure
    pub fn from_composition(comp: &Composition) -> Self {
        let mut tracks = Vec::with_capacity(comp.layers.len());
        for layer in &comp.layers {
            let media_ref = match &layer.layer_type {
                LayerType::Image { path } => Some(path.clone()),
                LayerType::Model3D { path } => Some(format!("model3d:{path}")),
                LayerType::Video {
                    source,
                    frames_dir,
                    frame_count,
                    audio_wav,
                    ..
                } => serde_json::to_string(&VideoMediaReference {
                    source: source.clone(),
                    frames_dir: frames_dir.clone(),
                    frame_count: *frame_count,
                    audio_wav: audio_wav.clone(),
                })
                .ok()
                .map(|value| format!("video-json:{value}")),
                LayerType::Solid { .. } => Some("color_solid".to_string()),
                LayerType::Text { text, .. } => Some(format!("text:{}", text)),
                LayerType::Shape { .. } => Some("vector_shape".to_string()),
                LayerType::Null => None,
                LayerType::PreComp { comp_id } => Some(format!("precomp:{}", comp_id)),
                LayerType::Audio { path, .. } => Some(format!("audio:{}", path)),
                LayerType::AdjustmentLayer => Some("adjustment_layer".to_string()),
                LayerType::Particle { .. } => Some("particle_emitter".to_string()),
            };

            let item = OtioItem {
                name: layer.name.clone(),
                source_range: [0, layer.out_frame.saturating_sub(layer.in_frame)],
                timeline_start_frame: Some(layer.in_frame),
                native_layer: serde_json::to_value(layer).ok(),
                media_reference: media_ref,
            };
            tracks.push(OtioTrack {
                name: layer.name.clone(),
                kind: if matches!(&layer.layer_type, LayerType::Audio { .. }) {
                    "audio".to_string()
                } else {
                    "video".to_string()
                },
                items: vec![item],
            });
        }

        Self {
            name: comp.name.clone(),
            global_start_time: 0,
            fps: comp.fps as f64,
            width: comp.width,
            height: comp.height,
            duration_frames: comp.duration_frames,
            tracks,
        }
    }

    /// Import and create a Composition model from an OTIO Timeline structure
    pub fn to_composition(&self) -> Result<Composition, String> {
        let mut comp = Composition::new(
            self.name.clone(),
            self.name.clone(),
            self.width.max(1),
            self.height.max(1),
            self.fps.round().max(1.0) as u32,
            self.duration_frames.max(1),
        );

        let mut layer_idx = 0;
        for track in &self.tracks {
            for item in &track.items {
                item.source_range[0]
                    .checked_add(item.source_range[1])
                    .ok_or_else(|| {
                        format!(
                            "OTIO item '{}' has a source range outside the supported frame range",
                            item.name
                        )
                    })?;
                let legacy_timeline_start = item.source_range[0];
                let timeline_start = item.timeline_start_frame.unwrap_or(legacy_timeline_start);
                if item.timeline_start_frame.is_some() && item.source_range[0] != 0 {
                    return Err(format!(
                        "OTIO item '{}' uses a trimmed source range that Kagari cannot represent",
                        item.name
                    ));
                }
                if item.native_layer.is_none()
                    && track.kind.eq_ignore_ascii_case("audio")
                    && item.source_range[0] != 0
                {
                    return Err(format!(
                        "OTIO audio item '{}' uses a trimmed source range Kagari cannot represent",
                        item.name
                    ));
                }
                let out_frame = timeline_start
                    .checked_add(item.source_range[1])
                    .ok_or_else(|| {
                        format!(
                            "OTIO item '{}' has a timeline range outside the supported frame range",
                            item.name
                        )
                    })?;
                let mut layer = if let Some(native) = &item.native_layer {
                    serde_json::from_value::<Layer>(native.clone()).map_err(|error| {
                        format!(
                            "OTIO item '{}' has invalid Kagari layer metadata: {error}",
                            item.name
                        )
                    })?
                } else {
                    let layer_type = match (&track.kind[..], &item.media_reference) {
                        ("audio", Some(ref_str)) if ref_str.starts_with("audio:") => {
                            LayerType::Audio {
                                path: ref_str["audio:".len()..].to_string(),
                                volume: crate::core::property::Animatable::new_constant(1.0),
                            }
                        }
                        (_, Some(ref_str)) if ref_str.starts_with("video-json:") => {
                            match serde_json::from_str::<VideoMediaReference>(
                                &ref_str["video-json:".len()..],
                            ) {
                                Ok(video) => LayerType::Video {
                                    source: video.source,
                                    frames_dir: video.frames_dir,
                                    frame_count: video.frame_count,
                                    audio_wav: video.audio_wav,
                                    audio_volume: crate::core::property::Animatable::new_constant(
                                        0.0,
                                    ),
                                    speed: 1.0,
                                },
                                Err(error) => {
                                    return Err(format!(
                                    "OTIO item '{}' has an invalid Kagari video reference: {error}",
                                    item.name
                                ));
                                }
                            }
                        }
                        (_, Some(ref_str)) if ref_str.starts_with("video:") => {
                            let mut parts = ref_str["video:".len()..].splitn(4, ':');
                            let source = parts.next().unwrap_or_default();
                            let frames_dir = parts.next().unwrap_or_default();
                            let frame_count_text = parts.next().unwrap_or_default();
                            let audio_wav = parts.next().filter(|value| !value.is_empty());
                            if source.is_empty() || frames_dir.is_empty() {
                                return Err(format!(
                                "OTIO item '{}' has an incomplete legacy Kagari video reference",
                                item.name
                            ));
                            }
                            let frame_count = frame_count_text.parse::<u32>().map_err(|_| {
                            format!(
                                "OTIO item '{}' has an invalid frame count in its legacy Kagari video reference",
                                item.name
                            )
                        })?;
                            if frame_count == 0 {
                                return Err(format!(
                                "OTIO item '{}' has a zero frame count in its legacy Kagari video reference",
                                item.name
                            ));
                            }
                            let source_end = item.source_range[0] + item.source_range[1];
                            if source_end > frame_count {
                                return Err(format!(
                                "OTIO item '{}' source range exceeds its legacy Kagari video frame count",
                                item.name
                            ));
                            }
                            LayerType::Video {
                                source: source.to_string(),
                                frames_dir: frames_dir.to_string(),
                                frame_count,
                                audio_wav: audio_wav.map(str::to_string),
                                audio_volume: crate::core::property::Animatable::new_constant(0.0),
                                speed: 1.0,
                            }
                        }
                        (_, Some(ref_str)) if ref_str.starts_with("model3d:") => {
                            LayerType::Model3D {
                                path: ref_str["model3d:".len()..].to_string(),
                            }
                        }
                        (_, Some(ref_str)) if ref_str.starts_with("text:") => {
                            LayerType::new_text(&ref_str["text:".len()..], 48, [1.0, 1.0, 1.0, 1.0])
                        }
                        (_, Some(ref_str)) if ref_str.starts_with("precomp:") => {
                            LayerType::PreComp {
                                comp_id: ref_str["precomp:".len()..].to_string(),
                            }
                        }
                        (_, Some(ref_str)) if ref_str == "color_solid" => LayerType::Solid {
                            color: [0.2, 0.5, 0.8, 1.0],
                        },
                        (_, Some(ref_str)) if ref_str == "vector_shape" => LayerType::Shape {
                            shape_type: crate::core::timeline::ShapeType::Rectangle {
                                width: crate::core::property::Animatable::new_constant(100.0),
                                height: crate::core::property::Animatable::new_constant(100.0),
                                corner_radius: crate::core::property::Animatable::new_constant(0.0),
                            },
                            color: [0.3, 0.6, 1.0, 1.0],
                            stroke_color: [0.0, 0.0, 0.0, 1.0],
                            stroke_width: 0.0,
                            fill_type: Default::default(),
                            extrusion_depth: 0.0,
                            bevel_depth: 0.0,
                        },
                        (_, Some(ref_str)) if ref_str == "adjustment_layer" => {
                            LayerType::AdjustmentLayer
                        }
                        (_, Some(ref_str)) if ref_str == "particle_emitter" => {
                            LayerType::Particle {
                                emitter: crate::core::particle_system::ParticleEmitter::default(),
                            }
                        }
                        (_, Some(path)) if is_video_media_path(path) => {
                            return Err(format!(
                            "OTIO item '{}' references a video file without Kagari's decoded frame cache; import the media through Kagari first",
                            item.name
                        ));
                        }
                        (_, Some(path)) => LayerType::Image { path: path.clone() },
                        (_, None) => LayerType::Null,
                    };
                    Layer::new(
                        format!("otio_layer_{}", layer_idx),
                        item.name.clone(),
                        layer_type,
                        item.source_range[1],
                    )
                };
                layer.in_frame = timeline_start;
                layer.out_frame = out_frame;
                comp.add_layer(layer);
                layer_idx += 1;
            }
        }

        Ok(comp)
    }

    /// Serialize as the OpenTimelineIO JSON schema.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        use serde_json::{json, Value};

        let rate = if self.fps.is_finite() && self.fps > 0.0 {
            self.fps
        } else {
            30.0
        };
        let rational_time =
            |value: u32| json!({"OTIO_SCHEMA": "RationalTime.1", "value": value, "rate": rate});
        let time_range = |start: u32, duration: u32| {
            json!({
                "OTIO_SCHEMA": "TimeRange.1",
                "start_time": rational_time(start),
                "duration": rational_time(duration)
            })
        };

        let mut tracks = Vec::with_capacity(self.tracks.len());
        for track in &self.tracks {
            let mut children = Vec::<Value>::new();
            let mut cursor = 0u32;
            for item in &track.items {
                let start = item.timeline_start_frame.unwrap_or(item.source_range[0]);
                let duration = item.source_range[1];
                let end = start.checked_add(duration).ok_or_else(|| {
                    <serde_json::Error as serde::de::Error>::custom(format!(
                        "OTIO item '{}' timeline range overflows",
                        item.name
                    ))
                })?;
                if start < cursor {
                    return Err(<serde_json::Error as serde::de::Error>::custom(format!(
                        "OTIO track '{}' contains overlapping clips; split them into parallel tracks",
                        track.name
                    )));
                }
                if start > cursor {
                    children.push(json!({
                        "OTIO_SCHEMA": "Gap.1",
                        "name": "",
                        "source_range": time_range(0, start - cursor),
                        "effects": [],
                        "markers": [],
                        "metadata": {}
                    }));
                }

                let kagari_reference = item.media_reference.clone();
                let (media_reference, generator) = match kagari_reference.as_deref() {
                    Some(reference) if reference.starts_with("audio:") => (
                        Some(json!({
                            "OTIO_SCHEMA": "ExternalReference.1",
                            "target_url": path_to_file_url(&reference[6..]),
                            "available_range": null,
                            "metadata": {},
                            "name": item.name
                        })),
                        None,
                    ),
                    Some(reference) if reference.starts_with("video-json:") => {
                        let video = serde_json::from_str::<VideoMediaReference>(
                            &reference["video-json:".len()..],
                        )
                        .map_err(<serde_json::Error as serde::de::Error>::custom)?;
                        (
                            Some(json!({
                                "OTIO_SCHEMA": "ExternalReference.1",
                                "target_url": path_to_file_url(&video.source),
                                "available_range": time_range(0, video.frame_count),
                                "metadata": {},
                                "name": item.name
                            })),
                            None,
                        )
                    }
                    Some(reference) if reference.starts_with("model3d:") => (
                        Some(json!({
                            "OTIO_SCHEMA": "ExternalReference.1",
                            "target_url": path_to_file_url(&reference[8..]),
                            "available_range": null,
                            "metadata": {},
                            "name": item.name
                        })),
                        None,
                    ),
                    Some(reference) if !reference.contains(':') => (
                        Some(json!({
                            "OTIO_SCHEMA": "ExternalReference.1",
                            "target_url": path_to_file_url(reference),
                            "available_range": null,
                            "metadata": {},
                            "name": item.name
                        })),
                        None,
                    ),
                    other => (
                        None,
                        Some(json!({
                            "OTIO_SCHEMA": "GeneratorReference.1",
                            "generator_kind": "KagariNativeLayer",
                            "parameters": {"media_reference": other},
                            "available_range": time_range(0, duration),
                            "metadata": {},
                            "name": item.name
                        })),
                    ),
                };
                let reference = media_reference.or(generator).ok_or_else(|| {
                    <serde_json::Error as serde::de::Error>::custom("missing OTIO media reference")
                })?;
                children.push(json!({
                    "OTIO_SCHEMA": "Clip.1",
                    "name": item.name,
                    "media_reference": reference,
                    "source_range": time_range(item.source_range[0], duration),
                    "effects": [],
                    "markers": [],
                    "metadata": {"Kagari": {
                        "media_reference": kagari_reference,
                        "native_layer": item.native_layer
                    }}
                }));
                cursor = end;
            }
            if cursor < self.duration_frames {
                children.push(json!({
                    "OTIO_SCHEMA": "Gap.1",
                    "name": "",
                    "source_range": time_range(0, self.duration_frames - cursor),
                    "effects": [],
                    "markers": [],
                    "metadata": {}
                }));
            }
            tracks.push(json!({
                "OTIO_SCHEMA": "Track.1",
                "name": track.name,
                "kind": if track.kind.eq_ignore_ascii_case("audio") { "Audio" } else { "Video" },
                "children": children,
                "source_range": null,
                "effects": [],
                "markers": [],
                "metadata": {}
            }));
        }

        let native_timeline = serde_json::to_value(self)?;
        let timeline = json!({
            "OTIO_SCHEMA": "Timeline.1",
            "name": self.name,
            "global_start_time": rational_time(self.global_start_time),
            "tracks": {
                "OTIO_SCHEMA": "Stack.1",
                "name": "tracks",
                "children": tracks,
                "source_range": null,
                "effects": [],
                "markers": [],
                "metadata": {"Kagari": {"native_timeline": native_timeline}}
            },
            "metadata": {"Kagari": {
                "width": self.width,
                "height": self.height,
                "duration_frames": self.duration_frames,
                "fps": rate
            }}
        });
        serde_json::to_string_pretty(&timeline)
    }

    /// Deserialize either Kagari's previous JSON or standard OTIO JSON.
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        use serde_json::Value;
        let value: Value = serde_json::from_str(json_str)?;
        if let Some(native) = value
            .get("tracks")
            .and_then(|tracks| tracks.get("metadata"))
            .and_then(|metadata| metadata.get("Kagari"))
            .and_then(|kagari| kagari.get("native_timeline"))
        {
            return serde_json::from_value(native.clone());
        }
        if value.get("OTIO_SCHEMA").is_none() {
            let mut legacy = serde_json::from_value::<Self>(value)?;
            if legacy.tracks.iter().any(|track| track.items.len() > 1) {
                legacy.tracks = legacy
                    .tracks
                    .into_iter()
                    .flat_map(|track| {
                        track.items.into_iter().map(move |mut item| {
                            if item.timeline_start_frame.is_none() {
                                item.timeline_start_frame = Some(item.source_range[0]);
                                item.source_range[0] = 0;
                            }
                            OtioTrack {
                                name: item.name.clone(),
                                kind: track.kind.clone(),
                                items: vec![item],
                            }
                        })
                    })
                    .collect();
            }
            return Ok(legacy);
        }
        parse_standard_otio(value)
    }
}

fn path_to_file_url(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let path = if normalized.starts_with('/') || normalized.as_bytes().get(1) == Some(&b':') {
        normalized
    } else {
        std::env::current_dir()
            .map(|directory| {
                directory
                    .join(&normalized)
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .unwrap_or(normalized)
    };
    let path = if path.as_bytes().get(1) == Some(&b':') {
        format!("/{path}")
    } else {
        path
    };
    let mut url = String::from("file://");
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~:".contains(&byte) {
            url.push(byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(url, "%{byte:02X}");
        }
    }
    url
}
