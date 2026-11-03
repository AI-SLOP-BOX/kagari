//! Generate Kagari Official Japanese Product Showcase Promo Video Project
use kagari_vfx::core::ae_effects_pack::camera_shake;
use kagari_vfx::core::keyframe::{InterpolationType, Keyframe};
use kagari_vfx::core::particle_system::ParticleEmitter;
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::timeline::*;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let width = 1920;
    let height = 1080;
    let fps = 30;
    let total_frames = 300; // 10 seconds promo

    let mut comp = Composition::new(
        "kagari_promo_comp".to_string(),
        "Kagari Official Product Showcase".to_string(),
        width,
        height,
        fps,
        total_frames,
    );

    // Color theme
    let bg_color = [0.03, 0.03, 0.05, 1.0];
    let cyan = [0.0, 0.95, 1.0, 1.0];
    let orange = [1.0, 0.45, 0.05, 1.0];
    let yellow = [1.0, 0.9, 0.1, 1.0];
    let white = [0.98, 0.98, 1.0, 1.0];

    // 1. Deep Space Backdrop with Camera Shake on Kicks
    let mut bg = Layer::new(
        "bg_layer".to_string(),
        "Studio Background".to_string(),
        LayerType::Solid { color: bg_color },
        total_frames,
    );
    bg.locked = true;

    // Apply Camera Shake & Bass Kick scale pops to background
    let mut bg_pos = Vec::new();
    let mut bg_scale = Vec::new();
    let beat_interval = 15; // 0.5s beat kicks at 30fps
    for f in (0..total_frames).step_by(beat_interval) {
        let t = f as f32 / fps as f32;
        let shake = camera_shake(t, 24.0, 10.0, 77);
        bg_pos.push(Keyframe::new(
            f,
            [960.0 + shake[0], 540.0 + shake[1]],
            InterpolationType::Linear,
        ));
        bg_scale.push(Keyframe::new(f, [103.0, 103.0], InterpolationType::Linear));
        let settle = (f + 4).min(total_frames.saturating_sub(1));
        bg_pos.push(Keyframe::new(
            settle,
            [960.0, 540.0],
            InterpolationType::Linear,
        ));
        bg_scale.push(Keyframe::new(
            settle,
            [100.0, 100.0],
            InterpolationType::Linear,
        ));
    }
    bg_pos.sort_by_key(|k| k.frame);
    bg_pos.dedup_by_key(|k| k.frame);
    bg_scale.sort_by_key(|k| k.frame);
    bg_scale.dedup_by_key(|k| k.frame);
    bg.transform.position = Animatable::Animated(bg_pos);
    bg.transform.scale = Animatable::Animated(bg_scale);
    comp.layers.push(bg);

    // 2. High Density Particle Sparks (Neon & Amber)
    let mut emitter = ParticleEmitter::default();
    emitter.rate = 75.0;
    emitter.lifetime = 2.5;
    emitter.speed = 85.0;
    emitter.spread_degrees = 360.0;
    emitter.color_start = cyan;
    emitter.color_end = [orange[0], orange[1], orange[2], 0.0];
    emitter.size_start = 4.5;
    emitter.size_end = 0.5;
    let mut particle_layer = Layer::new(
        "particles".to_string(),
        "Ambient Core Dust".to_string(),
        LayerType::Particle { emitter },
        total_frames,
    );
    particle_layer.transform.position =
        Animatable::new_constant([width as f32 / 2.0, height as f32 / 2.0]);
    comp.layers.push(particle_layer);

    // 3. Audio Layer (promo_soundtrack.wav)
    let audio_layer = Layer::new(
        "soundtrack".to_string(),
        "Promo Beat Track".to_string(),
        LayerType::Audio {
            path: "./promo_soundtrack.wav".to_string(),
            volume: Animatable::new_constant(1.0),
        },
        total_frames,
    );
    comp.layers.push(audio_layer);

    // 4. UI Showcase 1: Home Studio Hub (Frames 0 - 100)
    let mut shot1 = Layer::new(
        "shot1_home".to_string(),
        "UI: Modern Dark Studio Hub".to_string(),
        LayerType::Image {
            path: "docs/screenshots/home.png".to_string(),
        },
        total_frames,
    );
    shot1.in_frame = 0;
    shot1.out_frame = 100;
    // Dynamic Slide In from Bottom + Subtle Rotation Floating
    shot1.transform.position = Animatable::Animated(vec![
        Keyframe::new(0, [960.0, 720.0], InterpolationType::Linear),
        Keyframe::new(18, [960.0, 560.0], InterpolationType::Linear),
        Keyframe::new(100, [960.0, 530.0], InterpolationType::Linear),
    ]);
    shot1.transform.scale = Animatable::Animated(vec![
        Keyframe::new(0, [75.0, 75.0], InterpolationType::Linear),
        Keyframe::new(18, [85.0, 85.0], InterpolationType::Linear),
        Keyframe::new(100, [90.0, 90.0], InterpolationType::Linear),
    ]);
    shot1.transform.rotation = Animatable::Animated(vec![
        Keyframe::new(0, -3.0, InterpolationType::Linear),
        Keyframe::new(50, 0.5, InterpolationType::Linear),
        Keyframe::new(100, -1.0, InterpolationType::Linear),
    ]);
    shot1.transform.opacity = Animatable::Animated(vec![
        Keyframe::new(0, 0.0, InterpolationType::Linear),
        Keyframe::new(12, 100.0, InterpolationType::Linear),
        Keyframe::new(88, 100.0, InterpolationType::Linear),
        Keyframe::new(100, 0.0, InterpolationType::Linear),
    ]);
    comp.layers.push(shot1);

    // Title 1 Overlay (Japanese: 日本語)
    let mut t1 = Layer::new(
        "title1".to_string(),
        "Title: 次世代映像制作エンジン".to_string(),
        LayerType::Text {
            text: "KAGARI // 次世代モーショングラフィックスエンジン".to_string(),
            font_size: 48,
            color: cyan,
            font_family: "Hiragino Sans".to_string(),
            tracking: 6.0,
            leading: 1.1,
            align: 1,
            stroke_color: [0.0, 0.0, 0.0, 0.9],
            stroke_width: 3.0,
            text_on_path: false,
        },
        total_frames,
    );
    t1.in_frame = 0;
    t1.out_frame = 95;
    t1.transform.position = Animatable::Animated(vec![
        Keyframe::new(0, [960.0, 180.0], InterpolationType::Linear),
        Keyframe::new(20, [960.0, 140.0], InterpolationType::Linear),
        Keyframe::new(95, [960.0, 135.0], InterpolationType::Linear),
    ]);
    t1.transform.scale = Animatable::Animated(vec![
        Keyframe::new(0, [130.0, 130.0], InterpolationType::Linear), // Kinetic pop in
        Keyframe::new(12, [100.0, 100.0], InterpolationType::Linear),
        Keyframe::new(95, [102.0, 102.0], InterpolationType::Linear),
    ]);
    t1.effects.push(Effect {
        id: "t1_glow".to_string(),
        name: "Cyan Glow".to_string(),
        effect_type: EffectType::Glow {
            intensity: Animatable::new_constant(1.6),
            radius: Animatable::new_constant(22.0),
            threshold: Animatable::new_constant(0.3),
            color: Animatable::new_constant(cyan),
        },
        enabled: true,
    });
    comp.layers.push(t1);

    // Subtitle 1
    let mut sub1 = Layer::new(
        "sub1".to_string(),
        "Sub: Rustネイティブ超高速パイプライン".to_string(),
        LayerType::Text {
            text: "Rustネイティブ設計・直感的なダークグラスモフィズムUI".to_string(),
            font_size: 28,
            color: white,
            font_family: "Hiragino Sans".to_string(),
            tracking: 4.0,
            leading: 1.1,
            align: 1,
            stroke_color: [0.0, 0.0, 0.0, 0.8],
            stroke_width: 2.0,
            text_on_path: false,
        },
        total_frames,
    );
    sub1.in_frame = 10;
    sub1.out_frame = 95;
    sub1.transform.position = Animatable::new_constant([960.0, 205.0]);
    comp.layers.push(sub1);

    // 5. UI Showcase 2: Node Graph & Effects Panel (Frames 95 - 200)
    let mut shot2 = Layer::new(
        "shot2_graph".to_string(),
        "UI: Hybrid Node Graph".to_string(),
        LayerType::Image {
            path: "docs/screenshots/graph.png".to_string(),
        },
        total_frames,
    );
    shot2.in_frame = 95;
    shot2.out_frame = 200;
    // Dynamic Slide In from Right + Zoom
    shot2.transform.position = Animatable::Animated(vec![
        Keyframe::new(95, [1180.0, 560.0], InterpolationType::Linear),
        Keyframe::new(115, [960.0, 550.0], InterpolationType::Linear),
        Keyframe::new(200, [940.0, 540.0], InterpolationType::Linear),
    ]);
    shot2.transform.scale = Animatable::Animated(vec![
        Keyframe::new(95, [96.0, 96.0], InterpolationType::Linear),
        Keyframe::new(115, [86.0, 86.0], InterpolationType::Linear),
        Keyframe::new(200, [92.0, 92.0], InterpolationType::Linear),
    ]);
    shot2.transform.rotation = Animatable::Animated(vec![
        Keyframe::new(95, 2.0, InterpolationType::Linear),
        Keyframe::new(150, -0.5, InterpolationType::Linear),
        Keyframe::new(200, 1.0, InterpolationType::Linear),
    ]);
    shot2.transform.opacity = Animatable::Animated(vec![
        Keyframe::new(95, 0.0, InterpolationType::Linear),
        Keyframe::new(108, 100.0, InterpolationType::Linear),
        Keyframe::new(188, 100.0, InterpolationType::Linear),
        Keyframe::new(200, 0.0, InterpolationType::Linear),
    ]);
    comp.layers.push(shot2);

    // Title 2 Overlay (Japanese: 日本語)
    let mut t2 = Layer::new(
        "title2".to_string(),
        "Title: ハイブリッド・ノードグラフ".to_string(),
        LayerType::Text {
            text: "タイムライン × ノードグラフ ハイブリッド設計".to_string(),
            font_size: 48,
            color: orange,
            font_family: "Hiragino Sans".to_string(),
            tracking: 6.0,
            leading: 1.1,
            align: 1,
            stroke_color: [0.0, 0.0, 0.0, 0.9],
            stroke_width: 3.0,
            text_on_path: false,
        },
        total_frames,
    );
    t2.in_frame = 95;
    t2.out_frame = 195;
    t2.transform.position = Animatable::Animated(vec![
        Keyframe::new(95, [960.0, 180.0], InterpolationType::Linear),
        Keyframe::new(115, [960.0, 140.0], InterpolationType::Linear),
        Keyframe::new(195, [960.0, 135.0], InterpolationType::Linear),
    ]);
    t2.transform.scale = Animatable::Animated(vec![
        Keyframe::new(95, [130.0, 130.0], InterpolationType::Linear),
        Keyframe::new(108, [100.0, 100.0], InterpolationType::Linear),
        Keyframe::new(195, [102.0, 102.0], InterpolationType::Linear),
    ]);
    t2.effects.push(Effect {
        id: "t2_glow".to_string(),
        name: "Amber Glow".to_string(),
        effect_type: EffectType::Glow {
            intensity: Animatable::new_constant(1.6),
            radius: Animatable::new_constant(22.0),
            threshold: Animatable::new_constant(0.3),
            color: Animatable::new_constant(orange),
        },
        enabled: true,
    });
    comp.layers.push(t2);

    // Subtitle 2
    let mut sub2 = Layer::new(
        "sub2".to_string(),
        "Sub: 複雑なエフェクト合成も1キータブで相互切替".to_string(),
        LayerType::Text {
            text: "After Effectsの階層構造とNukeの自由度を完全融合".to_string(),
            font_size: 28,
            color: yellow,
            font_family: "Hiragino Sans".to_string(),
            tracking: 4.0,
            leading: 1.1,
            align: 1,
            stroke_color: [0.0, 0.0, 0.0, 0.8],
            stroke_width: 2.0,
            text_on_path: false,
        },
        total_frames,
    );
    sub2.in_frame = 105;
    sub2.out_frame = 195;
    sub2.transform.position = Animatable::new_constant([960.0, 205.0]);
    comp.layers.push(sub2);

    // 6. UI Showcase 3: Viewport & Render Farm Speed (Frames 195 - 300)
    let mut shot3 = Layer::new(
        "shot3_viewport".to_string(),
        "UI: Cinema Color Science Viewport".to_string(),
        LayerType::Image {
            path: "docs/screenshots/viewport.png".to_string(),
        },
        total_frames,
    );
    shot3.in_frame = 195;
    shot3.out_frame = 300;
    // Dynamic Slide In from Left + Zoom
    shot3.transform.position = Animatable::Animated(vec![
        Keyframe::new(195, [740.0, 560.0], InterpolationType::Linear),
        Keyframe::new(215, [960.0, 550.0], InterpolationType::Linear),
        Keyframe::new(300, [960.0, 530.0], InterpolationType::Linear),
    ]);
    shot3.transform.scale = Animatable::Animated(vec![
        Keyframe::new(195, [78.0, 78.0], InterpolationType::Linear),
        Keyframe::new(215, [86.0, 86.0], InterpolationType::Linear),
        Keyframe::new(300, [92.0, 92.0], InterpolationType::Linear),
    ]);
    shot3.transform.rotation = Animatable::Animated(vec![
        Keyframe::new(195, -2.0, InterpolationType::Linear),
        Keyframe::new(250, 0.8, InterpolationType::Linear),
        Keyframe::new(300, -0.5, InterpolationType::Linear),
    ]);
    shot3.transform.opacity = Animatable::Animated(vec![
        Keyframe::new(195, 0.0, InterpolationType::Linear),
        Keyframe::new(208, 100.0, InterpolationType::Linear),
        Keyframe::new(288, 100.0, InterpolationType::Linear),
        Keyframe::new(300, 0.0, InterpolationType::Linear),
    ]);
    comp.layers.push(shot3);

    // Title 3 Overlay (Japanese: 日本語)
    let mut t3 = Layer::new(
        "title3".to_string(),
        "Title: MFR並列爆速レンダリング".to_string(),
        LayerType::Text {
            text: "マルチフレーム並列レンダリング (MFR) ＆ OCIO".to_string(),
            font_size: 48,
            color: cyan,
            font_family: "Hiragino Sans".to_string(),
            tracking: 6.0,
            leading: 1.1,
            align: 1,
            stroke_color: [0.0, 0.0, 0.0, 0.9],
            stroke_width: 3.0,
            text_on_path: false,
        },
        total_frames,
    );
    t3.in_frame = 195;
    t3.out_frame = 300;
    t3.transform.position = Animatable::Animated(vec![
        Keyframe::new(195, [960.0, 180.0], InterpolationType::Linear),
        Keyframe::new(215, [960.0, 140.0], InterpolationType::Linear),
        Keyframe::new(300, [960.0, 135.0], InterpolationType::Linear),
    ]);
    t3.transform.scale = Animatable::Animated(vec![
        Keyframe::new(195, [130.0, 130.0], InterpolationType::Linear),
        Keyframe::new(208, [100.0, 100.0], InterpolationType::Linear),
        Keyframe::new(300, [102.0, 102.0], InterpolationType::Linear),
    ]);
    t3.effects.push(Effect {
        id: "t3_glow".to_string(),
        name: "Cyan Glow".to_string(),
        effect_type: EffectType::Glow {
            intensity: Animatable::new_constant(1.6),
            radius: Animatable::new_constant(22.0),
            threshold: Animatable::new_constant(0.3),
            color: Animatable::new_constant(cyan),
        },
        enabled: true,
    });
    comp.layers.push(t3);

    // Subtitle 3
    let mut sub3 = Layer::new(
        "sub3".to_string(),
        "Sub: 全CPUコアをフル活用・業界標準ACES/OCIO対応".to_string(),
        LayerType::Text {
            text: "ヘッドレスCLIとGUIの完全連動・映画規格のシネマ仕上げ".to_string(),
            font_size: 28,
            color: white,
            font_family: "Hiragino Sans".to_string(),
            tracking: 4.0,
            leading: 1.1,
            align: 1,
            stroke_color: [0.0, 0.0, 0.0, 0.8],
            stroke_width: 2.0,
            text_on_path: false,
        },
        total_frames,
    );
    sub3.in_frame = 205;
    sub3.out_frame = 300;
    sub3.transform.position = Animatable::new_constant([960.0, 205.0]);
    comp.layers.push(sub3);

    // 7. Master Adjustment: 35mm Film Grain & Intense RGB Glitch Spikes
    let mut master_adj = Layer::new(
        "master_finishing".to_string(),
        "Cinematic 35mm Lens Master".to_string(),
        LayerType::AdjustmentLayer,
        total_frames,
    );
    master_adj.is_adjustment_layer = true;
    master_adj.effects.push(Effect {
        id: "cinema_grain".to_string(),
        name: "Film Grain".to_string(),
        effect_type: EffectType::FilmGrain {
            intensity: Animatable::new_constant(0.13),
            grain_size: 1.5,
            color_film: true,
        },
        enabled: true,
    });
    master_adj.effects.push(Effect {
        id: "cinema_chroma".to_string(),
        name: "Chromatic Split Transitions".to_string(),
        effect_type: EffectType::ChromaticAberration {
            shift_r: Animatable::Animated(vec![
                Keyframe::new(0, 2.0, InterpolationType::Linear),
                Keyframe::new(94, 2.0, InterpolationType::Linear),
                Keyframe::new(95, 24.0, InterpolationType::Linear), // Violent Scene 1 -> 2 Glitch
                Keyframe::new(101, 2.0, InterpolationType::Linear),
                Keyframe::new(194, 2.0, InterpolationType::Linear),
                Keyframe::new(195, 24.0, InterpolationType::Linear), // Violent Scene 2 -> 3 Glitch
                Keyframe::new(201, 2.0, InterpolationType::Linear),
                Keyframe::new(300, 2.0, InterpolationType::Linear),
            ]),
            shift_b: Animatable::Animated(vec![
                Keyframe::new(0, -2.0, InterpolationType::Linear),
                Keyframe::new(94, -2.0, InterpolationType::Linear),
                Keyframe::new(95, -24.0, InterpolationType::Linear),
                Keyframe::new(101, -2.0, InterpolationType::Linear),
                Keyframe::new(194, -2.0, InterpolationType::Linear),
                Keyframe::new(195, -24.0, InterpolationType::Linear),
                Keyframe::new(201, -2.0, InterpolationType::Linear),
                Keyframe::new(300, -2.0, InterpolationType::Linear),
            ]),
            edge_falloff: Animatable::new_constant(1.0),
            iris_linked: true,
        },
        enabled: true,
    });
    master_adj.effects.push(Effect {
        id: "cinema_vignette".to_string(),
        name: "Anamorphic Vignette".to_string(),
        effect_type: EffectType::Vignette {
            intensity: Animatable::new_constant(40.0),
            roundness: Animatable::new_constant(0.0),
            feather: Animatable::new_constant(60.0),
            color: Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
        },
        enabled: true,
    });
    comp.layers.push(master_adj);

    let mut project = Project::default();
    project.compositions.push(comp);
    project.active_composition_idx = 0;

    let json = serde_json::to_string_pretty(&project)?;
    fs::write("kagari_product_promo.json", json)?;
    println!("✅ Generated Japanese Dynamic Kagari Promo JSON successfully!");

    Ok(())
}
