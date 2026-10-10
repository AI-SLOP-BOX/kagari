#![allow(dead_code)]
use crate::core::timeline::{Composition, Layer, LayerType};
use serde::{Deserialize, Serialize};

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

fn parse_standard_otio(value: serde_json::Value) -> Result<OtioTimeline, serde_json::Error> {
    use serde_json::Value;
    let invalid = |message: String| <serde_json::Error as serde::de::Error>::custom(message);
    let timeline_schema = value
        .get("OTIO_SCHEMA")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !timeline_schema.starts_with("Timeline.") {
        return Err(invalid(format!(
            "OTIO document root must be a Timeline schema, found '{timeline_schema}'"
        )));
    }
    let fps = value
        .get("metadata")
        .and_then(|metadata| metadata.get("Kagari"))
        .and_then(|kagari| kagari.get("fps"))
        .and_then(Value::as_f64)
        .or_else(|| {
            value
                .get("global_start_time")
                .and_then(|time| time.get("rate"))
                .and_then(Value::as_f64)
        })
        .unwrap_or(30.0);
    if !fps.is_finite() || fps <= 0.0 {
        return Err(invalid(
            "OTIO frame rate must be finite and positive".into(),
        ));
    }
    let rate = |time: &Value| -> Result<u32, serde_json::Error> {
        let time_rate = time.get("rate").and_then(Value::as_f64).unwrap_or(fps);
        let frames = time.get("value").and_then(Value::as_f64).unwrap_or(0.0) * fps / time_rate;
        if !frames.is_finite() || frames < 0.0 || frames > u32::MAX as f64 {
            return Err(invalid(
                "OTIO time is outside Kagari's supported frame range".into(),
            ));
        }
        Ok(frames.round() as u32)
    };
    let tracks_value = value
        .get("tracks")
        .filter(|stack| {
            stack
                .get("OTIO_SCHEMA")
                .and_then(Value::as_str)
                .is_some_and(|schema| schema.starts_with("Stack."))
        })
        .and_then(|tracks| tracks.get("children"))
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("OTIO Timeline has no tracks Stack".into()))?;
    let mut tracks = Vec::with_capacity(tracks_value.len());
    let mut duration_frames = 0u32;
    for track in tracks_value {
        let track_schema = track
            .get("OTIO_SCHEMA")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !track_schema.starts_with("Track.") {
            let track_name = track.get("name").and_then(Value::as_str).unwrap_or("Track");
            return Err(invalid(format!(
                "OTIO stack child '{track_name}' uses unsupported schema '{track_schema}'"
            )));
        }
        let kind = track
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("Video")
            .to_ascii_lowercase();
        let name = track
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("Track")
            .to_string();
        let children = track
            .get("children")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid(format!("OTIO track '{name}' has no children array")))?;
        let mut items = Vec::new();
        let mut cursor = 0u32;
        for child in children {
            let schema = child
                .get("OTIO_SCHEMA")
                .and_then(Value::as_str)
                .unwrap_or("");
            if schema.starts_with("Clip.") && !matches!(schema, "Clip.1" | "Clip.2") {
                return Err(invalid(format!(
                    "OTIO track '{name}' uses unsupported clip schema '{schema}'"
                )));
            }
            let source_range = child.get("source_range").unwrap_or(&Value::Null);
            if schema.starts_with("Gap.") {
                let item_duration = source_range
                    .get("duration")
                    .map(&rate)
                    .transpose()?
                    .ok_or_else(|| {
                        invalid(format!("OTIO gap on track '{name}' has no duration"))
                    })?;
                cursor = cursor.checked_add(item_duration).ok_or_else(|| {
                    invalid(format!("OTIO track '{name}' duration overflows u32 frames"))
                })?;
                continue;
            }
            if !matches!(schema, "Clip.1" | "Clip.2") {
                return Err(invalid(format!(
                    "OTIO track '{name}' contains unsupported item schema '{schema}'"
                )));
            }
            let clip_name = child
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("Clip")
                .to_string();
            let native_reference = child
                .get("metadata")
                .and_then(|metadata| metadata.get("Kagari"))
                .and_then(|kagari| kagari.get("media_reference"))
                .and_then(Value::as_str)
                .map(str::to_string);
            let native_layer = child
                .get("metadata")
                .and_then(|metadata| metadata.get("Kagari"))
                .and_then(|kagari| kagari.get("native_layer"))
                .cloned();
            let active_media_key = child
                .get("active_media_reference_key")
                .and_then(Value::as_str)
                .unwrap_or("DEFAULT_MEDIA");
            let media = if let Some(media) = child.get("media_reference") {
                media
            } else if let Some(media_references) =
                child.get("media_references").and_then(Value::as_object)
            {
                media_references.get(active_media_key).ok_or_else(|| {
                    invalid(format!(
                        "OTIO clip '{clip_name}' has no active media reference '{active_media_key}'"
                    ))
                })?
            } else {
                &Value::Null
            };
            let source_range = if !source_range.is_null() {
                source_range
            } else {
                media.get("available_range").unwrap_or(&Value::Null)
            };
            let source_start = source_range
                .get("start_time")
                .map(&rate)
                .transpose()?
                .ok_or_else(|| {
                    invalid(format!("OTIO clip '{clip_name}' has no source start time"))
                })?;
            let item_duration = source_range
                .get("duration")
                .map(&rate)
                .transpose()?
                .ok_or_else(|| {
                    invalid(format!("OTIO clip '{clip_name}' has no source duration"))
                })?;
            let reference = if let Some(reference) = native_reference {
                Some(reference)
            } else {
                let schema = media
                    .get("OTIO_SCHEMA")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if schema.starts_with("ExternalReference.") {
                    let target =
                        media
                            .get("target_url")
                            .and_then(Value::as_str)
                            .ok_or_else(|| {
                                invalid(format!("OTIO clip '{clip_name}' has no media target URL"))
                            })?;
                    let path = resolve_otio_media_target(target).ok_or_else(|| {
                        invalid(format!(
                            "OTIO clip '{clip_name}' uses an unsupported media URL"
                        ))
                    })?;
                    Some(if kind == "audio" {
                        format!("audio:{path}")
                    } else {
                        path
                    })
                } else {
                    return Err(invalid(format!(
                        "OTIO clip '{clip_name}' has a media reference Kagari cannot resolve"
                    )));
                }
            };
            items.push(OtioItem {
                name: clip_name,
                source_range: [source_start, item_duration],
                timeline_start_frame: Some(cursor),
                native_layer,
                media_reference: reference,
            });
            cursor = cursor.checked_add(item_duration).ok_or_else(|| {
                invalid(format!("OTIO track '{name}' duration overflows u32 frames"))
            })?;
        }
        duration_frames = duration_frames.max(cursor);
        tracks.push(OtioTrack { name, kind, items });
    }
    let metadata = value
        .get("metadata")
        .and_then(|metadata| metadata.get("Kagari"));
    Ok(OtioTimeline {
        name: value
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("Imported Timeline")
            .into(),
        global_start_time: value
            .get("global_start_time")
            .map(&rate)
            .transpose()?
            .unwrap_or(0),
        fps,
        width: metadata
            .and_then(|metadata| metadata.get("width"))
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or_else(default_otio_width),
        height: metadata
            .and_then(|metadata| metadata.get("height"))
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or_else(default_otio_height),
        duration_frames: metadata
            .and_then(|metadata| metadata.get("duration_frames"))
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or(duration_frames.max(1)),
        tracks,
    })
}

