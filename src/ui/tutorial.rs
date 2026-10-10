//! Interactive first-run tutorial: comprehensive multi-chapter walkthrough
//! covering the full Kagari VFX workflow from basics to advanced.
//!
//! Submodules:
//! - [`steps`]: Data model, static steps, chapters, and shortcut representations.
//! - [`card`]: Floating modal window renderer, progress indicators, and actions.

pub mod card;
pub mod steps;

pub use card::*;
pub use steps::*;

use crate::KagariApp;
use eframe::egui;

/// Persisted-per-session tutorial progress.
#[derive(Debug, Clone, Default)]
pub struct TutorialState {
    pub current_step: usize,
    pub completed: bool,
    /// Show the chapter-select overview card
    pub show_chapter_select: bool,
}

/// Draw the floating tutorial card. Call every frame; no-ops when not active.
pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    draw_tutorial_card(app, ctx);
}

/// Reopen the walkthrough from the beginning (Help menu entry).
pub fn restart(app: &mut KagariApp) {
    start_at(app, 0);
}

pub fn start_at(app: &mut KagariApp, step: usize) {
    app.tutorial = Some(TutorialState {
        current_step: step.min(steps().len().saturating_sub(1)),
        completed: false,
        show_chapter_select: false,
    });
}
