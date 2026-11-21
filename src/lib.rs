#![warn(unused_imports)]
#![warn(unused_variables)]
#![warn(dead_code)]
#![warn(unreachable_code)]
#![warn(unreachable_patterns)]

pub mod core;

pub use core::automation;
pub use core::ffmpeg_export::ExportEvent;
pub use core::timeline::Project;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewportMode {
    Comp2D,
    Camera3D,
}

#[derive(Debug, Clone)]
pub enum TrackerEvent {
    Progress(f32, String),
    Finished {
        layer_id: String,
        layer_idx: usize,
        tracker_idx: usize,
        keyframes: Vec<core::keyframe::Keyframe<[f32; 2]>>,
    },
    Error(String),
}
