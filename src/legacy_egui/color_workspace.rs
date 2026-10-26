use crate::KagariApp;
use eframe::egui;

const ASSET_TAB: &str = "color_workspace_asset_tab";
const INSPECTOR_TAB: &str = "color_workspace_inspector_tab";

pub fn draw(app: &mut KagariApp, ctx: &egui::Context) {
    let frame = app.playback.current_frame;
    let screen_w = ctx.screen_rect().width();
    let compact = screen_w < 1050.0;
    let ultra_compact = screen_w < 820.0;
    let scope_height = (ctx.screen_rect().height() * 0.29).clamp(170.0, 330.0);

    egui::TopBottomPanel::bottom("color_workspace_scopes")
        .default_height(scope_height)
        .height_range(150.0..=420.0)
        .resizable(true)
        .frame(panel_frame())
        .show(ctx, |ui| draw_scopes(app, ctx, ui, compact));

    if !ultra_compact {
        let left_max = (screen_w * 0.35).clamp(160.0, 420.0);
        let left_min = 160.0_f32.min(left_max);
        let left_def = if compact { 210.0_f32 } else { 286.0_f32 };
        egui::SidePanel::left("color_workspace_assets")
            .default_width(left_def.min(left_max))
            .width_range(left_min..=left_max)
            .resizable(true)
            .frame(panel_frame())
            .show(ctx, |ui| draw_assets(app, ctx, ui));
    }

    let right_max = (screen_w * 0.48).clamp(240.0, 520.0);
    let right_min = 220.0_f32.min(right_max);
    let right_def = if compact { 280.0_f32 } else { 380.0_f32 };
    egui::SidePanel::right("color_workspace_controls")
        .default_width(right_def.min(right_max))
        .width_range(right_min..=right_max)
        .resizable(true)
        .frame(panel_frame())
        .show(ctx, |ui| draw_controls(app, ctx, ui));

    crate::ui::viewport::draw(app, ctx, frame);
}

fn panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(crate::ui::theme::colors::BG_PANEL_BASE)
        .stroke(egui::Stroke::new(
            1.0_f32,
            crate::ui::theme::colors::BORDER_SUBTLE,
        ))
}

fn draw_assets(app: &mut KagariApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    let mut active = ctx
        .data(|data| data.get_temp::<usize>(egui::Id::new(ASSET_TAB)))
        .unwrap_or(0);
    ui.horizontal(|ui| {
        if crate::ui::kui::segmented_button(ui, active == 0, "Assets", "Project assets").clicked() {
            active = 0;
        }
        if crate::ui::kui::segmented_button(ui, active == 1, "Looks", "Color grading presets")
            .clicked()
        {
            active = 1;
        }
    });
    ctx.data_mut(|data| data.insert_temp(egui::Id::new(ASSET_TAB), active));
    crate::ui::kui::separator(ui);

    if active == 0 {
        crate::ui::project_panel::draw(app, ui);
    } else {
        draw_look_presets(app, ui);
    }
}

fn draw_look_presets(app: &mut KagariApp, ui: &mut egui::Ui) {
    crate::ui::kui::secondary(ui, "Color grading looks");
    crate::ui::kui::caption(ui, "Apply color correction to the selected layer.");
    crate::ui::kui::separator(ui);
    if app.selection.selected_layer_idx.is_none() {
        crate::ui::kui::empty_state(ui, "Select a footage layer first.", 96.0);
        return;
    }
    crate::ui::lumetri_color::draw_look_presets(app, ui);
}

fn draw_controls(app: &mut KagariApp, ctx: &egui::Context, ui: &mut egui::Ui) {
    let mut active = ctx
        .data(|data| data.get_temp::<usize>(egui::Id::new(INSPECTOR_TAB)))
        .unwrap_or(0);
    ui.horizontal(|ui| {
        for (index, label) in ["Color", "LUT", "Shot Match", "Info"]
            .into_iter()
            .enumerate()
        {
            if crate::ui::kui::segmented_button(ui, active == index, label, label).clicked() {
                active = index;
            }
        }
    });
    ctx.data_mut(|data| data.insert_temp(egui::Id::new(INSPECTOR_TAB), active));
    crate::ui::kui::separator(ui);

    egui::ScrollArea::vertical()
        .id_salt("color_workspace_controls_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| match active {
            0 => crate::ui::lumetri_color::draw_workspace_controls(app, ui),
            1 => crate::ui::color_management::draw_lut_controls(app, ui),
            2 => crate::ui::kui::empty_state(ui, "Choose a reference shot to match.", 120.0),
            _ => draw_composition_info(app, ui),
        });
}

fn draw_composition_info(app: &KagariApp, ui: &mut egui::Ui) {
    let comp = app.history.current().active_composition();
    crate::ui::kui::section(ui, "Composition");
    let grid = crate::ui::kui::PropertyGrid::new(
        ui,
        &["Name", "Resolution", "Frame rate", "Current frame"],
    );
    grid.row(ui, "Name", |ui| crate::ui::kui::secondary(ui, &comp.name));
    grid.row(ui, "Resolution", |ui| {
        crate::ui::kui::secondary(ui, format!("{} × {}", comp.width, comp.height))
    });
    grid.row(ui, "Frame rate", |ui| {
        crate::ui::kui::secondary(ui, format!("{} fps", comp.fps))
    });
    grid.row(ui, "Current frame", |ui| {
        crate::ui::kui::secondary(ui, app.playback.current_frame.to_string())
    });
}

fn draw_scopes(app: &KagariApp, ctx: &egui::Context, ui: &mut egui::Ui, compact: bool) {
    let mut mode = ctx
        .data(|data| data.get_temp::<u8>(egui::Id::new("color_workspace_scope_mode")))
        .unwrap_or(0);
    ui.horizontal(|ui| {
        crate::ui::kui::section_title(ui, "Scopes");
        ui.add_space(crate::ui::kui::space::SM);
        for (index, label) in ["All", "RGB Parade", "Vectorscope", "Histogram", "Waveform"]
            .into_iter()
            .enumerate()
        {
            if crate::ui::kui::segmented_button(ui, mode == index as u8, label, label).clicked() {
                mode = index as u8;
            }
        }
    });
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("color_workspace_scope_mode"), mode));
    crate::ui::kui::separator(ui);

    let Some(pixels) = crate::ui::color_scopes::frame_pixels(app, ctx) else {
        crate::ui::kui::empty_state(ui, "Preparing scopes…", 72.0);
        return;
    };
    if pixels.is_empty() {
        crate::ui::kui::empty_state(ui, "No frame is available for scopes.", 72.0);
        return;
    }
    if mode == 0 {
        ui.columns(3, |columns| {
            crate::ui::color_scopes::draw_parade(&mut columns[0], &pixels, compact);
            crate::ui::color_scopes::draw_vectorscope(&mut columns[1], &pixels, compact);
            crate::ui::color_scopes::draw_histogram(&mut columns[2], &pixels, compact);
        });
    } else {
        match mode {
            1 => crate::ui::color_scopes::draw_parade(ui, &pixels, compact),
            2 => crate::ui::color_scopes::draw_vectorscope(ui, &pixels, compact),
            3 => crate::ui::color_scopes::draw_histogram(ui, &pixels, compact),
            _ => crate::ui::color_scopes::draw_waveform(ui, &pixels, compact),
        }
    }
}
