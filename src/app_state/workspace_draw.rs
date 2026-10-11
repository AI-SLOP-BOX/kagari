use super::KagariApp;
use eframe::egui;

impl KagariApp {
    pub(super) fn draw_workspace(
        &mut self,
        ctx: &egui::Context,
        mut current_frame: u32,
        total_frames: u32,
    ) {
        if self.show_home {
            crate::ui::home_screen::draw(self, ctx);
            crate::ui::preferences_dialog::draw_preferences_dialog(self, ctx);
            crate::ui::shortcuts_dialog::draw_shortcuts_dialog(self, ctx);
            self.toasts.draw(ctx);
            self.draw_startup_animation(ctx);
            self.playback.current_frame = current_frame;
            return;
        }
        if self.show_guided_tutorial {
            crate::ui::tutorial_workspace::draw(self, ctx);
            crate::ui::preferences_dialog::draw_preferences_dialog(self, ctx);
            crate::ui::shortcuts_dialog::draw_shortcuts_dialog(self, ctx);
            self.toasts.draw(ctx);
            self.draw_startup_animation(ctx);
            self.playback.current_frame = current_frame;
            return;
        }
        if self.ui_tabs.left_tab_idx == 1
            && self.ui_tabs.right_panel == crate::app_state::RightPanel::Color
            && self.ui_tabs.bottom_dock_tab != 1
        {
            crate::ui::menu::draw(self, ctx);
            crate::ui::color_workspace::draw(self, ctx);
            crate::ui::preferences_dialog::draw_preferences_dialog(self, ctx);
            crate::ui::shortcuts_dialog::draw_shortcuts_dialog(self, ctx);
            self.toasts.draw(ctx);
            self.draw_startup_animation(ctx);
            self.playback.current_frame = current_frame;
            return;
        }
        let assets_workspace = ctx.data_mut(|data| {
            data.get_temp::<usize>(egui::Id::new("studio_active_workspace"))
                == Some(crate::ui::menu::ASSETS_WORKSPACE_INDEX)
        });
        if assets_workspace
            && self.ui_tabs.left_tab_idx == 0
            && self.ui_tabs.right_panel == crate::app_state::RightPanel::Properties
        {
            crate::ui::menu::draw(self, ctx);
            crate::ui::asset_library::draw(self, ctx);
            self.draw_startup_animation(ctx);
            self.playback.current_frame = current_frame;
            return;
        }
        if self.viewer_maximized {
            crate::ui::menu::draw(self, ctx);
            crate::ui::viewport::draw(self, ctx, current_frame);
            self.draw_startup_animation(ctx);
            self.playback.current_frame = current_frame;
            return;
        }
        crate::ui::menu::draw(self, ctx);
        // Establish the full-height inspector first, then split the remaining
        // work area into the upper project/viewer region and lower timeline.
        crate::ui::effects_library::draw(self, ctx, &mut current_frame);
        crate::ui::timeline::draw(self, ctx, &mut current_frame, total_frames);
        crate::ui::inspector::draw(self, ctx, &mut current_frame);

        let status_frame = egui::Frame::none()
            .fill(crate::ui::theme::colors::BG_DARKEST)
            .inner_margin(egui::Margin::symmetric(8.0, 3.0))
            .stroke(egui::Stroke::new(
                1.0_f32,
                crate::ui::theme::colors::BORDER_SUBTLE,
            ));

        egui::TopBottomPanel::bottom("ae_status_bar")
            .frame(status_frame)
            .default_height(22.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let status_width = ctx.screen_rect().width();
                    ui.style_mut().spacing.item_spacing.x = 6.0;
                    if !self.history.current().has_pristine_start_composition() {
                        let fps = self.history.current().active_composition().fps.max(1);
                        crate::ui::timeline::header::draw_transport_controls(
                            ui,
                            &mut current_frame,
                            total_frames,
                            fps,
                            &mut self.playback.is_playing,
                        );
                        crate::ui::kui::toolbar_separator(ui);
                    }
                    if status_width >= 1700.0 {
                        ui.label(
                            egui::RichText::new("フレーム描画時間")
                                .small()
                                .color(crate::ui::theme::colors::TEXT_MUTED),
                        );
                        let render_ms = self.playback.preview_render_ema_ms;
                        if render_ms > 0.0 {
                            ui.label(
                                egui::RichText::new(format!("{:.0}ms", render_ms))
                                    .small()
                                    .color(crate::ui::theme::colors::TEXT_MUTED),
                            );
                            ui.separator();
                        }
                    }
                    if status_width >= 1500.0 {
                        let (gpu_label, gpu_color) = if self.gpu_rendered {
                            (
                                "● Metal GPUレンダラー",
                                crate::ui::theme::colors::TEXT_MUTED,
                            )
                        } else {
                            (
                                "○ CPUソフトウェアレンダラー",
                                crate::ui::theme::colors::ACCENT_ORANGE,
                            )
                        };
                        ui.label(egui::RichText::new(gpu_label).small().color(gpu_color));
                        ui.separator();
                    }
                    if self.history.current().has_pristine_start_composition() {
                        ui.label(
                            egui::RichText::new("準備完了")
                                .small()
                                .color(crate::ui::theme::colors::TEXT_MUTED),
                        );
                    }
                    if status_width >= 1700.0 {
                        ui.separator();
                        ui.label(
                            egui::RichText::new("8-bit出力")
                                .small()
                                .color(crate::ui::theme::colors::TEXT_MUTED),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!(
                                "フレームキャッシュ {:.0}/{:.0} MB · {}フレーム",
                                self.frame_cache.current_memory_bytes as f32 / (1024.0 * 1024.0),
                                self.frame_cache.max_memory_bytes as f32 / (1024.0 * 1024.0),
                                self.frame_cache.cached_count(),
                            ))
                            .small()
                            .color(crate::ui::theme::colors::TEXT_MUTED),
                        );
                        ui.separator();
                        let render_ms = self.playback.preview_render_ema_ms;
                        let frame_budget_ms =
                            1000.0 / self.history.current().active_composition().fps.max(1) as f32;
                        let ms_color = if render_ms > frame_budget_ms {
                            crate::ui::theme::colors::ACCENT_ORANGE
                        } else {
                            crate::ui::theme::colors::TEXT_MUTED
                        };
                        if render_ms > 0.0 {
                            ui.label(
                                egui::RichText::new(format!("Render: {:.1} ms", render_ms))
                                    .small()
                                    .color(ms_color),
                            );
                            ui.separator();
                        }
                        // Selection summary: layers + keyframes
                        let kf_count = self.selected_keyframes.len();
                        let layer_count = self.selection.selected_layers.len();
                        if kf_count > 0 {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} keyframes selected (, . move | Del delete | Cmd+C/V)",
                                    kf_count
                                ))
                                .small()
                                .color(crate::ui::theme::colors::ACCENT_ORANGE),
                            );
                        } else if layer_count > 0 {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} layer{} selected",
                                    layer_count,
                                    if layer_count > 1 { "s" } else { "" }
                                ))
                                .small()
                                .color(crate::ui::theme::colors::ACCENT_BLUE),
                            );
                        }
                    }
                    if status_width >= 1700.0
                        && !self
                            .history
                            .current()
                            .active_composition()
                            .layers
                            .is_empty()
                    {
                        let comp_pixel = ctx.pointer_hover_pos().and_then(|pointer_pos| {
                            let rect = self.viewer_draw_rect?;
                            if !rect.contains(pointer_pos)
                                || rect.width() <= 0.0
                                || rect.height() <= 0.0
                            {
                                return None;
                            }
                            let comp = self.history.current().active_composition();
                            let x = ((pointer_pos.x - rect.left()) / rect.width()
                                * comp.width as f32)
                                .floor() as u32;
                            let y = ((pointer_pos.y - rect.top()) / rect.height()
                                * comp.height as f32)
                                .floor() as u32;
                            (x < comp.width && y < comp.height).then_some((x, y))
                        });
                        ui.separator();
                        if let Some((x, y)) = comp_pixel {
                            ui.label(
                                egui::RichText::new(format!("X: {x} Y: {y} px"))
                                    .small()
                                    .color(egui::Color32::from_rgb(0, 180, 255)),
                            );
                        }
                        ui.separator();
                        let pixel_rgba = {
                            let comp = self.history.current().active_composition();
                            if let Some((px, py)) = comp_pixel {
                                let layer_indices: Vec<usize> = comp
                                    .layers
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, l)| l.is_active(self.playback.current_frame))
                                    .map(|(i, _)| i)
                                    .collect();
                                if let Some(entry) = self
                                    .frame_cache
                                    .get_with_layers(self.playback.current_frame, &layer_indices)
                                {
                                    let idx = ((py * comp.width + px) * 4) as usize;
                                    if idx + 3 < entry.pixels.len() {
                                        Some([
                                            entry.pixels[idx],
                                            entry.pixels[idx + 1],
                                            entry.pixels[idx + 2],
                                            entry.pixels[idx + 3],
                                        ])
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        };
                        if let Some([r, g, b, a]) = pixel_rgba {
                            ui.label(
                                egui::RichText::new(format!("R: {} G: {} B: {} A: {}", r, g, b, a))
                                    .small()
                                    .color(egui::Color32::from_rgb(255, 200, 100)),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("R: – G: – B: – A: –")
                                    .small()
                                    .color(egui::Color32::from_rgb(255, 200, 100)),
                            );
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "Kagari VFX v{}",
                                env!("CARGO_PKG_VERSION")
                            ))
                            .small()
                            .color(egui::Color32::from_gray(120)),
                        );
                        if status_width >= 1600.0 {
                            ui.separator();
                            let active_tool_label = match self.active_tool {
                                crate::ui::toolbar::ActiveTool::Selection => "Selection",
                                crate::ui::toolbar::ActiveTool::Hand => "Hand",
                                crate::ui::toolbar::ActiveTool::Zoom => "Zoom",
                                crate::ui::toolbar::ActiveTool::Camera3D => "Camera 3D",
                                crate::ui::toolbar::ActiveTool::Rotation => "Rotation",
                                crate::ui::toolbar::ActiveTool::AnchorPoint => "Anchor Point",
                                crate::ui::toolbar::ActiveTool::Rectangle => "Rectangle",
                                crate::ui::toolbar::ActiveTool::Pen => "Pen",
                                crate::ui::toolbar::ActiveTool::Text => "Text",
                                crate::ui::toolbar::ActiveTool::Brush => "Brush",
                                crate::ui::toolbar::ActiveTool::CloneStamp => "Clone Stamp",
                                crate::ui::toolbar::ActiveTool::Eraser => "Eraser",
                                crate::ui::toolbar::ActiveTool::RotoBrush => "Roto Brush",
                                crate::ui::toolbar::ActiveTool::PuppetPin => "Puppet Pin",
                            };
                            ui.label(
                                egui::RichText::new(format!("Tool: {active_tool_label}"))
                                    .small()
                                    .color(egui::Color32::from_rgb(255, 230, 0)),
                            );
                            ui.separator();
                            let dl_status = if let Some(client) = &self.connected_app {
                                format!("Connected to {client}")
                            } else {
                                "Not connected".to_string()
                            };
                            ui.label(
                                egui::RichText::new(format!("Dynamic Link: {dl_status}"))
                                    .small()
                                    .color(egui::Color32::from_rgb(100, 180, 255)),
                            );
                            ui.separator();
                        }
                    });
                });
            });

        crate::ui::viewport::draw(self, ctx, current_frame);
        crate::ui::history_panel::draw_history_panel(self, ctx);
        crate::ui::quality_check_panel::draw(self, ctx);
        crate::ui::automation_panel::draw(self, ctx);
        crate::ui::drop_import::handle_dropped_files(self, ctx);
        crate::ui::welcome::draw(self, ctx);
        crate::ui::tutorial::draw(self, ctx);
        crate::ui::new_comp_dialog::draw_new_comp_dialog(self, ctx);
        crate::ui::preferences_dialog::draw_preferences_dialog(self, ctx);
        crate::ui::export_dialog::draw(self, ctx);
        crate::ui::comp_settings_dialog::draw_comp_settings_dialog(self, ctx);

        let cmd_k_pressed = ctx.input(|input| {
            crate::ui::shortcuts::shortcut_pressed(
                input,
                crate::ui::shortcuts::ShortcutId::CommandPalette,
            )
        });
        if cmd_k_pressed && (!self.has_blocking_dialog() || self.show_command_palette) {
            self.show_command_palette = !self.show_command_palette;
            if self.show_command_palette {
                self.command_palette_search.clear();
                self.command_palette_selected_idx = 0;
            }
        }

        crate::ui::command_palette::draw_command_palette(self, ctx);
        crate::ui::vectorscope::draw_vectorscope_window(self, ctx);
        crate::ui::shortcuts_dialog::draw_shortcuts_dialog(self, ctx);
        self.toasts.draw(ctx);
        self.draw_startup_animation(ctx);
        self.playback.current_frame = current_frame;
    }
}
