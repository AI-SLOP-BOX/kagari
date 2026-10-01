use eframe::egui;
use super::effect_factory::apply_effect_by_name;

pub fn draw_effect_menu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    ui.menu_button("Expression Controls", |ui| {
        for name in &["Slider Control", "Angle Control", "Point Control", "Color Control"] {
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
        if ui.button("Set Matte").on_hover_text("Replace or composite alpha channel with another layer").clicked() {
            apply_effect_by_name(app, "Set Matte");
            ui.close_menu();
        }
        if ui.button("Shift Channels").clicked() { apply_effect_by_name(app, "Shift Channels"); ui.close_menu(); }
        if ui.button("Channel Combiner").on_hover_text("Extract and remap color and alpha channels").clicked() {
            apply_effect_by_name(app, "Channel Combiner");
            ui.close_menu();
        }
    });
    ui.menu_button("Keying", |ui| {
        if ui.button("Linear Color Key").on_hover_text("Key out specific color ranges in RGB, Hue, or Chroma space").clicked() {
            apply_effect_by_name(app, "Linear Color Key");
            ui.close_menu();
        }
    });
    ui.menu_button("Time", |ui| {
        if ui.button("Echo").on_hover_text("Temporal visual trail blending across multiple frames").clicked() {
            apply_effect_by_name(app, "Echo");
            ui.close_menu();
        }
    });
    if crate::ui::mode::menu_visible(app.ui_mode, "OpenFX Plugins") {
        ui.menu_button("OpenFX Plugins", |ui| draw_ofx_submenu(app, ui));
    }
    ui.menu_button("Stylize", |ui| {
        for name in &["Glow", "Vignette", "Film Grain", "Drop Shadow", "Find Edges"] {
            if ui.button(*name).clicked() { apply_effect_by_name(app, name); ui.close_menu(); }
        }
        if ui.button("Lens Flare (GPU)").clicked() { apply_effect_by_name(app, "Lens Flare"); ui.close_menu(); }
        if ui.button("Optical Flares (Cinematic)").on_hover_text("Multi-element physical lens flare with anamorphic streaks and iris ghosts").clicked() {
            apply_effect_by_name(app, "Optical Flares");
            ui.close_menu();
        }
    });
    ui.menu_button("Distort", |ui| {
        for name in &["Twirl", "Bulge", "Mesh Warp", "Chromatic Aberration"] {
            if ui.button(*name).clicked() { apply_effect_by_name(app, name); ui.close_menu(); }
        }
        if ui.button("Motion Tile").on_hover_text("Seamlessly replicate layer content with mirror edges and phase").clicked() {
            apply_effect_by_name(app, "Motion Tile"); ui.close_menu();
        }
        if ui.button("CC Page Turn").on_hover_text("3D cylindrical page peel & curl deformation").clicked() {
            apply_effect_by_name(app, "CC Page Turn"); ui.close_menu();
        }
        if ui.button("Transform").on_hover_text("2D Affine transform in effect chain").clicked() {
            apply_effect_by_name(app, "Transform"); ui.close_menu();
        }
        if ui.button("Corner Pin").clicked() { apply_effect_by_name(app, "Corner Pin"); ui.close_menu(); }
    });
    ui.menu_button("Generate", |ui| {
        if ui.button("Lightning").on_hover_text("Procedural electric lightning arcs with glow").clicked() {
            apply_effect_by_name(app, "Lightning"); ui.close_menu();
        }
        if ui.button("Laser Beam").on_hover_text("High-energy projectile laser beam").clicked() {
            apply_effect_by_name(app, "Laser Beam"); ui.close_menu();
        }
    });
}

fn draw_ofx_submenu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui.button("Scan Standard Paths...").on_hover_text("Search /Library/OFX/Plugins for plugin bundles").clicked() {
        let found = crate::core::openfx_bridge::discover_all_ofx_plugins();
        if found.is_empty() {
            app.toasts.info("No OpenFX plugins found in standard paths");
        } else {
            let mut loadable = 0usize;
            let mut names: Vec<String> = Vec::new();
            for p in &found {
                if let crate::core::openfx_bridge::OfxProbeResult::Loaded { plugin_version, .. } =
                    crate::core::openfx_bridge::probe_ofx_plugin(&p.binary_path)
                {
                    loadable += 1;
                    if names.len() < 4 {
                        names.push(format!("{} v{}.{}", p.name, plugin_version.0, plugin_version.1));
                    }
                }
            }
            if loadable > 0 {
                app.toasts.info(format!("{loadable}/{} OFX effect(s) loadable: {}{}", found.len(), names.join(", "), if loadable > 4 { "…" } else { "" }));
            } else {
                app.toasts.info(format!("Found {} bundle(s), none expose OfxImageEffectAPI", found.len()));
            }
        }
        ui.close_menu();
    }
}

