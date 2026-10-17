use super::{
    draw_layer_waveform, header, layout, media_preview, pending_actions,
    state::{timeline_derived_data, would_create_parent_cycle, LayerBodyDragState},
    waveform_cache, LayerWaveformParams, MAX_TIMELINE_THUMBNAILS_PER_CLIP,
    TIMELINE_THUMBNAIL_WIDTH,
};
use crate::core::timeline::{BlendMode, LabelColor, LayerType, TrackMatteMode};
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;
use header::draw_timeline_header;

mod clip_draw;
mod dock_state;
mod graph_mode;
mod ruler_area;
mod ruler_draw;

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
                let parent_choices_ref = &derived.parent_choices;
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
                                        use crate::ui::custom_widgets::ae_svg_toggle;
                                        use crate::ui::icons::*;

                                        let visibility_icon = if layer.visible { SVG_EYE_OPEN } else { SVG_EYE_CLOSED };
                                        if ae_svg_toggle(ui, &mut layer.visible, visibility_icon, &format!("row_vis_{i}"), egui::vec2(15.0, 15.0), colors::TEXT_PRIMARY, "表示／非表示 (V)").clicked() {
                                            project_changed = true;
                                        }
                                        let lock_icon = if layer.locked { SVG_LOCK } else { SVG_UNLOCK };
                                        if ae_svg_toggle(ui, &mut layer.locked, lock_icon, &format!("row_lock_{i}"), egui::vec2(15.0, 15.0), colors::ACCENT_TOGGLE, "ロック (L)").clicked() {
                                            project_changed = true;
                                        }
                                        if ae_svg_toggle(ui, &mut layer.is_shy, SVG_SHY, &format!("row_shy_{i}"), egui::vec2(15.0, 15.0), colors::ACCENT_CYAN, "シャイレイヤー (Shy)").clicked() {
                                            project_changed = true;
                                        }
                                        if ae_svg_toggle(ui, &mut layer.is_collapsed, SVG_COLLAPSE_TRANSFORM, &format!("row_col_{i}"), egui::vec2(15.0, 15.0), colors::ACCENT_ORANGE, "トランスフォームの折りたたみ／コラプス").clicked() {
                                            project_changed = true;
                                        }
                                        if ae_svg_toggle(ui, &mut layer.is_3d, SVG_3D_CUBE, &format!("row_3d_{i}"), egui::vec2(15.0, 15.0), colors::ACCENT_TOGGLE, "3Dレイヤー").clicked() {
                                            project_changed = true;
                                        }
                                        if ae_svg_toggle(ui, &mut layer.motion_blur, SVG_MOTION_BLUR, &format!("row_mb_{i}"), egui::vec2(15.0, 15.0), colors::ACCENT_TOGGLE, "モーションブラー").clicked() {
                                            project_changed = true;
                                        }
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
                                        if i > 0 && ui.button("↑ 上へ移動").clicked() {
                                            actions.swap_request = Some((i, i - 1));
                                            ui.close_menu();
                                        }
                                        if i + 1 < layers_len && ui.button("↓ 下へ移動").clicked() {
                                            actions.swap_request = Some((i, i + 1));
                                            ui.close_menu();
                                        }
                                        ui.separator();
                                        if ui.button("Duplicate Layer (Cmd+D)").clicked() {
                                            actions.pending_dup_layer = Some(i);
                                            app.toasts.info("Duplicated selected layer");
                                            ui.close_menu();
                                        }
                                        if ui.button("Pre-compose… (Cmd+Shift+C)").on_hover_text("Nest this layer into a new composition").clicked() {
                                            app.selection.selected_layers.clear();
                                            app.selection.selected_layers.insert(i);
                                            app.selection.selected_layer_idx = Some(i);
                                            app.show_precompose_dialog = true;
                                            ui.close_menu();
                                        }
                                        if matches!(layer.layer_type, crate::core::timeline::LayerType::Text { .. })
                                            && ui.button("Create Shapes from Text").on_hover_text("Decompose this Text layer into animatable vector Bezier Shape paths").clicked()
                                        {
                                            actions.pending_create_shapes = Some(i);
                                            ui.close_menu();
                                        }
                                        if ui.button("Toggle 3D Layer").on_hover_text("Switch this layer between 2D and 3D").clicked() {
                                            layer.is_3d = !layer.is_3d;
                                            project_changed = true;
                                            ui.close_menu();
                                        }
                                        if ui.button(if layer.is_guide_layer { "Remove Guide Layer" } else { "Guide Layer" }).on_hover_text("Guide layers appear in composition preview but are excluded from exports and pre-comps").clicked() {
                                            layer.is_guide_layer = !layer.is_guide_layer;
                                            project_changed = true;
                                            ui.close_menu();
                                        }
                                        if ui.button("⏪ Time-Reverse Keyframes").on_hover_text("Reverse the order of all animated keyframes on this layer").clicked() {
                                            fn rev<T: Clone>(a: &mut crate::core::property::Animatable<T>, in_f: u32, out_f: u32) {
                                                if let crate::core::property::Animatable::Animated(kfs) = a {
                                                    let span = out_f.saturating_sub(in_f);
                                                    for k in kfs.iter_mut() {
                                                        k.frame = in_f + span.saturating_sub(k.frame.saturating_sub(in_f));
                                                    }
                                                    kfs.sort_by_key(|k| k.frame);
                                                }
                                            }
                                            let (inf, outf) = (layer.in_frame, layer.out_frame);
                                            rev(&mut layer.transform.position, inf, outf);
                                            rev(&mut layer.transform.scale, inf, outf);
                                            rev(&mut layer.transform.rotation, inf, outf);
                                            rev(&mut layer.transform.opacity, inf, outf);
                                            project_changed = true;
                                            app.toasts.info("Keyframes time-reversed");
                                            ui.close_menu();
                                        }
                                        if ui.button("⏱ Enable Time Remapping (Cmd+Opt+T)").on_hover_text("Enable variable speed time remapping on this layer").clicked() {
                                            layer.enable_time_remapping();
                                            project_changed = true;
                                            app.toasts.info("Time remapping enabled for layer");
                                            ui.close_menu();
                                        }
                                        if ui.button("Split Layer at Current Time (Cmd+Shift+D)").clicked() {
                                            actions.pending_split_layer = Some(i);
                                            ui.close_menu();
                                        }
                                        ui.separator();
                                        if ui.button("Into ← Set In-Point to Current Time (I)").clicked() {
                                            layer.in_frame = *current_frame;
                                            if layer.in_frame >= layer.out_frame {
                                                layer.out_frame = layer.in_frame + 1;
                                            }
                                            project_changed = true;
                                            ui.close_menu();
                                        }
                                        if ui.button("→ Set Out-Point to Current Time (O)").clicked() {
                                            layer.out_frame = *current_frame;
                                            if layer.out_frame <= layer.in_frame {
                                                layer.in_frame = layer.out_frame.saturating_sub(1);
                                            }
                                            project_changed = true;
                                            ui.close_menu();
                                        }
                                        ui.separator();
                                        if ui.button("Add Layer Marker at Playhead").clicked() {
                                            actions.pending_layer_marker = Some(i);
                                            ui.close_menu();
                                        }
                                        if !layer.markers.is_empty()
                                            && ui.button("Clear Layer Markers").clicked() {
                                            actions.pending_clear_markers = Some(i);
                                            ui.close_menu();
                                        }
                                        if ui.button("⬚ Select All Keyframes").on_hover_text("Select every keyframe on this layer (transform, effects, pins)").clicked() {
                                            actions.pending_select_all_kfs = Some(i);
                                            ui.close_menu();
                                        }
                                        if ui.button("Select Label Group").on_hover_text("Select all layers with the same label color").clicked() {
                                            actions.pending_select_label_group = Some(layer.label);
                                            ui.close_menu();
                                        }
                                        // ── Animation Presets ──
                                        ui.menu_button("Animation Presets", |ui| {
                                            let cf = *current_frame;
                                            let mut last_cat = "";
                                            for name in crate::core::presets::NAMES {
                                                let cat = crate::core::presets::category_of(name);
                                                if cat != last_cat && !cat.is_empty() {
                                                    if !last_cat.is_empty() { ui.separator(); }
                                                    ui.label(egui::RichText::new(cat).strong().color(colors::TEXT_SECONDARY));
                                                    last_cat = cat;
                                                }
                                                if ui.small_button(*name).clicked() {
                                                    let ok = crate::core::presets::apply_by_name(name, layer, cf, comp_w_f, 0.0);
                                                    if ok {
                                                        project_changed = true;
                                                        app.toasts.info(format!("{} applied", name));
                                                    } else {
                                                        app.toasts.error(format!("{}: not applicable here", name));
                                                    }
                                                    ui.close_menu();
                                                    ui.close_menu();
                                                }
                                            }
                                            ui.separator();
                                            ui.label(egui::RichText::new("Motion Library (keyframe data)").strong().color(colors::TEXT_SECONDARY));
                                            for preset in crate::core::animation_presets::all_presets() {
                                                if ui.small_button(preset.name.clone()).on_hover_text(preset.description.clone()).clicked() {
                                                    let ok = crate::core::animation_presets::apply_preset_to_layer(&preset, layer, cf);
                                                    if ok {
                                                        project_changed = true;
                                                        app.toasts.info(format!("{} applied", preset.name));
                                                    } else {
                                                        app.toasts.error(format!("{}: not applicable here", preset.name));
                                                    }
                                                    ui.close_menu();
                                                    ui.close_menu();
                                                }
                                            }
                                        });
                                        if !layer.paint_strokes.is_empty()
                                            && ui.button("Clear All Paint Strokes").clicked() {
                                            layer.paint_strokes.clear();
                                            project_changed = true;
                                            app.toasts.info("All paint strokes cleared");
                                            ui.close_menu();
                                        }
                                        if ui.button("Pre-Compose Selected... (Cmd+Shift+C)").clicked() {
                                            let selected_indices: Vec<usize> = if !app.selection.selected_layers.is_empty() {
                                                let mut s: Vec<usize> = app.selection.selected_layers.iter().copied().collect();
                                                s.sort();
                                                s
                                            } else {
                                                vec![i]
                                            };
                                            actions.pending_precomp_indices = Some(selected_indices);
                                            ui.close_menu();
                                        }
                                        if ui.button("Reset Transform").clicked() {
                                            layer.transform = crate::core::timeline::Transform2D::default();
                                            project_changed = true;
                                            ui.close_menu();
                                        }
                                        // ── Audio fades (Audio layers only) ──
                                        if matches!(layer.layer_type, crate::core::timeline::LayerType::Audio { .. }) {
                                            ui.separator();
                                            const FADE_F: u32 = 10;
                                            if ui.button("Add 10-frame Audio Fade In").clicked() {
                                                if let crate::core::timeline::LayerType::Audio { volume, .. } = &mut layer.layer_type {
                                                    let base = volume.evaluate(*current_frame);
                                                    let start_f = layer.in_frame;
                                                    let end_f = (start_f + FADE_F).min(layer.out_frame);
                                                    let mut kfs: Vec<crate::core::keyframe::Keyframe<f32>> = vec![
                                                        crate::core::keyframe::Keyframe::new(start_f, 0.0, crate::core::keyframe::InterpolationType::Linear),
                                                        crate::core::keyframe::Keyframe::new(end_f.max(start_f + 1), base, crate::core::keyframe::InterpolationType::Linear),
                                                    ];
                                                    if let Some(old) = volume.keyframes() {
                                                        for k in old { if k.frame > end_f { kfs.push(k.clone()); } }
                                                    }
                                                    kfs.sort_by_key(|k| k.frame);
                                                    *volume = crate::core::property::Animatable::Animated(kfs);
                                                }
                                                project_changed = true;
                                                app.toasts.info("Audio fade-in added");
                                                ui.close_menu();
                                            }
                                            if ui.button("Add 10-frame Audio Fade Out").clicked() {
                                                if let crate::core::timeline::LayerType::Audio { volume, .. } = &mut layer.layer_type {
                                                    let base = volume.evaluate(*current_frame);
                                                    let end_f = layer.out_frame.saturating_sub(1);
                                                    let start_f = end_f.saturating_sub(FADE_F).max(layer.in_frame);
                                                    let mut kfs: Vec<crate::core::keyframe::Keyframe<f32>> = vec![
                                                        crate::core::keyframe::Keyframe::new(end_f, 0.0, crate::core::keyframe::InterpolationType::Linear),
                                                        crate::core::keyframe::Keyframe::new(start_f, base, crate::core::keyframe::InterpolationType::Linear),
                                                    ];
                                                    if let Some(old) = volume.keyframes() {
                                                        for k in old { if k.frame < start_f { kfs.push(k.clone()); } }
                                                    }
                                                    kfs.sort_by_key(|k| k.frame);
                                                    *volume = crate::core::property::Animatable::Animated(kfs);
                                                }
                                                project_changed = true;
                                                app.toasts.info("Audio fade-out added");
                                                ui.close_menu();
                                            }
                                        }

                                        ui.separator();
                                        let any_disabled = layer.effects.iter().any(|e| !e.enabled);
                                        let fx_label = if any_disabled { "Enable All Effects" } else { "Disable All Effects" };
                                        if ui.button(fx_label).clicked() {
                                            let target = any_disabled;
                                            for fx in layer.effects.iter_mut() {
                                                fx.enabled = target;
                                            }
                                            project_changed = true;
                                            ui.close_menu();
                                        }
                                    });
                                    ui.style_mut().visuals.override_text_color = None;

                                    if !app.ui_tabs.show_switches_pane {
                                    ui.scope(|ui| {
                                    let widgets = &mut ui.style_mut().visuals.widgets;
                                    widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                                    widgets.inactive.bg_stroke = egui::Stroke::NONE;
                                    widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                                    widgets.hovered.bg_fill = colors::BG_HOVER;
                                    widgets.hovered.bg_stroke = egui::Stroke::NONE;
                                    widgets.hovered.weak_bg_fill = colors::BG_HOVER;
                                    ui.menu_button(egui::RichText::new("モード").small(), |ui| {
                                    // ── Blend Mode Dropdown ──
                                    let bm_text = format!("{:?}", layer.blend_mode);
                                    egui::ComboBox::from_id_salt(format!("tl_blend_{}", i))
                                        .selected_text(format!("Blend: {}", bm_text))
                                        .show_ui(ui, |ui| {
                                            for bm in [
                                                BlendMode::Normal,
                                                BlendMode::Multiply,
                                                BlendMode::Screen,
                                                BlendMode::Overlay,
                                                BlendMode::Add,
                                                BlendMode::Darken,
                                                BlendMode::Lighten,
                                                BlendMode::SoftLight,
                                                BlendMode::HardLight,
                                                BlendMode::Difference,
                                                BlendMode::Exclusion,
                                                BlendMode::Divide,
                                                BlendMode::Subtract,
                                            ] {
                                                if ui.selectable_label(layer.blend_mode == bm, format!("{:?}", bm)).clicked() {
                                                    layer.blend_mode = bm;
                                                    project_changed = true;
                                                }
                                            }
                                        });

                                    // ── Track Matte Dropdown ──
                                    let tm_text = match layer.track_matte {
                                        TrackMatteMode::None => "None",
                                        TrackMatteMode::AlphaMatte => "Alpha",
                                        TrackMatteMode::AlphaMatteInverted => "Alpha Inv",
                                        TrackMatteMode::LumaMatte => "Luma",
                                        TrackMatteMode::LumaMatteInverted => "Luma Inv",
                                    };
                                    egui::ComboBox::from_id_salt(format!("tl_matte_{}", i))
                                        .selected_text(format!("Matte: {}", tm_text))
                                        .show_ui(ui, |ui| {
                                            for (mode, label) in [
                                                (TrackMatteMode::None, "None"),
                                                (TrackMatteMode::AlphaMatte, "Alpha Matte"),
                                                (TrackMatteMode::AlphaMatteInverted, "Alpha Matte Inverted"),
                                                (TrackMatteMode::LumaMatte, "Luma Matte"),
                                                (TrackMatteMode::LumaMatteInverted, "Luma Matte Inverted"),
                                            ] {
                                                if ui.selectable_label(layer.track_matte == mode, label).clicked() {
                                                    layer.track_matte = mode;
                                                    project_changed = true;
                                                }
                                            }
                                        });

                                    // ── Parenting Pick Whip @ & Dropdown ──
                                    let pw_active = app.pick_whip_mode;
                                    let pw_btn = ui.selectable_label(pw_active, "@").on_hover_text("Parenting Pick Whip: Click to enter pick mode, then click target layer");
                                    if pw_btn.clicked() {
                                        app.pick_whip_mode = !app.pick_whip_mode;
                                        app.pick_whip_target = None;
                                        if app.pick_whip_mode {
                                            app.toasts.info("Pick Whip active: click a layer to set as parent");
                                        }
                                    }
                                    let parent_name_resolved = layer.parent_id.as_deref()
                                        .and_then(|pid| derived.parent_indices.get(pid))
                                        .and_then(|index| parent_choices_ref.get(*index))
                                        .map(|(_, name)| name.as_str())
                                        .unwrap_or("None");
                                    let parent_prefix = if layer.is_3d { "Parent (3D)" } else { "Parent" };
                                    egui::ComboBox::from_id_salt(format!("tl_parent_{}", i))
                                        .selected_text(format!("{}: {}", parent_prefix, parent_name_resolved))
                                        .show_ui(ui, |ui| {
                                            if ui.selectable_label(layer.parent_id.is_none(), "None").clicked() {
                                                layer.parent_id = None;
                                                project_changed = true;
                                            }
                                            for (p_idx, (p_id, p_name)) in parent_choices_ref.iter().enumerate() {
                                                if p_idx != i {
                                                    let is_p = layer.parent_id.as_deref() == Some(p_id);
                                                    if ui.selectable_label(is_p, p_name).clicked() {
                                                        let is_cycle = would_create_parent_cycle(
                                                            &layer.id,
                                                            p_id,
                                                            &derived.parents,
                                                        );

                                                        if is_cycle {
                                                            app.toasts.warning(format!("Cycle prevented! Cannot parent '{}' to '{}'", layer.name, p_name));
                                                        } else {
                                                            layer.parent_id = Some(p_id.clone());
                                                            project_changed = true;
                                                            app.toasts.info(format!("Parented '{}' ➔ '{}'", layer.name, p_name));
                                                        }
                                                    }
                                                }
                                            }
                                        });
                                    });
                                    });
                                    }
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

                            // ── Trim handles: drag bar edges to set in/out points ──
                            fn handle_rect_of(is_in: bool, lr: &egui::Rect) -> egui::Rect {
                                const HW: f32 = 6.0;
                                if is_in {
                                    egui::Rect::from_min_size(egui::pos2(lr.left() - HW * 0.5, lr.top()), egui::vec2(HW, lr.height()))
                                } else {
                                    egui::Rect::from_min_size(egui::pos2(lr.right() - HW * 0.5, lr.top()), egui::vec2(HW, lr.height()))
                                }
                            }
                            const HANDLE_W: f32 = 6.0;
                            let in_handle = egui::Rect::from_min_size(
                                egui::pos2(layer_rect.left() - HANDLE_W * 0.5, layer_rect.top()),
                                egui::vec2(HANDLE_W, layer_rect.height()),
                            );
                            let out_handle = egui::Rect::from_min_size(
                                egui::pos2(layer_rect.right() - HANDLE_W * 0.5, layer_rect.top()),
                                egui::vec2(HANDLE_W, layer_rect.height()),
                            );
                            let in_resp = ui.interact(in_handle, egui::Id::new(("trim_in", i)), egui::Sense::click_and_drag());
                            let out_resp = ui.interact(out_handle, egui::Id::new(("trim_out", i)), egui::Sense::click_and_drag());

                            // ── Body drag: slide the whole layer (in/out preserved) ──
                            {
                                const HEADER_W: f32 = 6.0;
                                let body_rect = egui::Rect::from_min_max(
                                    egui::pos2(layer_rect.left() + HEADER_W, layer_rect.top()),
                                    egui::pos2(layer_rect.right() - HEADER_W, layer_rect.bottom()),
                                );
                                let body_resp = ui.interact(
                                    body_rect,
                                    egui::Id::new(("layer_slide", i, layer.id.as_str())),
                                    egui::Sense::drag(),
                                );
                                let body_drag_id = egui::Id::new(("layer_slide_state", layer.id.as_str()));
                                if body_resp.drag_started() {
                                    ui.ctx().data_mut(|data| {
                                        data.insert_temp(body_drag_id, LayerBodyDragState::default());
                                    });
                                }
                                if body_resp.dragged() {
                                    let total_frame_offset =
                                        (body_resp.drag_delta().x / bar_rect.width() * zoom_span as f32).round() as i32;
                                    let mut drag_state = ui.ctx().data(|data| {
                                        data.get_temp::<LayerBodyDragState>(body_drag_id)
                                            .unwrap_or_default()
                                    });
                                    let delta_frames = total_frame_offset - drag_state.frame_offset;
                                    drag_state.frame_offset = total_frame_offset;
                                    // ── Vertical drag → reorder layers in stack ──
                                    let row_h = layout::LAYER_ROW_HEIGHT;
                                    let total_y = body_resp.drag_delta().y;
                                    let incremental_y = total_y - drag_state.previous_y;
                                    drag_state.previous_y = total_y;
                                    let new_accum = drag_state.y_remainder + incremental_y;
                                    let row_offset = (new_accum / row_h).trunc() as i32;
                                    if row_offset != 0 {
                                        let target = (i as i32 - row_offset).clamp(0, layers_len as i32 - 1) as usize;
                                        if target != i {
                                            actions.swap_request = Some((i, target));
                                        }
                                        drag_state.y_remainder = new_accum - row_offset as f32 * row_h;
                                    } else {
                                        drag_state.y_remainder = new_accum;
                                    }
                                    ui.ctx().data_mut(|data| data.insert_temp(body_drag_id, drag_state));
                                    if delta_frames != 0 {
                                        let alt_held = ui.input(|inp| inp.modifiers.alt);
                                        if alt_held {
                                            // ── Slip edit: bar stays, content timing shifts ──
                                            // Keyframes & markers move opposite the drag so a
                                            // different moment of the source plays at the same time.
                                            let shift = -delta_frames;
                                            for kf in layer.transform.position.keyframes_mut().into_iter().flatten() {
                                                kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                            }
                                            if let Some(kfs) = layer.transform.position.keyframes_mut() { kfs.sort_by_key(|k| k.frame); }
                                            for kf in layer.transform.scale.keyframes_mut().into_iter().flatten() {
                                                kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                            }
                                            if let Some(kfs) = layer.transform.scale.keyframes_mut() { kfs.sort_by_key(|k| k.frame); }
                                            for kf in layer.transform.rotation.keyframes_mut().into_iter().flatten() {
                                                kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                            }
                                            if let Some(kfs) = layer.transform.rotation.keyframes_mut() { kfs.sort_by_key(|k| k.frame); }
                                            for kf in layer.transform.opacity.keyframes_mut().into_iter().flatten() {
                                                kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                            }
                                            if let Some(kfs) = layer.transform.opacity.keyframes_mut() { kfs.sort_by_key(|k| k.frame); }
                                            for m in layer.markers.iter_mut() {
                                                m.frame = (m.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                            }
                                            // Slip effect keyframes too
                                            use crate::core::effect_params::ParamRef;
                                            for effect in layer.effects.iter_mut() {
                                                for (_, param) in effect.effect_type.animatable_params() {
                                                    match param {
                                                        ParamRef::Scalar(anim) => {
                                                            if let Some(kfs) = anim.keyframes_mut() {
                                                                for kf in kfs.iter_mut() {
                                                                    kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                                                }
                                                                kfs.sort_by_key(|k| k.frame);
                                                            }
                                                        }
                                                        ParamRef::Vec2(anim) => {
                                                            if let Some(kfs) = anim.keyframes_mut() {
                                                                for kf in kfs.iter_mut() {
                                                                    kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                                                }
                                                                kfs.sort_by_key(|k| k.frame);
                                                            }
                                                        }
                                                        ParamRef::Vec3(anim) => {
                                                            if let Some(kfs) = anim.keyframes_mut() {
                                                                for kf in kfs.iter_mut() {
                                                                    kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                                                }
                                                                kfs.sort_by_key(|k| k.frame);
                                                            }
                                                        }
                                                        ParamRef::Vec4Color(anim) => {
                                                            if let Some(kfs) = anim.keyframes_mut() {
                                                                for kf in kfs.iter_mut() {
                                                                    kf.frame = (kf.frame as i64 + shift as i64).clamp(0, u32::MAX as i64) as u32;
                                                                }
                                                                kfs.sort_by_key(|k| k.frame);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            // Slide: move the whole bar
                                            let span = layer.out_frame - layer.in_frame;
                                            let raw_in = (layer.in_frame as i32 + delta_frames)
                                                .clamp(0, (total_frames - span) as i32)
                                                as u32;
                                            let new_in = if app.snap_to_keyframes {
                                                crate::ui::timeline::snapping::snap_to_sorted_layer_edges(
                                                    raw_in,
                                                    i,
                                                    layer_edges,
                                                )
                                            } else {
                                                raw_in
                                            };
                                            layer.in_frame = new_in;
                                            layer.out_frame = new_in + span;
                                        }
                                        project_changed = true;
                                    }
                                }
                                if body_resp.drag_stopped() {
                                    ui.ctx().data_mut(|data| data.remove::<LayerBodyDragState>(body_drag_id));
                                }
                                if body_resp.hovered() && !in_resp.hovered() && !out_resp.hovered() {
                                    let alt_held = ui.input(|inp| inp.modifiers.alt);
                                    ui.ctx().set_cursor_icon(if alt_held { egui::CursorIcon::ResizeHorizontal } else { egui::CursorIcon::Grab });
                                    body_resp.on_hover_text(format!(
                                        "{} — In: {} Out: {} ({:.2}s)",
                                        layer.name,
                                        layer.in_frame,
                                        layer.out_frame,
                                        layer.duration_frames() as f32 / comp.fps.max(1) as f32
                                    ));
                                }
                            }

                            for (resp, is_in) in [(&in_resp, true), (&out_resp, false)] {
                                if resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                    ui.painter().rect_filled(handle_rect_of(is_in, &layer_rect), 2.0, colors::HANDLE_HOVER_FILL.linear_multiply(180.0 / 255.0));
                                }
                                if resp.dragged() {
                                    if let Some(pos) = resp.interact_pointer_pos() {
                                        let norm = ((pos.x - bar_rect.left()) / bar_rect.width()).clamp(0.0, 1.0);
                                        let frame = (start_frame as f32 + norm * zoom_span as f32).round() as u32;
                                        let frame = frame.min(total_frames);
                                        if is_in {
                                            layer.in_frame = frame.min(layer.out_frame.saturating_sub(1));
                                        } else {
                                            let old_out = layer.out_frame;
                                            layer.out_frame = frame.max(layer.in_frame + 1);
                                            // ── Shift+drag Out = ripple edit: close gaps for layers below ──
                                            if ui.input(|inp| inp.modifiers.shift) && layer.out_frame != old_out {
                                                let shift = layer.out_frame as i64 - old_out as i64;
                                                actions.pending_ripple = Some((i, old_out, shift));
                                            }
                                        }
                                        project_changed = true;
                                    }
                                }
                            }

                            // ── Layer markers: small triangles on the bar (AE parity) ──
                            for marker in &layer.markers {
                                if marker.frame < start_frame || marker.frame > start_frame + zoom_span { continue; }
                                let norm = (marker.frame - start_frame) as f32 / zoom_span as f32;
                                let mx = layer_rect.left() + norm * layer_rect.width();
                                let my = layer_rect.top() + 1.0;
                                let [r, g, b] = marker.color;
                                let mcol = egui::Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8);
                                let pts = vec![
                                    egui::pos2(mx, my + 5.0),
                                    egui::pos2(mx - 4.0, my),
                                    egui::pos2(mx + 4.0, my),
                                ];
                                ui.painter().add(egui::Shape::convex_polygon(pts, mcol, egui::Stroke::NONE));
                            }

                            // ── Real waveform for video layers with audio ──
                            if let LayerType::Video { audio_wav: Some(wav_path), .. } = &layer.layer_type {
                                if let Some(waveform) = waveform_cache::get(
                                    ui.ctx(),
                                    std::path::Path::new(wav_path),
                                ) {
                                    draw_layer_waveform(
                                        &ui.painter().with_clip_rect(layer_rect),
                                        LayerWaveformParams {
                                            timeline_rect: bar_rect,
                                            clip_rect: layer_rect,
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

                            if let LayerType::Audio { path, .. } = &layer.layer_type {
                                if let Some(waveform) = waveform_cache::get(
                                    ui.ctx(),
                                    std::path::Path::new(path),
                                ) {
                                    draw_layer_waveform(
                                        &ui.painter().with_clip_rect(layer_rect),
                                        LayerWaveformParams {
                                            timeline_rect: bar_rect,
                                            clip_rect: layer_rect,
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

                            let playhead_norm =
                                (*current_frame as f32 - start_frame as f32) / zoom_span as f32;
                            if (0.0..=1.0).contains(&playhead_norm) {
                                let playhead_x =
                                    bar_rect.left() + playhead_norm * bar_rect.width();
                                ui.painter()
                                    .with_clip_rect(bar_rect)
                                    .line_segment(
                                        [
                                            egui::pos2(playhead_x, bar_rect.top()),
                                            egui::pos2(playhead_x, bar_rect.bottom()),
                                        ],
                                        egui::Stroke::new(
                                            1.5_f32,
                                            colors::TIMELINE_PLAYHEAD,
                                        ),
                                    );
                            }
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
