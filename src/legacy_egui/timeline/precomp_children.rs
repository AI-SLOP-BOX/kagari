//! Pre-comp nested timeline: snapshot child layers of expanded PreComp layers
//! and render them as indented rows beneath the parent.
use crate::core::timeline::{LayerType, Project};
use crate::ui::theme::colors;
use eframe::egui;
use std::sync::Arc;

#[derive(Clone)]
pub struct PreCompChild {
    pub name: String,
    pub type_icon: &'static str,
    pub label_rgb: [f32; 3],
    pub in_frame: u32,
    pub out_frame: u32,
}

#[derive(Clone)]
struct CachedPreCompChildren {
    project_session: u64,
    history_generation: u64,
    active_comp_id: String,
    expanded_layers: Vec<usize>,
    display_fingerprint: u64,
    children: Arc<Vec<(usize, Vec<PreCompChild>, String)>>,
}

/// Snapshot child layers for every expanded PreComp layer.
/// Runs BEFORE the mutable composition borrow to avoid aliasing.
pub fn collect(
    project: &Project,
    expanded_layers: &std::collections::HashSet<usize>,
) -> Vec<(usize, Vec<PreCompChild>, String)> {
    let comp_ro = project.active_composition();
    let mut parent_indices = expanded_layers.iter().copied().collect::<Vec<_>>();
    parent_indices.sort_unstable();
    parent_indices
        .into_iter()
        .filter_map(|parent_i| {
            let layer = comp_ro.layers.get(parent_i)?;
            if let LayerType::PreComp { comp_id } = &layer.layer_type {
                let sub = comp_ro.find_sub_comp(comp_id)?;
                let children: Vec<PreCompChild> = sub
                    .layers
                    .iter()
                    .map(|cl| {
                        let rgb = cl.label.to_rgb();
                        PreCompChild {
                            name: cl.name.clone(),
                            type_icon: crate::ui::icons::layer_icon(&cl.layer_type),
                            label_rgb: rgb,
                            in_frame: cl.in_frame,
                            out_frame: cl.out_frame,
                        }
                    })
                    .collect();
                Some((parent_i, children, sub.id.clone()))
            } else {
                None
            }
        })
        .collect()
}

pub fn collect_cached(
    context: &egui::Context,
    project: &Project,
    expanded_layers: &std::collections::HashSet<usize>,
    project_session: u64,
    history_generation: u64,
) -> Arc<Vec<(usize, Vec<PreCompChild>, String)>> {
    use std::hash::Hasher;

    let mut expanded = expanded_layers.iter().copied().collect::<Vec<_>>();
    expanded.sort_unstable();
    let active_comp_id = project.active_composition().id.clone();
    let cache_id = egui::Id::new("timeline_precomp_children_snapshot");

    context.data_mut(|data| {
        if let Some(mut cached) = data.get_temp::<CachedPreCompChildren>(cache_id) {
            let same_view = cached.project_session == project_session
                && cached.active_comp_id == active_comp_id
                && cached.expanded_layers == expanded;
            if same_view && cached.history_generation == history_generation {
                return cached.children;
            }
            if same_view {
                let fingerprint = display_fingerprint(project, &expanded);
                if cached.display_fingerprint == fingerprint {
                    cached.history_generation = history_generation;
                    let children = cached.children.clone();
                    data.insert_temp(cache_id, cached);
                    return children;
                }
            }
        }
        let collected = Arc::new(collect(project, expanded_layers));
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        hash_display_metadata(project, &expanded, &mut hasher);
        data.insert_temp(
            cache_id,
            CachedPreCompChildren {
                project_session,
                history_generation,
                active_comp_id,
                expanded_layers: expanded,
                display_fingerprint: hasher.finish(),
                children: collected.clone(),
            },
        );
        collected
    })
}

fn display_fingerprint(project: &Project, expanded: &[usize]) -> u64 {
    use std::hash::Hasher;

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_display_metadata(project, expanded, &mut hasher);
    hasher.finish()
}

fn hash_display_metadata(
    project: &Project,
    expanded: &[usize],
    hasher: &mut impl std::hash::Hasher,
) {
    use std::hash::Hash;

    let comp = project.active_composition();
    for &parent_index in expanded {
        parent_index.hash(hasher);
        let Some(parent) = comp.layers.get(parent_index) else {
            0u8.hash(hasher);
            continue;
        };
        let LayerType::PreComp { comp_id } = &parent.layer_type else {
            1u8.hash(hasher);
            continue;
        };
        2u8.hash(hasher);
        comp_id.hash(hasher);
        let Some(subcomp) = comp.find_sub_comp(comp_id) else {
            0u8.hash(hasher);
            continue;
        };
        1u8.hash(hasher);
        subcomp.id.hash(hasher);
        subcomp.layers.len().hash(hasher);
        for child in &subcomp.layers {
            child.name.hash(hasher);
            crate::ui::icons::layer_icon(&child.layer_type).hash(hasher);
            for channel in child.label.to_rgb() {
                channel.to_bits().hash(hasher);
            }
            child.in_frame.hash(hasher);
            child.out_frame.hash(hasher);
        }
    }
}

