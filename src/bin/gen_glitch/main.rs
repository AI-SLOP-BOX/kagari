//! Glitch reconstruct title: KAGARI VFX (5s, 1920x1080, 30fps).
//!
//! Scene-built only: per-character text layers, procedural shapes, animated
//! masks, and generic engine effects (no pre-rendered material, no hacks).
//! All "random" placement uses build-time hashing baked into keyframes, so
//! every render of this file is byte-identical.

mod common;
mod master;
mod signals;
mod text;

use common::*;
use kagari_vfx::core::project_migration::save_project_atomic;
use kagari_vfx::core::timeline::{Composition, Project};
use master::add_distortion_and_master;
use signals::add_signals_and_background;
use text::add_glitch_text_layers;

fn build_glitch() -> Composition {
    let mut comp = Composition::new(
        "GlitchTitle".into(),
        "KAGARI VFX reconstruct".into(),
        W as u32,
        H as u32,
        FPS,
        DUR,
    );

    add_signals_and_background(&mut comp);
    add_glitch_text_layers(&mut comp);
    add_distortion_and_master(&mut comp);

    comp
}

fn main() {
    let project = Project {
        compositions: vec![build_glitch()],
        ..Default::default()
    };
    save_project_atomic(&project, "glitch_project.json").unwrap();
    println!("Wrote glitch_project.json");
}
