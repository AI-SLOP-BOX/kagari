use super::*;

#[derive(Default)]
pub(super) struct ProjectToolbarActions {
    pub(super) add_comp_requested: bool,
    pub(super) import_file_requested: Option<std::path::PathBuf>,
    pub(super) add_folder_requested: bool,
    pub(super) remove_unused_requested: bool,
    pub(super) reduce_project_requested: bool,
}

pub(super) fn draw(
    ui: &mut egui::Ui,
    grid_view: bool,
    grid_view_id: egui::Id,
) -> ProjectToolbarActions {
    let mut actions = ProjectToolbarActions::default();
    ui.horizontal(|ui| {
        if project_icon_action(
            ui,
            "project-new-comp",
            crate::ui::icons::SVG_FILE_PLUS,
            "新規コンポジションを作成",
        )
        .clicked()
        {
            actions.add_comp_requested = true;
        }

        if project_icon_action(
            ui,
            "project-import",
            crate::ui::icons::SVG_IMPORT,
            "映像・音声を読み込む",
        )
        .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter(
                    "Media Footage",
                    &[
                        "png", "jpg", "jpeg", "webp", "obj", "wav", "mp3", "m4a", "flac", "ogg",
                        "aiff", "mp4", "mov", "mkv", "webm", "avi", "m4v", "mpeg", "mpg", "ts",
                        "m2ts", "av1", "ivf",
                    ],
                )
                .pick_file()
            {
                actions.import_file_requested = Some(path);
            }
        }

        ui.menu_button("...", |ui| {
            if ui.button("新規フォルダー").clicked() {
                actions.add_folder_requested = true;
            }

            if ui
                .button("未使用素材を削除")
                .on_hover_text("プロジェクトで使われていない素材を削除します")
                .clicked()
            {
                actions.remove_unused_requested = true;
            }

            if ui
                .button("プロジェクトを整理")
                .on_hover_text("アクティブなコンポジションと参照素材だけを残します")
                .clicked()
            {
                actions.reduce_project_requested = true;
            }
        });
        ui.separator();
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if project_view_toggle(
                ui,
                "project-view-list",
                SVG_LIST_VIEW,
                !grid_view,
                "一覧表示",
            )
            .clicked()
            {
                ui.ctx()
                    .data_mut(|data| data.insert_temp(grid_view_id, false));
            }
            if project_view_toggle(
                ui,
                "project-view-grid",
                crate::ui::icons::SVG_GRID,
                grid_view,
                "カード表示",
            )
            .clicked()
            {
                ui.ctx()
                    .data_mut(|data| data.insert_temp(grid_view_id, true));
            }
        });
    });
    actions
}
