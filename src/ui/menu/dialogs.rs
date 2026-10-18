use eframe::egui;

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
    if !app.show_guided_tutorial {
        return;
    }
    let mut show_tutorial = app.show_guided_tutorial;
    let mut finish_tutorial = false;
    crate::ui::modal::window("Kagari VFX — Quickstart Guided Tour")
        .open(&mut show_tutorial)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Welcome to Kagari VFX Motion Graphics!");
            ui.add_space(4.0);
            match app.tutorial_step {
                0 => {
                    ui.label("Step 1: Composition & Layers");
                    ui.label("• Left panel contains your Project assets.");
                }
                1 => {
                    ui.label("Step 2: Keyframing & Animation");
                    ui.label("• Click the stopwatch icon next to any property to add a keyframe.");
                }
                2 => {
                    ui.label("Step 3: Effects & 3D Extrusion");
                    ui.label("• Drag effects from the Effects Library into your layers.");
                }
                _ => {
                    ui.label("Step 4: Export & Sharing");
                    ui.label("• Press Cmd+M to render high-quality MP4, ProRes, or Lottie JSON.");
                }
            }
            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                if app.tutorial_step > 0 && ui.button("◀ Previous").clicked() {
                    app.tutorial_step -= 1;
                }
                if app.tutorial_step < 3 {
                    if ui.button("Next Step ▶").clicked() {
                        app.tutorial_step += 1;
                    }
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
        crate::ui::shortcuts_dialog::draw_shortcuts_reference_dialog(ctx, &mut show_help);
        ctx.data_mut(|d| d.insert_temp(help_id, show_help));
    }
}

fn draw_floating_panels(app: &mut crate::KagariApp, ctx: &egui::Context) {
    if app.show_the_smoother {
        let mut open = app.show_the_smoother;
        crate::ui::modal::window("The Smoother")
            .open(&mut open)
            .resizable(false)
            .default_width(280.0)
            .show(ctx, |ui| {
                crate::ui::the_smoother_panel::draw_the_smoother_panel(app, ui);
            });
        app.show_the_smoother = open;
    }
    if app.show_the_wiggler {
        let mut open = app.show_the_wiggler;
        crate::ui::modal::window("The Wiggler")
            .open(&mut open)
            .resizable(false)
            .default_width(280.0)
            .show(ctx, |ui| {
                crate::ui::the_wiggler_panel::draw_the_wiggler_panel(app, ui);
            });
        app.show_the_wiggler = open;
    }
    if app.show_motion_sketch {
        let mut open = app.show_motion_sketch;
        crate::ui::modal::window("Motion Sketch")
            .open(&mut open)
            .resizable(false)
            .default_width(280.0)
            .show(ctx, |ui| {
                crate::ui::motion_sketch_panel::draw_motion_sketch_panel(app, ui);
            });
        app.show_motion_sketch = open;
    }
    if app.show_physics {
        let mut open = app.show_physics;
        crate::ui::modal::window("Physics & Dynamics")
            .open(&mut open)
            .resizable(false)
            .default_width(300.0)
            .show(ctx, |ui| {
                crate::ui::physics_panel::draw_physics_panel(app, ui);
            });
        app.show_physics = open;
    }
}