fn file_url_to_path(url: &str) -> Option<String> {
    let path = url.strip_prefix("file://")?;
    if !path.starts_with('/') {
        return None;
    }
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = *bytes.get(index + 1)?;
            let low = *bytes.get(index + 2)?;
            decoded.push((hex_value(high)? << 4) | hex_value(low)?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    let path = String::from_utf8(decoded).ok()?;
    #[cfg(windows)]
    let path = path.strip_prefix('/').unwrap_or(&path).to_string();
    Some(path)
}

fn resolve_otio_media_target(target: &str) -> Option<String> {
    if target.starts_with("file://") {
        file_url_to_path(target)
    } else if target.contains("://") {
        None
    } else {
        Some(target.to_string())
    }
}

fn is_video_media_path(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "mov" | "mp4" | "m4v" | "mkv" | "webm" | "avi" | "mpeg" | "mpg" | "mxf"
            )
        })
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

// --- 2. Dynamic Link Server (Real-time synchronization protocol) ---

pub const DYNAMIC_LINK_FRAME_QUEUE_CAPACITY: usize = 1;
pub const DYNAMIC_LINK_CONNECTION_QUEUE_CAPACITY: usize = 8;

#[derive(Clone)]
pub struct LatestFrameSender {
    latest: std::sync::Arc<std::sync::atomic::AtomicU32>,
    wake: std::sync::mpsc::SyncSender<()>,
}

