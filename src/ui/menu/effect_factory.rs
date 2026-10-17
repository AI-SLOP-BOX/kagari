use crate::core::property::Animatable;
use crate::core::timeline::{Effect, EffectType};

pub fn apply_effect_by_name(app: &mut crate::KagariApp, effect_name: &str) {
    let Some(idx) = app.selection.selected_layer_idx else {
        app.toasts.error("Select a layer before adding an effect.");
        return;
    };
    let effect_name = effect_name.to_string();
    let mut added = false;
    let mut unsupported = false;
    app.modify_project(|project| {
        let comp = project.active_composition_mut();
        if idx >= comp.layers.len() {
            return;
        }
        let layer = &mut comp.layers[idx];
        let len = layer.effects.len();
        let (cw, ch) = (comp.width as f32, comp.height as f32);
        let effect = build_effect(&effect_name, len, cw, ch, layer);
        match effect {
            Some(e) => {
                layer.effects.push(e);
                added = true;
            }
            None => {
                unsupported = true;
            }
        }
    });
    if added {
        crate::core::frame_cache::bump_version();
        app.toasts.info(format!("Added '{}' effect", effect_name));
    } else if unsupported {
        app.toasts.error(format!(
            "'{}' is not available in the effect menu yet.",
            effect_name
        ));
    } else {
        app.toasts
            .error("Could not add the effect because the selected layer is no longer available.");
    }
}

