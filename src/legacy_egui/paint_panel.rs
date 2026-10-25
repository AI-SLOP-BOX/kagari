use crate::KagariApp;
use eframe::egui;

pub fn draw_paint_panel(app: &mut KagariApp, ui: &mut egui::Ui) {
    ui.heading("Paint & Paint Brushes");
    ui.separator();

    let brush_size_id = egui::Id::new("paint_size");
    let mut brush_size: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(brush_size_id, || 12.0));
    ui.horizontal(|ui| {
        ui.label("Size:");
        if ui
            .add(egui::Slider::new(&mut brush_size, 1.0..=200.0).suffix(" px"))
            .changed()
        {
            ui.ctx()
                .data_mut(|d| d.insert_temp(brush_size_id, brush_size));
        }
    });

    let color_id = egui::Id::new("paint_color");
    let mut color = ui.ctx().data_mut(|d| {
        d.get_temp::<[f32; 4]>(color_id)
            .unwrap_or([1.0, 1.0, 1.0, 1.0])
    });
    ui.horizontal(|ui| {
        ui.label("Color:");
        if ui.color_edit_button_rgba_unmultiplied(&mut color).changed() {
            ui.ctx().data_mut(|d| d.insert_temp(color_id, color));
        }
    });

    let hardness_id = egui::Id::new("paint_hardness");
    let mut hardness: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(hardness_id, || 80.0));
    ui.horizontal(|ui| {
        ui.label("Hardness:");
        if ui
            .add(egui::Slider::new(&mut hardness, 0.0..=100.0).suffix(" %"))
            .changed()
        {
            ui.ctx().data_mut(|d| d.insert_temp(hardness_id, hardness));
        }
    });

    let opacity_id = egui::Id::new("paint_opacity");
    let mut opacity: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(opacity_id, || 100.0));
    ui.horizontal(|ui| {
        ui.label("Opacity:");
        if ui
            .add(egui::Slider::new(&mut opacity, 0.0..=100.0).suffix(" %"))
            .changed()
        {
            ui.ctx().data_mut(|d| d.insert_temp(opacity_id, opacity));
        }
    });

    let flow_id = egui::Id::new("paint_flow");
    let mut flow: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(flow_id, || 100.0));
    ui.horizontal(|ui| {
        ui.label("Flow:");
        if ui
            .add(egui::Slider::new(&mut flow, 0.0..=100.0).suffix(" %"))
            .changed()
        {
            ui.ctx().data_mut(|d| d.insert_temp(flow_id, flow));
        }
    });

    ui.add_space(8.0);
    ui.separator();

    ui.label("Paint Mode & Channels:");
    let mut mode_idx = match app.active_tool {
        crate::ui::toolbar::ActiveTool::Eraser => 1,
        crate::ui::toolbar::ActiveTool::CloneStamp => 2,
        _ => 0,
    };
    ui.horizontal(|ui| {
        if ui.selectable_value(&mut mode_idx, 0, "Normal").clicked() {
            app.active_tool = crate::ui::toolbar::ActiveTool::Brush;
        }
        if ui.selectable_value(&mut mode_idx, 1, "Eraser").clicked() {
            app.active_tool = crate::ui::toolbar::ActiveTool::Eraser;
        }
        if ui
            .selectable_value(&mut mode_idx, 2, "Clone Stamp")
            .clicked()
        {
            app.active_tool = crate::ui::toolbar::ActiveTool::CloneStamp;
        }
    });
}
