use crate::KagariApp;
use eframe::egui;

use super::layer_actions::handle_layer_manipulation;
use super::timeline_nav::handle_navigation_and_markers;

/// Handles timeline cursor navigation, markers, layer manipulation, and work area shortcuts.
pub fn handle_timeline_and_layers(
    app: &mut KagariApp,
    i: &egui::InputState,
    current_frame: &mut u32,
    total_frames: u32,
    allow_single_key: bool,
    cmd: bool,
    shift: bool,
) {
    handle_navigation_and_markers(
        app,
        i,
        current_frame,
        total_frames,
        allow_single_key,
        cmd,
        shift,
    );
    handle_layer_manipulation(
        app,
        i,
        current_frame,
        total_frames,
        allow_single_key,
        cmd,
        shift,
    );
}