fn visible_child_range(
    clip_top: f32,
    clip_bottom: f32,
    row_top: f32,
    row_step: f32,
    child_count: usize,
) -> std::ops::Range<usize> {
    let first = (((clip_top - row_top) / row_step).floor().max(0.0) as usize).min(child_count);
    let end = (((clip_bottom - row_top) / row_step).ceil().max(0.0) as usize)
        .min(child_count)
        .max(first);
    first..end
}

/// Draw the indented child rows under an expanded PreComp layer.
/// Returns true when the user clicks "Open Pre-comp".
pub fn draw_children_rows(
    ui: &mut egui::Ui,
    sub_id: &str,
    children: &[PreCompChild],
    left_pane_w: f32,
    start_frame: u32,
    zoom_span: u32,
) -> bool {
    let row_height = super::layout::PROPERTY_ROW_HEIGHT;
    let columns = super::layout::TimelineColumns::new(
        ui.available_width(),
        left_pane_w,
        ui.spacing().item_spacing.x,
    );
    let spacing_y = ui.spacing().item_spacing.y;
    let row_step = row_height + spacing_y;
    let row_top = ui.cursor().top();
    let clip = ui.clip_rect();
    let visible = visible_child_range(clip.top(), clip.bottom(), row_top, row_step, children.len());
    let reserve_rows = |ui: &mut egui::Ui, count: usize| {
        if count > 0 {
            ui.add_space((count as f32 * row_step - spacing_y).max(0.0));
        }
    };
    reserve_rows(ui, visible.start);
    for child in children.iter().take(visible.end).skip(visible.start) {
        ui.horizontal(|ui| {
            let (_, time_column, row_rects) = super::layout::allocate_columns(
                ui,
                columns,
                row_height,
                |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(34.0);
                        let connector = ui
                            .allocate_exact_size(egui::vec2(12.0, 18.0), egui::Sense::hover())
                            .0;
                        let connector_painter = ui.painter().with_clip_rect(connector);
                        connector_painter.line_segment(
                            [connector.left_center(), connector.center()],
                            egui::Stroke::new(1.0_f32, colors::TEXT_MUTED),
                        );
                        connector_painter.line_segment(
                            [
                                connector.center(),
                                egui::pos2(connector.right(), connector.center().y),
                            ],
                            egui::Stroke::new(1.0_f32, colors::TEXT_MUTED),
                        );
                        crate::ui::kui::icon_glyph(
                            ui,
                            child.type_icon,
                            14.0,
                            colors::TEXT_SECONDARY,
                        );
                        let chip = egui::Color32::from_rgb(
                            (child.label_rgb[0] * 255.0) as u8,
                            (child.label_rgb[1] * 255.0) as u8,
                            (child.label_rgb[2] * 255.0) as u8,
                        );
                        let (chip_rect, _) =
                            ui.allocate_exact_size(egui::vec2(6.0, 11.0), egui::Sense::hover());
                        ui.painter().rect_filled(chip_rect, 2.0, chip);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&child.name)
                                    .small()
                                    .color(colors::TEXT_SECONDARY),
                            )
                            .truncate(),
                        )
                        .on_hover_text(&child.name);
                    });
                },
                |ui| {
                    ui.allocate_exact_size(
                        egui::vec2(columns.time_width, row_height),
                        egui::Sense::hover(),
                    )
                    .1
                },
            );
            let bar_rect = row_rects.time;
            let response = time_column.inner;
            ui.painter().rect_filled(bar_rect, 0.0, colors::BG_DARK);
            let end_frame = start_frame.saturating_add(zoom_span.max(1));
            if child.out_frame > start_frame && child.in_frame < end_frame {
                let span = zoom_span.max(1) as f32;
                let x_for = |frame: u32| {
                    bar_rect.left()
                        + (frame.saturating_sub(start_frame) as f32 / span) * bar_rect.width()
                };
                let clip = egui::Rect::from_min_max(
                    egui::pos2(x_for(child.in_frame.max(start_frame)), bar_rect.top() + 4.0),
                    egui::pos2(
                        x_for(child.out_frame.min(end_frame)),
                        bar_rect.bottom() - 4.0,
                    ),
                )
                .intersect(bar_rect);
                let label = egui::Color32::from_rgb(
                    (child.label_rgb[0] * 115.0 + 14.0) as u8,
                    (child.label_rgb[1] * 115.0 + 16.0) as u8,
                    (child.label_rgb[2] * 115.0 + 19.0) as u8,
                );
                if clip.is_positive() {
                    ui.painter()
                        .rect_filled(clip.shrink2(egui::vec2(0.0, 2.0)), 2.0, label);
                }
            }
            response.on_hover_text(format!(
                "{}: frames {}–{}",
                child.name, child.in_frame, child.out_frame
            ));
        });
    }
    reserve_rows(ui, children.len() - visible.end);
    // Open the nested composition without adding another miniature navigation row.
    let mut open_requested = false;
    let open_probe = egui::Rect::from_min_size(
        ui.cursor().min,
        egui::vec2(ui.available_width(), row_height + 8.0),
    );
    if ui.is_rect_visible(open_probe) {
        ui.horizontal(|ui| {
            ui.add_space(34.0);
            if crate::ui::kui::button(
                ui,
                "コンポジションを開く",
                crate::ui::kui::ButtonKind::Quiet,
            )
            .on_hover_text("ネストされたコンポジションを編集")
            .clicked()
            {
                open_requested = true;
            }
        });
    } else {
        ui.add_space(row_height + 8.0);
    }
    let _ = sub_id;
    open_requested
}