pub fn draw_animation_menu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    ui.menu_button("Keyframe Assistant", |ui| draw_keyframe_assistant(app, ui));
    if ui.button("The Smoother...").on_hover_text("Reduce keyframe density with RDP curve simplification").clicked() {
        app.show_the_smoother = true;
        ui.close_menu();
    }
    if ui.button("The Wiggler...").on_hover_text("Bake procedural noise keyframes into layer properties").clicked() {
        app.show_the_wiggler = true;
        ui.close_menu();
    }
    if ui.button("✏️ Motion Sketch...").on_hover_text("Record real-time mouse dragging in viewport to position keyframes").clicked() {
        app.show_motion_sketch = true;
        ui.close_menu();
    }
    if ui.button("⚛ Physics & Dynamics...").on_hover_text("2D Rigid Body Collision Simulation & Keyframe Baker").clicked() {
        app.show_physics = true;
        ui.close_menu();
    }
}

fn draw_keyframe_assistant(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    if ui.button("Convert Audio to Keyframes").on_hover_text("Extract RMS amplitude from audio layer into Slider Controls").clicked() {
        let src = get_audio_source(app);
        if let Some(src) = src {
            let mut temp_proj = app.history.current().clone();
            match crate::core::audio_to_keyframes::convert_audio_to_keyframes(temp_proj.active_composition_mut(), &src) {
                Ok(name) => { app.commit_project(temp_proj); app.toasts.info(format!("Created '{}' with Left/Right/Both channels", name)); }
                Err(e) => app.toasts.error(e),
            }
        } else {
            app.toasts.error("Select an Audio or Video-with-Audio layer first");
        }
        ui.close_menu();
    }
    if ui.button("Convert Multi-Band Audio (Bass/Mid/Treble)").on_hover_text("Extract frequency-separated amplitude into Sliders").clicked() {
        let src = get_audio_source(app);
        if let Some(src) = src {
            let mut temp_proj = app.history.current().clone();
            match crate::core::audio_to_keyframes::convert_multiband_audio_to_keyframes(temp_proj.active_composition_mut(), &src, None) {
                Ok(name) => { app.commit_project(temp_proj); app.toasts.info(format!("Created '{}' with Master/Bass/Mid/Treble", name)); }
                Err(e) => app.toasts.error(e),
            }
        } else {
            app.toasts.error("Select an Audio or Video-with-Audio layer first");
        }
        ui.close_menu();
    }
    ui.separator();
    if ui.add(egui::Button::new("Easy Ease").shortcut_text("F9")).clicked() {
        if let Some(idx) = app.selection.selected_layer_idx {
            app.modify_project(move |p| {
                if let Some(l) = p.active_composition_mut().layers.get_mut(idx) { l.easy_ease_transform(); }
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
    app.selection.selected_layer_idx.and_then(|idx| comp.layers.get(idx)).and_then(|l| {
        match &l.layer_type {
            crate::core::timeline::LayerType::Audio { path, .. } => Some(path.clone()),
            crate::core::timeline::LayerType::Video { audio_wav: Some(w), .. } => Some(w.clone()),
            _ => None,
        }
    })
}

pub fn draw_view_menu(app: &mut crate::KagariApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("UI Mode").small().color(egui::Color32::from_rgb(140, 140, 148)));
        let mode = app.ui_mode;
        if ui.selectable_label(mode.is_beginner(), "初心者").on_hover_text("Hide advanced panels and menus").clicked() {
            app.ui_mode = crate::ui::mode::UiMode::Beginner;
            crate::ui::mode::save_mode(app.ui_mode);
            app.toasts.info("初心者モード: 高度なパネルを非表示にしました");
        }
        if ui.selectable_label(mode.is_advanced(), "上級者").on_hover_text("Show every panel, menu and tool").clicked() {
            app.ui_mode = crate::ui::mode::UiMode::Advanced;
            crate::ui::mode::save_mode(app.ui_mode);
            app.toasts.info("上級者モード: 全機能を表示中");
        }
    });
    ui.separator();
    if ui.button("Restart Tutorial").clicked() { crate::ui::tutorial::restart(app); ui.close_menu(); }
    ui.separator();
    ui.checkbox(&mut app.show_grid, "Show Grid");
    ui.checkbox(&mut app.show_guides, "Show Safe Zones");
    ui.checkbox(&mut app.show_handles, "Show Handles");
    if ui.button("Analyze / Quality Check…").clicked() { app.show_quality_check_panel = true; ui.close_menu(); }
    if ui.button("Automation Bindings…").clicked() { app.show_automation_panel = true; ui.close_menu(); }
    ui.separator();
    let mut gpu_fx = crate::core::compute_pipeline::gpu_effects_enabled();
    if ui.checkbox(&mut gpu_fx, "GPU Compute Effects (beta)").changed() {
        crate::core::compute_pipeline::set_gpu_effects_enabled(gpu_fx);
        app.modify_project(|project| project.use_gpu_compute = gpu_fx);
        if gpu_fx {
            match crate::core::compute_pipeline::global() {
                Some(ctx) => app.toasts.info(format!("GPU compute: {}", ctx.backend_label())),
                None => {
                    app.toasts.error("No GPU adapter — staying on CPU");
                    crate::core::compute_pipeline::set_gpu_effects_enabled(false);
                    app.modify_project(|project| project.use_gpu_compute = false);
                }
            }
        } else {
            app.toasts.info("GPU compute off — deterministic CPU rendering");
        }
    }
    if ui.button("Reset Timeline Zoom (100%)").clicked() { app.timeline_zoom = 1.0; ui.close_menu(); }
    if ui.button("Purge All RAM Cache").clicked() {
        app.frame_cache.invalidate_all();
        crate::core::frame_cache::bump_version();
        ui.close_menu();
    }
    ui.menu_button("Workspaces", |ui| {
        if ui.button("Standard").clicked() { app.ui_tabs.right_tab_idx = 0; app.show_graph_editor = false; ui.close_menu(); }
        if ui.button("Motion Graphics").clicked() { app.ui_tabs.right_tab_idx = 0; app.show_graph_editor = true; ui.close_menu(); }
        if ui.button("VFX & Color").clicked() { app.ui_tabs.right_tab_idx = 30; app.show_graph_editor = false; ui.close_menu(); }
        if ui.button("Audio Editing").clicked() { app.ui_tabs.right_tab_idx = 7; app.show_graph_editor = false; ui.close_menu(); }
    });
}

pub fn draw_help_menu(app: &mut crate::KagariApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    if ui.add(egui::Button::new("⚙ Preferences…").shortcut_text(
        crate::ui::shortcuts::format_shortcut(",", true, false, false),
    )).clicked() {
        app.show_preferences = true;
        ui.close_menu();
    }
    if ui.button("Show Welcome Screen").clicked() { app.show_welcome = true; ui.close_menu(); }
    if ui.button("Keyboard Shortcuts Reference...").clicked() {
        let help_id = egui::Id::new("show_shortcuts_modal");
        ctx.data_mut(|d| d.insert_temp(help_id, true));
        ui.close_menu();
    }
    if ui.button("Start Guided Tutorial...").clicked() {
        app.show_guided_tutorial = true;
        app.tutorial_step = 0;
        ui.close_menu();
    }
    ui.separator();
    if ui.button("ℹ️ About Kagari Studio...").clicked() {
        let about_id = egui::Id::new("show_about_modal");
        ctx.data_mut(|d| d.insert_temp(about_id, true));
        ui.close_menu();
    }
}

pub fn draw_dialogs(app: &mut crate::KagariApp, ctx: &egui::Context) {
    draw_tutorial_dialog(app, ctx);
    draw_about_dialog(ctx);
    draw_shortcuts_dialog(ctx);
    crate::ui::precompose_dialog::draw_precompose_dialog(app, ctx);
    crate::ui::recovery_dialog::draw_recovery_dialog(app, ctx);
    crate::ui::sequence_layers_dialog::draw_sequence_layers_dialog(app, ctx);
    draw_floating_panels(app, ctx);
}

fn draw_tutorial_dialog(app: &mut crate::KagariApp, ctx: &egui::Context) {
    if !app.show_guided_tutorial { return; }
    let mut show_tutorial = app.show_guided_tutorial;
    let mut finish_tutorial = false;
    crate::ui::modal::window("Kagari VFX — Quickstart Guided Tour")
        .open(&mut show_tutorial)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Welcome to Kagari VFX Motion Graphics!");
            ui.add_space(4.0);
            match app.tutorial_step {
                0 => { ui.label("Step 1: Composition & Layers"); ui.label("• Left panel contains your Project assets."); }
                1 => { ui.label("Step 2: Keyframing & Animation"); ui.label("• Click the stopwatch icon next to any property to add a keyframe."); }
                2 => { ui.label("Step 3: Effects & 3D Extrusion"); ui.label("• Drag effects from the Effects Library into your layers."); }
                _ => { ui.label("Step 4: Export & Sharing"); ui.label("• Press Cmd+M to render high-quality MP4, ProRes, or Lottie JSON."); }
            }
            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                if app.tutorial_step > 0 && ui.button("◀ Previous").clicked() { app.tutorial_step -= 1; }
                if app.tutorial_step < 3 {
                    if ui.button("Next Step ▶").clicked() { app.tutorial_step += 1; }
                } else if ui.button("Finish Tour").clicked() {
                    finish_tutorial = true;
                }
            });
        });
    if finish_tutorial {
        show_tutorial = false;
        app.toasts.info("Tutorial completed! Have fun animating!");
    }
    app.show_guided_tutorial = show_tutorial;
}