fn build_effect(
    name: &str,
    len: usize,
    cw: f32,
    ch: f32,
    layer: &crate::core::timeline::Layer,
) -> Option<Effect> {
    let e = match name {
        "Slider Control" => Effect {
            id: format!("slider_{len}"),
            name: "Slider Control".into(),
            effect_type: EffectType::SliderControl {
                value: Animatable::new_constant(50.0),
            },
            enabled: true,
        },
        "Angle Control" => Effect {
            id: format!("angle_{len}"),
            name: "Angle Control".into(),
            effect_type: EffectType::AngleControl {
                angle_degrees: Animatable::new_constant(0.0),
            },
            enabled: true,
        },
        "Point Control" => Effect {
            id: format!("point_{len}"),
            name: "Point Control".into(),
            effect_type: EffectType::PointControl {
                point: Animatable::new_constant([960.0, 540.0]),
            },
            enabled: true,
        },
        "Color Control" => Effect {
            id: format!("color_{len}"),
            name: "Color Control".into(),
            effect_type: EffectType::ColorControl {
                color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
            },
            enabled: true,
        },
        "Checkbox Control" => Effect {
            id: format!("checkbox_{len}"),
            name: "Checkbox Control".into(),
            effect_type: EffectType::CheckboxControl { checked: false },
            enabled: true,
        },
        "Dropdown Control" => Effect {
            id: format!("dropdown_{len}"),
            name: "Dropdown Control".into(),
            effect_type: EffectType::DropdownControl {
                value: 0,
                options: vec!["Option 1".into(), "Option 2".into(), "Option 3".into()],
            },
            enabled: true,
        },
        "3D Point Control" => Effect {
            id: format!("point3d_{len}"),
            name: "3D Point Control".into(),
            effect_type: EffectType::Point3DControl {
                point: Animatable::new_constant([0.0, 0.0, 0.0]),
            },
            enabled: true,
        },
        "Lens Flare" => Effect {
            id: format!("flare_{len}"),
            name: "Lens Flare".into(),
            effect_type: EffectType::LensFlare {
                enabled: Animatable::new_constant(1.0),
                position_x: Animatable::new_constant(0.5),
                position_y: Animatable::new_constant(0.35),
                intensity: Animatable::new_constant(1.0),
                threshold: Animatable::new_constant(0.8),
                color: Animatable::new_constant([1.0, 0.95, 0.9, 1.0]),
                link_to_light: None,
            },
            enabled: true,
        },
        "Gaussian Blur" => Effect {
            id: format!("blur_{len}"),
            name: "Gaussian Blur".into(),
            effect_type: EffectType::GaussianBlur {
                blur_radius: Animatable::new_constant(5.0),
            },
            enabled: true,
        },
        "Directional Blur" => Effect {
            id: format!("dirblur_{len}"),
            name: "Directional Blur".into(),
            effect_type: EffectType::DirectionalBlur {
                angle: Animatable::new_constant(0.0),
                length: Animatable::new_constant(10.0),
            },
            enabled: true,
        },
        "Radial Blur" => Effect {
            id: format!("radblur_{len}"),
            name: "Radial Blur".into(),
            effect_type: EffectType::RadialBlur {
                amount: Animatable::new_constant(10.0),
            },
            enabled: true,
        },
        "Sharpen" => Effect {
            id: format!("sharp_{len}"),
            name: "Sharpen".into(),
            effect_type: EffectType::Sharpen {
                amount: Animatable::new_constant(50.0),
            },
            enabled: true,
        },
        "Color Tint" => Effect {
            id: format!("tint_{len}"),
            name: "Color Tint".into(),
            effect_type: EffectType::ColorTint {
                color: Animatable::new_constant([1.0, 0.2, 0.4, 1.0]),
                intensity: Animatable::new_constant(1.0),
            },
            enabled: true,
        },
        "Levels" => Effect {
            id: format!("levels_{len}"),
            name: "Levels".into(),
            effect_type: EffectType::Levels {
                input_black: Animatable::new_constant(0.0),
                input_white: Animatable::new_constant(255.0),
                gamma: Animatable::new_constant(1.0),
                output_black: Animatable::new_constant(0.0),
                output_white: Animatable::new_constant(255.0),
            },
            enabled: true,
        },
        "Hue/Saturation" => Effect {
            id: format!("hs_{len}"),
            name: "Hue/Saturation".into(),
            effect_type: EffectType::HueSaturation {
                hue_shift: Animatable::new_constant(0.0),
                saturation: Animatable::new_constant(0.0),
                lightness: Animatable::new_constant(0.0),
            },
            enabled: true,
        },
        "Vibrance" => Effect {
            id: format!("vib_{len}"),
            name: "Vibrance".into(),
            effect_type: EffectType::Vibrance {
                amount: Animatable::new_constant(50.0),
            },
            enabled: true,
        },
        "Glow" => Effect {
            id: format!("glow_{len}"),
            name: "Glow".into(),
            effect_type: EffectType::Glow {
                threshold: Animatable::new_constant(60.0),
                radius: Animatable::new_constant(10.0),
                intensity: Animatable::new_constant(1.0),
                color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
            },
            enabled: true,
        },
        "Vignette" => Effect {
            id: format!("vig_{len}"),
            name: "Vignette".into(),
            effect_type: EffectType::Vignette {
                intensity: Animatable::new_constant(0.5),
                roundness: Animatable::new_constant(0.5),
                feather: Animatable::new_constant(0.5),
                color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
            },
            enabled: true,
        },
        "Film Grain" => Effect {
            id: format!("grain_{len}"),
            name: "Film Grain".into(),
            effect_type: EffectType::FilmGrain {
                intensity: Animatable::new_constant(0.1),
                grain_size: 2.0,
                color_film: false,
            },
            enabled: true,
        },
        "Drop Shadow" => Effect {
            id: format!("ds_{len}"),
            name: "Drop Shadow".into(),
            effect_type: EffectType::DropShadow {
                color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                opacity: Animatable::new_constant(75.0),
                direction: Animatable::new_constant(120.0),
                distance: Animatable::new_constant(5.0),
                softness: Animatable::new_constant(5.0),
            },
            enabled: true,
        },
        "Twirl" => Effect {
            id: format!("twirl_{len}"),
            name: "Twirl".into(),
            effect_type: EffectType::Twirl {
                angle: Animatable::new_constant(50.0),
                radius: Animatable::new_constant(100.0),
            },
            enabled: true,
        },
        "Bulge" => Effect {
            id: format!("bulge_{len}"),
            name: "Bulge".into(),
            effect_type: EffectType::Bulge {
                amount: Animatable::new_constant(50.0),
                radius: Animatable::new_constant(100.0),
            },
            enabled: true,
        },
        "Mesh Warp" => Effect {
            id: format!("meshwarp_{len}"),
            name: "Mesh Warp".into(),
            effect_type: EffectType::MeshWarp {
                top_left: Animatable::new_constant([0.0, 0.0]),
                top_right: Animatable::new_constant([1.0, 0.0]),
                bottom_left: Animatable::new_constant([0.0, 1.0]),
                bottom_right: Animatable::new_constant([1.0, 1.0]),
            },
            enabled: true,
        },
        "Corner Pin" => Effect {
            id: format!("cornerpin_{len}"),
            name: "Corner Pin".into(),
            effect_type: EffectType::CornerPin {
                top_left: Animatable::new_constant([0.0, 0.0]),
                top_right: Animatable::new_constant([cw, 0.0]),
                bottom_right: Animatable::new_constant([cw, ch]),
                bottom_left: Animatable::new_constant([0.0, ch]),
            },
            enabled: true,
        },
        "Chromatic Aberration" => Effect {
            id: format!("ca_{len}"),
            name: "Chromatic Aberration".into(),
            effect_type: EffectType::ChromaticAberration {
                shift_r: Animatable::new_constant(5.0),
                shift_b: Animatable::new_constant(-5.0),
                edge_falloff: Animatable::new_constant(0.5),
                iris_linked: true,
            },
            enabled: true,
        },
        "Bass & Treble" => Effect {
            id: format!("bass_treble_{len}"),
            name: "Bass & Treble".into(),
            effect_type: EffectType::BassTreble {
                bass_gain: Animatable::new_constant(0.0),
                treble_gain: Animatable::new_constant(0.0),
                crossover_freq: Animatable::new_constant(300.0),
            },
            enabled: true,
        },
        "Flanger" => Effect {
            id: format!("flanger_{len}"),
            name: "Flanger".into(),
            effect_type: EffectType::Flanger {
                max_delay_ms: Animatable::new_constant(5.0),
                lfo_rate: Animatable::new_constant(0.5),
                feedback: Animatable::new_constant(0.5),
                wet_dry: Animatable::new_constant(0.5),
            },
            enabled: true,
        },
        "Chorus" => Effect {
            id: format!("chorus_{len}"),
            name: "Chorus".into(),
            effect_type: EffectType::Chorus {
                delay_ms: Animatable::new_constant(15.0),
                depth_ms: Animatable::new_constant(5.0),
                rate_hz: Animatable::new_constant(1.0),
                voices: Animatable::new_constant(3.0),
                feedback: Animatable::new_constant(0.3),
            },
            enabled: true,
        },
        "Parametric EQ" => Effect {
            id: format!("peq_{len}"),
            name: "Parametric EQ".into(),
            effect_type: EffectType::ParametricEQ {
                freq_hz: Animatable::new_constant(1000.0),
                gain_db: Animatable::new_constant(0.0),
                q_factor: Animatable::new_constant(1.0),
            },
            enabled: true,
        },
        "Optical Flares" => Effect {
            id: format!("optflare_{len}"),
            name: "Optical Flares".into(),
            effect_type: EffectType::OpticalFlares {
                position: Animatable::new_constant([cw * 0.5, ch * 0.5]),
                brightness: Animatable::new_constant(1.0),
                scale: Animatable::new_constant(1.0),
            },
            enabled: true,
        },
        "Motion Tile" => Effect {
            id: format!("motiontile_{len}"),
            name: "Motion Tile".into(),
            effect_type: EffectType::MotionTile {
                tile_center: Animatable::new_constant([cw * 0.5, ch * 0.5]),
                tile_width: Animatable::new_constant(100.0),
                tile_height: Animatable::new_constant(100.0),
                output_width: Animatable::new_constant(100.0),
                output_height: Animatable::new_constant(100.0),
                mirror_edges: true,
                phase: Animatable::new_constant(0.0),
            },
            enabled: true,
        },
        "CC Page Turn" => Effect {
            id: format!("pageturn_{len}"),
            name: "CC Page Turn".into(),
            effect_type: EffectType::PageTurn {
                fold_position: Animatable::new_constant([cw, ch]),
                fold_radius: Animatable::new_constant(120.0),
                fold_direction_deg: Animatable::new_constant(-45.0),
                light_direction_deg: Animatable::new_constant(-45.0),
                back_opacity: Animatable::new_constant(100.0),
                back_color: Animatable::new_constant([0.92, 0.92, 0.94, 1.0]),
            },
            enabled: true,
        },
        "Set Matte" => Effect {
            id: format!("setmatte_{len}"),
            name: "Set Matte".into(),
            effect_type: EffectType::SetMatte {
                source_layer_idx: 0,
                source_channel: crate::core::set_matte::MatteSourceChannel::Alpha,
                invert_matte: false,
                composite_mode: crate::core::set_matte::MatteCompositeMode::Replace,
            },
            enabled: true,
        },
        "Shift Channels" => Effect {
            id: format!("shiftch_{len}"),
            name: "Shift Channels".into(),
            effect_type: EffectType::ShiftChannels {
                take_red: Animatable::new_constant(0.0),
                take_green: Animatable::new_constant(1.0),
                take_blue: Animatable::new_constant(2.0),
                take_alpha: Animatable::new_constant(3.0),
            },
            enabled: true,
        },
        "Echo" => Effect {
            id: format!("echo_{len}"),
            name: "Echo".into(),
            effect_type: EffectType::Echo {
                echo_time_seconds: Animatable::new_constant(-0.033),
                num_echoes: 3,
                starting_intensity: Animatable::new_constant(1.0),
                decay: Animatable::new_constant(0.5),
                operator: crate::core::echo_effect::EchoOperator::Add,
            },
            enabled: true,
        },
        "Find Edges" => Effect {
            id: format!("findedges_{len}"),
            name: "Find Edges".into(),
            effect_type: EffectType::FindEdges { invert: false },
            enabled: true,
        },
        "Transform" => Effect {
            id: format!("transform_{len}"),
            name: "Transform".into(),
            effect_type: EffectType::Transform {
                anchor_point: Animatable::new_constant([
                    layer.transform.anchor_point.evaluate(0)[0],
                    layer.transform.anchor_point.evaluate(0)[1],
                ]),
                position: Animatable::new_constant([
                    layer.transform.position.evaluate(0)[0],
                    layer.transform.position.evaluate(0)[1],
                ]),
                scale_width: Animatable::new_constant(100.0),
                scale_height: Animatable::new_constant(100.0),
                uniform_scale: true,
                skew_deg: Animatable::new_constant(0.0),
                skew_axis_deg: Animatable::new_constant(0.0),
                rotation_deg: Animatable::new_constant(0.0),
                opacity: Animatable::new_constant(100.0),
            },
            enabled: true,
        },
        "Camera Lens Blur" => Effect {
            id: format!("cameralensblur_{len}"),
            name: "Camera Lens Blur".into(),
            effect_type: EffectType::CameraLensBlur {
                blur_radius: Animatable::new_constant(15.0),
                iris_blades: 6,
                iris_rotation_deg: Animatable::new_constant(0.0),
                iris_roundness: Animatable::new_constant(0.0),
                highlight_gain: Animatable::new_constant(1.5),
                highlight_threshold: Animatable::new_constant(0.8),
            },
            enabled: true,
        },
        "Linear Color Key" => Effect {
            id: format!("linearcolorkey_{len}"),
            name: "Linear Color Key".into(),
            effect_type: EffectType::LinearColorKey {
                key_color: Animatable::new_constant([0.0, 1.0, 0.0]),
                match_mode: crate::core::linear_color_key::ColorMatchMode::UsingRGB,
                tolerance: Animatable::new_constant(15.0),
                softness: Animatable::new_constant(10.0),
            },
            enabled: true,
        },
        "Channel Combiner" => Effect {
            id: format!("channelcombiner_{len}"),
            name: "Channel Combiner".into(),
            effect_type: EffectType::ChannelCombiner {
                from_channel: crate::core::channel_combiner::ChannelCombinerFrom::Luminance,
                to_target: crate::core::channel_combiner::ChannelCombinerTo::Alpha,
                invert: false,
            },
            enabled: true,
        },
        "Lightning" => Effect {
            id: format!("lightning_{len}"),
            name: "Lightning".into(),
            effect_type: EffectType::LightningArc {
                start_x: Animatable::new_constant(0.2),
                start_y: Animatable::new_constant(0.2),
                end_x: Animatable::new_constant(0.8),
                end_y: Animatable::new_constant(0.8),
                seed: Animatable::new_constant(12345.0),
                glow: Animatable::new_constant(1.0),
            },
            enabled: true,
        },
        "Laser Beam" => Effect {
            id: format!("laser_{len}"),
            name: "Laser Beam".into(),
            effect_type: EffectType::LaserBeam {
                start_x: Animatable::new_constant(0.1),
                start_y: Animatable::new_constant(0.5),
                end_x: Animatable::new_constant(0.9),
                end_y: Animatable::new_constant(0.5),
                progress: Animatable::new_constant(0.5),
                length: Animatable::new_constant(40.0),
                starting_thickness: Animatable::new_constant(12.0),
                ending_thickness: Animatable::new_constant(4.0),
                core_color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
                glow_color: Animatable::new_constant([1.0, 0.2, 0.1, 0.8]),
            },
            enabled: true,
        },
        "VR Horizon" => Effect {
            id: format!("vrhorizon_{len}"),
            name: "VR Horizon".into(),
            effect_type: EffectType::VRHorizon {
                pitch_deg: Animatable::new_constant(0.0),
                yaw_deg: Animatable::new_constant(0.0),
                roll_deg: Animatable::new_constant(0.0),
            },
            enabled: true,
        },
        "VR Spherical Blur" => Effect {
            id: format!("vrblur_{len}"),
            name: "VR Spherical Blur".into(),
            effect_type: EffectType::VRSphericalBlur {
                radius: Animatable::new_constant(5.0),
            },
            enabled: true,
        },
        _ => return None,
    };
    Some(e)
}
