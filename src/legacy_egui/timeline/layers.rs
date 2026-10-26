use super::geometry::TimelineViewport;
use super::keyframe_tick::{draw_keyframe_tick, KeyframeTickResult};
use eframe::egui;

pub enum PropertyRowAction {
    MoveKeyframe {
        old_frame: u32,
        new_frame: u32,
    },
    SelectKeyframe {
        property_key: String,
        frame: u32,
        shift: bool,
        command: bool,
    },
    OpenMenu {
        property_key: String,
        frame: u32,
        response: egui::Response,
    },
    BoxSelect {
        property_key: String,
        frames: Vec<u32>,
        additive: bool,
    },
    MoveSelection {
        property_key: String,
        dragged_frame: u32,
        delta: i32,
    },
    SelectAll {
        property_key: String,
    },
}

pub struct PropertyRowResponse {
    pub requested_frame: Option<u32>,
    pub actions: Vec<PropertyRowAction>,
}

/// Legacy signature kept for backward compatibility (no selection support).
#[allow(clippy::too_many_arguments)]
pub fn draw_prop_row(
    ui: &mut egui::Ui,
    label: &str,
    kfs: &[(u32, crate::core::keyframe::InterpolationType)],
    current_frame: &mut u32,
    start_frame: u32,
    zoom_span: u32,
    left_pane_w: f32,
    mut on_move: Option<&mut dyn FnMut(u32, u32)>,
) -> Option<u32> {
    let response = draw_prop_row_ext(
        ui,
        label,
        kfs,
        current_frame,
        start_frame,
        zoom_span,
        left_pane_w,
        &std::collections::HashSet::new(),
        "",
        &[],
    );
    for action in response.actions {
        if let PropertyRowAction::MoveKeyframe {
            old_frame,
            new_frame,
        } = action
        {
            if let Some(callback) = on_move.as_deref_mut() {
                callback(old_frame, new_frame);
            }
        }
    }
    response.requested_frame
}

