use super::*;
use crate::core::history::ProjectHistory;
use crate::core::property::Animatable;
use crate::core::timeline::{Composition, Effect, EffectType, Layer, LayerType, Project};

fn make_test_project() -> Project {
    let mut project = Project::default();
    project.compositions.clear();
    let mut comp = Composition::new("c".into(), "Test".into(), 100, 100, 10, 10);
    let mut layer = Layer::new(
        "l0".into(),
        "Layer0".into(),
        LayerType::Solid {
            color: [1.0, 1.0, 1.0, 1.0],
        },
        10,
    );
    layer.effects.push(Effect {
        id: "blur_0".into(),
        name: "Gaussian Blur".into(),
        effect_type: EffectType::GaussianBlur {
            blur_radius: Animatable::new_constant(10.0),
        },
        enabled: true,
    });
    comp.layers.push(layer);
    project.compositions.push(comp);
    project
}
include!("tests/cases_01.rs");
include!("tests/cases_02.rs");
