use crate::KagariApp;
use eframe::egui;

pub fn draw_flowchart_inspector(app: &mut KagariApp, ui: &mut egui::Ui) {
    crate::ui::kui::section(ui, "Composition Flowchart & Hierarchy");

    let comp = app.history.current().active_composition();
    crate::ui::kui::secondary(
        ui,
        crate::ui::localization::tr_format("Root Composition: {}", &[&comp.name]),
    );
    crate::ui::kui::secondary(
        ui,
        crate::ui::localization::tr_format("Total Layers: {}", &[&comp.layers.len().to_string()]),
    );

    ui.add_space(crate::ui::kui::space::SM);
    crate::ui::kui::separator(ui);

    crate::ui::kui::caption(ui, "Composition Dependency Graph (DAG):");
    egui::ScrollArea::vertical()
        .max_height(200.0)
        .show(ui, |ui| {
            crate::ui::kui::truncated_secondary(ui, "[Comp 1] (Active Root)");
            ui.indent("comp_root_indent", |ui| {
                for (i, layer) in comp.layers.iter().enumerate() {
                    let icon = match layer.layer_type {
                        crate::core::timeline::LayerType::Text { .. } => "Text",
                        crate::core::timeline::LayerType::Solid { .. } => "Solid",
                        crate::core::timeline::LayerType::Image { .. } => "Image",
                        crate::core::timeline::LayerType::Model3D { .. } => "Model 3D",
                        crate::core::timeline::LayerType::Video { .. } => "Video",
                        crate::core::timeline::LayerType::Shape { .. } => "Shape",
                        crate::core::timeline::LayerType::Null => "Null",
                        crate::core::timeline::LayerType::PreComp { .. } => "PreComp",
                        crate::core::timeline::LayerType::Audio { .. } => "Audio",
                        crate::core::timeline::LayerType::AdjustmentLayer => "Adjustment",
                        crate::core::timeline::LayerType::Particle { .. } => "Particle",
                    };
                    let parent_info = if let Some(ref p_id) = layer.parent_id {
                        format!(
                            " {}",
                            crate::ui::localization::tr_format("Parent: {}", &[p_id])
                        )
                    } else {
                        "".to_string()
                    };
                    let label = format!(
                        "└─ {:02}. {} {}{}",
                        i + 1,
                        crate::ui::localization::tr(icon),
                        layer.name,
                        parent_info
                    );
                    crate::ui::kui::truncated_secondary(ui, &label);
                }
            });
        });
}
