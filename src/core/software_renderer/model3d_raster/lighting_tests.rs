use super::*;
use crate::core::property::Animatable;
use crate::core::timeline::{Composition, LayerType, Light3D, LightType};

fn test_light(light_type: LightType, position: [f32; 3]) -> Light3D {
    Light3D {
        id: "test-light".into(),
        name: "Test light".into(),
        light_type,
        color: [1.0; 4],
        intensity: 100.0,
        intensity_animation: None,
        position: Animatable::new_constant(position),
        casts_shadows: false,
        shadow_darkness: 60.0,
        falloff: 0.0,
        max_radius: 0.0,
    }
}

#[test]
fn parallel_mesh_light_ignores_its_position() {
    let mut comp = Composition::new("mesh-light".into(), "Mesh light".into(), 64, 64, 30, 1);
    comp.lights = vec![test_light(LightType::Parallel, [-1000.0, 600.0, -800.0])];
    let layer = Layer::new("mesh".into(), "Mesh".into(), LayerType::Null, 1);
    let first = shade(
        &layer,
        &comp,
        [0.0; 3],
        [0.0, 0.0, 1.0],
        [1.0; 4],
        None,
        [0.0; 3],
        0,
    );

    comp.lights[0].position = Animatable::new_constant([1000.0, -600.0, 800.0]);
    let second = shade(
        &layer,
        &comp,
        [0.0; 3],
        [0.0, 0.0, 1.0],
        [1.0; 4],
        None,
        [0.0; 3],
        0,
    );
    assert_eq!(first, second);
    assert!(first[0] > 180, "parallel light should illuminate the face");
}

#[test]
fn mesh_spot_light_does_not_illuminate_outside_its_cone() {
    let mut comp = Composition::new("mesh-spot".into(), "Mesh spot".into(), 64, 64, 30, 1);
    comp.lights = vec![test_light(
        LightType::Spot {
            cone_angle_deg: 30.0,
            cone_feather_pct: 0.0,
        },
        [0.0, 0.0, 100.0],
    )];
    let layer = Layer::new("mesh".into(), "Mesh".into(), LayerType::Null, 1);
    let center = shade(
        &layer,
        &comp,
        [0.0; 3],
        [0.0, 0.0, 1.0],
        [1.0; 4],
        None,
        [0.0; 3],
        0,
    );
    let outside = shade(
        &layer,
        &comp,
        [100.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0; 4],
        None,
        [0.0; 3],
        0,
    );
    assert!(center[0] > outside[0] + 100);
}

#[test]
fn ambient_mesh_light_is_position_independent_and_colored() {
    let mut comp = Composition::new("mesh-ambient".into(), "Mesh ambient".into(), 64, 64, 30, 1);
    let mut light = test_light(LightType::Ambient, [-800.0, 1000.0, -500.0]);
    light.color = [1.0, 0.0, 0.0, 1.0];
    light.intensity = 50.0;
    comp.lights = vec![light];
    let layer = Layer::new("mesh".into(), "Mesh".into(), LayerType::Null, 1);
    let first = shade(
        &layer,
        &comp,
        [0.0; 3],
        [0.0, 0.0, 1.0],
        [1.0; 4],
        None,
        [0.0; 3],
        0,
    );
    comp.lights[0].position = Animatable::new_constant([800.0, -1000.0, 500.0]);
    let second = shade(
        &layer,
        &comp,
        [0.0; 3],
        [0.0, 0.0, 1.0],
        [1.0; 4],
        None,
        [0.0; 3],
        0,
    );
    assert_eq!(first, second);
    assert!(
        first[0] > first[1] + 20,
        "ambient light color must tint the mesh"
    );
}

#[test]
fn mesh_metalness_tints_specular_reflection() {
    let mut comp = Composition::new("mesh-metal".into(), "Mesh metal".into(), 64, 64, 30, 1);
    comp.lights = vec![test_light(LightType::Point, [0.0, 0.0, 1000.0])];
    comp.active_camera.transform.position = Animatable::new_constant([0.0, 0.0, 1000.0]);
    let mut layer = Layer::new("mesh".into(), "Mesh".into(), LayerType::Null, 1);
    layer.material.ambient = 0.0;
    layer.material.diffuse = 0.0;
    layer.material.specular = 0.5;
    layer.material.specular_exponent = 8.0;
    let base = [0.8, 0.2, 0.1, 1.0];
    let dielectric = shade(
        &layer,
        &comp,
        [0.0; 3],
        [0.0, 0.0, 1.0],
        base,
        None,
        [0.0; 3],
        0,
    );
    layer.material.metalness = 1.0;
    let metallic = shade(
        &layer,
        &comp,
        [0.0; 3],
        [0.0, 0.0, 1.0],
        base,
        None,
        [0.0; 3],
        0,
    );
    assert!(metallic[0] > dielectric[0]);
    assert!(metallic[1] < dielectric[1]);
    assert!(metallic[2] < dielectric[2]);
}

#[test]
fn imported_pbr_material_controls_metallic_tint_and_roughness() {
    let mut comp = Composition::new("gltf-pbr".into(), "glTF PBR".into(), 64, 64, 30, 1);
    comp.lights = vec![test_light(LightType::Point, [400.0, 0.0, 1000.0])];
    comp.active_camera.transform.position = Animatable::new_constant([0.0, 0.0, 1000.0]);
    let mut layer = Layer::new("mesh".into(), "Mesh".into(), LayerType::Null, 1);
    layer.material.ambient = 0.0;
    layer.material.diffuse = 0.0;
    let base = [0.8, 0.2, 0.1, 1.0];
    let imported = crate::core::obj_loader::PbrMaterial3D {
        metallic: 0.0,
        roughness: 0.5,
        ..Default::default()
    };
    let dielectric = shade(
        &layer,
        &comp,
        [0.0; 3],
        [0.0, 0.0, 1.0],
        base,
        Some(&imported),
        [0.0; 3],
        0,
    );

    let smooth_metal = crate::core::obj_loader::PbrMaterial3D {
        metallic: 1.0,
        roughness: 0.5,
        ..Default::default()
    };
    let smooth_metal = shade(
        &layer,
        &comp,
        [0.0; 3],
        [0.0, 0.0, 1.0],
        base,
        Some(&smooth_metal),
        [0.0; 3],
        0,
    );
    let rough_metal = crate::core::obj_loader::PbrMaterial3D {
        metallic: 1.0,
        roughness: 1.0,
        ..Default::default()
    };
    let rough_metal = shade(
        &layer,
        &comp,
        [0.0; 3],
        [0.0, 0.0, 1.0],
        base,
        Some(&rough_metal),
        [0.0; 3],
        0,
    );

    assert!(smooth_metal[0] > smooth_metal[1]);
    assert!(rough_metal[0] > smooth_metal[0]);
    assert!(dielectric[0] < smooth_metal[0]);
}
