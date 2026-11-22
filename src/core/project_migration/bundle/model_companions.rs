use super::{model_line_text, read_model_line};
use std::collections::HashMap;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

pub fn model_companion_references(path: &Path) -> Result<Vec<String>, String> {
    let extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension == "gltf" || extension == "glb" {
        let bytes = crate::core::gltf_loader::read_gltf_file(path)?;
        let json = if extension == "glb" {
            crate::core::gltf_loader::glb_json_chunk(&bytes)?
        } else {
            &bytes
        };
        let root: serde_json::Value = serde_json::from_slice(json).map_err(|error| {
            format!(
                "Cannot parse glTF dependencies in '{}': {error}",
                path.display()
            )
        })?;
        let buffers = root
            .get("buffers")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut references = Vec::new();
        for buffer in &buffers {
            let Some(uri) = buffer.get("uri").and_then(serde_json::Value::as_str) else {
                if extension == "glb" {
                    continue;
                }
                return Err(format!(
                    "glTF '{}' has a buffer without a URI",
                    path.display()
                ));
            };
            if uri.starts_with("data:") {
                continue;
            }
            let resolved = crate::core::gltf_loader::resolve_gltf_buffer_path(path, uri)?;
            references.push(resolved.to_string_lossy().into_owned());
        }
        if let Some(images) = root.get("images").and_then(serde_json::Value::as_array) {
            for image in images {
                if let Some(uri) = image
                    .get("uri")
                    .and_then(serde_json::Value::as_str)
                    .filter(|uri| !uri.starts_with("data:"))
                {
                    references.push(
                        crate::core::gltf_loader::resolve_gltf_buffer_path(path, uri)?
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
            }
        }
        return Ok(references);
    }
    if extension != "obj" && extension != "mtl" {
        return Ok(Vec::new());
    }
    let file = std::fs::File::open(path).map_err(|error| {
        format!(
            "Cannot inspect model companion file '{}': {error}",
            path.display()
        )
    })?;
    let mut reader = BufReader::new(file);
    let mut line_bytes = Vec::new();
    let mut references = Vec::new();
    loop {
        if read_model_line(&mut reader, &mut line_bytes)? == 0 {
            break;
        }
        let line = model_line_text(&line_bytes)?;
        let content = line.split('#').next().unwrap_or_default().trim();
        let mut parts = content.split_whitespace();
        let Some(tag) = parts.next() else { continue };
        if extension == "obj" && tag == "mtllib" {
            let rest = content[tag.len()..].trim();
            if let Some(quoted) = rest
                .strip_prefix('"')
                .and_then(|value| value.split_once('"'))
            {
                references.push(quoted.0.to_string());
            } else {
                references.extend(rest.split_whitespace().map(str::to_string));
            }
        } else if extension == "mtl"
            && (tag.to_ascii_lowercase().starts_with("map_")
                || matches!(
                    tag.to_ascii_lowercase().as_str(),
                    "bump" | "norm" | "disp" | "decal" | "refl"
                ))
        {
            let rest = content[tag.len()..].trim();
            if let Some(start) = rest.find('"') {
                if let Some(end) = rest[start + 1..].find('"') {
                    references.push(rest[start + 1..start + 1 + end].to_string());
                }
            } else if let Some(filename) = rest.split_whitespace().last() {
                references.push(filename.to_string());
            }
        }
    }
    Ok(references)
}

pub fn rewrite_model_companions(
    source: &Path,
    destination: &Path,
    canonical_names: &HashMap<PathBuf, String>,
) -> Result<(), String> {
    let extension = source
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension == "gltf" || extension == "glb" {
        return rewrite_gltf_buffer_uris(source, destination, canonical_names);
    }
    if extension != "obj" && extension != "mtl" {
        return Ok(());
    }
    let input = std::fs::File::open(source)
        .map_err(|error| format!("Cannot rewrite model file '{}': {error}", source.display()))?;
    let output = std::fs::File::create(destination).map_err(|error| {
        format!(
            "Cannot save collected model file '{}': {error}",
            destination.display()
        )
    })?;
    let mut reader = BufReader::new(input);
    let mut writer = BufWriter::new(output);
    let mut line_bytes = Vec::new();
    loop {
        if read_model_line(&mut reader, &mut line_bytes)? == 0 {
            break;
        }
        let line = model_line_text(&line_bytes)?;
        let mut rewritten = String::with_capacity(line.len().saturating_add(64));
        let content = line.split('#').next().unwrap_or_default();
        let indent_len = content.len() - content.trim_start().len();
        let trimmed = &content[indent_len..];
        let indent = &content[..indent_len];
        let mut parts = trimmed.split_whitespace();
        let Some(tag) = parts.next() else {
            rewritten.push_str(line);
            rewritten.push('\n');
            writer.write_all(rewritten.as_bytes()).map_err(|error| {
                format!(
                    "Cannot write collected model file '{}': {error}",
                    destination.display()
                )
            })?;
            continue;
        };
        if extension == "obj" && tag == "mtllib" {
            let rest = trimmed[tag.len()..].trim();
            let references = if let Some(start) = rest.find('"') {
                rest[start + 1..]
                    .find('"')
                    .map(|end| vec![rest[start + 1..start + 1 + end].to_string()])
                    .unwrap_or_default()
            } else {
                rest.split_whitespace().map(str::to_string).collect()
            };
            let mut names = Vec::with_capacity(references.len());
            for reference in references {
                let canonical = std::fs::canonicalize(
                    source
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join(&reference),
                )
                .map_err(|error| format!("Cannot resolve OBJ material '{reference}': {error}"))?;
                names.push(
                    canonical_names
                        .get(&canonical)
                        .ok_or_else(|| {
                            format!("Collected OBJ material is missing for '{reference}'")
                        })?
                        .clone(),
                );
            }
            rewritten.push_str(indent);
            rewritten.push_str("mtllib");
            for name in names {
                rewritten.push(' ');
                rewritten.push_str(&name);
            }
            rewritten.push('\n');
            writer.write_all(rewritten.as_bytes()).map_err(|error| {
                format!(
                    "Cannot write collected model file '{}': {error}",
                    destination.display()
                )
            })?;
            continue;
        }
        let is_mtl_map = extension == "mtl"
            && (tag.to_ascii_lowercase().starts_with("map_")
                || matches!(
                    tag.to_ascii_lowercase().as_str(),
                    "bump" | "norm" | "disp" | "decal" | "refl"
                ));
        if is_mtl_map {
            let rest_start = indent.len() + tag.len();
            let rest = line[rest_start..].trim_start();
            let quote_start = rest.find('"');
            let (reference, line_start, line_end) = if let Some(start) = quote_start {
                if let Some(end) = rest[start + 1..].find('"') {
                    (
                        rest[start + 1..start + 1 + end].to_string(),
                        rest_start + (line[rest_start..].len() - rest.len()) + start + 1,
                        rest_start + (line[rest_start..].len() - rest.len()) + start + 1 + end,
                    )
                } else {
                    rewritten.push_str(line);
                    rewritten.push('\n');
                    writer.write_all(rewritten.as_bytes()).map_err(|error| {
                        format!(
                            "Cannot write collected model file '{}': {error}",
                            destination.display()
                        )
                    })?;
                    continue;
                }
            } else if let Some(filename) = rest.split_whitespace().last() {
                let start = content
                    .rfind(filename)
                    .unwrap_or(content.len() - filename.len());
                (filename.to_string(), start, start + filename.len())
            } else {
                rewritten.push_str(line);
                rewritten.push('\n');
                writer.write_all(rewritten.as_bytes()).map_err(|error| {
                    format!(
                        "Cannot write collected model file '{}': {error}",
                        destination.display()
                    )
                })?;
                continue;
            };
            let canonical = std::fs::canonicalize(
                source
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(&reference),
            )
            .map_err(|error| format!("Cannot resolve MTL texture '{reference}': {error}"))?;
            let name = canonical_names
                .get(&canonical)
                .ok_or_else(|| format!("Collected MTL texture is missing for '{reference}'"))?;
            rewritten.push_str(&line[..line_start]);
            rewritten.push_str(name);
            rewritten.push_str(&line[line_end..]);
            rewritten.push('\n');
        } else {
            rewritten.push_str(line);
            rewritten.push('\n');
        }
        writer.write_all(rewritten.as_bytes()).map_err(|error| {
            format!(
                "Cannot write collected model file '{}': {error}",
                destination.display()
            )
        })?;
    }
    writer.flush().map_err(|error| {
        format!(
            "Cannot finish collected model file '{}': {error}",
            destination.display()
        )
    })
}

fn rewrite_gltf_buffer_uris(
    source: &Path,
    destination: &Path,
    canonical_names: &HashMap<PathBuf, String>,
) -> Result<(), String> {
    let is_glb = source
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case("glb"));
    let source_bytes = crate::core::gltf_loader::read_gltf_file(source)?;
    let json = if is_glb {
        crate::core::gltf_loader::glb_json_chunk(&source_bytes)?
    } else {
        &source_bytes
    };
    let mut root: serde_json::Value = serde_json::from_slice(json)
        .map_err(|error| format!("Cannot parse glTF '{}': {error}", source.display()))?;
    let buffers = root
        .get_mut("buffers")
        .and_then(serde_json::Value::as_array_mut);
    let Some(buffers) = buffers else {
        return Err(format!(
            "glTF '{}' has invalid buffer list",
            source.display()
        ));
    };
    for buffer in buffers {
        let Some(uri) = buffer
            .get("uri")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
        else {
            if is_glb {
                continue;
            }
            return Err(format!(
                "glTF '{}' has a buffer without a URI",
                source.display()
            ));
        };
        if uri.starts_with("data:") {
            continue;
        }
        let referenced_path = crate::core::gltf_loader::resolve_gltf_buffer_path(source, &uri)?;
        let canonical = std::fs::canonicalize(&referenced_path).map_err(|error| {
            format!(
                "Cannot resolve glTF buffer '{}': {error}",
                referenced_path.display()
            )
        })?;
        let collected_name = canonical_names.get(&canonical).ok_or_else(|| {
            format!(
                "Collected glTF buffer is missing for '{}'",
                referenced_path.display()
            )
        })?;
        buffer["uri"] = serde_json::Value::String(collected_name.clone());
    }
    if let Some(images) = root
        .get_mut("images")
        .and_then(serde_json::Value::as_array_mut)
    {
        for image in images {
            let Some(uri) = image
                .get("uri")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
            else {
                continue;
            };
            if uri.starts_with("data:") {
                continue;
            }
            let source_path = crate::core::gltf_loader::resolve_gltf_buffer_path(source, &uri)?;
            let canonical = std::fs::canonicalize(&source_path).map_err(|error| {
                format!(
                    "Cannot resolve glTF image '{}': {error}",
                    source_path.display()
                )
            })?;
            let name = canonical_names.get(&canonical).ok_or_else(|| {
                format!(
                    "Collected glTF image is missing for '{}'",
                    source_path.display()
                )
            })?;
            image["uri"] = serde_json::Value::String(name.clone());
        }
    }
    if is_glb {
        let mut rewritten_json = serde_json::to_vec(&root).map_err(|error| {
            format!(
                "Cannot rewrite collected GLB '{}': {error}",
                destination.display()
            )
        })?;
        while !rewritten_json.len().is_multiple_of(4) {
            rewritten_json.push(b' ');
        }
        return rewrite_glb_json_chunk(&source_bytes, destination, &rewritten_json);
    }
    let output = std::fs::File::create(destination).map_err(|error| {
        format!(
            "Cannot save collected glTF '{}': {error}",
            destination.display()
        )
    })?;
    let mut writer = BufWriter::new(output);
    serde_json::to_writer(&mut writer, &root).map_err(|error| {
        format!(
            "Cannot rewrite collected glTF '{}': {error}",
            destination.display()
        )
    })?;
    writer.flush().map_err(|error| {
        format!(
            "Cannot finish collected glTF '{}': {error}",
            destination.display()
        )
    })
}

fn rewrite_glb_json_chunk(source: &[u8], destination: &Path, json: &[u8]) -> Result<(), String> {
    if source.len() < 12
        || u32::from_le_bytes(source[0..4].try_into().unwrap()) != 0x46546C67
        || u32::from_le_bytes(source[4..8].try_into().unwrap()) != 2
        || u32::from_le_bytes(source[8..12].try_into().unwrap()) as usize != source.len()
    {
        return Err("Cannot rewrite invalid GLB header".into());
    }
    let mut output = Vec::with_capacity(source.len().saturating_add(json.len()));
    output.extend_from_slice(&0x46546C67u32.to_le_bytes());
    output.extend_from_slice(&2u32.to_le_bytes());
    output.extend_from_slice(&0u32.to_le_bytes());
    let mut offset = 12usize;
    let mut replaced_json = false;
    while offset < source.len() {
        let header_end = offset
            .checked_add(8)
            .filter(|&end| end <= source.len())
            .ok_or("Truncated GLB chunk header")?;
        let length = u32::from_le_bytes(source[offset..offset + 4].try_into().unwrap()) as usize;
        let kind = u32::from_le_bytes(source[offset + 4..header_end].try_into().unwrap());
        let end = header_end
            .checked_add(length)
            .filter(|&end| end <= source.len())
            .ok_or("GLB chunk is out of bounds")?;
        let payload = if kind == 0x4E4F534A {
            if replaced_json {
                return Err("GLB contains multiple JSON chunks".into());
            }
            replaced_json = true;
            json
        } else {
            &source[header_end..end]
        };
        let chunk_len =
            u32::try_from(payload.len()).map_err(|_| "Rewritten GLB chunk exceeds format limit")?;
        output.extend_from_slice(&chunk_len.to_le_bytes());
        output.extend_from_slice(&kind.to_le_bytes());
        output.extend_from_slice(payload);
        offset = end;
    }
    if !replaced_json {
        return Err("Missing JSON chunk in GLB".into());
    }
    let total_len =
        u32::try_from(output.len()).map_err(|_| "Rewritten GLB exceeds format limit")?;
    output[8..12].copy_from_slice(&total_len.to_le_bytes());
    std::fs::write(destination, output).map_err(|error| {
        format!(
            "Cannot save collected GLB '{}': {error}",
            destination.display()
        )
    })
}
