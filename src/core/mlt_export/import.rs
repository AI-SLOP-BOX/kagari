//! MLT XML document parser and clip extractor.

use super::xml_util::{timecode_to_frames, unescape_xml};
use std::collections::HashMap;

/// A layer recovered from an MLT document.
#[derive(Debug, Clone)]
pub struct MltImportedClip {
    pub name: String,
    pub resource: String,
    pub in_frame: u32,
    pub out_frame: u32,
    pub timeline_start_frame: u32,
    pub is_audio: bool,
}

/// Parses an MLT XML document by walking producer/entry elements with a
/// lightweight tag scanner (no XML dependency; MLT output is machine-generated
/// and regular). Returns clips in playlist order.
pub fn import_from_mlt(xml: &str, fps: u32) -> Result<Vec<MltImportedClip>, String> {
    if !xml.contains("<mlt") {
        return Err("not an MLT XML document".into());
    }

    // Pass 1: producers id -> (resource, name, service)
    let mut producers: HashMap<String, (String, String, String)> = HashMap::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<producer ") {
        let Some(tag_end) = rest[start..].find('>').map(|i| start + i) else {
            break;
        };
        // Self-closing producers carry no properties
        if rest[..tag_end].ends_with('/') {
            rest = &rest[tag_end..];
            continue;
        }
        let header = &rest[start..tag_end];
        let id = header
            .split("id=\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .unwrap_or("")
            .to_string();
        let Some(body_end) = rest[tag_end..].find("</producer>").map(|i| tag_end + i) else {
            break;
        };
        let body = &rest[tag_end..body_end];
        let get_prop = |key: &str| -> String {
            body.split(&format!("name=\"{}\"", key))
                .nth(1)
                .and_then(|s| s.split('>').nth(1))
                .and_then(|s| s.split('<').next())
                .unwrap_or("")
                .to_string()
        };
        producers.insert(
            id,
            (
                get_prop("resource"),
                get_prop("name"),
                get_prop("mlt_service"),
            ),
        );
        rest = &rest[body_end..];
    }

    // Pass 2: walk each playlist so blanks and entry durations preserve timeline placement.
    let mut clips = Vec::new();
    let mut rest = xml;
    while let Some(playlist_start) = rest.find("<playlist ") {
        let Some(playlist_tag_end) = rest[playlist_start..].find('>').map(|i| playlist_start + i)
        else {
            break;
        };
        let Some(playlist_end) = rest[playlist_tag_end..]
            .find("</playlist>")
            .map(|i| playlist_tag_end + i)
        else {
            break;
        };
        let mut playlist_position = 0u32;
        let mut items = &rest[playlist_tag_end + 1..playlist_end];
        loop {
            let blank_start = items.find("<blank ");
            let entry_start = items.find("<entry ");
            let (is_blank, start) = match (blank_start, entry_start) {
                (None, None) => break,
                (Some(blank), None) => (true, blank),
                (None, Some(entry)) => (false, entry),
                (Some(blank), Some(entry)) => (blank < entry, blank.min(entry)),
            };
            let Some(tag_end) = items[start..].find('>').map(|i| start + i) else {
                return Err("Malformed MLT playlist item".into());
            };
            let header = &items[start..tag_end];
            if is_blank {
                let length = header
                    .split("length=\"")
                    .nth(1)
                    .and_then(|s| s.split('"').next())
                    .and_then(|value| timecode_to_frames(value, fps))
                    .ok_or_else(|| "MLT playlist contains an invalid blank length".to_string())?;
                playlist_position = playlist_position.checked_add(length).ok_or_else(|| {
                    "MLT playlist position exceeds the supported range".to_string()
                })?;
                items = &items[tag_end + 1..];
                continue;
            }

            let producer_id = header
                .split("producer=\"")
                .nth(1)
                .and_then(|s| s.split('"').next())
                .unwrap_or("");
            let in_tc = header
                .split("in=\"")
                .nth(1)
                .and_then(|s| s.split('"').next())
                .unwrap_or("0");
            let out_tc = header
                .split("out=\"")
                .nth(1)
                .and_then(|s| s.split('"').next())
                .unwrap_or("0");
            let self_closing = items[start..=tag_end].ends_with("/>");
            let body_end = if self_closing {
                tag_end + 1
            } else {
                items[tag_end + 1..]
                    .find("</entry>")
                    .map(|i| tag_end + 1 + i)
                    .ok_or_else(|| "Unterminated MLT entry".to_string())?
            };
            let body = &items[tag_end + 1..body_end];
            let name = body
                .split("name=\"name\"")
                .nth(1)
                .and_then(|s| s.split('>').nth(1))
                .and_then(|s| s.split('<').next())
                .unwrap_or("Imported Clip");
            items = &items[body_end..];

            let in_frames = timecode_to_frames(in_tc, fps).ok_or_else(|| {
                format!(
                    "MLT entry for producer '{producer_id}' has an invalid or out-of-range in timecode"
                )
            })?;
            let out_frames = timecode_to_frames(out_tc, fps).ok_or_else(|| {
                format!(
                    "MLT entry for producer '{producer_id}' has an invalid or out-of-range out timecode"
                )
            })?;
            let duration_frames = out_frames
                .checked_sub(in_frames)
                .and_then(|duration| duration.checked_add(1))
                .ok_or_else(|| {
                    format!("MLT entry for producer '{producer_id}' has a reversed frame range")
                })?;

            if producer_id != "black" && producer_id != "background" {
                if let Some((resource, prod_name, service)) = producers.get(producer_id) {
                    let name = if name.is_empty() || name == "Imported Clip" {
                        if prod_name.is_empty() {
                            producer_id.to_string()
                        } else {
                            prod_name.clone()
                        }
                    } else {
                        name.to_string()
                    };
                    clips.push(MltImportedClip {
                        name: unescape_xml(&name),
                        resource: unescape_xml(resource),
                        in_frame: in_frames,
                        out_frame: out_frames.checked_add(1).ok_or_else(|| {
                            format!("MLT entry for producer '{producer_id}' exceeds the supported frame range")
                        })?,
                        timeline_start_frame: playlist_position,
                        is_audio: service.contains("novalidate") && resource.ends_with(".wav"),
                    });
                }
            }
            playlist_position = playlist_position
                .checked_add(duration_frames)
                .ok_or_else(|| "MLT playlist position exceeds the supported range".to_string())?;
        }
        rest = &rest[playlist_end + "</playlist>".len()..];
    }

    if clips.is_empty() {
        return Err(format!(
            "MLT document contains no media entries (producers: {})",
            producers.len()
        ));
    }
    Ok(clips)
}
