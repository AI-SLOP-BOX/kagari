//! Stock assets pack modal dialog and copying stock assets into user projects.

use crate::KagariApp;
use eframe::egui;

pub fn draw_stock_assets_dialog(app: &mut KagariApp, ctx: &egui::Context) {
    let dialog_id = egui::Id::new("asset-library-stock-dialog");
    let mut open = ctx
        .data(|data| data.get_temp::<bool>(dialog_id))
        .unwrap_or(false);
    if !open {
        return;
    }
    let stock = [
        (
            "フィルムグレイン",
            "texture_film_grain.webp",
            "粒子感・質感合成",
        ),
        (
            "クラウドノイズ",
            "texture_cloud_noise.webp",
            "変位・ルミナンスマット",
        ),
    ];
    egui::Window::new("Kagari 素材パック")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label("素材をプロジェクト用フォルダーへコピーし、アセットに追加します。");
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                for (name, filename, kind) in stock {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_width(184.0);
                        let texture_id = egui::Id::new(("stock-asset-preview", filename));
                        let texture =
                            ctx.data(|data| data.get_temp::<egui::TextureHandle>(texture_id));
                        let texture = texture.or_else(|| {
                            let path = std::path::PathBuf::from("assets/studio").join(filename);
                            let image = crate::ui::embedded_assets::open_image(path)
                                .ok()?
                                .thumbnail(320, 180)
                                .to_rgba8();
                            let size = [image.width() as usize, image.height() as usize];
                            let texture = ctx.load_texture(
                                format!("stock-asset:{filename}"),
                                egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
                                egui::TextureOptions::LINEAR,
                            );
                            ctx.data_mut(|data| data.insert_temp(texture_id, texture.clone()));
                            Some(texture)
                        });
                        if let Some(texture) = texture {
                            ui.image((texture.id(), egui::vec2(168.0, 94.0)));
                        }
                        ui.label(name);
                        ui.weak(kind);
                        if ui.button("プロジェクトに追加").clicked() {
                            add_stock_asset_to_project(app, filename, name);
                        }
                    });
                }
            });
        });
    ctx.data_mut(|data| data.insert_temp(dialog_id, open));
}

pub fn add_stock_asset_to_project(app: &mut KagariApp, filename: &str, name: &str) {
    let project_path = std::path::PathBuf::from(&app.project_path);
    if !project_path.is_absolute() || !project_path.is_file() {
        app.toasts
            .info("先にプロジェクトを保存すると、素材を同じ場所へ追加できます");
        return;
    }
    let Some(project_dir) = project_path.parent() else {
        app.toasts
            .error("プロジェクトの保存先を確認できませんでした");
        return;
    };
    let media_dir = project_dir.join("Kagari Media");
    let source_name = std::path::Path::new(filename);
    let stem = source_name
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("stock_asset");
    let extension = source_name
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("webp");
    let mut destination = media_dir.join(filename);
    let mut suffix = 2;
    while destination.exists() {
        destination = media_dir.join(format!("{stem}_{suffix}.{extension}"));
        suffix += 1;
    }
    let project = app.history.current();
    if project.assets.iter().any(|item| {
        item.name == name
            && item.media_path().is_some_and(|path| {
                std::path::Path::new(path).parent().is_some_and(|parent| {
                    parent
                        .file_name()
                        .is_some_and(|folder| folder == "Kagari Media")
                })
            })
    }) {
        app.toasts
            .info("この素材はすでにプロジェクトに追加されています");
        return;
    }
    let embedded_path = std::path::PathBuf::from("assets/studio").join(filename);
    let Some(bytes) = crate::ui::embedded_assets::bytes_for_path(&embedded_path) else {
        app.toasts.error("内蔵素材を読み込めませんでした");
        return;
    };
    if let Err(error) =
        std::fs::create_dir_all(&media_dir).and_then(|()| std::fs::write(&destination, bytes))
    {
        app.toasts
            .error(format!("素材をコピーできませんでした: {error}"));
        return;
    }
    let Ok((width, height)) = image::image_dimensions(&destination) else {
        app.toasts
            .error("コピーした素材の画像情報を読み込めませんでした");
        return;
    };
    let path = destination.to_string_lossy().into_owned();
    let display_name = name.to_owned();
    app.modify_project(|project| {
        let id = project.next_project_item_id("stock_image");
        project.assets.push(crate::core::timeline::ProjectItem::new(
            id,
            display_name,
            crate::core::timeline::ProjectItemType::Image {
                path,
                width,
                height,
            },
        ));
    });
    app.toasts.info(format!("「{name}」を素材に追加しました"));
}
