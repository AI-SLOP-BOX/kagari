use crate::core::editor::EditorSession;
use crate::KagariApp;
use eframe::egui;

pub(crate) fn apply_effect(app: &mut KagariApp, name: &str) {
    let Some(idx) = app.selection.selected_layer_idx else {
        app.toasts
            .info("レイヤーを選択するとエフェクトを適用できます");
        return;
    };
    let effect_start = app
        .history
        .current()
        .active_composition()
        .layers
        .get(idx)
        .map_or(0, |layer| layer.effects.len());
    let look = |index: usize, suffix: &str, effect_name: &str, effect_type| {
        crate::core::timeline::Effect {
            id: format!("look_{effect_start}_{index}_{suffix}"),
            name: effect_name.to_owned(),
            effect_type,
            enabled: true,
        }
    };
    let stack = match name {
        "アナログフィルム" => Some(vec![
            look(
                0,
                "grain",
                "フィルムグレイン",
                crate::core::timeline::EffectType::FilmGrain {
                    intensity: crate::core::property::Animatable::new_constant(12.0),
                    grain_size: 1.4,
                    color_film: true,
                },
            ),
            look(
                1,
                "vignette",
                "レンズ周辺減光",
                crate::core::timeline::EffectType::Vignette {
                    intensity: crate::core::property::Animatable::new_constant(16.0),
                    roundness: crate::core::property::Animatable::new_constant(0.9),
                    feather: crate::core::property::Animatable::new_constant(72.0),
                    color: crate::core::property::Animatable::new_constant([0.0, 0.0, 0.0, 1.0]),
                },
            ),
            look(
                2,
                "ca",
                "レンズ色収差",
                crate::core::timeline::EffectType::ChromaticAberration {
                    shift_r: crate::core::property::Animatable::new_constant(0.45),
                    shift_b: crate::core::property::Animatable::new_constant(0.45),
                    edge_falloff: crate::core::property::Animatable::new_constant(0.85),
                    iris_linked: true,
                },
            ),
        ]),
        "ソフトブルーム" => Some(vec![look(
            0,
            "glow",
            "ソフトブルーム",
            crate::core::timeline::EffectType::Glow {
                threshold: crate::core::property::Animatable::new_constant(0.82),
                radius: crate::core::property::Animatable::new_constant(24.0),
                intensity: crate::core::property::Animatable::new_constant(24.0),
                color: crate::core::property::Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
            },
        )]),
        "プリズムディストーション" => Some(vec![
            look(
                0,
                "displace",
                "乱流変位",
                crate::core::timeline::EffectType::TurbulentDisplace {
                    amount: crate::core::property::Animatable::new_constant(6.0),
                    size: crate::core::property::Animatable::new_constant(110.0),
                    evolution: crate::core::property::Animatable::new_constant(0.0),
                    complexity: crate::core::property::Animatable::new_constant(1.0),
                },
            ),
            look(
                1,
                "ca",
                "色収差",
                crate::core::timeline::EffectType::ChromaticAberration {
                    shift_r: crate::core::property::Animatable::new_constant(1.1),
                    shift_b: crate::core::property::Animatable::new_constant(1.1),
                    edge_falloff: crate::core::property::Animatable::new_constant(0.6),
                    iris_linked: false,
                },
            ),
        ]),
        _ => None,
    };
    let effects = if let Some(stack) = stack {
        stack
    } else {
        let Some(create_fn) = crate::ui::effects_controls::get_all_effect_presets()
            .iter()
            .find(|preset| preset.name == name)
            .map(|preset| preset.create_fn)
        else {
            return;
        };
        vec![create_fn(effect_start)]
    };
    let mut session = EditorSession::new(&mut app.history, "Add Effect Library Effect");
    let comp = session.current_mut().active_composition_mut();
    if idx < comp.layers.len() {
        comp.layers[idx].effects.extend(effects);
        session.commit();
        app.toasts.info(format!("{} を適用しました", name));
    }
}

pub(crate) fn texture(ctx: &egui::Context, path: &str, key: &str) -> Option<egui::TextureId> {
    let id = egui::Id::new(("effects-workspace", key));
    if let Some(t) = ctx.data_mut(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t.id());
    }
    let img = crate::ui::embedded_assets::open_image(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path),
    )
    .ok()?
    .to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let handle = ctx.load_texture(
        key,
        egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
        egui::TextureOptions::LINEAR,
    );
    let out = handle.id();
    ctx.data_mut(|d| d.insert_temp(id, handle));
    Some(out)
}
