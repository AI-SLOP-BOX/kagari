//! Helper functions for export range and path generation.

use crate::ui::localization::{tr, tr_format};

/// Resolve the export range used by both the UI preview and every encoder.
/// Work Area In/Out are inclusive in the timeline, while encoder APIs take a
/// start plus a frame count (and audio uses an exclusive end frame).
pub fn export_frame_range(
    comp_total: u32,
    range_mode: u8,
    work_area_in: Option<u32>,
    work_area_out: Option<u32>,
) -> (u32, u32) {
    if comp_total == 0 {
        return (0, 0);
    }
    if range_mode == 1 && (work_area_in.is_some() || work_area_out.is_some()) {
        let last = comp_total.saturating_sub(1);
        let start = work_area_in.unwrap_or(0).min(last);
        let end = work_area_out.unwrap_or(last).min(last).max(start);
        return (start, end.saturating_sub(start).saturating_add(1));
    }
    (0, comp_total)
}

pub fn batch_output_path(base: &str, composition_name: &str, composition_id: &str) -> String {
    let path = std::path::Path::new(base);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("render");
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty());
    let sanitize = |value: &str| {
        let cleaned: String = value
            .chars()
            .map(|ch| {
                if ch.is_alphanumeric() || matches!(ch, '-' | '_') {
                    ch
                } else {
                    '_'
                }
            })
            .collect();
        if cleaned.is_empty() {
            "comp".to_string()
        } else {
            cleaned
        }
    };
    let filename = format!(
        "{}_{}_{}{}",
        stem,
        sanitize(composition_name),
        sanitize(composition_id),
        extension
            .map(|value| format!(".{value}"))
            .unwrap_or_default()
    );
    path.with_file_name(filename).to_string_lossy().into_owned()
}

pub fn localized_export_status(status: &str) -> String {
    if let Some(message) = status.strip_prefix("Error: ") {
        return format!("{}: {}", tr("Error"), tr(message));
    }
    if let Some(path) = status.strip_prefix("Export complete → ") {
        return tr_format("Export complete → {}", &[path]);
    }
    if let Some(path) = status.strip_prefix("Lottie export complete → ") {
        return tr_format("Lottie export complete → {}", &[path]);
    }
    tr(status)
}
