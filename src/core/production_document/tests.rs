use super::validation::{
    effect_debug_is_finite, light_type_is_valid, particle_emitter_is_valid, valid_gradient_stops,
};
use super::*;
use crate::core::timeline::Layer;

include!("tests/cases_01.rs");
include!("tests/cases_02.rs");
include!("tests/cases_03.rs");
include!("tests/cases_04.rs");