#[cfg(test)]
mod tests {
    use super::{collect_cached, draw_children_rows, visible_child_range, PreCompChild};
    use eframe::egui;
    use std::collections::HashSet;

    #[test]
    fn child_snapshot_is_reused_for_non_display_edits() {
        let mut project = crate::core::timeline::Project::empty_for_new_project();
        let mut nested = crate::core::timeline::Composition::new(
            "nested".into(),
            "Nested".into(),
            1920,
            1080,
            30,
            30,
        );
        nested.add_layer(crate::core::timeline::Layer::new(
            "child".into(),
            "Child".into(),
            crate::core::timeline::LayerType::Null,
            30,
        ));
        let parent = crate::core::timeline::Layer::new(
            "parent".into(),
            "PreComp".into(),
            crate::core::timeline::LayerType::PreComp {
                comp_id: "nested".into(),
            },
            30,
        );
        let active = project.active_composition_mut();
        active.sub_compositions.push(nested);
        active.layers.push(parent);
        let expanded = HashSet::from([0]);
        let context = egui::Context::default();

        let first = collect_cached(&context, &project, &expanded, 9, 1);
        project.active_composition_mut().sub_compositions[0].layers[0]
            .transform
            .position = crate::core::property::Animatable::new_constant([10.0, 20.0]);
        let after_transform = collect_cached(&context, &project, &expanded, 9, 2);
        assert!(std::sync::Arc::ptr_eq(&first, &after_transform));

        project.active_composition_mut().sub_compositions[0].layers[0].name = "Renamed".into();
        let after_rename = collect_cached(&context, &project, &expanded, 9, 3);
        assert!(!std::sync::Arc::ptr_eq(&after_transform, &after_rename));
        assert_eq!(after_rename[0].1[0].name, "Renamed");
    }

    #[test]
    fn nested_rows_only_paint_the_visible_children() {
        let children = (0..5_000)
            .map(|index| PreCompChild {
                name: format!("Layer {index}"),
                type_icon: "L",
                label_rgb: [0.2, 0.4, 0.6],
                in_frame: 0,
                out_frame: 30,
            })
            .collect::<Vec<_>>();
        let context = egui::Context::default();
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(640.0, 240.0),
                )),
                ..Default::default()
            },
            |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(96.0)
                        .show(ui, |ui| {
                            draw_children_rows(ui, "nested", &children, 240.0, 0, 30);
                        });
                });
            },
        );

        assert!(
            output.shapes.len() < 2_000,
            "off-screen child rows must not add one row of shapes each: {}",
            output.shapes.len()
        );
    }

    #[test]
    fn child_virtualization_selects_only_rows_intersecting_the_clip() {
        assert_eq!(visible_child_range(100.0, 196.0, 0.0, 24.0, 5_000), 4..9);
        assert_eq!(visible_child_range(0.0, 10.0, 1_000.0, 24.0, 5_000), 0..0);
        assert_eq!(
            visible_child_range(200_000.0, 200_100.0, 0.0, 24.0, 5_000),
            5_000..5_000
        );
    }
}
