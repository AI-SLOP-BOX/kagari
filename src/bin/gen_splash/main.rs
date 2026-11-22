mod elements;
mod particles;
mod title;

use kagari_vfx::core::keyframe::*;
use kagari_vfx::core::project_migration::save_project_atomic;
use kagari_vfx::core::property::*;
use kagari_vfx::core::timeline::*;
use particles::add_embers_and_burst;
use title::add_title_and_tagline;

fn main() {
    let mut comp = Composition::new(
        "splash".into(),
        "Kagari VFX Splash".into(),
        1920,
        1080,
        30,
        150,
    );

    comp.add_layer(Layer::new(
        "bg".into(),
        "BG".into(),
        LayerType::Solid { color: [0.0; 4] },
        150,
    ));
    comp.layers.last_mut().unwrap().transform.position = Animatable::new_constant([960.0, 540.0]);

    comp.add_layer(Layer::new(
        "ambient".into(),
        "Ambient".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(900.0),
                height: Animatable::new_constant(900.0),
            },
            color: [0.10, 0.05, 0.01, 1.0],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        150,
    ));
    {
        let a = comp.layers.last_mut().unwrap();
        a.transform.position = Animatable::new_constant([960.0, 480.0]);
        a.transform.opacity = Animatable::new_animated(vec![
            Keyframe::new(0, 0.0, InterpolationType::Linear),
            Keyframe::new(30, 30.0, InterpolationType::Linear),
            Keyframe::new(135, 30.0, InterpolationType::Linear),
            Keyframe::new(150, 0.0, InterpolationType::Linear),
        ]);
    }

    add_embers_and_burst(&mut comp);
    add_title_and_tagline(&mut comp);

    let project = Project {
        compositions: vec![comp],
        ..Default::default()
    };
    save_project_atomic(&project, "splash_project.json").unwrap();
    println!("Wrote splash_project.json");
}
