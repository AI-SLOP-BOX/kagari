use crate::core::timeline::EffectType;
use crate::ui::localization::{tr, tr_format};
use eframe::egui;

pub fn draw(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    _current_frame: u32,
    project_changed: &mut bool,
    _next_frame: &mut Option<u32>,
) {
    if let EffectType::CustomShader {
        wgsl_source,
        uniform_values,
    } = effect_type
    {
        crate::ui::kui::section(ui, "Custom WGSL Shader Plugin");

        // Templates dropdown
        ui.horizontal(|ui| {
            ui.label(tr("Templates:"));
            if crate::ui::kui::button(ui, "CRT Scanlines", crate::ui::kui::ButtonKind::Quiet)
                .clicked()
            {
                *wgsl_source = crate::core::custom_shader_cpu::CRT_TEMPLATE.to_string();
                if uniform_values.len() < 2 {
                    *uniform_values = vec![1.0, 0.4];
                }
                *project_changed = true;
            }
            if crate::ui::kui::button(ui, "Chromatic Twist", crate::ui::kui::ButtonKind::Quiet)
                .clicked()
            {
                *wgsl_source = crate::core::custom_shader_cpu::CHROMATIC_TWIST_TEMPLATE.to_string();
                if uniform_values.len() < 2 {
                    *uniform_values = vec![1.5, 0.5];
                }
                *project_changed = true;
            }
            if crate::ui::kui::button(ui, "Neon Edge", crate::ui::kui::ButtonKind::Quiet).clicked()
            {
                *wgsl_source = crate::core::custom_shader_cpu::NEON_EDGE_TEMPLATE.to_string();
                if uniform_values.len() < 2 {
                    *uniform_values = vec![0.5, 2.0];
                }
                *project_changed = true;
            }
        });

        ui.add_space(4.0);
        if ui
            .add(
                egui::TextEdit::multiline(wgsl_source)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(8),
            )
            .changed()
        {
            *project_changed = true;
        }

        // Real-time naga WGSL validation & hot-reload status
        let status = crate::core::custom_shader_runtime::CustomShaderRegistry::global()
            .validate_wgsl(wgsl_source);
        let cpu_supported = crate::core::custom_shader_cpu::cpu_supported(wgsl_source);
        ui.horizontal(|ui| {
                if status.is_valid {
                    crate::ui::kui::status(
                        ui,
                        crate::ui::kui::StatusKind::Success,
                        "WGSL syntax and semantics valid (GPU pipeline not verified)",
                    );
                } else {
                    let fallback_error = tr("WGSL Syntax Error");
                    let err_snippet = status
                        .error_message
                        .as_deref()
                        .unwrap_or(&fallback_error);
                    let short_err = err_snippet.lines().next().unwrap_or(&fallback_error);
                    crate::ui::kui::status(
                        ui,
                        crate::ui::kui::StatusKind::Error,
                        short_err,
                    )
                    .on_hover_text(err_snippet);
                }

                if cpu_supported {
                    crate::ui::kui::status(
                        ui,
                        crate::ui::kui::StatusKind::Success,
                        "CPU export fallback ready",
                    );
                } else {
                    crate::ui::kui::status(
                        ui,
                        crate::ui::kui::StatusKind::Warning,
                        "CPU fallback unavailable",
                    )
                    .on_hover_text(tr(
                        "Arbitrary WGSL is validated for syntax only; CPU preview/export supports the built-in templates only.",
                    ));
                }

                if crate::ui::kui::button(
                    ui,
                    "Load .wgsl File",
                    crate::ui::kui::ButtonKind::Quiet,
                )
                    .on_hover_text(tr("Load and hot-reload shader from disk"))
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("WGSL Shader", &["wgsl", "frag"])
                        .pick_file()
                    {
                        if let Ok(content) = crate::core::project_migration::read_bounded_text_file(
                            &path,
                            4 * 1024 * 1024,
                        ) {
                            *wgsl_source = content;
                            *project_changed = true;
                        }
                    }
                }

                if crate::ui::kui::button(ui, "+ Add Float", crate::ui::kui::ButtonKind::Quiet)
                    .clicked()
                {
                    uniform_values.push(1.0);
                    *project_changed = true;
                }
                if !uniform_values.is_empty()
                    && crate::ui::kui::button(
                        ui,
                        "- Remove",
                        crate::ui::kui::ButtonKind::Quiet,
                    )
                    .clicked()
                {
                    uniform_values.pop();
                    *project_changed = true;
                }
            });

        // Interactive dynamic uniform sliders
        for (i, v) in uniform_values.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                let index = i.to_string();
                let axis = match i {
                    0 => "x",
                    1 => "y",
                    2 => "z",
                    3 => "w",
                    _ => "?",
                };
                ui.label(tr_format("Param {} (u_params.{}):", &[&index, axis]));
                if ui.add(crate::ui::kui::drag(v).speed(0.01)).changed() {
                    *project_changed = true;
                }
            });
        }
    }
}
