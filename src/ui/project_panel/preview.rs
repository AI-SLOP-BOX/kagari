use crate::core::timeline::{Project, ProjectItem, ProjectItemType};
use crate::ui::custom_widgets;
use eframe::egui;

pub fn draw_metadata_preview(
    ui: &mut egui::Ui,
    current_project: &Project,
    selected_asset_idx: Option<usize>,
    add_to_timeline_item: &mut Option<ProjectItem>,
) {
    if let Some(idx) = selected_asset_idx {
        if let Some(item) = current_project.assets.get(idx) {
                egui::Frame::none()
                    .inner_margin(egui::Margin::symmetric(0.0, 4.0))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(&item.name).strong());
                        match &item.item_type {
                            ProjectItemType::Composition { comp_idx } => {
                                if let Some(c) = current_project.compositions.get(*comp_idx) {
                                    ui.small(format!("Resolution: {} x {} px", c.width, c.height));
                                    ui.small(format!("Frame Rate: {} fps", c.fps));
                                    ui.small(format!("Duration: {} frames", c.duration_frames));
                                }
                            }
                            ProjectItemType::Image {
                                path,
                                width,
                                height,
                            } => {
                                ui.small(format!("File: {}", path));
                                ui.small(format!("Dimensions: {} x {} px", width, height));
                            }
                            ProjectItemType::Model3D { path } => {
                                ui.small(format!("File: {}", path));
                                ui.small("Format: Wavefront OBJ");
                            }
                            ProjectItemType::Video { path, duration_sec } => {
                                ui.small(format!("File: {}", path));
                                if duration_sec.is_finite() && *duration_sec > 0.0 {
                                    ui.small(format!("Duration: {:.1}s", duration_sec));
                                } else {
                                    ui.small("Duration: not measured");
                                }
                            }
                            ProjectItemType::Audio { path, duration_sec } => {
                                ui.small(format!("File: {}", path));
                                if duration_sec.is_finite() && *duration_sec > 0.0 {
                                    ui.small(format!("Length: {:.2} seconds", duration_sec));
                                } else {
                                    ui.small("Length: not measured");
                                }
                            }
                            ProjectItemType::Solid { color } => {
                                ui.small(format!(
                                    "Color: R:{:.0} G:{:.0} B:{:.0}",
                                    color[0] * 255.0,
                                    color[1] * 255.0,
                                    color[2] * 255.0
                                ));
                            }
                            ProjectItemType::Folder { .. } => {
                                ui.small("Project Bin Directory");
                            }
                        }

                        ui.add_space(4.0);
                        if custom_widgets::ae_button(ui, "Add to Active Comp").clicked() {
                            *add_to_timeline_item = Some(item.clone());
                        }
                    });
            }
        }
}
