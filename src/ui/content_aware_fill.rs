//! Content-aware fill tool UI panel, background sequence baking, and layer injection.
//!
//! Submodules:
//! - [`bake`]: Sequence baking, mask isolation, WebP frame generation, and folder resolution.
//! - [`job`]: Asynchronous thread orchestration and frame completion polling.

pub mod bake;
pub mod job;

#[cfg(test)]
mod tests;

pub use bake::*;
pub use job::*;

use crate::ui::localization::{tr, tr_format};
use crate::KagariApp;
use eframe::egui;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub fn draw_content_aware_fill(app: &mut KagariApp, ui: &mut egui::Ui) {
    poll_caf_job(app, ui.ctx());
    crate::ui::kui::title(ui, "Content-Aware Fill");
    crate::ui::kui::separator(ui);

    let method_id = egui::Id::new("ae_caf_fill_method");
    let mut method_idx = ui
        .ctx()
        .data_mut(|data| *data.get_temp_mut_or_insert_with(method_id, || 0));
    crate::ui::kui::combo(
        ui,
        "caf_method_combo",
        &mut method_idx,
        &[
            (0, "Object (Motion Objects Removal)"),
            (1, "Surface (Flat Texture Fill)"),
            (2, "Edge Blend (Smooth Gradient)"),
        ],
    );
    ui.ctx()
        .data_mut(|data| data.insert_temp(method_id, method_idx));

    let alpha_exp_id = egui::Id::new("ae_caf_alpha_expansion");
    let mut alpha_exp: f32 = ui
        .ctx()
        .data_mut(|data| *data.get_temp_mut_or_insert_with(alpha_exp_id, || 5.0));
    crate::ui::kui::property(ui, "Alpha Expansion:", |ui| {
        crate::ui::kui::slider(ui, &mut alpha_exp, 0.0..=50.0, " px")
    });
    ui.ctx()
        .data_mut(|data| data.insert_temp(alpha_exp_id, alpha_exp));

    let range_id = egui::Id::new("ae_caf_range");
    let mut range_idx = ui
        .ctx()
        .data_mut(|data| *data.get_temp_mut_or_insert_with(range_id, || 0));
    ui.horizontal(|ui| {
        if crate::ui::kui::tool_button(ui, range_idx == 0, "Work Area").clicked() {
            range_idx = 0;
        }
        if crate::ui::kui::tool_button(ui, range_idx == 1, "Entire Duration").clicked() {
            range_idx = 1;
        }
    });
    ui.ctx()
        .data_mut(|data| data.insert_temp(range_id, range_idx));
    ui.add_space(8.0);
    crate::ui::kui::separator(ui);

    let job_id = egui::Id::new("content_aware_fill_job");
    let job = ui.ctx().data(|data| data.get_temp::<Arc<CafJob>>(job_id));
    if let Some(job) = job {
        let done = job.progress.load(Ordering::Relaxed).min(job.total);
        ui.label(tr_format(
            "Generating WebP frames: {} / {}",
            &[&done.to_string(), &job.total.to_string()],
        ));
        crate::ui::kui::progress_bar(ui, done as f32 / job.total.max(1) as f32, None);
        if crate::ui::kui::button(ui, "Cancel", crate::ui::kui::ButtonKind::Default).clicked() {
            job.cancelled.store(true, Ordering::Relaxed);
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(120));
    } else if crate::ui::kui::button(
        ui,
        "Generate Fill Layer",
        crate::ui::kui::ButtonKind::Accent,
    )
    .on_hover_text(tr(
        "Generate a WebP fill sequence for the selected layer and range",
    ))
    .clicked()
    {
        let method = match method_idx {
            1 => crate::core::content_aware_engine::FillMethod::Surface,
            2 => crate::core::content_aware_engine::FillMethod::EdgeBlend,
            _ => crate::core::content_aware_engine::FillMethod::Object,
        };
        if let Err(error) = start_caf_job(app, ui.ctx(), range_idx, alpha_exp, method) {
            app.toasts.error(tr(&error));
        }
    }

    if crate::ui::kui::button(
        ui,
        "Create Reference Frame",
        crate::ui::kui::ButtonKind::Default,
    )
    .on_hover_text(tr(
        "Export current frame as a reference PNG for manual painting/cleanup",
    ))
    .clicked()
    {
        let comp = app.history.current().active_composition();
        let (width, height) = (comp.width, comp.height);
        let pixels = crate::core::software_renderer::render_frame_to_pixels(
            comp,
            app.playback.current_frame,
            width,
            height,
            0.0,
            0,
        );
        let out_path =
            std::env::temp_dir().join(format!("ref_frame_{}.png", app.playback.current_frame));
        if image::save_buffer(&out_path, &pixels, width, height, image::ColorType::Rgba8).is_ok() {
            crate::ui::project_io::reveal_in_file_manager(&out_path);
            app.toasts.info(tr_format(
                "Exported Reference Frame: {}",
                &[&out_path.display().to_string()],
            ));
        }
    }
}
