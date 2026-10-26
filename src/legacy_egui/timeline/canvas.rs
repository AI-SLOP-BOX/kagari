use super::{
    draw_layer_waveform, header, layout, media_preview, pending_actions,
    state::timeline_derived_data, waveform_cache, LayerWaveformParams,
    MAX_TIMELINE_THUMBNAILS_PER_CLIP, TIMELINE_THUMBNAIL_WIDTH,
};
use crate::core::timeline::LabelColor;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;
use header::draw_timeline_header;

mod clip_draw;
mod clip_interaction;
mod dock_state;
mod graph_mode;
mod layer_context_menu;
mod ruler_area;
mod ruler_draw;
mod switches_col;

pub fn draw(app: &mut KagariApp, ctx: &egui::Context, current_frame: &mut u32, total_frames: u32) {
    let screen_height = ctx.screen_rect().height();
    let narrow_timeline = ctx.screen_rect().width() < 950.0;
    // Keep the timeline large enough to show several tracks without forcing the
    // user to resize the dock on every launch.
    let (timeline_default_height, timeline_min_height, viewer_min_height) = if narrow_timeline {
        (
            (screen_height * 0.30).clamp(160.0, 250.0),
            (screen_height * 0.15).clamp(80.0, 140.0),
            (screen_height * 0.35).clamp(180.0, 320.0),
        )
    } else {
        (
            (screen_height * 0.38).clamp(200.0, 340.0),
            (screen_height * 0.15).clamp(90.0, 150.0),
            (screen_height * 0.40).clamp(200.0, 420.0),
        )
    };
    let timeline_max_height =
        (screen_height * 0.94).min((screen_height - viewer_min_height).max(timeline_min_height));
    egui::TopBottomPanel::bottom("timeline_panel_editor_v12")
        .resizable(true)
        .default_height(timeline_default_height)
        .min_height(timeline_min_height)
        .max_height(timeline_max_height)
        .show(ctx, |ui| {
            if dock_state::draw_special_state(app, ui) {
                return;
            }

            let mut project_changed = false;
            // A full project clone is only needed while a pointer gesture can
            // mutate the timeline. Cloning merely because Graph Editor is
            // visible made an idle editor pay the snapshot cost every frame.
            let pointer_gesture = ui.input(|i| {
                i.pointer.any_pressed() || i.pointer.any_down() || i.pointer.any_released()
            });
            let pre_edit_snapshot = if !app.drag_active() && pointer_gesture {
                Some(app.history.current().clone())
            } else {
                None
            };
            let _compact_mode = ui.ctx().data_mut(|d| {
                *d.get_temp_mut_or_insert_with(egui::Id::new("ae_compact_timeline"), || false)
            });
            let mut actions = pending_actions::TimelineActions::default();

            crate::ui::timeline::breadcrumb::draw_comp_breadcrumb(app, ui);

            let mut header_state = header::TimelineHeaderState {
                bottom_dock_tab: &mut app.ui_tabs.bottom_dock_tab,
                show_switches_pane: &mut app.ui_tabs.show_switches_pane,
                timeline_zoom: &mut app.timeline_zoom,
                snap_to_keyframes: &mut app.snap_to_keyframes,
                show_graph_editor: &mut app.show_graph_editor,
                layer_filter_text: &mut app.ui_tabs.layer_filter_text,
                timeline_view_start: &mut app.timeline_view_start,
                work_area_in: &mut app.playback.work_area_in,
                work_area_out: &mut app.playback.work_area_out,
                expanded_layers: &mut app.selection.expanded_layers,
                fit_to_selection: &mut app.timeline_fit_to_selection,
                fit_all: &mut app.timeline_fit_all,
            };

            // Access live project mutably without per-frame cloning
            {
                let comp_mut = app.history.current_mut().active_composition_mut();
                if draw_timeline_header(&mut header_state, ui, comp_mut, current_frame, total_frames) {
                    project_changed = true;
                }

                // ── Fit to Selection: zoom timeline to selected layers' time range ──
                if app.timeline_fit_to_selection {
                    app.timeline_fit_to_selection = false;
                    let comp = app.history.current().active_composition();
                    let layers = if app.timeline_fit_all {
                        comp.layers.iter().enumerate().collect::<Vec<_>>()
                    } else {
                        let sel = &app.selection.selected_layers;
                        comp.layers.iter().enumerate().filter(|(i, _)| sel.contains(i)).collect::<Vec<_>>()
                    };
                    if !layers.is_empty() {
                        let mut min_f = u32::MAX;
                        let mut max_f = 0u32;
                        for &(_, layer) in &layers {
                            min_f = min_f.min(layer.in_frame);
                            max_f = max_f.max(layer.out_frame);
                        }
                        if max_f > min_f {
                            let span = (max_f - min_f).max(10);
                            app.timeline_zoom = (total_frames as f32 / span as f32).clamp(0.1, 20.0);
                            app.timeline_view_start = min_f;
                        }
                    }
                    app.timeline_fit_all = false;
                }
            }


            let timeline_history_generation = app.history.generation();
            // Rebuild while a model-edit transaction or row reorder is live,
            // because keyframe frames and layer indices can change mid-gesture.
            // Unrelated pointer activity (hover, scroll, toolbar clicks) does
            // not invalidate these derived lists and should keep using cache.
            let timeline_interacting = app.drag_active() || app.dragging_layer.is_some();
            let derived_timeline = timeline_derived_data(
                ui.ctx(),
                app.history.current().active_composition(),
                timeline_history_generation,
                timeline_interacting,
            );

            let mut automation_curve = app
                .production_document
                .as_ref()
                .and_then(|document| app.selected_automation_binding
                    .and_then(|index| document.bindings.get(index)))
                .map(|binding| binding.curve.clone());
            // Graph mode is the animation curve workspace; dependency nodes are
            // intentionally kept out of the compositing timeline.
            if app.show_graph_editor {
                graph_mode::draw(
                    app,
                    ui,
                    &mut automation_curve,
                    &mut project_changed,
                    pre_edit_snapshot,
                );
                return;
            }

            let temp_project = app.history.current_mut();

            // ── Pre-collect sub-comp layer data (before mutable borrow) ──
            let precomp_children = crate::ui::timeline::precomp_children::collect_cached(
                ui.ctx(),
                temp_project,
                &app.selection.expanded_layers,
                app.project_session_id,
                timeline_history_generation,
            );

            // ── Tracks Mode: Layer List & Keyframe Ruler Area ──
            let comp = temp_project.active_composition_mut();

            let mut ruler_state = ruler_area::RulerAreaState {
                frame_cache: &app.frame_cache,
                work_area_in: &mut app.playback.work_area_in,
                work_area_out: &mut app.playback.work_area_out,
                selected_layer_idx: app.selection.selected_layer_idx,
                snap_to_keyframes: app.snap_to_keyframes,
                timeline_view_start: &mut app.timeline_view_start,
                timeline_zoom: &mut app.timeline_zoom,
                global_shy_active: &mut app.ui_tabs.global_shy_active,
                toasts: &mut app.toasts,
                actions: &mut actions,
                comp,
                current_frame,
                total_frames,
                keyframe_frames: &derived_timeline.keyframe_frames,
                project_changed: &mut project_changed,
            };
            let (total_w, left_pane_w) = ruler_area::draw(ui, &mut ruler_state);

            crate::ui::theme::draw_separator(ui);

            // ── Scrollable Layer Rows & Property Tracks ──
            let zoom_span = (total_frames as f32 / app.timeline_zoom.max(0.01)).max(10.0) as u32;
            let start_frame = app.timeline_view_start;
            // Capture effect drag info before closure to avoid borrow conflicts
            let drag_info = app.dragging_effect.clone();
            // Collect pending effect drops to apply after the closure

            let layer_scroll_height = (ui.available_height() - 4.0).max(32.0);
            let _scroll_resp = egui::ScrollArea::vertical()
                .max_height(layer_scroll_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                let layers_len = comp.layers.len();
                let comp_w_f = comp.width as f32;
                let comp_h_f = comp.height.max(1) as f32;
                let derived = &derived_timeline;
                let layer_edges = &derived.layer_edges;
                let all_kf_frames = &derived.keyframe_frames;
                if layers_len == 0 {
                    ui.add_space(4.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("素材を追加するか、ここにドロップして編集を始めます")
                                .small()
                                .color(colors::TEXT_MUTED),
                        );
                    });
                    ui.add_space(4.0);
                }
                for i in 0..layers_len {
                    let layer_preview_textures = comp.layers.get(i).and_then(|layer| {
                        if !media_preview::layer_has_timeline_preview(&layer.layer_type)
                            || (app.ui_tabs.global_shy_active && layer.is_shy)
                            || (!app.ui_tabs.layer_filter_text.is_empty()
                                && !layer
                                    .name
                                    .to_lowercase()
                                    .contains(&app.ui_tabs.layer_filter_text.to_lowercase()))
                        {
                            return None;
                        }
                        let mut layer_height = layout::LAYER_ROW_HEIGHT;
                        if app.selection.expanded_layers.contains(&i) {
                            layer_height +=
                                (5 + layer.effects.len() + layer.masks.len()) as f32
                                    * layout::PROPERTY_ROW_HEIGHT;
                        }
                        let row_probe = egui::Rect::from_min_size(
                            ui.cursor().min,
                            egui::vec2(ui.available_width(), layer_height),
                        );
                        if !ui.is_rect_visible(row_probe) {
                            return None;
                        }
                        let visible_start = layer.in_frame.max(start_frame);
                        let visible_end = layer
                            .out_frame
                            .min(start_frame.saturating_add(zoom_span));
                        let visible_span = visible_end.saturating_sub(visible_start);
                        if visible_span == 0 {
                            return None;
                        }
                        let time_canvas_width =
                            (ui.available_width() - left_pane_w).max(0.0);
                        let visible_width = visible_span as f32 / zoom_span.max(1) as f32
                            * time_canvas_width;
                        let sample_count = (visible_width / TIMELINE_THUMBNAIL_WIDTH)
                            .ceil()
                            .clamp(1.0, MAX_TIMELINE_THUMBNAILS_PER_CLIP as f32)
                            as usize;
                        let clip_span = layer.out_frame.saturating_sub(layer.in_frame).max(1) as f32;
                        let preview_start = visible_start
                            .saturating_sub(layer.in_frame) as f32
                            / clip_span;
                        let preview_end = visible_end
                            .saturating_sub(layer.in_frame) as f32
                            / clip_span;
                        Some((
                            preview_start.clamp(0.0, 1.0),
                            preview_end.clamp(0.0, 1.0),
                            (0..sample_count)
                                .map(|sample| {
                                    let sample_t = (sample as f32 + 0.5) / sample_count as f32;
                                    let frame = visible_start.saturating_add(
                                        (visible_span as f32 * sample_t).round() as u32,
                                    );
                                    media_preview::timeline_layer_preview_texture(
                                        ui.ctx(),
                                        comp,
                                        layer,
                                        frame,
                                    )
                                })
                                .collect::<Vec<_>>(),
                        ))
                    });
                    // Safe index access (.get_mut(i))
                    if let Some(layer) = comp.layers.get_mut(i) {
                        if app.ui_tabs.global_shy_active && layer.is_shy {
                            continue;
                        }
                        if !app.ui_tabs.layer_filter_text.is_empty() && !layer.name.to_lowercase().contains(&app.ui_tabs.layer_filter_text.to_lowercase()) {
                            continue;
                        }

                        // Calculate exact estimated height for this layer based on expanded rows
                        let mut layer_height = layout::LAYER_ROW_HEIGHT;
                        if app.selection.expanded_layers.contains(&i) {
                            let prop_count = 5 + layer.effects.len() + layer.masks.len();
                            layer_height += prop_count as f32 * layout::PROPERTY_ROW_HEIGHT;
                        }

                        // ── Visibility Culling: skip off-screen rows ──
                        let row_probe = egui::Rect::from_min_size(
                            ui.cursor().min,
                            egui::vec2(ui.available_width(), layer_height),
                        );
                        if !ui.is_rect_visible(row_probe) {
                            // Still advance cursor by exact layer height to keep scroll extent accurate
                            ui.add_space(layer_height);
                            continue;
                        }

                        let row_columns = layout::TimelineColumns::new(
                            total_w,
                            left_pane_w,
                            ui.spacing().item_spacing.x,
                        );

                        ui.horizontal(|ui| {
                            // Denser rows: shrink button padding for the pill
                            // controls (Switches/Modes/reorder) in this row.
                            ui.style_mut().spacing.button_padding = egui::vec2(4.0, 1.0);
                            ui.allocate_ui(egui::vec2(left_pane_w, layout::LAYER_ROW_HEIGHT), |ui| {
                                ui.set_min_width(left_pane_w);
                                ui.spacing_mut().item_spacing.y = 0.0;
                                ui.spacing_mut().item_spacing.x = if left_pane_w < 240.0 { 2.0 } else { 4.0 };
                                ui.spacing_mut().interact_size.y = layout::LAYER_ROW_HEIGHT;
                                ui.spacing_mut().button_padding = egui::vec2(2.0, 0.0);
                                ui.horizontal(|ui| {
                                    if left_pane_w >= 240.0 {
                                        ui.label(egui::RichText::new(format!("{:02}", i + 1)).small().strong().color(colors::TEXT_SECONDARY));
                                        ui.add_space(2.0);
                                    }

                                    // ── Layer type icon ──
                                    {
let type_icon = crate::ui::icons::layer_icon(&layer.layer_type);
                                        crate::ui::kui::icon_glyph(
                                            ui,
                                            type_icon,
                                            14.0,
                                            colors::TEXT_SECONDARY,
                                        );
                                    }
                                    ui.add_space(1.0);

                                    // ── Label color chip: click cycles through AE label colors ──
                                    {
                                        let rgb = layer.label.to_rgb();
                                        let chip_color = egui::Color32::from_rgb(
                                            (rgb[0] * 255.0) as u8,
                                            (rgb[1] * 255.0) as u8,
                                            (rgb[2] * 255.0) as u8,
                                        );
                                        let (chip_rect, chip_resp) = ui.allocate_exact_size(
                                            egui::vec2(10.0, 14.0),
                                            egui::Sense::click(),
                                        );
                                        ui.painter().rect_filled(chip_rect, 2.0, chip_color);
                                        let row_selected = app.selection.selected_layers.contains(&i) || app.selection.selected_layer_idx == Some(i);
                                        if row_selected {
                                            ui.painter().rect_stroke(chip_rect, 2.0, egui::Stroke::new(1.0_f32, egui::Color32::WHITE));
                                        }
                                        if chip_resp.clicked() {
                                            let next = match layer.label {
                                                crate::core::timeline::LabelColor::None => crate::core::timeline::LabelColor::Red,
                                                crate::core::timeline::LabelColor::Red => crate::core::timeline::LabelColor::Yellow,
                                                crate::core::timeline::LabelColor::Yellow => crate::core::timeline::LabelColor::Aqua,
                                                crate::core::timeline::LabelColor::Aqua => crate::core::timeline::LabelColor::Pink,
                                                crate::core::timeline::LabelColor::Pink => crate::core::timeline::LabelColor::Lavender,
                                                crate::core::timeline::LabelColor::Lavender => crate::core::timeline::LabelColor::Peach,
                                                crate::core::timeline::LabelColor::Peach => crate::core::timeline::LabelColor::Sea,
                                                crate::core::timeline::LabelColor::Sea => crate::core::timeline::LabelColor::Blue,
                                                crate::core::timeline::LabelColor::Blue => crate::core::timeline::LabelColor::Purple,
                                                crate::core::timeline::LabelColor::Purple => crate::core::timeline::LabelColor::None,
                                            };
                                            // Direct mutation: layer is already &mut from the row loop
                                            layer.label = next;
                                            project_changed = true;
                                        }
                                        if chip_resp.secondary_clicked() {
                                            actions.pending_select_label_group = Some(layer.label);
                                        }
                                        if chip_resp.hovered() {
                                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                        }
                                    }
                                    ui.add_space(2.0);

                                    ui.add_space(if left_pane_w < 240.0 { 1.0 } else { 4.0 });
                                    let is_expanded = app.selection.expanded_layers.contains(&i);
                                    let arrow = if is_expanded { "⌄" } else { "›" };
                                    let (arrow_rect, arrow_response) = ui.allocate_exact_size(
                                        egui::vec2(if left_pane_w < 240.0 { 14.0 } else { 20.0 }, 22.0),
                                        egui::Sense::click(),
                                    );
                                    if arrow_response.hovered() {
                                        ui.painter().rect_filled(arrow_rect, 3.0, colors::BG_HOVER);
                                    }
                                    ui.painter().text(
                                        arrow_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        arrow,
                                        egui::FontId::proportional(15.0),
                                        if is_expanded { colors::TEXT_PRIMARY } else { colors::TEXT_MUTED },
                                    );
                                    if arrow_response.clicked() {
                                        if is_expanded {
                                            app.selection.expanded_layers.remove(&i);
                                        } else {
                                            app.selection.expanded_layers.insert(i);
                                        }
                                    }

                                    if left_pane_w >= 240.0 {
                                    ui.scope(|ui| {
                                    let widgets = &mut ui.style_mut().visuals.widgets;
                                    widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                                    widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                    widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                                    widgets.hovered.bg_fill = colors::BG_HOVER;
                                    widgets.hovered.bg_stroke = egui::Stroke::NONE;
                                    widgets.hovered.weak_bg_fill = colors::BG_HOVER;
                                    ui.menu_button(egui::RichText::new("色").small(), |ui| {
                                    // ── AE Layer Color Label Square Picker ──
                                    let label_rgb = layer.label.to_rgb();
                                    let label_c32 = egui::Color32::from_rgb(
                                        (label_rgb[0] * 255.0) as u8,
                                        (label_rgb[1] * 255.0) as u8,
                                        (label_rgb[2] * 255.0) as u8,
                                    );
                                    let (lbl_rect, _) = ui.allocate_exact_size(
                                        egui::vec2(12.0, 12.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(lbl_rect, 1.0, label_c32);
                                    let lbl_resp = ui.interact(
                                        lbl_rect.expand(
                                            crate::ui::theme::layout::HIT_TARGET_MIN * 0.5 - 6.0,
                                        ),
                                        ui.make_persistent_id(("layer_label_chip", layer.id.as_str())),
                                        egui::Sense::click(),
                                    );
                                    lbl_resp.context_menu(|ui| {
                                        ui.label("Label Color:");
                                        for label_choice in [
                                            LabelColor::Red, LabelColor::Yellow, LabelColor::Aqua,
                                            LabelColor::Pink, LabelColor::Lavender, LabelColor::Peach,
                                            LabelColor::Sea, LabelColor::Blue, LabelColor::Purple
                                        ] {
                                            if ui.button(format!("{:?}", label_choice)).clicked() {
                                                layer.label = label_choice;
                                                project_changed = true;
                                                ui.close_menu();
                    }
                }
                // ── Click empty area below layers → deselect all ──
                let remaining = ui.available_rect_before_wrap();
                let bg_resp = ui.interact(remaining, egui::Id::new("timeline_bg_deselect"), egui::Sense::click());
                if bg_resp.clicked() {
                    app.selection.selected_layers.clear();
                    app.selection.selected_layer_idx = None;
                }
            });

                                    if !app.ui_tabs.show_switches_pane {
                                    // Track Matte Mode Selection (AE Standard)
                                    let matte_id = ui.make_persistent_id(format!("tm_combo_{}", i));
                                    let matte_label = match layer.track_matte {
                                        crate::core::timeline::TrackMatteMode::AlphaMatte => "Alpha Matte",
                                        crate::core::timeline::TrackMatteMode::AlphaMatteInverted => "Alpha Inverted",
                                        crate::core::timeline::TrackMatteMode::LumaMatte => "Luma Matte",
                                        crate::core::timeline::TrackMatteMode::LumaMatteInverted => "Luma Inverted",
                                        crate::core::timeline::TrackMatteMode::None => "No Matte",
                                    };
                                    egui::ComboBox::from_id_salt(matte_id)
                                        .selected_text(matte_label)
                                        .show_ui(ui, |ui| {
                                            if ui.selectable_value(&mut layer.track_matte, crate::core::timeline::TrackMatteMode::None, "No Matte").clicked() {
                                                project_changed = true;
                                            }
                                            if ui.selectable_value(&mut layer.track_matte, crate::core::timeline::TrackMatteMode::AlphaMatte, "Alpha Matte").clicked() {
                                                project_changed = true;
                                            }
                                            if ui.selectable_value(&mut layer.track_matte, crate::core::timeline::TrackMatteMode::AlphaMatteInverted, "Alpha Inverted").clicked() {
                                                project_changed = true;
                                            }
                                            if ui.selectable_value(&mut layer.track_matte, crate::core::timeline::TrackMatteMode::LumaMatte, "Luma Matte").clicked() {
                                                project_changed = true;
                                            }
                                            if ui.selectable_value(&mut layer.track_matte, crate::core::timeline::TrackMatteMode::LumaMatteInverted, "Luma Inverted").clicked() {
                                                project_changed = true;
                                            }
                                        });

                                    // Blend Mode Selection (AE Standard)
                                    let blend_id = ui.make_persistent_id(format!("bm_combo_{}", i));
                                    let blend_label = match layer.blend_mode {
                                        crate::core::timeline::BlendMode::Normal => "Normal",
                                        crate::core::timeline::BlendMode::Multiply => "Multiply",
                                        crate::core::timeline::BlendMode::Screen => "Screen",
                                        crate::core::timeline::BlendMode::Overlay => "Overlay",
                                        crate::core::timeline::BlendMode::Add => "Add",
                                        crate::core::timeline::BlendMode::Darken => "Darken",
                                        crate::core::timeline::BlendMode::Lighten => "Lighten",
                                        crate::core::timeline::BlendMode::SoftLight => "Soft Light",
                                        crate::core::timeline::BlendMode::HardLight => "Hard Light",
                                        crate::core::timeline::BlendMode::Difference => "Difference",
                                        crate::core::timeline::BlendMode::Exclusion => "Exclusion",
                                        crate::core::timeline::BlendMode::Divide => "Divide",
                                        crate::core::timeline::BlendMode::Subtract => "Subtract",
                                            crate::core::timeline::BlendMode::ColorBurn => "Color Burn",
                                            crate::core::timeline::BlendMode::LinearBurn => "Linear Burn",
                                            crate::core::timeline::BlendMode::VividLight => "Vivid Light",
                                            crate::core::timeline::BlendMode::ColorDodge => "Color Dodge",
                                            crate::core::timeline::BlendMode::LinearDodge => "Linear Dodge",
                                            crate::core::timeline::BlendMode::Color => "Color",
                                            crate::core::timeline::BlendMode::Hue => "Hue",
                                            crate::core::timeline::BlendMode::Saturation => "Saturation",
                                            crate::core::timeline::BlendMode::Luminosity => "Luminosity",
                                            crate::core::timeline::BlendMode::StencilAlpha => "Stencil Alpha",
                                            crate::core::timeline::BlendMode::StencilLuma => "Stencil Luma",
                                            crate::core::timeline::BlendMode::SilhouetteAlpha => "Silhouette Alpha",
                                            crate::core::timeline::BlendMode::SilhouetteLuma => "Silhouette Luma",
                                            crate::core::timeline::BlendMode::Behind => "Behind",
                                            crate::core::timeline::BlendMode::AlphaAdd => "Alpha Add",
                                            crate::core::timeline::BlendMode::LinearLight => "Linear Light",
                                    };
                                    egui::ComboBox::from_id_salt(blend_id)
                                        .selected_text(blend_label)
                                        .show_ui(ui, |ui| {
                                            for (bm, name) in [
                                                (crate::core::timeline::BlendMode::Normal, "Normal"),
                                                (crate::core::timeline::BlendMode::Multiply, "Multiply"),
                                                (crate::core::timeline::BlendMode::Screen, "Screen"),
                                                (crate::core::timeline::BlendMode::Overlay, "Overlay"),
                                                (crate::core::timeline::BlendMode::Add, "Add"),
                                                (crate::core::timeline::BlendMode::Darken, "Darken"),
                                                (crate::core::timeline::BlendMode::Lighten, "Lighten"),
                                                (crate::core::timeline::BlendMode::SoftLight, "Soft Light"),
                                                (crate::core::timeline::BlendMode::HardLight, "Hard Light"),
                                                (crate::core::timeline::BlendMode::Difference, "Difference"),
                                                (crate::core::timeline::BlendMode::Exclusion, "Exclusion"),
                                                (crate::core::timeline::BlendMode::Divide, "Divide"),
                                                (crate::core::timeline::BlendMode::Subtract, "Subtract"),
                                                (crate::core::timeline::BlendMode::ColorBurn, "Color Burn"),
                                                (crate::core::timeline::BlendMode::LinearBurn, "Linear Burn"),
                                                (crate::core::timeline::BlendMode::VividLight, "Vivid Light"),
                                                (crate::core::timeline::BlendMode::ColorDodge, "Color Dodge"),
                                                (crate::core::timeline::BlendMode::LinearDodge, "Linear Dodge"),
                                                (crate::core::timeline::BlendMode::Color, "Color"),
                                                (crate::core::timeline::BlendMode::Hue, "Hue"),
                                                (crate::core::timeline::BlendMode::Saturation, "Saturation"),
                                                (crate::core::timeline::BlendMode::Luminosity, "Luminosity"),
                                                (crate::core::timeline::BlendMode::StencilAlpha, "Stencil Alpha"),
                                                (crate::core::timeline::BlendMode::StencilLuma, "Stencil Luma"),
                                                (crate::core::timeline::BlendMode::SilhouetteAlpha, "Silhouette Alpha"),
                                                (crate::core::timeline::BlendMode::SilhouetteLuma, "Silhouette Luma"),
                                                (crate::core::timeline::BlendMode::Behind, "Behind"),
                                                (crate::core::timeline::BlendMode::AlphaAdd, "Alpha Add"),
                                                (crate::core::timeline::BlendMode::LinearLight, "Linear Light"),
                                            ] {
                                                if ui.selectable_value(&mut layer.blend_mode, bm, name).clicked() {
                                                    project_changed = true;
                                                }
                                            }
                                        });
                                    }


                                    });
                                    });
                                    }
                                    let is_selected = app.selection.selected_layers.contains(&i) || app.selection.selected_layer_idx == Some(i);
                                    let name_width = layout::TimelineColumns::layer_name_width(
                                        ui.available_width(),
                                        app.ui_tabs.show_switches_pane,
                                    );
                                    let (name_rect, click_resp) = ui.allocate_exact_size(
                                        egui::vec2(name_width, layout::LAYER_ROW_HEIGHT),
                                        egui::Sense::click_and_drag(),
                                    );
                                    if is_selected || click_resp.hovered() {
                                        let fill = if is_selected {
                                            egui::Color32::from_rgba_premultiplied(45, 81, 112, 82)
                                        } else {
                                            colors::BG_HOVER.linear_multiply(0.55)
                                        };
                                        ui.painter().rect_filled(name_rect, 3.0, fill);
                                    }
                                    if is_selected {
                                        ui.painter().rect_filled(
                                            egui::Rect::from_min_size(
                                                name_rect.left_top(),
                                                egui::vec2(2.0, name_rect.height()),
                                            ),
                                            1.0,
                                            colors::ACCENT_SELECTION,
                                        );
                                    }
                                    ui.painter().with_clip_rect(name_rect.shrink2(egui::vec2(6.0, 0.0))).text(
                                        egui::pos2(name_rect.left() + 8.0, name_rect.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        &layer.name,
                                        egui::FontId::proportional(12.0),
                                        if is_selected { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY },
                                    );
                                    let click_resp = click_resp.on_hover_text(&layer.name);

                                    if app.ui_tabs.show_switches_pane {
                                        switches_col::draw_switches(ui, layer, i, &mut project_changed);
                                    }

                                    // ── Pick Whip: clicking a layer in pick mode sets it as parent ──
                                    if click_resp.clicked() && app.pick_whip_mode {
                                        if let Some(sel) = app.selection.selected_layer_idx {
                                            if sel != i {
                                                actions.pending_pick_whip = Some((sel, i));
                                            }
                                        }
                                        app.pick_whip_mode = false;
                                        app.pick_whip_target = None;
                                    }

                                    if click_resp.double_clicked() {
                                        // Double-click a PreComp layer opens its nested composition (AE parity)
                                        // (deferred: resolved after the row loop)
                                        if let crate::core::timeline::LayerType::PreComp { comp_id } = &layer.layer_type {
                                            actions.pending_open_comp = Some(comp_id.clone());
                                        } else {
                                            app.renaming_layer = Some(i);
                                        }
                                    }
                                    if app.renaming_layer == Some(i) {
                                        let mut name_buf = layer.name.clone();
                                        let rename_resp = ui.text_edit_singleline(&mut name_buf);
                                        if rename_resp.lost_focus() || ui.input(|inp| inp.key_pressed(egui::Key::Enter)) {
                                            if !name_buf.is_empty() && name_buf != layer.name {
                                                layer.name = name_buf;
                                                project_changed = true;
                                            }
                                            app.renaming_layer = None;
                                        }
                                    }

                                    // ── Drag & drop reordering (live, AE-style) ──
                                    if click_resp.dragged() && app.dragging_layer.is_none() {
                                        app.dragging_layer = Some(i);
                                    }
                                    if let Some(src) = app.dragging_layer {
                                        if click_resp.hovered() && src != i {
                                            // Live swap; the dragged row follows the pointer
                                            actions.swap_request = Some((src, i));
                                            app.dragging_layer = Some(i);
                                        }
                                    }
                                    if click_resp.drag_stopped() || (app.dragging_layer.is_some() && !click_resp.dragged()) {
                                        // Drag ended — clear state (row index moved with the pointer)
                                        app.dragging_layer = None;
                                    }

                                    // ── Effect drop zone: drag from Effects & Presets library ──
                                    if let Some((ref effect_name, preset_idx)) = drag_info {
                                        if click_resp.hovered() {
                                            // Visual drop indicator: blue glow on the layer row
                                            let row_rect = click_resp.rect;
                                             ui.painter().rect_stroke(
                                                 row_rect, 4.0,
                                                 egui::Stroke::new(2.0_f32, colors::ACCENT_BLUE)
                                             );
                                             ui.painter().rect_filled(
                                                 row_rect, 4.0,
                                                 colors::TIMELINE_SELECTION
                                             );
                                        }
                                        // Collect effect on drop (apply after loop to avoid borrow conflicts)
                                        if click_resp.drag_stopped() && click_resp.hovered() {
                                            actions.pending_effect_drops.push((i, effect_name.clone(), preset_idx));
                                            app.dragging_effect = None;
                                        }
                                    }

                                    if click_resp.clicked() {
                                        let modifiers = ui.input(|inp| inp.modifiers);
                                        if modifiers.shift || modifiers.command || modifiers.ctrl {
                                            if app.selection.selected_layers.contains(&i) {
                                                app.selection.selected_layers.remove(&i);
                                                if app.selection.selected_layer_idx == Some(i) {
                                                    app.selection.selected_layer_idx = app.selection.selected_layers.iter().next().copied();
                                                }
                                            } else {
                                                app.selection.selected_layers.insert(i);
                                                app.selection.selected_layer_idx = Some(i);
                                            }
                                        } else {
                                            app.selection.selected_layers.clear();
                                            app.selection.selected_layers.insert(i);
                                            app.selection.selected_layer_idx = Some(i);
                                        }
                                    }
                                    click_resp.context_menu(|ui| {
                                        let mut menu_ctx = layer_context_menu::LayerContextMenuCtx {
                                            selected_layers: &app.selection.selected_layers,
                                            toasts: &mut app.toasts,
                                        };
                                        layer_context_menu::draw_layer_context_menu(
                                            ui,
                                            &mut menu_ctx,
                                            layer,
                                            i,
                                            layers_len,
                                            *current_frame,
                                            comp_w_f,
                                            &mut actions,
                                            &mut project_changed,
                                        );
                                    });
                                    ui.style_mut().visuals.override_text_color = None;

                                    switches_col::draw_modes_and_parenting(
                                        ui,
                                        switches_col::SwitchesColParams {
                                            layer,
                                            i,
                                            show_switches_pane: app.ui_tabs.show_switches_pane,
                                            derived,
                                            pick_whip_mode: &mut app.pick_whip_mode,
                                            pick_whip_target: &mut app.pick_whip_target,
                                            toasts: &mut app.toasts,
                                        },
                                        &mut project_changed,
                                    );
                                });
                            });

                            // Render Layer Bar Span & Waveform + parent link line
                            let avail_w = ui.available_width().min(row_columns.time_width).max(0.0);
                            let (bar_rect, _bar_sense) = ui.allocate_exact_size(
                                egui::vec2(avail_w, layout::LAYER_ROW_HEIGHT),
                                egui::Sense::hover().union(egui::Sense::drag()),
                            );
                            let layer_rect = clip_draw::draw_clip_bar(
                                ui,
                                clip_draw::ClipBarParams {
                                    layer,
                                    selected: app.selection.selected_layer_idx == Some(i),
                                    has_parent: layer
                                        .parent_id
                                    .as_ref()
                                        .is_some_and(|parent| {
                                            derived.parent_indices.contains_key(parent)
                                        }),
                                    bar_rect,
                                    start_frame,
                                    zoom_span,
                                    comp_width: comp_w_f,
                                    comp_height: comp_h_f,
                                    fps: comp.fps,
                                    preview_textures: layer_preview_textures.as_ref(),
                                },
                            );

                            clip_interaction::handle_layer_bar_drag(
                                ui,
                                clip_interaction::LayerBarDragParams {
                                    layer,
                                    i,
                                    layers_len,
                                    layer_rect,
                                    bar_rect,
                                    start_frame,
                                    zoom_span,
                                    total_frames,
                                    fps: comp.fps,
                                    snap_to_keyframes: app.snap_to_keyframes,
                                    layer_edges,
                                    current_frame: *current_frame,
                                },
                                &mut actions,
                                &mut project_changed,
                            );
                        });

                        // ── Expanded waveform lane ──
                        if app.expanded_waveform_layers.contains(&i) {
                            let audio_path: Option<String> = match &layer.layer_type {
                                crate::core::timeline::LayerType::Audio { path, .. } => Some(path.clone()),
                                crate::core::timeline::LayerType::Video { audio_wav: Some(w), .. } => Some(w.clone()),
                                _ => None,
                            };
                            if let Some(apath) = audio_path {
                                let wf_height = 48.0;
                                let (response, painter) = ui.allocate_painter(egui::vec2(ui.available_width(), wf_height), egui::Sense::hover());
                                let wf_rect = response.rect;
                                painter.rect_filled(wf_rect, 0.0, egui::Color32::from_rgba_premultiplied(20, 20, 25, 255));
                                if let Some(waveform) =
                                    waveform_cache::get(ui.ctx(), std::path::Path::new(&apath))
                                {
                                    let timeline_rect = egui::Rect::from_min_max(
                                        egui::pos2(wf_rect.left() + left_pane_w, wf_rect.top()),
                                        wf_rect.right_bottom(),
                                    );
                                    let start_norm =
                                        layer.in_frame.saturating_sub(start_frame) as f32
                                            / zoom_span.max(1) as f32;
                                    let end_norm =
                                        layer.out_frame.saturating_sub(start_frame) as f32
                                            / zoom_span.max(1) as f32;
                                    let clip_rect = egui::Rect::from_min_max(
                                        egui::pos2(
                                            timeline_rect.left() + start_norm * timeline_rect.width(),
                                            wf_rect.top(),
                                        ),
                                        egui::pos2(
                                            timeline_rect.left() + end_norm * timeline_rect.width(),
                                            wf_rect.bottom(),
                                        ),
                                    );
                                    draw_layer_waveform(
                                        &painter.with_clip_rect(clip_rect),
                                        LayerWaveformParams {
                                            timeline_rect,
                                            clip_rect,
                                            layer,
                                            waveform: &waveform,
                                            start_frame,
                                            visible_span: zoom_span,
                                            fps: comp.fps,
                                            color: colors::TIMELINE_WAVEFORM,
                                        },
                                    );
                                }
                            }
                        }

                        // If expanded, render transform properties & effects
                        if app.selection.expanded_layers.contains(&i) {
                            crate::ui::timeline::keyframe_rows::draw_expanded_rows(
                                ui,
                                crate::ui::timeline::keyframe_rows::ExpandedRowsState {
                                    layer,
                                    layer_index: i,
                                    selected_keyframes: &mut app.selected_keyframes,
                                    selected_property: &mut app.selection.selected_property,
                                    current_frame,
                                    start_frame,
                                    zoom_span,
                                    left_pane_w,
                                    all_kf_frames,
                                    precomp_children: &precomp_children,
                                    open_comp_request: &mut actions.pending_open_comp,
                                    project_changed: &mut project_changed,
                                },
                            );
                        }
                    }
                }
                ui.add_space(48.0);
            });

            crate::ui::timeline::pending_actions::apply_deferred(
                app,
                ui,
                *current_frame,
                &mut project_changed,
                &mut actions,
            );

            if project_changed {
                // Transactional undo commit: snapshot on pointer-down, single entry on release
                let is_pointer_down = ui.input(|i| i.pointer.any_down());
                if is_pointer_down {
                    let edit_label = if app.show_graph_editor { "Graph Editor Edit" } else { "Timeline Edit" };
                    if let Some(snapshot) = pre_edit_snapshot {
                        app.begin_drag_with_snapshot(snapshot, edit_label);
                    } else if !app.drag_active() {
                        app.begin_drag(edit_label);
                    }
                } else {
                    app.commit_drag();
                }
                crate::core::frame_cache::bump_version();
            }
        });
}
