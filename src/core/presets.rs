//! One-click animation presets for the timeline (YouTube-title tier):
//! fades, pops, slides, punch-zooms. Each bakes keyframes onto the
//! selected layer relative to its current state / playhead.
//!
//! All functions are deterministic and unit-tested.

pub mod basics;
pub mod cinematic;
pub mod compound;
pub mod expressions_transitions;
pub mod helpers;
pub mod text_time;

pub use basics::*;
pub use cinematic::*;
pub use compound::*;
pub use expressions_transitions::*;
pub use text_time::*;

use crate::core::timeline::Layer;

/// Returns the display category for a preset name (used by the UI submenu).
pub fn category_of(name: &str) -> &'static str {
    match name {
        "Fade In" | "Fade Out" | "Pop In" | "Slide In \u{2190}" | "Slide In \u{2192}"
        | "Zoom Punch" | "Slam In" => "Basics",
        "\u{1f39e} Film Look"
        | "\u{1f3ac} Handheld"
        | "\u{1f4a5} Quake"
        | "\u{1f3ac} Letterbox 2.39"
        | "\u{26a1} Whip Out \u{2192}"
        | "\u{26a1} Whip In \u{2190}" => "Cinematic",
        "\u{23f1} Speed Ramp: Slow-Mo" | "\u{23f1} Speed Ramp: Fast \u{00d7}4" => "Speed",
        "\u{1f3a5} Ken Burns In" | "\u{1f3a5} Ken Burns Out" => "Camera",
        "Fade to Black" | "Fade from Black" | "Dip to White" => "Fade to Color",
        "\u{26a1} Flash Cut" => "Flash",
        "\u{1f39e} Film Reel Intro"
        | "YouTube Vlog"
        | "Music Video"
        | "Cinematic Reveal"
        | "Documentary Opener" => "Compound",
        "\u{1f9f2} Bounce" | "\u{1f300} Elastic" | "\u{1f30a} Sine Wave" | "\u{1f4a1} Strobe" => {
            "Live Expressions"
        }
        "\u{1f3ac} Slide Out \u{2192}" | "\u{1f3ac} Zoom Out" => "Transitions",
        "Typewriter" | "Bounce In Text" | "Scale Up Text" | "Fade Up Words" => "Text Animation",
        "Freeze Frame" | "Reverse" | "Slow Motion 0.5\u{00d7}" | "Fast Forward 2\u{00d7}" => {
            "Time Remap"
        }
        "Reset Layer" => "Utility",
        _ => "",
    }
}

pub const NAMES: &[&str] = &[
    "Fade In",
    "Fade Out",
    "Pop In",
    "Slide In ←",
    "Slide In →",
    "Zoom Punch",
    "Slam In",
    // ── Cinematic set ──
    "🎞 Film Look",
    "🎬 Handheld",
    "💥 Quake",
    "🎬 Letterbox 2.39",
    "⚡ Whip Out →",
    "⚡ Whip In ←",
    // ── Speed ramps ──
    "⏱ Speed Ramp: Slow-Mo",
    "⏱ Speed Ramp: Fast ×4",
    // ── Ken Burns ──
    "🎥 Ken Burns In",
    "🎥 Ken Burns Out",
    // ── Fade to color ──
    "Fade to Black",
    "Fade from Black",
    "Dip to White",
    // ── Flash ──
    "⚡ Flash Cut",
    // ── Compound cinematic ──
    "🎞 Film Reel Intro",
    // ── Live expression presets ──
    "🧲 Bounce",
    "🌀 Elastic",
    "🌊 Sine Wave",
    "💡 Strobe",
    // ── Scene transitions (at out-point) ──
    "🎬 Slide Out →",
    "🎬 Zoom Out",
    // ── Text animation presets ──
    "Typewriter",
    "Bounce In Text",
    "Scale Up Text",
    "Fade Up Words",
    // ── Time Remap presets ──
    "Freeze Frame",
    "Reverse",
    "Slow Motion 0.5×",
    "Fast Forward 2×",
    // ── Compound cinematic v2 ──
    "YouTube Vlog",
    "Music Video",
    "Cinematic Reveal",
    "Documentary Opener",
    // ── Utility ──
    "Reset Layer",
];

/// Dispatch by name; returns whether the preset applied.
pub fn apply_by_name(name: &str, l: &mut Layer, cf: u32, comp_w: f32, _comp_h: f32) -> bool {
    match name {
        "Fade In" => fade_in(l, cf, 20),
        "Fade Out" => fade_out(l, cf, 20),
        "Pop In" => pop_in(l, cf),
        "Slide In ←" => slide_in(l, cf, comp_w, false),
        "Slide In →" => slide_in(l, cf, comp_w, true),
        "Zoom Punch" => zoom_punch(l, cf),
        "Slam In" => slam_in(l, cf),
        "🎞 Film Look" => film_look(l),
        "🎬 Handheld" => handheld(l),
        "💥 Quake" => quake(l, cf),
        "🎬 Letterbox 2.39" => letterbox_239(l, cf),
        "⚡ Whip Out →" => whip(l, cf, true),
        "⚡ Whip In ←" => whip(l, cf, false),
        "⏱ Speed Ramp: Slow-Mo" => speed_ramp(l, cf, 0.25),
        "⏱ Speed Ramp: Fast ×4" => speed_ramp(l, cf, 4.0),
        "🎥 Ken Burns In" => ken_burns(l, cf, comp_w, true),
        "🎥 Ken Burns Out" => ken_burns(l, cf, comp_w, false),
        "Fade to Black" => fade_to_color(l, cf, [0.0; 4]),
        "Fade from Black" => fade_from_color(l, cf, [0.0; 4]),
        "Dip to White" => fade_to_color(l, cf, [1.0; 4]),
        "⚡ Flash Cut" => flash_cut(l, cf),
        "🎞 Film Reel Intro" => film_reel_intro(l, cf),
        "🧲 Bounce" => expr_bounce(l),
        "🌀 Elastic" => expr_elastic(l),
        "🌊 Sine Wave" => expr_sine_wave(l),
        "💡 Strobe" => expr_strobe(l, comp_w),
        "🎬 Slide Out →" => slide_out(l, comp_w),
        "🎬 Zoom Out" => zoom_out(l),
        "Typewriter" => text_typewriter(l, cf),
        "Bounce In Text" => text_bounce_in(l, cf),
        "Scale Up Text" => text_scale_up(l, cf),
        "Fade Up Words" => text_fade_up(l, cf),
        "Freeze Frame" => freeze_frame(l, cf),
        "Reverse" => reverse_time(l),
        "Slow Motion 0.5×" => time_scale(l, 0.5),
        "Fast Forward 2×" => time_scale(l, 2.0),
        "YouTube Vlog" => youtube_vlog(l, cf),
        "Music Video" => music_video(l, cf),
        "Cinematic Reveal" => cinematic_reveal(l, cf, comp_w),
        "Documentary Opener" => documentary_opener(l, cf),
        "Reset Layer" => reset_layer(l),
        _ => false,
    }
}

#[cfg(test)]
pub mod tests;
