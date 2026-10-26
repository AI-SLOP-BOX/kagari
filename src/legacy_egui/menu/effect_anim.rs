use super::effect_factory::apply_effect_by_name;
use eframe::egui;

pub fn draw_effect_menu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    ui.menu_button("Expression Controls", |ui| {
        for name in &[
            "Slider Control",
            "Angle Control",
            "Point Control",
            "Color Control",
        ] {
            if ui.button(*name).clicked() {
                apply_effect_by_name(app, name);
                ui.close_menu();
            }
        }
    });
    ui.menu_button("Blur & Sharpen", |ui| {
        for name in &["Gaussian Blur", "Directional Blur", "Radial Blur", "Sharpen"] {
            if ui.button(*name).clicked() {
                apply_effect_by_name(app, name);
                ui.close_menu();
            }
        }
        if ui.button("Camera Lens Blur").on_hover_text("Optical camera aperture defocus with iris polygonal blade shapes and highlight gain").clicked() {
            apply_effect_by_name(app, "Camera Lens Blur");
            ui.close_menu();
        }
        if ui
            .button("Temporal Denoise")
            .on_hover_text("Multi-frame temporal motion-compensated noise reduction")
            .clicked()
        {
            apply_effect_by_name(app, "Temporal Denoise");
            ui.close_menu();
        }
        if ui
            .button("Bilateral Filter")
            .on_hover_text("Edge-preserving spatial-chromatic noise reduction")
            .clicked()
        {
            apply_effect_by_name(app, "Bilateral Filter");
            ui.close_menu();
        }
    });
    ui.menu_button("Color Correction", |ui| {
        for name in &["Color Tint", "Levels", "Hue/Saturation", "Vibrance"] {
            if ui.button(*name).clicked() {
                apply_effect_by_name(app, name);
                ui.close_menu();
            }
        }
    });
    ui.menu_button("Channel", |ui| {
        if ui
            .button("Set Matte")
            .on_hover_text("Replace or composite alpha channel with another layer")
            .clicked()
        {
            apply_effect_by_name(app, "Set Matte");
            ui.close_menu();
        }
        if ui.button("Shift Channels").clicked() {
            apply_effect_by_name(app, "Shift Channels");
            ui.close_menu();
        }
        if ui
            .button("Channel Combiner")
            .on_hover_text("Extract and remap color and alpha channels")
            .clicked()
        {
            apply_effect_by_name(app, "Channel Combiner");
            ui.close_menu();
        }
    });
    ui.menu_button("Keying", |ui| {
        if ui
            .button("Linear Color Key")
            .on_hover_text("Key out specific color ranges in RGB, Hue, or Chroma space")
            .clicked()
        {
            apply_effect_by_name(app, "Linear Color Key");
            ui.close_menu();
        }
    });
    ui.menu_button("Time", |ui| {
        if ui
            .button("Echo")
            .on_hover_text("Temporal visual trail blending across multiple frames")
            .clicked()
        {
            apply_effect_by_name(app, "Echo");
            ui.close_menu();
        }
    });
    if crate::ui::mode::menu_visible(app.ui_mode, "OpenFX Plugins") {
        ui.menu_button("OpenFX Plugins", |ui| draw_ofx_submenu(app, ui));
    }
    ui.menu_button("Stylize", |ui| {
        for name in &[
            "Glow",
            "Vignette",
            "Film Grain",
            "Drop Shadow",
            "Find Edges",
        ] {
            if ui.button(*name).clicked() {
                apply_effect_by_name(app, name);
                ui.close_menu();
            }
        }
        if ui.button("Lens Flare (GPU)").clicked() {
            apply_effect_by_name(app, "Lens Flare");
            ui.close_menu();
        }
        if ui
            .button("Optical Flares (Cinematic)")
            .on_hover_text(
                "Multi-element physical lens flare with anamorphic streaks and iris ghosts",
            )
            .clicked()
        {
            apply_effect_by_name(app, "Optical Flares");
            ui.close_menu();
        }
        if ui
            .button("Strobe Light")
            .on_hover_text("Periodic strobe light flashing")
            .clicked()
        {
            apply_effect_by_name(app, "Strobe Light");
            ui.close_menu();
        }
        if ui
            .button("SSAO")
            .on_hover_text("Screen-Space Ambient Occlusion contact shadows")
            .clicked()
        {
            apply_effect_by_name(app, "SSAO");
            ui.close_menu();
        }
        if ui
            .button("Z-Depth Fog")
            .on_hover_text("Atmospheric depth haze and distance fog")
            .clicked()
        {
            apply_effect_by_name(app, "Z-Depth Fog");
            ui.close_menu();
        }
    });
    ui.menu_button("Distort", |ui| {
        for name in &["Twirl", "Bulge", "Mesh Warp", "Chromatic Aberration"] {
            if ui.button(*name).clicked() {
                apply_effect_by_name(app, name);
                ui.close_menu();
            }
        }
        if ui
            .button("Motion Tile")
            .on_hover_text("Seamlessly replicate layer content with mirror edges and phase")
            .clicked()
        {
            apply_effect_by_name(app, "Motion Tile");
            ui.close_menu();
        }
        if ui
            .button("CC Page Turn")
            .on_hover_text("3D cylindrical page peel & curl deformation")
            .clicked()
        {
            apply_effect_by_name(app, "CC Page Turn");
            ui.close_menu();
        }
        if ui
            .button("Transform")
            .on_hover_text("2D Affine transform in effect chain")
            .clicked()
        {
            apply_effect_by_name(app, "Transform");
            ui.close_menu();
        }
        if ui.button("Corner Pin").clicked() {
            apply_effect_by_name(app, "Corner Pin");
            ui.close_menu();
        }
        if ui
            .button("CC Kaleida")
            .on_hover_text("Multi-sided kaleidoscope reflection deformation")
            .clicked()
        {
            apply_effect_by_name(app, "CC Kaleida");
            ui.close_menu();
        }
        if ui
            .button("CC Pixel Polly")
            .on_hover_text("CC Pixel Polly shatter and disintegration")
            .clicked()
        {
            apply_effect_by_name(app, "CC Pixel Polly");
            ui.close_menu();
        }
    });
    ui.menu_button("Immersive Video (VR 360)", |ui| {
        if ui
            .button("VR Horizon")
            .on_hover_text("3-axis spherical orientation realignment (Pitch, Yaw, Roll)")
            .clicked()
        {
            apply_effect_by_name(app, "VR Horizon");
            ui.close_menu();
        }
        if ui
            .button("VR Spherical Blur")
            .on_hover_text(
                "Geodesic latitude-compensated Gaussian blur for 360 equirectangular footage",
            )
            .clicked()
        {
            apply_effect_by_name(app, "VR Spherical Blur");
            ui.close_menu();
        }
    });
    ui.menu_button("Generate", |ui| {
        if ui
            .button("Lightning")
            .on_hover_text("Procedural electric lightning arcs with glow")
            .clicked()
        {
            apply_effect_by_name(app, "Lightning");
            ui.close_menu();
        }
        if ui
            .button("Laser Beam")
            .on_hover_text("High-energy projectile laser beam")
            .clicked()
        {
            apply_effect_by_name(app, "Laser Beam");
            ui.close_menu();
        }
        if ui
            .button("Gradient Ramp")
            .on_hover_text("Linear or radial 2-color gradient ramp")
            .clicked()
        {
            apply_effect_by_name(app, "Gradient Ramp");
            ui.close_menu();
        }
        if ui
            .button("Checkerboard")
            .on_hover_text("2-color procedural checkerboard grid pattern")
            .clicked()
        {
            apply_effect_by_name(app, "Checkerboard");
            ui.close_menu();
        }
    });
}