fn draw_about_dialog(ctx: &egui::Context) {
    let about_id = egui::Id::new("show_about_modal");
    let mut show_about = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(about_id, || false));
    if show_about {
        crate::ui::modal::window("About Kagari Studio")
            .open(&mut show_about)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Kagari Studio (篝)");
                ui.label(format!("Version: {} (Open Source)", env!("CARGO_PKG_VERSION")));
                ui.label("Licensed MIT OR Apache-2.0 — see LICENSE-MIT, LICENSE-APACHE,");
                ui.label("and THIRD-PARTY-NOTICES in the repository root.");
                ui.label("A high-performance Motion Graphics & 32bpc HDR Visual Effects engine written in Rust.");
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);
                ui.small("Kagari Studio is an independent open-source project unrelated to any commercial compositing software.");
                ui.add_space(8.0);
            });
        ctx.data_mut(|d| d.insert_temp(about_id, show_about));
    }
}

fn draw_shortcuts_dialog(ctx: &egui::Context) {
    let help_id = egui::Id::new("show_shortcuts_modal");
    let mut show_help = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(help_id, || false));
    if show_help {
        crate::ui::modal::window("Keyboard Shortcuts Reference")
            .open(&mut show_help)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Kagari VFX — Shortcuts Reference");
                ui.separator();
                egui::Grid::new("shortcuts_grid").striped(true).show(ui, |ui| {
                    let rows = [
                        ("Spacebar", "Play / Pause RAM Preview"),
                        ("V", "Selection Tool"),
                        ("H", "Hand Tool (Pan)"),
                        ("Z", "Zoom Tool"),
                        ("W", "Rotation Tool"),
                        ("Cmd + Z", "Undo"),
                        ("Cmd + Shift + Z", "Redo"),
                        ("J / K / L", "Prev Keyframe / Stop / Next Keyframe"),
                        ("Arrow Keys", "Nudge Selected Layer 1px (Shift = 10px)"),
                        ("PageUp / PageDown", "Step Frame Backward / Forward"),
                        ("Home / End", "First / Last Frame"),
                        ("B / N", "Set Work Area Start / End"),
                        ("= / -", "Timeline Zoom In / Out"),
                        ("F9", "Apply Easy Ease to Keyframes"),
                        ("Delete", "Delete Selected Keyframes"),
                        ("Cmd + A", "Select All Keyframes of Layer"),
                        ("Esc", "Deselect"),
                        ("M", "Add / Remove Timeline Marker"),
                    ];
                    for (key, desc) in &rows {
                        ui.label(*key);
                        ui.label(*desc);
                        ui.end_row();
                    }
                });
            });
        ctx.data_mut(|d| d.insert_temp(help_id, show_help));
    }
}

