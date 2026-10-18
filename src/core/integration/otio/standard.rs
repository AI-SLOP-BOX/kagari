use super::*;

pub(super) fn parse_standard_otio(
    value: serde_json::Value,
) -> Result<OtioTimeline, serde_json::Error> {
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

pub(super) fn is_video_media_path(path: &str) -> bool {
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
