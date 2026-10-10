use crate::core::property::Animatable;
use crate::core::the_wiggler::{WiggleDimension, WiggleNoiseType};
use crate::ui::kui::{self, ButtonKind};
use crate::KagariApp;
use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WiggleTarget {
    Position,
    Rotation,
    Scale,
    Opacity,
}

pub fn draw_the_wiggler_panel(app: &mut KagariApp, ui: &mut egui::Ui) {
    kui::section(ui, "The Wiggler");
    kui::secondary(
        ui,
        "Inject procedural noise keyframes into layer transforms",
    );

    let target_id = egui::Id::new("ae_wiggler_target_prop");
    let mut target_prop: WiggleTarget = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(target_id, || WiggleTarget::Position));

    let noise_type_id = egui::Id::new("ae_wiggler_noise_type");
    let mut noise_type: WiggleNoiseType = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(noise_type_id, || WiggleNoiseType::Smooth));

    let freq_id = egui::Id::new("ae_wiggler_freq");
    let mut freq: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(freq_id, || 4.0));

    let mag_id = egui::Id::new("ae_wiggler_mag");
    let mut mag: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(mag_id, || 30.0));

    let grid = kui::PropertyGrid::new(ui, &["Property", "Noise Type", "Frequency", "Magnitude"]);
    if grid.row(ui, "Property", |ui| {
        kui::combo(
            ui,
            "wiggler_prop_combo",
            &mut target_prop,
            &[
                (WiggleTarget::Position, "Position"),
                (WiggleTarget::Rotation, "Rotation"),
                (WiggleTarget::Scale, "Scale"),
                (WiggleTarget::Opacity, "Opacity"),
            ],
        )
    }) {
        ui.ctx().data_mut(|d| d.insert_temp(target_id, target_prop));
    }
    if grid.row(ui, "Noise Type", |ui| {
        kui::combo(
            ui,
            noise_type_id,
            &mut noise_type,
            &[
                (WiggleNoiseType::Smooth, "Smooth"),
                (WiggleNoiseType::Jagged, "Jagged"),
            ],
        )
    }) {
        ui.ctx()
            .data_mut(|d| d.insert_temp(noise_type_id, noise_type));
    }
    if grid
        .row(ui, "Frequency", |ui| {
            kui::slider(ui, &mut freq, 0.5..=30.0, " wiggles/s")
        })
        .changed()
    {
        ui.ctx().data_mut(|d| d.insert_temp(freq_id, freq));
    }
    if grid
        .row(ui, "Magnitude", |ui| {
            kui::slider(ui, &mut mag, 1.0..=200.0, "")
        })
        .changed()
    {
        ui.ctx().data_mut(|d| d.insert_temp(mag_id, mag));
    }

    kui::separator(ui);

    if kui::button_enabled(
        ui,
        "Apply Wiggle Keyframes",
        ButtonKind::Accent,
        app.selection.selected_layer_idx.is_some(),
    )
    .on_hover_text("Bake procedural noise into layer keyframes")
    .clicked()
    {
        let Some(layer_idx) = app.selection.selected_layer_idx else {
            app.toasts
                .error(crate::ui::localization::tr("Select a layer first"));
            return;
        };

        let mut temp_proj = app.history.current().clone();
        let comp = temp_proj.active_composition_mut();
        let fps = comp.fps;
        let Some(layer) = comp.layers.get_mut(layer_idx) else {
            return;
        };

        let start_f = layer.in_frame;
        let end_f = layer.out_frame;
        let seed = layer_idx as u32 * 100 + 7;
        let settings = crate::core::the_wiggler::WiggleSettings {
            start_frame: start_f,
            end_frame: end_f,
            fps,
            frequency: freq,
            magnitude: mag,
            noise_type,
            seed,
        };

        match target_prop {
            WiggleTarget::Position => {
                let pos_anim = layer.transform.position.clone();
                let kfs = crate::core::the_wiggler::generate_wiggle_vec2_additive_with_settings(
                    |f| pos_anim.evaluate(f),
                    settings,
                    WiggleDimension::AllIndependent,
                );
                layer.transform.position = Animatable::Animated(kfs);
            }
            WiggleTarget::Rotation => {
                let rot_anim = layer.transform.rotation.clone();
                let kfs = crate::core::the_wiggler::generate_wiggle_scalar_additive_with_settings(
                    |f| rot_anim.evaluate(f),
                    settings,
                );
                layer.transform.rotation = Animatable::Animated(kfs);
            }
            WiggleTarget::Scale => {
                let scale_anim = layer.transform.scale.clone();
                let kfs = crate::core::the_wiggler::generate_wiggle_vec2_additive_with_settings(
                    |f| scale_anim.evaluate(f),
                    settings,
                    WiggleDimension::AllSame,
                );
                layer.transform.scale = Animatable::Animated(kfs);
            }
            WiggleTarget::Opacity => {
                let op_anim = layer.transform.opacity.clone();
                let kfs = crate::core::the_wiggler::generate_wiggle_scalar_additive_with_settings(
                    |f| op_anim.evaluate(f),
                    settings,
                );
                layer.transform.opacity = Animatable::Animated(kfs);
            }
        }

        app.commit_project(temp_proj);
        app.toasts.info(crate::ui::localization::tr(
            "The Wiggler: Baked procedural wiggle keyframes",
        ));
    }
}