fn draw_ofx_submenu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui
        .button("Scan Standard Paths...")
        .on_hover_text("Search /Library/OFX/Plugins for plugin bundles")
        .clicked()
    {
        let found = crate::core::openfx_bridge::discover_all_ofx_plugins();
        if found.is_empty() {
            app.toasts.info("No OpenFX plugins found in standard paths");
        } else {
            let mut detected = 0usize;
            let mut names: Vec<String> = Vec::new();
            for p in &found {
                if let crate::core::openfx_bridge::OfxProbeResult::ApiDetected {
                    plugin_version,
                    ..
                } = crate::core::openfx_bridge::probe_ofx_plugin(&p.binary_path)
                {
                    detected += 1;
                    if names.len() < 4 {
                        names.push(format!(
                            "{} v{}.{}",
                            p.name, plugin_version.0, plugin_version.1
                        ));
                    }
                }
            }
            if detected > 0 {
                app.toasts.info(format!(
                    "{detected}/{} OFX plugin(s) detected (rendering is not supported yet): {}{}",
                    found.len(),
                    names.join(", "),
                    if detected > 4 { "…" } else { "" }
                ));
            } else {
                app.toasts.info(format!(
                    "Found {} bundle(s), none expose OfxImageEffectAPI",
                    found.len()
                ));
            }
        }
        ui.close_menu();
    }
}

