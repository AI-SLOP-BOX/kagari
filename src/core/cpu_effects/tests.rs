#![allow(clippy::module_inception)]
//! Unit tests for the CPU pixel-effect pipeline.

use super::*;
use crate::core::property::Animatable;
use crate::core::timeline::{Composition, Effect, EffectType};

fn solid_layer(w: u32, h: u32, r: u8, g: u8, b: u8) -> Vec<u8> {
    let mut px = vec![0u8; (w * h * 4) as usize];
    for p in px.as_chunks_mut::<4>().0 {
        p[0] = r;
        p[1] = g;
        p[2] = b;
        p[3] = 255;
    }
    px
}

fn effect(name: &str, et: EffectType) -> Effect {
    Effect {
        id: name.to_string(),
        name: name.to_string(),
        effect_type: et,
        enabled: true,
    }
}

include!("tests/cases_01.rs");
include!("tests/cases_02.rs");
include!("tests/cases_03.rs");
include!("tests/cases_04.rs");