fn draw_floating_panels(app: &mut crate::KagariApp, ctx: &egui::Context) {
    if app.show_the_smoother {
        let mut open = app.show_the_smoother;
        crate::ui::modal::window("The Smoother").open(&mut open).resizable(false).default_width(280.0).show(ctx, |ui| {
            crate::ui::the_smoother_panel::draw_the_smoother_panel(app, ui);
        });
        app.show_the_smoother = open;
    }
    if app.show_the_wiggler {
        let mut open = app.show_the_wiggler;
        crate::ui::modal::window("The Wiggler").open(&mut open).resizable(false).default_width(280.0).show(ctx, |ui| {
            crate::ui::the_wiggler_panel::draw_the_wiggler_panel(app, ui);
        });
        app.show_the_wiggler = open;
    }
    if app.show_motion_sketch {
        let mut open = app.show_motion_sketch;
        crate::ui::modal::window("Motion Sketch").open(&mut open).resizable(false).default_width(280.0).show(ctx, |ui| {
            crate::ui::motion_sketch_panel::draw_motion_sketch_panel(app, ui);
        });
        app.show_motion_sketch = open;
    }
    if app.show_physics {
        let mut open = app.show_physics;
        crate::ui::modal::window("Physics & Dynamics").open(&mut open).resizable(false).default_width(300.0).show(ctx, |ui| {
            crate::ui::physics_panel::draw_physics_panel(app, ui);
        });
        app.show_physics = open;
    }
}
