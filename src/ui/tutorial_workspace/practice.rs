//! Main hero area and video practice workflow for tutorial workspace.

use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

use super::{selected_chapter, ORANGE, PANEL};

pub fn draw_main(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    rect: egui::Rect,
    compact: bool,
    mobile: bool,
    app: &mut KagariApp,
) {
    let p = ui.painter();
    let pad = if mobile {
        18.0
    } else if compact {
        24.0
    } else {
        34.0
    };
    let selected = selected_chapter(app.tutorial_step);
    let short = rect.height() < 560.0;
    let chapter = &crate::ui::tutorial::chapters()[selected];
    let chapter_title = chapter.title;
    let chapter_description = chapter.description;
    let lesson_index = chapter.first_step;
    let lesson = &crate::ui::tutorial::steps()[lesson_index];
    p.text(
        egui::pos2(
            rect.left() + pad,
            rect.top() + if short { 38.0 } else { 75.0 },
        ),
        egui::Align2::LEFT_CENTER,
        "チュートリアル",
        egui::FontId::proportional(11.0),
        colors::TEXT_SECONDARY,
    );
    p.text(
        egui::pos2(
            rect.left() + pad,
            rect.top() + if short { 76.0 } else { 119.0 },
        ),
        egui::Align2::LEFT_CENTER,
        chapter_title,
        egui::FontId::proportional(if mobile { 32.0 } else { 42.0 }),
        colors::TEXT_PRIMARY,
    );
    p.text(
        egui::pos2(
            rect.left() + pad,
            rect.top() + if short { 110.0 } else { 157.0 },
        ),
        egui::Align2::LEFT_CENTER,
        chapter_description,
        egui::FontId::proportional(if mobile { 16.0 } else { 21.0 }),
        colors::TEXT_SECONDARY,
    );
    let hero_top = rect.top() + if short { 132.0 } else { 190.0 };
    let hero_h = if mobile {
        if short {
            104.0
        } else {
            160.0
        }
    } else if compact {
        if short {
            108.0
        } else {
            170.0
        }
    } else if short {
        128.0
    } else {
        310.0
    };
    let hero = egui::Rect::from_min_max(
        egui::pos2(rect.left() + pad, hero_top),
        egui::pos2(rect.right() - pad, hero_top + hero_h),
    );
    p.rect_filled(hero, 8.0, PANEL);
    p.rect_stroke(hero, 8.0, egui::Stroke::new(1.0_f32, colors::BORDER_MEDIUM));
    p.text(
        egui::pos2(hero.left() + 20.0, hero.top() + 27.0),
        egui::Align2::LEFT_CENTER,
        "このレッスンで学ぶこと",
        egui::FontId::proportional(14.0),
        colors::TEXT_PRIMARY,
    );
    let lesson_lines: Vec<&str> = lesson
        .body
        .iter()
        .copied()
        .filter(|line| {
            !line.is_empty() && !line.contains('┌') && !line.contains('│') && !line.contains('└')
        })
        .take(if short { 2 } else { 3 })
        .collect();
    for (index, line) in lesson_lines.iter().enumerate() {
        let line_rect = egui::Rect::from_min_size(
            egui::pos2(
                hero.left() + 20.0,
                hero.top()
                    + if short { 43.0 } else { 50.0 }
                    + index as f32 * if short { 29.0 } else { 34.0 },
            ),
            egui::vec2(hero.width() - 40.0, if short { 27.0 } else { 31.0 }),
        );
        ui.put(
            line_rect,
            egui::Label::new(
                egui::RichText::new(*line)
                    .size(12.0)
                    .color(colors::TEXT_SECONDARY),
            )
            .wrap(),
        );
    }
    let p = ui.painter().clone();
    let button_w = if mobile {
        rect.width().min(320.0)
    } else if compact {
        320.0
    } else {
        388.0
    };
    let button_h = if mobile || compact || short {
        46.0
    } else {
        62.0
    };
    let button = egui::Rect::from_min_size(
        egui::pos2(
            rect.center().x - button_w * 0.5,
            hero.bottom() + if compact || short { 8.0 } else { 14.0 },
        ),
        egui::vec2(button_w, button_h),
    );
    let response = ui.interact(button, egui::Id::new("tutorial-play"), egui::Sense::click());
    p.rect_filled(
        button,
        10.0,
        if response.hovered() {
            egui::Color32::from_rgb(255, 125, 32)
        } else {
            ORANGE
        },
    );
    let primary_label = if selected == 3 {
        "実映像でエフェクト実習を始める"
    } else {
        "レッスンを始める"
    };
    p.text(
        button.center(),
        egui::Align2::CENTER_CENTER,
        primary_label,
        egui::FontId::proportional(if mobile { 18.0 } else { 21.0 }),
        egui::Color32::WHITE,
    );
    if selected != 3 {
        p.text(
            egui::pos2(
                rect.center().x,
                button.bottom() + if compact { 10.0 } else { 16.0 },
            ),
            egui::Align2::CENTER_CENTER,
            lesson.hint,
            egui::FontId::proportional(13.0),
            colors::TEXT_SECONDARY,
        );
    }
    let _ = p;
    if response.clicked() {
        if selected == 3 {
            match materialize_tutorial_video() {
                Ok(path) => start_video_practice(app, ctx, path),
                Err(error) => app.toasts.error(error),
            }
        } else {
            app.show_guided_tutorial = false;
            app.show_home = false;
            app.ui_tabs.left_tab_idx = 0;
            app.ui_tabs.right_panel = crate::app_state::RightPanel::Properties;
            crate::ui::tutorial::start_at(app, lesson_index);
        }
    }

    if selected == 3 {
        p.text(
            egui::pos2(
                rect.center().x,
                button.bottom() + if short { 12.0 } else { 17.0 },
            ),
            egui::Align2::CENTER_CENTER,
            "同梱の短い動画にColor Tintを適用し、Viewerで再生します。",
            egui::FontId::proportional(12.0),
            colors::TEXT_SECONDARY,
        );
        let own_video_rect = egui::Rect::from_center_size(
            egui::pos2(
                rect.center().x,
                button.bottom() + if short { 38.0 } else { 53.0 },
            ),
            egui::vec2(button_w, 22.0),
        );
        let own_video = ui.interact(
            own_video_rect,
            egui::Id::new("tutorial-own-video-practice"),
            egui::Sense::click(),
        );
        p.text(
            own_video_rect.center(),
            egui::Align2::CENTER_CENTER,
            "自分の動画ファイルで実習する",
            egui::FontId::proportional(12.0),
            if own_video.hovered() {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_SECONDARY
            },
        );
        if own_video.clicked() {
            choose_video_for_practice(app, ctx);
        }
        p.text(
            egui::pos2(rect.center().x, own_video_rect.bottom() + 9.0),
            egui::Align2::CENTER_CENTER,
            "教材映像: Big Buck Bunny / Blender Foundation (CC BY 3.0)",
            egui::FontId::proportional(9.0),
            colors::TEXT_MUTED,
        );
    }
}

