use std::time::{Duration, Instant};

use choegui::design::{ButtonKind, StatusKind, Theme};
use choegui::{Button, Spring, Surface};
use eframe::egui;

struct Showcase {
    expanded: bool,
    toast_until: Option<Instant>,
    progress_target: bool,
    reduce_motion: bool,
    light_theme: bool,
    exposure: f32,
    contrast: f32,
    lut_enabled: bool,
}

impl Default for Showcase {
    fn default() -> Self {
        Self {
            expanded: true,
            toast_until: None,
            progress_target: true,
            reduce_motion: false,
            light_theme: false,
            exposure: 0.35,
            contrast: 12.0,
            lut_enabled: true,
        }
    }
}

impl eframe::App for Showcase {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let theme = if self.light_theme {
            Theme::light()
        } else {
            Theme::dark()
        }
        .reduced_motion(self.reduce_motion);
        let motion = if self.reduce_motion {
            choegui::MotionPolicy::Reduced
        } else {
            choegui::MotionPolicy::Full
        };
        ctx.set_visuals(if self.light_theme {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_max_width(760.0);
            ui.add_space(26.0);
            ui.heading("choegui");
            ui.label("Make egui 超えぐい.");
            ui.add_space(22.0);
            Surface::new().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("NOW PLAYING").small().color(egui::Color32::from_rgb(167, 169, 190)));
                        ui.heading("A softer kind of motion");
                        ui.label("A tiny prototype beside ordinary egui controls.");
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if Button::new("Play / Pause").motion_policy(motion).show(ui).clicked() {
                            self.progress_target = !self.progress_target;
                            self.toast_until = Some(Instant::now() + Duration::from_secs(2));
                        }
                    });
                });
                ui.add_space(14.0);
                let progress = choegui::animate_with_policy(
                    ctx,
                    ui.id().with("progress"),
                    if self.progress_target { 0.82 } else { 0.18 },
                    Spring::gentle(),
                    motion,
                );
                ui.add(egui::ProgressBar::new(progress).text("smooth value change"));
            });
            ui.add_space(18.0);
            theme.card(ui, |ui| {
                theme.title(ui, "Reusable design primitives");
                theme.secondary(ui, "The host application owns labels, icons, and state.");
                theme.separator(ui);
                let grid = theme.property_grid(ui, &["Exposure", "Contrast"]);
                grid.row(ui, "Exposure", |ui| {
                    theme.slider(ui, &mut self.exposure, -5.0..=5.0, " EV")
                });
                grid.row(ui, "Contrast", |ui| {
                    theme.number(ui, &mut self.contrast, -100.0..=100.0, 0.1, "%")
                });
                theme.toggle(ui, &mut self.lut_enabled, "Preview LUT");
                theme.progress_bar(ui, self.exposure.abs() / 5.0, Some("Preview analysis"));
                theme.level_meter(ui, self.contrast.abs() / 100.0, "Signal level");
                ui.horizontal(|ui| {
                    if theme.button(ui, "Apply look", ButtonKind::Accent).clicked() {
                        self.toast_until = Some(Instant::now() + Duration::from_secs(2));
                    }
                    theme.status(ui, StatusKind::Success, "Viewer ready");
                    theme.icon_button(
                        ui,
                        "reset-look",
                        choegui::design::IconOptions {
                            glyph: "↺",
                            tooltip: "Reset look",
                            selected: false,
                            enabled: true,
                            size: 30.0,
                        },
                    );
                });
                theme.section(ui, "curve-section", "Curves", false, |ui| {
                    theme.caption(ui, "The application can place custom Painter content here.");
                });
            });
            ui.add_space(18.0);
            ui.horizontal(|ui| {
                if ui.button(if self.expanded { "Hide details" } else { "Show details" }).clicked() { self.expanded = !self.expanded; }
                ui.label("← standard egui button");
            });
            ui.checkbox(&mut self.reduce_motion, "Reduce motion");
            ui.checkbox(&mut self.light_theme, "Light appearance");
            let amount = choegui::animated_visibility_with_policy(
                ctx, ui.id().with("details"), self.expanded, Spring::gentle(), motion,
            );
            if amount > 0.01 {
                ui.add_space(10.0 * amount);
                Surface::new().opacity(amount).show(ui, |ui| {
                    ui.set_max_width(700.0 * amount);
                    ui.label("Animated visibility uses a continuous spring. Toggle quickly to retarget it mid-flight.");
                    ui.label("The translucent surface is portable; live backdrop blur needs a renderer integration.");
                });
            }
            ui.add_space(20.0);
            ui.label("Hover and hold the accent button. The response remains a normal egui Response.");
        });
        let toast_visible = self.toast_until.is_some_and(|until| Instant::now() < until);
        if self
            .toast_until
            .is_some_and(|until| !toast_visible && Instant::now() >= until)
        {
            self.toast_until = None;
        }
        choegui::Toast::new("demo-toast", "Playback toggled", toast_visible)
            .motion_policy(motion)
            .show(ctx);
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "choegui showcase",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::<Showcase>::default())),
    )
}
