use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

#[derive(Clone, Copy)]
enum ExportFormat {
    Mp4,
    Mov,
    PngSequence,
}

impl ExportFormat {
    fn state_index(self) -> usize {
        match self {
            Self::Mp4 => 0,
            Self::Mov => 1,
            Self::PngSequence => 2,
        }
    }
}

#[derive(Clone, Copy)]
enum ExportCodec {
    FormatDefault,
    ProRes422Hq,
    ProRes4444,
}

impl ExportCodec {
    fn state_index(self) -> usize {
        match self {
            Self::FormatDefault => 0,
            Self::ProRes422Hq => 1,
            Self::ProRes4444 => 2,
        }
    }
}

struct RenderPreset {
    format: ExportFormat,
    codec: ExportCodec,
    label: &'static str,
}

/// Output-module presets bound to the real export pipeline
/// (`app.export_format_preset` + the shared codec selector).
pub fn draw_render_presets(app: &mut KagariApp, ui: &mut egui::Ui) {
    ui.heading("Render Queue Output Presets");
    ui.separator();

    const PRESETS: [RenderPreset; 4] = [
        RenderPreset {
            format: ExportFormat::Mp4,
            codec: ExportCodec::FormatDefault,
            label: "H.264 High Quality (MP4)",
        },
        RenderPreset {
            format: ExportFormat::Mov,
            codec: ExportCodec::ProRes422Hq,
            label: "ProRes 422 HQ (MOV Master)",
        },
        RenderPreset {
            format: ExportFormat::Mov,
            codec: ExportCodec::ProRes4444,
            label: "Lossless ProRes 4444 (MOV + Alpha)",
        },
        RenderPreset {
            format: ExportFormat::PngSequence,
            codec: ExportCodec::FormatDefault,
            label: "PNG Image Sequence (RGBA)",
        },
    ];

    let current = PRESETS
        .iter()
        .position(|preset| {
            preset.format.state_index() == app.export_format_preset
                && preset.codec.state_index() == app.export_codec_idx
        })
        .unwrap_or(1);
    let mut selected = current;
    let combo = egui::ComboBox::from_id_salt("render_preset_combo")
        .selected_text(PRESETS[selected].label)
        .show_ui(ui, |ui| {
            for (i, preset) in PRESETS.iter().enumerate() {
                if ui
                    .selectable_value(&mut selected, i, preset.label)
                    .changed()
                {
                    app.export_format_preset = preset.format.state_index();
                    app.export_codec_idx = preset.codec.state_index();
                }
            }
        });
    combo
        .response
        .on_hover_text("Configures the Render Queue / Export dialog output module");

    ui.add_space(6.0);
    let target = app.history.current().active_composition().id.clone();
    if !app.render_queue_items.contains(&target)
        && custom_add_button(ui, "＋ Add Active Comp to Queue")
    {
        app.render_queue_items.push(target.clone());
        app.render_item_status
            .insert(target, crate::app_state::QueueItemStatus::Queued);
    }
    ui.weak(
        egui::RichText::new("Presets apply to the next render started from the Render Queue.")
            .small()
            .color(colors::TEXT_MUTED),
    );
}

fn custom_add_button(ui: &mut egui::Ui, label: &str) -> bool {
    ui.button(label).clicked()
}
