use super::*;

impl AfterEffectsStudio {
    pub(crate) fn handle_navigation_setup_action(
        &mut self,
        action_name: &str,
        action_value: Option<ActionValue>,
    ) -> bool {
        match (action_name, action_value) {
            ("go_home", _) => {
                self.screen = Screen::Home;
                self.render_queue = false;
                self.docs_detail_page = false;
                true
            }
            ("go_projects", _) => {
                self.screen = Screen::Projects;
                self.render_queue = false;
                true
            }
            ("go_activity", _) => {
                self.screen = Screen::Activity;
                self.render_queue = false;
                true
            }
            ("go_documents", _) => {
                self.screen = Screen::Documents;
                self.render_queue = false;
                self.docs_detail_page = false;
                true
            }
            ("go_settings", _) => {
                self.settings_open = true;
                self.settings_category = 0;
                self.settings_feedback.clear();
                self.settings_snapshot = Some(self.settings_draft());
                true
            }
            ("docs_project_basics", _) => {
                self.set_document(
                "プロジェクトの基本",
                "新規プロジェクトを選び、名前、幅と高さ、フレームレート、長さを指定します。\n\nHD、Full HD、QHD、4K、縦型のプリセットを選んだ後も、幅や高さは個別に変更できます。作成すると編集画面に移動します。\n\n既存のプロジェクトはホームまたはプロジェクト一覧の「開く」から選びます。",
            );
                true
            }
            ("docs_editor_workflow", _) => {
                self.set_document(
                "編集画面",
                "左のシーン欄では粒子、文字、長方形、楕円レイヤーを追加できます。選択レイヤーは複製、重なり順の変更、名前変更、削除ができます。レイヤーを選ぶと、右のプロパティ欄に位置や種類に応じた設定が表示されます。\n\n素材欄では画像、音声、動画を読み込めます。タイムライン上のクリップをドラッグすると位置を移動でき、端をドラッグすると長さを調整できます。キーのひし形をクリックするとそのフレームへ移動し、上部の前後ボタンでもキー間を移動できます。\n\n編集内容は保存ボタンまたは ⌘ / Ctrl + S で保存します。保存先がない新規プロジェクトでは、最初の保存時にファイル名を指定します。",
            );
                true
            }
            ("docs_import", _) => {
                self.set_document(
                "素材の読み込み",
                "画像は PNG、JPEG、WebP、BMP、TIFF、SVG、音声は WAV、MP3、M4A、AAC、FLAC、OGG、AIFF、CAF を読み込めます。PNG・JPEG・WebPの連番は「連番」から先頭画像を選択してください。連番のファイル名は末尾が連続したフレーム番号で、画像サイズもそろっている必要があります。連番と動画はフレーム展開をバックグラウンドで行い、素材欄から中止できます。SVG内の外部ファイル参照は安全のため読み込みません。動画は MP4、MOV、MKV、AVI、WebM、M4V、MPEG、TS などに対応し、読み込みには FFmpeg が必要です。\n\n素材欄の画像・連番・音声・動画ボタンからファイルを指定するか、画像・音声・動画・3Dモデルを素材欄へドラッグ＆ドロップしてください。読み込み後は選択中のコンポジションにレイヤーとして追加されます。",
            );
                true
            }
            ("docs_export", _) => {
                self.set_document(
                "書き出し",
                "編集画面から「書き出し」を開きます。「現在のフレームを画像保存」では表示中の1フレームをPNGで保存できます。\n\n「動画を書き出す」ではコンポジション全体をH.264のMP4として保存します。動画出力にはFFmpegが必要です。保存先を選び、既存ファイルを置き換える場合は確認画面で上書きを選んでください。進行中の動画出力はキャンセルできます。",
            );
                true
            }
            ("docs_shortcuts", _) => {
                self.set_document(
                    "キーボードショートカット",
                    "⌘ / Ctrl + S  編集中のプロジェクトを保存\n⌘ / Ctrl + N  新規プロジェクト設定を開く\n⌘ / Ctrl + M  書き出し画面を開く（編集画面）",
                );
                true
            }
            ("docs_roto_brush", _)
            | ("docs_pro_keyer", _)
            | ("docs_aces_color", _)
            | ("docs_node_graph", _) => {
                let topic = match action_name {
                    "docs_roto_brush" => "roto_brush",
                    "docs_pro_keyer" => "pro_keyer",
                    "docs_aces_color" => "aces_color",
                    _ => "node_graph",
                };
                if let Some(entry) = kagari_vfx::core::studio_ux::lookup_japanese_help(topic) {
                    let body = format!(
                        "{}\n\n{}\n\nヒント: {}",
                        entry.summary_ja,
                        entry.steps_ja.join("\n"),
                        entry.shortcut_hint
                    );
                    self.set_document(entry.title_ja, &body);
                }
                true
            }
            ("open_project", _) => {
                #[cfg(target_os = "macos")]
                if let Some(sender) = self.app_event_sender.clone() {
                    std::thread::spawn(move || {
                        if let Ok(output) = std::process::Command::new("osascript")
                            .arg("-e")
                            .arg(r#"POSIX path of (choose file with prompt "プロジェクトを開く" of type {"json", "kgtpl", "mogrt", "otio"})"#)
                            .output()
                        {
                            if output.status.success() {
                                let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                                if !path.is_empty() {
                                    let _ = sender.send(format!("open_project_path:{path}"));
                                }
                            }
                        }
                    });
                    return true;
                }
                #[cfg(not(target_os = "macos"))]
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(
                        "Kagari プロジェクト / テンプレート / OTIO",
                        &["json", "kgtpl", "mogrt", "otio"],
                    )
                    .pick_file()
                {
                    self.request_project_open(&path);
                };
                true
            }
            ("new_project", _) => {
                self.request_new_project();
                true
            }
            ("quick_start_1080p60", _) => {
                self.create_project_with_settings("フルHD プロジェクト".to_owned(), 1920, 1080, 60, 10);
                true
            }
            ("quick_start_4k24", _) => {
                self.create_project_with_settings("4K プロジェクト".to_owned(), 3840, 2160, 24, 10);
                true
            }
            ("quick_start_vertical", _) => {
                self.create_project_with_settings("縦型ショート".to_owned(), 1080, 1920, 60, 10);
                true
            }
            ("quick_start_square", _) => {
                self.create_project_with_settings("正方形 SNS".to_owned(), 1080, 1080, 30, 10);
                true
            }
            ("setup_cancel", _) => {
                self.screen = self.setup_return_screen;
                true
            }
            ("setup_name_changed", Some(ActionValue::Text(value))) => {
                self.setup_name = value;
                true
            }
            ("setup_location_changed", Some(ActionValue::Text(value))) => {
                self.setup_location = value;
                true
            }
            ("setup_comp_name_changed", Some(ActionValue::Text(value))) => {
                self.setup_comp_name = value;
                true
            }
            ("setup_create_initial_comp_changed", Some(ActionValue::Checked(enabled))) => {
                self.setup_create_initial_comp = enabled;
                true
            }
            ("setup_toggle_advanced", _) => {
                self.setup_advanced_open = !self.setup_advanced_open;
                true
            }
            ("setup_preset_changed", Some(ActionValue::Text(preset))) => {
                match preset.as_str() {
                    "full_hd" => {
                        self.setup_width = "1920".to_owned();
                        self.setup_height = "1080".to_owned();
                        self.setup_fps = "60".to_owned();
                    }
                    "4k" => {
                        self.setup_width = "3840".to_owned();
                        self.setup_height = "2160".to_owned();
                        self.setup_fps = "24".to_owned();
                    }
                    "vertical" => {
                        self.setup_width = "1080".to_owned();
                        self.setup_height = "1920".to_owned();
                        self.setup_fps = "60".to_owned();
                    }
                    "square" => {
                        self.setup_width = "1080".to_owned();
                        self.setup_height = "1080".to_owned();
                        self.setup_fps = "30".to_owned();
                    }
                    _ => {}
                }
                self.setup_error.clear();
                true
            }
            ("setup_width_changed", Some(ActionValue::Text(value))) => {
                self.setup_width = value;
                self.setup_error.clear();
                true
            }
            ("setup_height_changed", Some(ActionValue::Text(value))) => {
                self.setup_height = value;
                self.setup_error.clear();
                true
            }
            ("setup_fps_changed", Some(ActionValue::Text(value))) => {
                self.setup_fps = value;
                self.setup_error.clear();
                true
            }
            ("setup_duration_changed", Some(ActionValue::Text(value))) => {
                self.setup_duration_seconds = value;
                self.setup_error.clear();
                true
            }
            ("setup_preset_hd", _) => {
                self.setup_width = "1280".to_owned();
                self.setup_height = "720".to_owned();
                self.setup_error.clear();
                true
            }
            ("setup_preset_full_hd", _) => {
                self.setup_width = "1920".to_owned();
                self.setup_height = "1080".to_owned();
                self.setup_error.clear();
                true
            }
            ("setup_preset_uhd", _) => {
                self.setup_width = "3840".to_owned();
                self.setup_height = "2160".to_owned();
                self.setup_error.clear();
                true
            }
            ("setup_preset_qhd", _) => {
                self.setup_width = "2560".to_owned();
                self.setup_height = "1440".to_owned();
                self.setup_error.clear();
                true
            }
            ("setup_preset_vertical", _) => {
                self.setup_width = "1080".to_owned();
                self.setup_height = "1920".to_owned();
                self.setup_error.clear();
                true
            }
            ("setup_preset_square", _) => {
                self.setup_width = "1080".to_owned();
                self.setup_height = "1080".to_owned();
                self.setup_error.clear();
                true
            }
            ("setup_preset_portrait_4_5", _) => {
                self.setup_width = "1080".to_owned();
                self.setup_height = "1350".to_owned();
                self.setup_error.clear();
                true
            }
            ("setup_fps_24", _) => {
                self.setup_fps = "24".to_owned();
                self.setup_error.clear();
                true
            }
            ("setup_fps_25", _) => {
                self.setup_fps = "25".to_owned();
                self.setup_error.clear();
                true
            }
            ("setup_fps_30", _) => {
                self.setup_fps = "30".to_owned();
                self.setup_error.clear();
                true
            }
            ("setup_fps_50", _) => {
                self.setup_fps = "50".to_owned();
                self.setup_error.clear();
                true
            }
            ("setup_fps_60", _) => {
                self.setup_fps = "60".to_owned();
                self.setup_error.clear();
                true
            }
            _ => false,
        }
    }
}