pub fn materialize_tutorial_video() -> Result<std::path::PathBuf, String> {
    use std::io::Write;

    let bytes = crate::ui::embedded_assets::bytes_for_path(std::path::Path::new(
        "assets/library/tutorial_practice.mp4",
    ))
    .ok_or_else(|| "同梱の実習動画が見つかりません。".to_string())?;
    let path = std::env::temp_dir().join(format!(
        "kagari_tutorial_practice_{}.mp4",
        std::process::id()
    ));
    if path.is_file() {
        if std::fs::read(&path).is_ok_and(|existing| existing.as_slice() == bytes) {
            return Ok(path);
        }
        std::fs::remove_file(&path)
            .map_err(|error| format!("実習動画の古い一時ファイルを削除できません: {error}"))?;
    }
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut file) => match file.write_all(bytes) {
            Ok(()) => Ok(path),
            Err(error) => {
                drop(file);
                let _ = std::fs::remove_file(&path);
                Err(format!("実習動画を準備できませんでした: {error}"))
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if std::fs::read(&path).is_ok_and(|existing| existing.as_slice() == bytes) {
                Ok(path)
            } else {
                Err("実習動画を別の処理が準備中です。もう一度お試しください。".to_string())
            }
        }
        Err(error) => Err(format!("実習動画を準備できませんでした: {error}")),
    }
}

pub fn choose_video_for_practice(app: &mut KagariApp, ctx: &egui::Context) {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("動画", &["mp4", "mov", "mkv", "avi", "webm", "av1"])
        .pick_file()
    else {
        return;
    };
    start_video_practice(app, ctx, path);
}

pub fn start_video_practice(app: &mut KagariApp, ctx: &egui::Context, path: std::path::PathBuf) {
    if !crate::core::video_import::ffmpeg_available() {
        app.toasts
            .error("動画実習にはFFmpegが必要です。FFmpegをPATHに追加してください。");
        return;
    }

    let name = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "video".to_string());
    app.show_guided_tutorial = false;
    app.show_home = false;
    app.ui_tabs.left_tab_idx = 0;
    app.ui_tabs.right_panel = crate::app_state::RightPanel::Properties;
    crate::ui::tutorial::start_at(app, 6);
    crate::ui::drop_import::start_video_tutorial_practice(app, ctx, path, name);
}
