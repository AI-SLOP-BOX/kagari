use super::*;
use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::property::Animatable;
use crate::core::timeline::{Composition, Layer, LayerType, ShapeFillType, ShapeType};

fn shadow_test_comp(caster_casts: bool) -> Composition {
    let mut comp = Composition::new("sh".into(), "Shadows".into(), 64, 64, 30, 30);
    // Receiver: full-frame white solid (bottom of stack)
    let mut recv = Layer::new(
        "r1".into(),
        "Floor".into(),
        LayerType::Solid { color: [1.0; 4] },
        30,
    );
    recv.transform.scale = Animatable::new_constant([100.0, 100.0]);
    recv.transform.position = Animatable::new_constant([32.0, 32.0]);
    comp.layers.push(recv);

    // Caster: red solid raised on +z between light and floor plane
    let mut caster = Layer::new(
        "c1".into(),
        "Card".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    );
    caster.is_3d = true;
    caster.material.cast_shadows = caster_casts;
    caster.transform.scale = Animatable::new_constant([20.0, 20.0]);
    caster.transform.position = Animatable::new_constant([36.0, 36.0]);
    caster.transform_3d.position = Animatable::new_constant([36.0, 36.0, 100.0]);
    comp.layers.push(caster);

    // Point light upper-left of the caster, above the plane (+z), casting
    let light = crate::core::timeline::Light3D {
        id: "key".into(),
        name: "Key".into(),
        light_type: crate::core::timeline::LightType::Point,
        color: [1.0, 1.0, 1.0, 1.0],
        intensity: 100.0,
        intensity_animation: None,
        position: Animatable::new_constant([16.0, 16.0, 400.0]),
        casts_shadows: true,
        shadow_darkness: 90.0,
        falloff: 1.0,
        max_radius: 0.0,
    };
    comp.lights = vec![light];
    comp
}

include!("tests/shadow_tests/cases_01.rs");
include!("tests/shadow_tests/cases_02.rs");