pub fn draw_animation_menu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    ui.menu_button("Keyframe Assistant", |ui| draw_keyframe_assistant(app, ui));
    if ui
        .button("The Smoother...")
        .on_hover_text("Reduce keyframe density with RDP curve simplification")
        .clicked()
    {
        app.show_the_smoother = true;
        ui.close_menu();
    }
    if ui
        .button("The Wiggler...")
        .on_hover_text("Bake procedural noise keyframes into layer properties")
        .clicked()
    {
        app.show_the_wiggler = true;
        ui.close_menu();
    }
    if ui
        .button("✏️ Motion Sketch...")
        .on_hover_text("Record real-time mouse dragging in viewport to position keyframes")
        .clicked()
    {
        app.show_motion_sketch = true;
        ui.close_menu();
    }
    if ui
        .button("⚛ Physics & Dynamics...")
        .on_hover_text("2D Rigid Body Collision Simulation & Keyframe Baker")
        .clicked()
    {
        app.show_physics = true;
        ui.close_menu();
    }
}

fn draw_keyframe_assistant(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui
        .button("Convert Audio to Keyframes")
        .on_hover_text("Extract RMS amplitude from audio layer into Slider Controls")
        .clicked()
    {
        let src = get_audio_source(app);
        if let Some(src) = src {
            let mut temp_proj = app.history.current().clone();
            match crate::core::audio_to_keyframes::convert_audio_to_keyframes(
                temp_proj.active_composition_mut(),
                &src,
            ) {
                Ok(name) => {
                    app.commit_project(temp_proj);
                    app.toasts
                        .info(format!("Created '{}' with Left/Right/Both channels", name));
                }
                Err(e) => app.toasts.error(e),
            }
        } else {
            app.toasts
                .error("Select an Audio or Video-with-Audio layer first");
        }
        ui.close_menu();
    }
    if ui
        .button("Convert Multi-Band Audio (Bass/Mid/Treble)")
        .on_hover_text("Extract frequency-separated amplitude into Sliders")
        .clicked()
    {
        let src = get_audio_source(app);
        if let Some(src) = src {
            let mut temp_proj = app.history.current().clone();
            match crate::core::audio_to_keyframes::convert_multiband_audio_to_keyframes(
                temp_proj.active_composition_mut(),
                &src,
                None,
            ) {
                Ok(name) => {
                    app.commit_project(temp_proj);
                    app.toasts
                        .info(format!("Created '{}' with Master/Bass/Mid/Treble", name));
                }
                Err(e) => app.toasts.error(e),
            }
        } else {
            app.toasts
                .error("Select an Audio or Video-with-Audio layer first");
        }
        ui.close_menu();
    }
    ui.separator();
    if ui
        .add(egui::Button::new("Easy Ease").shortcut_text("F9"))
        .clicked()
    {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) {
                    l.easy_ease_transform();
                }
            });
            app.toasts.info("Easy Ease applied to transform keyframes");
        }
        ui.close_menu();
    }
    if ui.button("Sequence Layers...").clicked() {
        app.show_sequence_layers = true;
        ui.close_menu();
    }
}

fn get_audio_source(app: &crate::KagariApp) -> Option<String> {
    let comp = app.history.current().active_composition();
    app.selection
        .selected_layer_idx
        .and_then(|idx| comp.layers.get(idx))
        .and_then(|l| match &l.layer_type {
            crate::core::timeline::LayerType::Audio { path, .. } => Some(path.clone()),
            crate::core::timeline::LayerType::Video {
                audio_wav: Some(w), ..
            } => Some(w.clone()),
            _ => None,
        })
}

