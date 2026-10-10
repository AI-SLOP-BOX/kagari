//! Playhead transport, markers navigation, and work area shortcuts.

use crate::KagariApp;
use eframe::egui::{self, Key};
use super::defs::{shortcut_pressed, ShortcutId};

pub fn handle_navigation_and_markers(
    app: &mut KagariApp,
    i: &egui::InputState,
    current_frame: &mut u32,
    total_frames: u32,
    allow_single_key: bool,
    cmd: bool,
    shift: bool,
) {
    // B → Set Work Area Start, N → Set Work Area End (single-key)
    if allow_single_key && shortcut_pressed(i, ShortcutId::WorkAreaStart) {
        app.playback.work_area_in = Some(*current_frame);
    }
    if allow_single_key && shortcut_pressed(i, ShortcutId::WorkAreaEnd) {
        app.playback.work_area_out = Some(*current_frame);
    }

    // ── AE-style keyframe navigation & transport ──
    // J → previous keyframe, K → next keyframe (AE standard).
    if allow_single_key
        && (shortcut_pressed(i, ShortcutId::PreviousKeyframe)
            || shortcut_pressed(i, ShortcutId::NextKeyframe))
    {
        let going_next = shortcut_pressed(i, ShortcutId::NextKeyframe);
        if let Some(idx) = app.selection.selected_layer_idx {
            let comp = app.history.current().active_composition();
            if let Some(layer) = comp.layers.get(idx) {
                let t = &layer.transform;
                let mut kf_frames: Vec<u32> = Vec::new();
                for prop in [&t.position, &t.scale] {
                    if let Some(kfs) = prop.keyframes() {
                        kf_frames.extend(kfs.iter().map(|k| k.frame));
                    }
                }
                for prop in [&t.rotation, &t.opacity] {
                    if let Some(kfs) = prop.keyframes() {
                        kf_frames.extend(kfs.iter().map(|k| k.frame));
                    }
                }
                kf_frames.sort_unstable();
                kf_frames.dedup();
                let cur = *current_frame;
                let target = if going_next {
                    kf_frames.iter().find(|&&f| f > cur).copied()
                } else {
                    kf_frames.iter().rev().find(|&&f| f < cur).copied()
                };
                if let Some(f) = target {
                    *current_frame = f;
                }
            }
        }
    }

    // Home → first frame, End → last frame
    if allow_single_key && i.key_pressed(Key::Home) {
        *current_frame = 0;
    }
    if allow_single_key && i.key_pressed(Key::End) {
        *current_frame = total_frames.saturating_sub(1);
    }

    // Page Up / Down → frame step backward/forward
    if allow_single_key && i.key_pressed(Key::PageUp) {
        *current_frame = current_frame.saturating_sub(1);
    }
    if allow_single_key && i.key_pressed(Key::PageDown) {
        *current_frame = (*current_frame + 1).min(total_frames.saturating_sub(1));
    }

    // Shift+Home / Shift+End: Jump to Work Area In / Out
    if allow_single_key && shift && i.key_pressed(Key::Home) {
        *current_frame = app.playback.work_area_in.unwrap_or(0);
    }
    if allow_single_key && shift && i.key_pressed(Key::End) {
        let last = total_frames.saturating_sub(1);
        *current_frame = app.playback.work_area_out.unwrap_or(last).min(last);
    }

    // ── Timeline markers: M adds/removes a marker at the playhead ──
    if allow_single_key && !cmd && i.key_pressed(Key::M) {
        let frame = *current_frame;
        app.modify_project(|p| {
            let comp = p.active_composition_mut();
            if let Some(existing) = comp.markers.iter().position(|m| m.frame == frame) {
                comp.markers.remove(existing);
            } else {
                comp.markers.push(crate::core::timeline::TimelineMarker {
                    frame,
                    label: format!("M{}", comp.markers.len() + 1),
                    color: [0.2, 0.9, 0.5],
                });
            }
        });
    }

    // ── Shift+; → Go to Next Marker, Cmd+; → Go to Previous Marker ──
    if i.key_pressed(Key::Semicolon) && shift && !cmd {
        let comp = app.history.current().active_composition();
        let cur = *current_frame;
        let next = comp
            .markers
            .iter()
            .map(|m| m.frame)
            .filter(|&f| f > cur)
            .min();
        if let Some(f) = next {
            *current_frame = f;
        } else if let Some(f) = comp.markers.iter().map(|m| m.frame).min() {
            *current_frame = f;
        }
    }
    if i.key_pressed(Key::Semicolon) && cmd {
        let comp = app.history.current().active_composition();
        let cur = *current_frame;
        let prev = comp
            .markers
            .iter()
            .map(|m| m.frame)
            .filter(|&f| f < cur)
            .max();
        if let Some(f) = prev {
            *current_frame = f;
        } else if let Some(f) = comp.markers.iter().map(|m| m.frame).max() {
            *current_frame = f;
        }
    }

    // ── Jump between markers with [ ] keys ──
    if allow_single_key && (i.key_pressed(Key::OpenBracket) || i.key_pressed(Key::CloseBracket)) {
        let forward = i.key_pressed(Key::CloseBracket);
        let project = app.history.current();
        let mut frames: Vec<u32> = project
            .active_composition()
            .markers
            .iter()
            .map(|m| m.frame)
            .collect();
        frames.sort_unstable();
        frames.dedup();
        if !frames.is_empty() {
            let target = if forward {
                frames.iter().find(|&&f| f > *current_frame).copied()
            } else {
                frames.iter().rev().find(|&&f| f < *current_frame).copied()
            };
            if let Some(f) = target {
                *current_frame = f;
            } else {
                *current_frame = if forward {
                    frames[0]
                } else {
                    frames[frames.len() - 1]
                };
            }
        }
    }
}