/// Extended version with keyframe selection support.
#[allow(clippy::too_many_arguments)]
pub fn draw_prop_row_ext(
    ui: &mut egui::Ui,
    label: &str,
    kfs: &[(u32, crate::core::keyframe::InterpolationType)],
    current_frame: &mut u32,
    start_frame: u32,
    zoom_span: u32,
    left_pane_w: f32,
    // Selected keyframes for this property (prop_key, frame). Empty => no highlight.
    selected_kfs: &std::collections::HashSet<(String, u32)>,
    // Stable key identifying this property within the layer (e.g. "position").
    prop_key: &str,
    snap_to: &[u32],
) -> PropertyRowResponse {
    let mut requested_frame = None;
    let mut actions = Vec::new();

    // Marquee state persists across frames of an active Shift+drag.
    let marquee_id = egui::Id::new(("kf_marquee", prop_key));
    let mut marquee_origin: Option<egui::Pos2> =
        ui.ctx().data_mut(|d| d.get_temp::<egui::Pos2>(marquee_id));
    let columns = super::layout::TimelineColumns::new(
        ui.available_width(),
        left_pane_w,
        ui.spacing().item_spacing.x,
    );

    ui.horizontal(|ui| {
        let (_, time_column, row_rects) = super::layout::allocate_columns(
            ui,
            columns,
            super::layout::PROPERTY_ROW_HEIGHT,
            |ui| {
                let lbl_resp = ui.add_sized(
                    [columns.left_width, super::layout::PROPERTY_ROW_HEIGHT],
                    egui::Label::new(
                        egui::RichText::new(label)
                            .small()
                            .color(crate::ui::theme::colors::TEXT_SECONDARY),
                    )
                    .sense(egui::Sense::click()),
                );
                if lbl_resp.double_clicked() {
                    actions.push(PropertyRowAction::SelectAll {
                        property_key: prop_key.to_owned(),
                    });
                }
            },
            |ui| {
                ui.allocate_exact_size(
                    egui::vec2(columns.time_width, super::layout::PROPERTY_ROW_HEIGHT),
                    egui::Sense::click_and_drag(),
                )
                .1
            },
        );
        let rect = row_rects.time;
        let viewport = TimelineViewport::new(start_frame, zoom_span, rect);
        let response = time_column.inner;

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let target_f = viewport.x_to_frame(pos.x);
                *current_frame = target_f;
                requested_frame = Some(target_f);
            }
        }

        // ── Shift+Drag marquee box-select over keyframe ticks (AE parity) ──
        if response.drag_started() && ui.input(|i| i.modifiers.shift) {
            marquee_origin = response.interact_pointer_pos();
            ui.ctx()
                .data_mut(|d| d.insert_temp(marquee_id, marquee_origin));
        }

        let mut marquee_rect: Option<egui::Rect> = None;
        if response.dragged() && marquee_origin.is_some() {
            if let Some(origin) = marquee_origin {
                if let Some(cur) = response.interact_pointer_pos() {
                    let r = egui::Rect::from_two_pos(origin, cur);
                    // Only treat as marquee when dragging mostly horizontally
                    // inside this row; otherwise it's a playhead scrub.
                    if (r.width() > 6.0 || r.height().abs() > 2.0) && r.height() < 60.0 {
                        marquee_rect = Some(r);
                        ui.ctx().request_repaint();
                    }
                }
            }
        } else if marquee_origin.is_some() && !response.dragged() && !response.drag_started() {
            ui.ctx().data_mut(|d| d.remove::<egui::Pos2>(marquee_id));
            marquee_origin = None;
        }

        if let Some(r) = marquee_rect {
            // Translucent selection rectangle + border
            ui.painter()
                .rect_filled(r, 2.0, crate::ui::theme::colors::TIMELINE_SELECTION);
            ui.painter().rect_stroke(
                r,
                2.0,
                egui::Stroke::new(1.0_f32, crate::ui::theme::colors::BORDER_ACTIVE),
            );

            let mut boxed: Vec<u32> = Vec::new();
            for &(kf_frame, _) in kfs {
                let kf_x = viewport.frame_to_x(kf_frame);
                if kf_x >= r.left() && kf_x <= r.right() {
                    boxed.push(kf_frame);
                }
            }
            if response.drag_stopped() {
                if !boxed.is_empty() {
                    actions.push(PropertyRowAction::BoxSelect {
                        property_key: prop_key.to_owned(),
                        frames: boxed,
                        additive: true,
                    });
                }
                ui.ctx().data_mut(|d| d.remove::<egui::Pos2>(marquee_id));
                marquee_origin = None;
            }
        }

        for &(kf_frame, interpolation) in kfs {
            if viewport.contains(kf_frame) {
                let is_selected = selected_kfs.contains(&(prop_key.to_string(), kf_frame));
                match draw_keyframe_tick(
                    ui,
                    viewport,
                    true,
                    current_frame,
                    kf_frame,
                    Some(interpolation),
                    is_selected,
                ) {
                    (KeyframeTickResult::Clicked { shift, cmd }, _resp) => {
                        requested_frame = Some(kf_frame);
                        actions.push(PropertyRowAction::SelectKeyframe {
                            property_key: prop_key.to_owned(),
                            frame: kf_frame,
                            shift,
                            command: cmd,
                        });
                    }
                    (KeyframeTickResult::RightClicked, resp) => {
                        actions.push(PropertyRowAction::OpenMenu {
                            property_key: prop_key.to_owned(),
                            frame: kf_frame,
                            response: resp,
                        });
                    }
                    (KeyframeTickResult::Dragged { new_frame }, _resp) => {
                        // Snap dragged keyframe to nearby keyframes on other properties
                        let snapped = if !snap_to.is_empty() {
                            let threshold = 4u32;
                            let mut best = new_frame;
                            let mut best_dist = threshold + 1;
                            for &candidate in snap_to {
                                let dist = (new_frame as i32 - candidate as i32).unsigned_abs();
                                if dist < best_dist {
                                    best_dist = dist;
                                    best = candidate;
                                }
                            }
                            best
                        } else {
                            new_frame
                        };
                        requested_frame = Some(snapped);
                        if snapped != kf_frame {
                            actions.push(PropertyRowAction::MoveKeyframe {
                                old_frame: kf_frame,
                                new_frame: snapped,
                            });
                            // Selected keyframes travel with the dragged one (AE group move)
                            if is_selected && selected_kfs.len() > 1 {
                                let delta = snapped as i64 - kf_frame as i64;
                                let clamped = delta.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
                                actions.push(PropertyRowAction::MoveSelection {
                                    property_key: prop_key.to_owned(),
                                    dragged_frame: kf_frame,
                                    delta: clamped,
                                });
                            }
                        }
                    }
                    (KeyframeTickResult::None, _resp) => {}
                }
            }
        }
    });

    PropertyRowResponse {
        requested_frame,
        actions,
    }
}