pub fn draw_view_menu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("UI Mode")
                .small()
                .color(egui::Color32::from_rgb(140, 140, 148)),
        );
        let mode = app.ui_mode;
        if ui
            .selectable_label(mode.is_beginner(), "初心者")
            .on_hover_text("Hide advanced panels and menus")
            .clicked()
        {
            app.ui_mode = crate::ui::mode::UiMode::Beginner;
            crate::ui::mode::save_mode(app.ui_mode);
            app.toasts
                .info("初心者モード: 高度なパネルを非表示にしました");
        }
        if ui
            .selectable_label(mode.is_advanced(), "上級者")
            .on_hover_text("Show every panel, menu and tool")
            .clicked()
        {
            app.ui_mode = crate::ui::mode::UiMode::Advanced;
            crate::ui::mode::save_mode(app.ui_mode);
            app.toasts.info("上級者モード: 全機能を表示中");
        }
    });
    ui.separator();
    ui.checkbox(&mut app.show_grid, "Show Grid");
    ui.checkbox(&mut app.show_guides, "Show Safe Zones");
    ui.checkbox(&mut app.show_handles, "Show Handles");
    if ui.button("Analyze / Quality Check…").clicked() {
        app.show_quality_check_panel = true;
        ui.close_menu();
    }
    if ui.button("Automation Bindings…").clicked() {
        app.show_automation_panel = true;
        ui.close_menu();
    }
    ui.separator();
    let mut gpu_fx = crate::core::compute_pipeline::gpu_effects_enabled();
    if ui
        .checkbox(&mut gpu_fx, "GPU Compute Effects (beta)")
        .changed()
    {
        crate::core::compute_pipeline::set_gpu_effects_enabled(gpu_fx);
        app.modify_project(|project| project.use_gpu_compute = gpu_fx);
        if gpu_fx {
            match crate::core::compute_pipeline::global() {
                Some(ctx) => app
                    .toasts
                    .info(format!("GPU compute: {}", ctx.backend_label())),
                None => {
                    app.toasts.error("No GPU adapter — staying on CPU");
                    crate::core::compute_pipeline::set_gpu_effects_enabled(false);
                    app.modify_project(|project| project.use_gpu_compute = false);
                }
            }
        } else {
            app.toasts
                .info("GPU compute off — deterministic CPU rendering");
        }
    }
    if ui.button("Reset Timeline Zoom (100%)").clicked() {
        app.timeline_zoom = 1.0;
        ui.close_menu();
    }
    if ui.button("Purge All RAM Cache").clicked() {
        app.frame_cache.invalidate_all();
        crate::core::frame_cache::bump_version();
        ui.close_menu();
    }
    ui.menu_button("Workspaces", |ui| {
        if ui.button("Standard").clicked() {
            app.ui_tabs.right_panel = crate::app_state::RightPanel::Effects;
            app.show_graph_editor = false;
            ui.close_menu();
        }
        if ui.button("Motion Graphics").clicked() {
            app.ui_tabs.right_panel = crate::app_state::RightPanel::Effects;
            app.show_graph_editor = true;
            ui.close_menu();
        }
        if ui.button("VFX & Color").clicked() {
            app.ui_tabs.right_panel = crate::app_state::RightPanel::Properties;
            app.show_graph_editor = false;
            ui.close_menu();
        }
        if ui.button("Audio Editing").clicked() {
            app.ui_tabs.right_panel = crate::app_state::RightPanel::Audio;
            app.show_graph_editor = false;
            ui.close_menu();
        }
    });
}

pub fn draw_help_menu(app: &mut crate::KagariApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    if ui
        .add(egui::Button::new("⚙ Preferences…").shortcut_text(
            crate::ui::shortcuts::format_shortcut(",", true, false, false),
        ))
        .clicked()
    {
        app.show_preferences = true;
        ui.close_menu();
    }
    if ui.button("Show Welcome Screen").clicked() {
        app.show_welcome = true;
        ui.close_menu();
    }
    if ui.button("Keyboard Shortcuts Reference...").clicked() {
        let help_id = egui::Id::new("show_shortcuts_modal");
        ctx.data_mut(|d| d.insert_temp(help_id, true));
        ui.close_menu();
    }
    if ui.button("ℹ️ About Kagari Studio...").clicked() {
        let about_id = egui::Id::new("show_about_modal");
        ctx.data_mut(|d| d.insert_temp(about_id, true));
        ui.close_menu();
    }
}