pub struct LatestFrameReceiver {
    latest: std::sync::Arc<std::sync::atomic::AtomicU32>,
    wake: std::sync::mpsc::Receiver<()>,
}

pub fn latest_frame_channel() -> (LatestFrameSender, LatestFrameReceiver) {
    let latest = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let (wake, receiver) = std::sync::mpsc::sync_channel(DYNAMIC_LINK_FRAME_QUEUE_CAPACITY);
    (
        LatestFrameSender {
            latest: latest.clone(),
            wake,
        },
        LatestFrameReceiver {
            latest,
            wake: receiver,
        },
    )
}

impl LatestFrameSender {
    pub fn publish(&self, frame: u32) {
        self.latest
            .store(frame, std::sync::atomic::Ordering::Release);
        let _ = self.wake.try_send(());
    }
}

impl LatestFrameReceiver {
    pub fn try_recv(&self) -> Result<u32, std::sync::mpsc::TryRecvError> {
        self.wake.try_recv()?;
        Ok(self.latest.load(std::sync::atomic::Ordering::Acquire))
    }

    pub fn recv_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<u32, std::sync::mpsc::RecvTimeoutError> {
        self.wake.recv_timeout(timeout)?;
        Ok(self.latest.load(std::sync::atomic::Ordering::Acquire))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DynamicLinkMessage {
    /// Synchronize playhead position
    SyncPlayhead {
        frame: u32,
    },
    /// Notify or request asset addition
    AddAsset {
        name: String,
        path: String,
    },
    /// Trigger frame render request from external NLE
    TriggerRender {
        comp_id: String,
        frame: u32,
    },
    /// Connection handshake / status
    Handshake {
        client_name: String,
        protocol_version: String,
    },
    Ping,
    Pong,
    /// Confirmation of asset addition
    AssetAdded {
        name: String,
        path: String,
    },
    /// Acknowledgment or result of render trigger
    RenderTriggered {
        comp_id: String,
        frame: u32,
    },
    Error {
        message: String,
    },
}

pub struct DynamicLinkServer {
    pub is_connected: bool,
    pub connected_app: Option<String>, // e.g. "Kdenlive" or "Shotcut"
    pub sync_port: u16,
}

impl Default for DynamicLinkServer {
    fn default() -> Self {
        Self {
            is_connected: false,
            connected_app: None,
            sync_port: 8080,
        }
    }
}

fn dynamic_link_version_compatible(version: &str) -> bool {
    let mut parts = version.split('.');
    let Some(major) = parts.next().and_then(|part| part.parse::<u32>().ok()) else {
        return false;
    };
    parts
        .next()
        .and_then(|part| part.parse::<u32>().ok())
        .is_some()
        && parts
            .next()
            .and_then(|part| part.parse::<u32>().ok())
            .is_some()
        && parts.next().is_none()
        && major == 1
}

impl DynamicLinkServer {
    pub fn new(port: u16) -> Self {
        Self {
            is_connected: false,
            connected_app: None,
            sync_port: port,
        }
    }

    /// Process an incoming message from the WebSocket link and return a response message if necessary
    pub fn handle_message(&mut self, raw_msg: &str) -> Option<DynamicLinkMessage> {
        let msg: Result<DynamicLinkMessage, serde_json::Error> = serde_json::from_str(raw_msg);

        match msg {
            Ok(DynamicLinkMessage::Handshake {
                client_name,
                protocol_version,
            }) => {
                if !dynamic_link_version_compatible(&protocol_version) {
                    return Some(DynamicLinkMessage::Error {
                        message: format!(
                            "Incompatible Dynamic Link protocol version '{protocol_version}'; supported major version is 1"
                        ),
                    });
                }
                self.is_connected = true;
                self.connected_app = Some(client_name.clone());
                Some(DynamicLinkMessage::Handshake {
                    client_name: "Kagari VFX".to_string(),
                    protocol_version: "1.0.0".to_string(),
                })
            }
            Ok(DynamicLinkMessage::Ping) => Some(DynamicLinkMessage::Pong),
            Ok(DynamicLinkMessage::SyncPlayhead { .. }) => {
                // Return None, application layer will handle playhead update
                None
            }
            Ok(DynamicLinkMessage::AddAsset { name, path }) => {
                if std::path::Path::new(&path).exists() {
                    Some(DynamicLinkMessage::AssetAdded { name, path })
                } else {
                    Some(DynamicLinkMessage::Error {
                        message: format!("Asset file does not exist: {path}"),
                    })
                }
            }
            Ok(DynamicLinkMessage::TriggerRender { comp_id, frame }) => {
                Some(DynamicLinkMessage::RenderTriggered { comp_id, frame })
            }
            Err(error) => Some(DynamicLinkMessage::Error {
                message: format!("Invalid Dynamic Link message: {error}"),
            }),
            Ok(
                DynamicLinkMessage::Pong
                | DynamicLinkMessage::AssetAdded { .. }
                | DynamicLinkMessage::RenderTriggered { .. }
                | DynamicLinkMessage::Error { .. },
            ) => None,
        }
    }
}

/// Spawns a background thread running a TCP synchronization server.
pub fn start_sync_server(
    port: u16,
    frame_tx: LatestFrameSender,
    connection_tx: std::sync::mpsc::SyncSender<Option<String>>,
) -> Result<u16, std::io::Error> {
    use std::io::Write;
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
    let bound_port = listener.local_addr()?.port();
    log::info!("Dynamic Link Sync Server listening on port {}", bound_port);
    let client_active = Arc::new(AtomicBool::new(false));

    thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            if client_active
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                let response = DynamicLinkMessage::Error {
                    message: "Only one Dynamic Link client can be connected at a time".into(),
                };
                if let Ok(json) = serde_json::to_vec(&response) {
                    let mut stream = stream;
                    let _ = stream.write_all(&json);
                    let _ = stream.write_all(b"\n");
                    let _ = stream.flush();
                }
                continue;
            }
            let frame_tx = frame_tx.clone();
            let connection_tx = connection_tx.clone();
            let client_active_for_worker = client_active.clone();
            if let Err(error) = thread::Builder::new()
                .name("kagari-dynamic-link-client".into())
                .spawn(move || {
                    handle_sync_client(stream, frame_tx, connection_tx);
                    client_active_for_worker.store(false, Ordering::Release);
                })
            {
                client_active.store(false, Ordering::Release);
                log::error!("Could not start Dynamic Link client worker: {error}");
            }
        }
    });

    Ok(bound_port)
}

