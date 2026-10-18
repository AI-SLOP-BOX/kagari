#![allow(dead_code)]

mod algorithms;
pub use algorithms::*;

/// Simple CPU-based Motion Tracking Engine inspired by SAD (Sum of Absolute Differences).
/// Refines and tracks TrackerPoints frame-by-frame.
pub struct TrackerEngine;

mod pose;
mod tracking;

use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::core::timeline::{Composition, Layer};
