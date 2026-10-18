//! MLT XML and helper string encoding functions.

pub fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn unescape_xml(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

pub fn timecode_to_frames(tc: &str, fps: u32) -> Option<u32> {
    if fps == 0 {
        return None;
    }
    if let Ok(frames) = tc.parse::<u32>() {
        return Some(frames);
    }
    let parts: Vec<&str> = tc.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let h: u64 = parts[0].parse().ok()?;
    let m: u64 = parts[1].parse().ok()?;
    let mut sec_parts = parts[2].split('.');
    let s: u64 = sec_parts.next()?.parse().ok()?;
    let ms: u64 = sec_parts.next()?.parse().ok()?;
    if sec_parts.next().is_some() || m >= 60 || s >= 60 || ms >= 1000 {
        return None;
    }
    let total_sec = h
        .checked_mul(3600)?
        .checked_add(m.checked_mul(60)?)?
        .checked_add(s)?;
    let fractional_frames = ms.checked_mul(u64::from(fps))?;
    let frames = total_sec
        .checked_mul(u64::from(fps))?
        .checked_add(fractional_frames.checked_add(500)? / 1000)?;
    u32::try_from(frames).ok()
}