fn handle_sync_client(
    stream: std::net::TcpStream,
    frame_tx: LatestFrameSender,
    connection_tx: std::sync::mpsc::SyncSender<Option<String>>,
) {
    use std::io::{BufRead, BufReader, Read};
    const MAX_MESSAGE_BYTES: usize = 65_536;

    let stream_clone = match stream.try_clone() {
        Ok(stream) => stream,
        Err(error) => {
            log::warn!("Could not monitor Dynamic Link connection: {error}");
            return;
        }
    };
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
    let mut reader = BufReader::new(stream_clone);
    let mut writer = stream;
    let mut active_client: Option<String> = None;
    let mut line = Vec::with_capacity(MAX_MESSAGE_BYTES.min(4096));
    loop {
        line.clear();
        let read_result = reader
            .by_ref()
            .take((MAX_MESSAGE_BYTES + 1) as u64)
            .read_until(b'\n', &mut line);
        match read_result {
            Ok(0) => break,
            Ok(_) => {
                if line.len() > MAX_MESSAGE_BYTES {
                    log::warn!("Dynamic Link message exceeded the line limit; closing client");
                    break;
                }
                let trimmed = match std::str::from_utf8(&line) {
                    Ok(value) => value.trim(),
                    Err(_) => {
                        log::warn!("Dynamic Link message was not valid UTF-8");
                        continue;
                    }
                };
                if trimmed.is_empty() {
                    continue;
                }
                let Ok(message) = serde_json::from_str::<DynamicLinkMessage>(trimmed) else {
                    log::warn!("Ignoring malformed Dynamic Link message");
                    continue;
                };
                match message {
                    DynamicLinkMessage::Handshake {
                        client_name,
                        protocol_version,
                    } => {
                        if !dynamic_link_version_compatible(&protocol_version) {
                            write_dynamic_link_message(
                                &mut writer,
                                &DynamicLinkMessage::Error {
                                    message: format!(
                                        "Incompatible Dynamic Link protocol version '{protocol_version}'; supported major version is 1"
                                    ),
                                },
                            );
                            break;
                        }
                        if active_client.as_deref() != Some(client_name.as_str()) {
                            active_client = Some(client_name.clone());
                            if connection_tx.try_send(Some(client_name)).is_err() {
                                log::warn!("Dynamic Link connection notification queue is full");
                            }
                        }
                        write_dynamic_link_message(
                            &mut writer,
                            &DynamicLinkMessage::Handshake {
                                client_name: "Kagari VFX".into(),
                                protocol_version: "1.0.0".into(),
                            },
                        );
                    }
                    DynamicLinkMessage::Ping => {
                        write_dynamic_link_message(&mut writer, &DynamicLinkMessage::Pong);
                    }
                    DynamicLinkMessage::SyncPlayhead { frame } => frame_tx.publish(frame),
                    DynamicLinkMessage::AddAsset { name, path } => {
                        if std::path::Path::new(&path).exists() {
                            write_dynamic_link_message(
                                &mut writer,
                                &DynamicLinkMessage::AssetAdded { name, path },
                            );
                        } else {
                            write_dynamic_link_message(
                                &mut writer,
                                &DynamicLinkMessage::Error {
                                    message: format!("Asset file does not exist: {path}"),
                                },
                            );
                        }
                    }
                    DynamicLinkMessage::TriggerRender { comp_id, frame } => {
                        write_dynamic_link_message(
                            &mut writer,
                            &DynamicLinkMessage::RenderTriggered { comp_id, frame },
                        );
                    }
                    DynamicLinkMessage::Pong
                    | DynamicLinkMessage::AssetAdded { .. }
                    | DynamicLinkMessage::RenderTriggered { .. }
                    | DynamicLinkMessage::Error { .. } => {}
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                break;
            }
            Err(_) => break,
        }
    }
    if active_client.is_some() && connection_tx.try_send(None).is_err() {
        log::warn!("Dynamic Link disconnect notification queue is full");
    }
}

fn write_dynamic_link_message(writer: &mut std::net::TcpStream, message: &DynamicLinkMessage) {
    use std::io::Write;
    if let Ok(json) = serde_json::to_vec(message) {
        let _ = writer.write_all(&json);
        let _ = writer.write_all(b"\n");
        let _ = writer.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otio_serialization() {
        let mut comp = Composition::new(
            "test_comp".to_string(),
            "test_comp".to_string(),
            1920,
            1080,
            30,
            120,
        );
        let layer = Layer::new(
            "layer_1".to_string(),
            "Solid Layer".to_string(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            120,
        );
        comp.add_layer(layer);

        let otio = OtioTimeline::from_composition(&comp);
        assert_eq!(otio.name, "test_comp");
        assert_eq!(otio.tracks[0].items[0].name, "Solid Layer");

        let json = otio.to_json().unwrap();
        let parsed_otio = OtioTimeline::from_json(&json).unwrap();
        assert_eq!(parsed_otio.name, "test_comp");

        let parsed_comp = parsed_otio.to_composition().unwrap();
        assert_eq!(parsed_comp.layers.len(), 1);
        assert_eq!(parsed_comp.layers[0].name, "Solid Layer");
    }

    #[test]
    fn otio_clip_v2_resolves_only_its_active_media_reference() {
        let mut document = serde_json::json!({
            "OTIO_SCHEMA": "Timeline.1",
            "name": "Clip v2",
            "global_start_time": {
                "OTIO_SCHEMA": "RationalTime.1",
                "value": 0,
                "rate": 24
            },
            "tracks": {
                "OTIO_SCHEMA": "Stack.1",
                "children": [{
                    "OTIO_SCHEMA": "Track.1",
                    "name": "V1",
                    "kind": "Video",
                    "children": [{
                        "OTIO_SCHEMA": "Clip.2",
                        "name": "Shot A",
                        "source_range": {
                            "OTIO_SCHEMA": "TimeRange.1",
                            "start_time": {"OTIO_SCHEMA": "RationalTime.1", "value": 0, "rate": 24},
                            "duration": {"OTIO_SCHEMA": "RationalTime.1", "value": 24, "rate": 24}
                        },
                        "active_media_reference_key": "proxy",
                        "media_references": {
                            "DEFAULT_MEDIA": {"OTIO_SCHEMA": "MissingReference.1"},
                            "proxy": {
                                "OTIO_SCHEMA": "ExternalReference.1",
                                "target_url": "file:///tmp/shot-a.png",
                                "available_range": {
                                    "OTIO_SCHEMA": "TimeRange.1",
                                    "start_time": {"OTIO_SCHEMA": "RationalTime.1", "value": 0, "rate": 24},
                                    "duration": {"OTIO_SCHEMA": "RationalTime.1", "value": 48, "rate": 24}
                                }
                            }
                        }
                    }]
                }]
            }
        });
        document["tracks"]["children"][0]["children"][0]["source_range"] = serde_json::Value::Null;

        let timeline = OtioTimeline::from_json(&document.to_string()).unwrap();
        assert_eq!(
            timeline.tracks[0].items[0].media_reference.as_deref(),
            Some("/tmp/shot-a.png")
        );
        assert_eq!(timeline.tracks[0].items[0].source_range, [0, 48]);
        let composition = timeline.to_composition().unwrap();
        assert!(matches!(
            &composition.layers[0].layer_type,
            LayerType::Image { path } if path == "/tmp/shot-a.png"
        ));

        document["tracks"]["children"][0]["children"][0]["active_media_reference_key"] =
            serde_json::json!("missing");
        let error = OtioTimeline::from_json(&document.to_string()).unwrap_err();
        assert!(error
            .to_string()
            .contains("active media reference 'missing'"));
    }

    #[test]
    fn otio_clip_without_any_time_range_is_rejected() {
        let mut document = serde_json::json!({
            "OTIO_SCHEMA": "Timeline.1",
            "name": "Missing range",
            "tracks": {
                "OTIO_SCHEMA": "Stack.1",
                "children": [{
                    "OTIO_SCHEMA": "Track.1",
                    "name": "V1",
                    "kind": "Video",
                    "children": [{
                        "OTIO_SCHEMA": "Clip.2",
                        "name": "Shot without range",
                        "active_media_reference_key": "DEFAULT_MEDIA",
                        "media_references": {
                            "DEFAULT_MEDIA": {
                                "OTIO_SCHEMA": "ExternalReference.1",
                                "target_url": "file:///tmp/shot.png"
                            }
                        }
                    }]
                }]
            }
        });

        let error = OtioTimeline::from_json(&document.to_string()).unwrap_err();
        assert!(error
            .to_string()
            .contains("OTIO clip 'Shot without range' has no source start time"));

        document["tracks"]["children"][0]["children"][0]["OTIO_SCHEMA"] =
            serde_json::json!("Gap.1");
        let error = OtioTimeline::from_json(&document.to_string()).unwrap_err();
        assert!(error
            .to_string()
            .contains("OTIO gap on track 'V1' has no duration"));
    }

    #[test]
    fn test_dynamic_link_handshake() {
        let mut server = DynamicLinkServer::new(9000);
        assert!(!server.is_connected);

        let handshake_req =
            r#"{"type":"handshake","client_name":"Kdenlive","protocol_version":"1.0.0"}"#;
        let resp = server.handle_message(handshake_req);

        assert!(server.is_connected);
        assert_eq!(server.connected_app.unwrap(), "Kdenlive");

        if let Some(DynamicLinkMessage::Handshake { client_name, .. }) = resp {
            assert_eq!(client_name, "Kagari VFX");
        } else {
            panic!("Expected handshake response");
        }
    }

    #[test]
    fn otio_rejects_overflowing_source_range() {
        let timeline = OtioTimeline {
            name: "overflow".into(),
            global_start_time: 0,
            fps: 30.0,
            width: 1920,
            height: 1080,
            duration_frames: 300,
            tracks: vec![OtioTrack {
                name: "V1".into(),
                kind: "video".into(),
                items: vec![OtioItem {
                    name: "bad range".into(),
                    source_range: [u32::MAX, 1],
                    timeline_start_frame: None,
                    native_layer: None,
                    media_reference: None,
                }],
            }],
        };

        let error = timeline.to_composition().unwrap_err();
        assert!(error.contains("bad range"));
        assert!(error.contains("source range"));
    }

    #[test]
    fn otio_rejects_corrupt_kagari_video_reference() {
        let timeline = OtioTimeline {
            name: "corrupt media".into(),
            global_start_time: 0,
            fps: 30.0,
            width: 1920,
            height: 1080,
            duration_frames: 300,
            tracks: vec![OtioTrack {
                name: "V1".into(),
                kind: "video".into(),
                items: vec![OtioItem {
                    name: "broken clip".into(),
                    source_range: [0, 30],
                    timeline_start_frame: None,
                    native_layer: None,
                    media_reference: Some("video-json:{broken".into()),
                }],
            }],
        };

        let error = timeline.to_composition().unwrap_err();
        assert!(error.contains("broken clip"));
        assert!(error.contains("invalid Kagari video reference"));
    }

    #[test]
    fn otio_roundtrip_preserves_audio_and_video_layers() {
        let mut comp = Composition::new(
            "media_comp".to_string(),
            "Media Comp".to_string(),
            1280,
            720,
            24,
            240,
        );
        comp.add_layer(Layer::new(
            "video_1".to_string(),
            "Footage".to_string(),
            LayerType::Video {
                source: "/media/scene.mov".to_string(),
                frames_dir: "/cache/scene".to_string(),
                frame_count: 240,
                audio_wav: Some("/cache/scene.wav".to_string()),
                speed: 1.0,
            },
            240,
        ));
        comp.add_layer(Layer::new(
            "audio_1".to_string(),
            "Music".to_string(),
            LayerType::Audio {
                path: "/audio/music.wav".to_string(),
                volume: crate::core::property::Animatable::new_constant(1.0),
            },
            240,
        ));

        let otio = OtioTimeline::from_composition(&comp);
        assert_eq!(otio.tracks.len(), 2);
        assert_eq!(otio.tracks[0].kind, "video");
        assert_eq!(otio.tracks[1].kind, "audio");

        let round_tripped = otio.to_composition().unwrap();
        assert_eq!(round_tripped.width, 1280);
        assert_eq!(round_tripped.height, 720);
        assert_eq!(round_tripped.duration_frames, 240);
        assert!(matches!(
            round_tripped.layers[0].layer_type,
            LayerType::Video { .. }
        ));
        assert!(matches!(
            round_tripped.layers[1].layer_type,
            LayerType::Audio { .. }
        ));
        if let LayerType::Video {
            source,
            frames_dir,
            frame_count,
            audio_wav,
            ..
        } = &round_tripped.layers[0].layer_type
        {
            assert_eq!(source, "/media/scene.mov");
            assert_eq!(frames_dir, "/cache/scene");
            assert_eq!(*frame_count, 240);
            assert_eq!(audio_wav.as_deref(), Some("/cache/scene.wav"));
        } else {
            panic!("video layer did not survive OTIO roundtrip");
        }
        if let LayerType::Audio { path, .. } = &round_tripped.layers[1].layer_type {
            assert_eq!(path, "/audio/music.wav");
        } else {
            panic!("audio layer did not survive OTIO roundtrip");
        }
    }

    #[test]
    fn otio_roundtrip_preserves_model3d_layers() {
        let mut comp = Composition::new(
            "model_comp".to_string(),
            "Model Comp".to_string(),
            1920,
            1080,
            30,
            120,
        );
        comp.add_layer(Layer::new(
            "model_1".to_string(),
            "Hero OBJ".to_string(),
            LayerType::Model3D {
                path: "/assets/hero.obj".to_string(),
            },
            120,
        ));

        let otio = OtioTimeline::from_composition(&comp);
        assert_eq!(
            otio.tracks[0].items[0].media_reference.as_deref(),
            Some("model3d:/assets/hero.obj")
        );

        let json = otio.to_json().unwrap();
        let parsed = OtioTimeline::from_json(&json).unwrap();
        let round_tripped = parsed.to_composition().unwrap();
        assert!(matches!(
            &round_tripped.layers[0].layer_type,
            LayerType::Model3D { path } if path == "/assets/hero.obj"
        ));
    }

    #[test]
    fn test_real_tcp_server_sync() {
        use std::io::{BufRead, BufReader, Write};
        use std::net::TcpStream;
        let (frame_tx, frame_rx) = latest_frame_channel();
        let (conn_tx, conn_rx) =
            std::sync::mpsc::sync_channel(DYNAMIC_LINK_CONNECTION_QUEUE_CAPACITY);

        // Start server on dynamic OS port 0
        let port = match start_sync_server(0, frame_tx, conn_tx) {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return,
            Err(e) => panic!("start_sync_server failed: {e:?}"),
        };

        // Connect client
        let mut stream = match TcpStream::connect(format!("127.0.0.1:{}", port)) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return,
            Err(e) => panic!("Failed to connect to test TCP server: {e:?}"),
        };
        let mut reader = BufReader::new(stream.try_clone().unwrap());

        // Send Handshake Request
        let handshake_req =
            r#"{"type":"handshake","client_name":"Kdenlive","protocol_version":"1.0.0"}"#;
        stream.write_all(handshake_req.as_bytes()).unwrap();
        stream.write_all(b"\n").unwrap();
        stream.flush().unwrap();

        // Read Handshake Response
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert!(line.contains("Kagari VFX"));

        // Verify connection state channel
        let conn_state = conn_rx.recv().unwrap();
        assert_eq!(conn_state, Some("Kdenlive".to_string()));

        // Repeated handshakes from one connected client must not queue redundant UI events.
        stream.write_all(handshake_req.as_bytes()).unwrap();
        stream.write_all(b"\n").unwrap();
        stream.flush().unwrap();
        line.clear();
        reader.read_line(&mut line).unwrap();
        assert!(line.contains("Kagari VFX"));
        assert!(matches!(
            conn_rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));

        // Send Playhead Sync
        let sync_msg = r#"{"type":"sync_playhead","frame":123}"#;
        stream.write_all(sync_msg.as_bytes()).unwrap();
        stream.write_all(b"\n").unwrap();
        stream.flush().unwrap();

        // Verify frame sync channel
        let frame = frame_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        assert_eq!(frame, 123);

        drop(reader);
        drop(stream);
        assert_eq!(
            conn_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap(),
            None
        );
    }

    #[test]
    fn latest_frame_mailbox_coalesces_updates_without_stale_queue_growth() {
        let (tx, rx) = latest_frame_channel();
        for frame in 0..10_000 {
            tx.publish(frame);
        }
        assert_eq!(rx.try_recv(), Ok(9_999));
        assert!(matches!(
            rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
    }
}
