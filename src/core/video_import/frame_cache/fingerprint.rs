use super::*;

pub(super) fn frame_cache_fingerprint(
    frames_dir: &Path,
    frame_count: u32,
    cancel: Option<&AtomicBool>,
) -> Option<(u64, u64)> {
    use io::Read;

    let mut total_bytes = 0u64;
    let mut hash = 0xcbf29ce484222325u64;
    let mut buffer = [0u8; 64 * 1024];
    for frame in 0..frame_count {
        if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return None;
        }
        let path = frame_path_in_dir(&frames_dir.to_string_lossy(), frame);
        let mut file = std::fs::File::open(&path).ok()?;
        hash = fnv1a_update(hash, &frame.to_le_bytes());
        loop {
            if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                return None;
            }
            let count = file.read(&mut buffer).ok()?;
            if count == 0 {
                break;
            }
            total_bytes = total_bytes.checked_add(count as u64)?;
            hash = fnv1a_update(hash, &buffer[..count]);
        }
    }
    Some((total_bytes, hash))
}

pub(super) fn source_fingerprint(source: &Path) -> Option<SourceFingerprint> {
    let metadata = std::fs::metadata(source).ok()?;
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    Some(SourceFingerprint {
        canonical_path: std::fs::canonicalize(source)
            .ok()?
            .to_string_lossy()
            .into_owned(),
        size_bytes: metadata.len(),
        modified_unix_nanos: modified,
        media_revision: crate::core::media_revision::media_file_revision_from_metadata(
            source, &metadata,
        ),
    })
}

pub(super) fn image_sequence_source_fingerprint(
    source: &Path,
    cancel: Option<&AtomicBool>,
) -> Option<SourceFingerprint> {
    let file_name = source.file_name()?;
    let extension = source.extension()?.to_str()?.to_ascii_lowercase();
    let stem = source.file_stem()?.to_str()?;
    let split_at = stem
        .char_indices()
        .rev()
        .find(|(_, ch)| !ch.is_ascii_digit())
        .map(|(index, ch)| index + ch.len_utf8())?;
    let prefix = &stem[..split_at];
    if prefix.is_empty() || split_at == stem.len() {
        return None;
    }
    stem[split_at..].parse::<u32>().ok()?;
    let parent = source
        .parent()
        .filter(|path| !path.as_os_str().is_empty())?;
    let canonical_parent = std::fs::canonicalize(parent).ok()?;
    let mut frames = Vec::new();
    for entry in std::fs::read_dir(parent).ok()? {
        if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return None;
        }
        let path = entry.ok()?.path();
        if !path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(&extension))
        {
            continue;
        }
        let Some(candidate_stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(suffix) = candidate_stem.strip_prefix(prefix) else {
            continue;
        };
        let Ok(frame) = suffix.parse::<u32>() else {
            continue;
        };
        if frames.len() >= MAX_IMPORT_FRAMES as usize {
            return None;
        }
        frames.try_reserve(1).ok()?;
        frames.push((frame, path));
    }
    if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        return None;
    }
    frames.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    if frames.is_empty()
        || !frames
            .iter()
            .any(|(_, path)| path.file_name() == Some(file_name))
    {
        return None;
    }

    let mut total_bytes = 0u64;
    let mut stamp_hash = 0xcbf29ce484222325u64;
    let mut revision_hash = 0xcbf29ce484222325u64;
    for (frame, path) in frames {
        if cancel.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return None;
        }
        let metadata = std::fs::metadata(&path).ok()?;
        let modified = metadata
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_nanos();
        total_bytes = total_bytes.checked_add(metadata.len())?;
        stamp_hash = fnv1a_update(stamp_hash, &frame.to_le_bytes());
        stamp_hash = fnv1a_update(stamp_hash, &metadata.len().to_le_bytes());
        stamp_hash = fnv1a_update(stamp_hash, &modified.to_le_bytes());
        let canonical_path = std::fs::canonicalize(&path).ok()?;
        stamp_hash = fnv1a_update(stamp_hash, canonical_path.to_string_lossy().as_bytes());
        let revision =
            crate::core::media_revision::media_file_revision_from_metadata(&path, &metadata);
        revision_hash = fnv1a_update(revision_hash, &frame.to_le_bytes());
        revision_hash = fnv1a_update(revision_hash, &revision.to_le_bytes());
    }

    Some(SourceFingerprint {
        canonical_path: format!(
            "{}{}*.{}",
            canonical_parent.to_string_lossy(),
            std::path::MAIN_SEPARATOR,
            extension
        ),
        size_bytes: total_bytes,
        modified_unix_nanos: u128::from(stamp_hash),
        media_revision: revision_hash,
    })
}

fn fnv1a_update(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
