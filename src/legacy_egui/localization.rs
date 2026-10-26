use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::RwLock;

static CUSTOM_OVERRIDES: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);

fn overrides_file_path() -> std::path::PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let mut path = std::path::PathBuf::from(home);
        path.push(".kagari_ui_texts.json");
        path
    } else {
        std::path::PathBuf::from(".kagari_ui_texts.json")
    }
}

pub fn load_custom_texts() {
    let path = overrides_file_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&content) {
            if let Ok(mut lock) = CUSTOM_OVERRIDES.write() {
                *lock = Some(map);
            }
        }
    }
}

pub fn save_custom_text(key: &str, replacement: &str) {
    let mut map = {
        let lock = CUSTOM_OVERRIDES.read().ok();
        lock.and_then(|l| l.clone()).unwrap_or_default()
    };
    if replacement.trim().is_empty() {
        map.remove(key);
    } else {
        map.insert(key.to_string(), replacement.to_string());
    }
    let path = overrides_file_path();
    if let Ok(json) = serde_json::to_string_pretty(&map) {
        let _ = std::fs::write(&path, json);
    }
    if let Ok(mut lock) = CUSTOM_OVERRIDES.write() {
        *lock = Some(map);
    }
}

pub fn get_all_custom_texts() -> HashMap<String, String> {
    let lock = CUSTOM_OVERRIDES.read().ok();
    lock.and_then(|l| l.clone()).unwrap_or_default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum LanguagePreference {
    #[default]
    System,
    English,
    Japanese,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiLanguage {
    English,
    Japanese,
}

static ACTIVE_LANGUAGE: AtomicU8 = AtomicU8::new(0);

fn system_language() -> UiLanguage {
    for locale in sys_locale::get_locales() {
        let language = locale
            .split(['-', '_'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        match language.as_str() {
            "ja" => return UiLanguage::Japanese,
            "en" => return UiLanguage::English,
            _ => {}
        }
    }
    UiLanguage::English
}

pub fn set_preference(preference: LanguagePreference) {
    let language = match preference {
        LanguagePreference::System => system_language(),
        LanguagePreference::English => UiLanguage::English,
        LanguagePreference::Japanese => UiLanguage::Japanese,
    };
    ACTIVE_LANGUAGE.store(
        match language {
            UiLanguage::English => 1,
            UiLanguage::Japanese => 2,
        },
        Ordering::Relaxed,
    );
}

pub fn current_language() -> UiLanguage {
    match ACTIVE_LANGUAGE.load(Ordering::Relaxed) {
        1 => UiLanguage::English,
        2 => UiLanguage::Japanese,
        _ => system_language(),
    }
}

pub fn tr(english: &str) -> String {
    // Check user dynamic overrides first!
    if let Ok(lock) = CUSTOM_OVERRIDES.read() {
        if let Some(ref map) = *lock {
            if let Some(custom) = map.get(english) {
                return custom.clone();
            }
        }
    }

    if current_language() == UiLanguage::English {
        return english.to_owned();
    }
    match english {
        "Home" => "ホーム",
        "Edit" => "編集",
        "File" => "ファイル",
        "Viewer" => "ビューア",
        "Panels" => "パネル",
        "Undo" => "元に戻す",
        "Redo" => "やり直す",
        "Assets" => "素材",
        "Projects" => "プロジェクト",
        "Recent" => "最近使った項目",
        "Starred" => "お気に入り",
        "Templates" => "テンプレート",
        "Project" => "プロジェクト",
        "Fit" => "フィット",
        "Composition Settings" => "コンポジション設定",
        "Full Quality" => "フル品質",
        "Adaptive Resolution" => "適応解像度",
        "Fast Draft" => "高速ドラフト",
        "Wireframe" => "ワイヤーフレーム",
        "Open Project" => "プロジェクトを開く",
        "New Project" => "新規プロジェクト",
        "Compositing" => "編集",
        "Create a New Project" => "新しいプロジェクトを作成",
        "Project Name" => "プロジェクト名",
        "Untitled Project" => "名称未設定のプロジェクト",
        "Frame Rate" => "フレームレート",
        "Resolution" => "解像度",
        "Location" => "保存場所",
        "Color Space" => "カラースペース",
        "Advanced Settings" => "詳細設定",
        "Project defaults and cache options" => "プロジェクトの初期値とキャッシュ設定",
        "Create Project" => "プロジェクトを作成",
        "Settings" => "設定",
        "Startup" => "起動時",
        "On launch" => "起動時に開く画面",
        "Studio" => "スタジオ",
        "Theme" => "テーマ",
        "Confirm before quitting the application" => "アプリケーション終了前に確認する",
        "Use GPU acceleration for supported effects" => "対応エフェクトでGPUアクセラレーションを使用",
        "GPU effects use the active adapter when available." => "GPU対応エフェクトでは利用可能なデバイスを使用します。",
        "Browse Recovery Snapshots…" => "復旧スナップショットを表示…",
        "Undo History" => "操作履歴",
        "Tracker" => "トラッカー",
        "Motion Source: {}" => "モーションソース: {}",
        "Tracker points: {}" => "トラックポイント: {}",
        "+ Add Tracker Point" => "+ トラックポイントを追加",
        "Active Tracker Point:" => "アクティブなトラックポイント:",
        "Analyze Forward (Work Area)" => "前方へ解析（ワークエリア）",
        "Markerless Optical Flow" => "マーカーレス光学フロー",
        "Estimate Markerless Pose" => "マーカーレスポーズを推定",
        "3D Camera Tracker (Scene Reconstruction)" => "3Dカメラトラッカー（シーン再構築）",
        "Shot Type:" => "ショットタイプ:",
        "Fixed Angle of View" => "画角固定",
        "Variable Zoom" => "可変ズーム",
        "Track Points:" => "トラックポイント:",
        "Track & Solve 3D Camera" => "3Dカメラをトラッキング＆解析",
        "Solving 3D Camera…" => "3Dカメラを解析中…",
        "☕ Mocha Planar Surface Tracker" => "☕ Mocha 平面サーフェストラッカー",
        "Track Motion:" => "モーションをトラッキング:",
        "Auto-Trace & Roto Assist" => "オートトレース ＆ Roto アシスト",
        "Auto-Trace Settings" => "オートトレース設定",
        "Roto Brush & Refine Edge" => "Roto Brush ＆ エッジ調整",
        "Select a layer to perform motion tracking." => {
            "モーショントラッキングを実行するレイヤーを選択してください。"
        }
        "No layer selected. Select a layer in timeline." => {
            "レイヤーが選択されていません。タイムラインでレイヤーを選択してください。"
        }
        "No layer selected" => "レイヤーが選択されていません",
        "Paint" => "ペイント",
        "Audio" => "オーディオ",
        "Audio Mixer" => "オーディオミキサー",
        "Export composition (Cmd+M)" => "コンポジションを書き出し (Cmd+M)",
        "Anchor Point" => "アンカーポイント",
        "Inspector" => "インスペクター",
        "Preview" => "プレビュー",
        "Timeline" => "タイムライン",
        "Project Panel" => "プロジェクトパネル",
        "Render Queue" => "レンダークュー",
        "Video" => "ビデオ",
        "Image" => "イメージ",
        "Solid" => "ソリッド",
        "Text" => "テキスト",
        "Null" => "ヌル",
        "Mask" => "マスク",
        "Layer" => "レイヤー",
        "Transform" => "トランスフォーム",
        "Effects" => "エフェクト",
        "Tools" => "ツール",
        "Opacity" => "不透明度",
        "Scale" => "スケール",
        "Rotation" => "回転",
        "Crop" => "トリム",
        "Parent" => "親",
        "Motion" => "モーション",
        "Adjust" => "調整",
        "Open in Viewer" => "ビューアで開く",
        "Render" => "書き出し",
        "Preferences" => "環境設定",
        "General" => "一般",
        "Appearance" => "外観",
        "Interface" => "インターフェース",
        "Media & Cache" => "メディアとキャッシュ",
        "Memory & Performance" => "メモリとパフォーマンス",
        "Audio Hardware" => "オーディオハードウェア",
        "Custom Workspaces" => "カスタムワークスペース",
        "Workspaces" => "ワークスペース",
        "Standard" => "標準",
        "Default AE 3-Column Layout" => "標準の3列レイアウト",
        "Small Screen" => "小画面",
        "Compact mode for laptops" => "ノートPC向けのコンパクト表示",
        "Motion Tracking" => "モーショントラッキング",
        "Tracker & viewport controls" => "トラッカーとビューポート操作",
        "Paint & Roto" => "ペイントとロト",
        "Paint brushes & vector masks" => "ペイントブラシとベクターマスク",
        "Text & Type" => "テキストと文字組み",
        "Character & paragraph panels" => "文字と段落のパネル",
        "Color & Grading" => "カラーとグレーディング",
        "Color management & VU meters" => "カラーマネジメントとVUメーター",
        "Minimal" => "ミニマル",
        "Maximizes viewport canvas" => "ビューポートを最大化",
        "Audio mixer & waveform" => "オーディオミキサーと波形",
        "3D Layout" => "3Dレイアウト",
        "Camera views & 3D options" => "カメラビューと3Dオプション",
        "Animation" => "アニメーション",
        "Keyframes, graph editor, presets" => "キーフレーム、グラフエディター、プリセット",
        "Effect controls & library focus" => "エフェクト操作とライブラリ",
        "Essential Graphics" => "エッセンシャルグラフィックス",
        "Master properties panel" => "マスタープロパティパネル",
        "Save Current" => "現在の配置を保存",
        "Workspace saved" => "ワークスペースを保存しました",
        "Workspace reset to default" => "ワークスペースを既定値に戻しました",
        "Performance" => "パフォーマンス",
        "Cache" => "キャッシュ",
        "CPU" => "CPU",
        "Color Management" => "カラーマネジメント",
        "Auto-save" => "自動保存",
        "UI Appearance" => "外観・UI",
        "Keyboard Shortcuts" => "キーボードショートカット",
        "Plugins" => "プラグイン",
        "Language" => "言語",
        "All" => "すべて",
        "Layers" => "レイヤー",
        "Command" => "コマンド",
        "Shortcut" => "ショートカット",
        "Unassigned" => "未割り当て",
        "Edit shortcut" => "ショートカットを変更",
        "Search the commands available in Kagari VFX" => "Kagari VFXで使えるコマンドを検索できます",
        "Keyboard Shortcuts Reference..." => "ショートカットを表示・変更…",
        "Changes are saved automatically." => "変更は自動で保存されます。",
        "Press a key combination…  Esc cancels · Backspace clears" => "キーの組み合わせを入力… Escでキャンセル・Backspaceで解除",
        "Import…" => "読み込み…",
        "Export…" => "書き出し…",
        "Reset" => "既定値に戻す",
        "Close" => "閉じる",
        "Apply" => "適用",
        "Reset to Defaults" => "既定値に戻す",
        "Disk cache usage" => "ディスクキャッシュ使用量",
        "Disk Cache" => "ディスクキャッシュ",
        "Preview and Memory" => "プレビューとメモリ",
        "Recovery and Auto-save" => "復旧と自動保存",
        "Auto-save interval:" => "自動保存間隔:",
        "Snapshot retention:" => "復旧スナップショットの保持数:",
        "1 minute" => "1分",
        "3 minutes" => "3分",
        "5 minutes" => "5分",
        "10 minutes" => "10分",
        "15 minutes" => "15分",
        "30 minutes" => "30分",
        "Custom" => "カスタム",
        "Studio Theme" => "スタジオテーマ",
        "Audio Preview" => "オーディオプレビュー",
        "Working Color Space" => "作業色空間",
        "Plugin Support" => "プラグイン対応",
        "Current composition" => "現在のコンポジション",
        "Bit depth" => "ビット深度",
        "Open Composition Settings" => "コンポジション設定を開く",
        "Recovery snapshots" => "復旧スナップショット",
        "Search commands or shortcuts..." => "コマンドやショートカットを検索...",
        "No matching shortcuts" => "一致するショートカットはありません",
        "Shortcuts are read-only in this version." => "このバージョンではショートカットは参照のみです。",
        "The current studio theme" => "現在のスタジオテーマ",
        "Warm orange selection · high-contrast panels" => "オレンジの選択色・高コントラストのパネル",
        "Current RAM cache" => "現在のRAMキャッシュ",
        "Preview renderer" => "プレビューレンダラー",
        "Software" => "ソフトウェア",
        "Available" => "利用可能",
        "Host bridge" => "ホストブリッジ",
        "Host supported" => "ホスト対応",
        "Kagari VFX · Application settings" => "Kagari VFX・アプリケーション設定",
        "5 rotating slots" => "5世代をローテーション",
        "Recovery snapshots are created after project changes and retained across restarts." => "変更後に復旧用スナップショットを作成し、再起動後も保持します。",
        "Audio output follows the active system device." => "音声は現在のシステム出力デバイスで再生します。",
        "Color space and bit depth are configured per composition." => "色空間とビット深度はコンポジションごとに設定します。",
        "Browse the commands and gestures available in the current workspace." => "現在のワークスペースで使えるコマンドと操作方法を確認できます。",
        "Basic Composition Settings" => "コンポジションの基本設定",
        "Composition Name" => "コンポジション名",
        "Restore previous state when loading a project" => "プロジェクト読込時に前回の状態を復元",
        "Confirm before closing a project" => "プロジェクトを閉じる前に確認",
        "Close Project?" => "プロジェクトを閉じますか？",
        "Close Project" => "プロジェクトを閉じる",
        "Return to Home and leave the current project?" => "ホームに戻って現在のプロジェクトを閉じますか？",
        "Duration" => "デュレーション",
        "Advanced" => "詳細設定",
        "Black" => "黒",
        "White" => "白",
        "Transparent" => "透明",
        "Preset" => "プリセット",
        "Width" => "幅",
        "Height" => "高さ",
        "Pixel Aspect" => "ピクセル縦横比",
        "Start Timecode" => "開始タイムコード",
        "Frame Rate (FPS)" => "フレームレート (FPS)",
        "Duration (Frames)" => "デュレーション (フレーム)",
        "Depth" => "ビット深度",
        "Shutter Angle" => "シャッター角度",
        "Shutter Phase" => "シャッターフェーズ",
        "Background Color" => "背景色",
        "One-Tap Smart SNS Reframer" => "SNS向けアスペクト比に自動調整",
        "Auto-remap layer positions to new Aspect Ratio:" => "レイヤー位置を新しいアスペクト比に合わせて調整:",
        "Shorts 9:16" => "ショート動画 9:16",
        "Square 1:1" => "スクエア 1:1",
        "Cinema 21:9" => "シネマ 21:9",
        "Blending" => "合成",
        "Blend colors in linear light (1.0 gamma)" => "リニア光で色を合成 (ガンマ 1.0)",
        "Output dithering (smooth gradients)" => "出力ディザリング (グラデーションを滑らかに)",
        "Color Management & Working Space" => "カラーマネジメントと作業色空間",
        "Rec.709 / sRGB" => "Rec.709 / sRGB",
        "Rec.2020 (UHDTV)" => "Rec.2020 (UHDTV)",
        "Linear sRGB" => "リニア sRGB",
        "ACEScg (AP1)" => "ACEScg (AP1)",
        "8 bpc (Integer)" => "8 bpc (整数)",
        "16 bpc (Half Float)" => "16 bpc (半精度浮動小数点)",
        "32 bpc (Float / HDR)" => "32 bpc (浮動小数点 / HDR)",
        "Working color space converts scene pixels into the target color gamut and monitor display profile." => "作業色空間はシーンの画素を出力色域とモニタープロファイルに合わせます。",
        "Shifts the shutter window relative to the frame time" => "フレーム時刻に対するシャッター区間の位置を調整します。",
        "Film (24 fps)" => "フィルム (24 fps)",
        "PAL (25 fps)" => "PAL (25 fps)",
        "NTSC Video (30 fps)" => "NTSCビデオ (30 fps)",
        "Smooth (60 fps)" => "スムーズ (60 fps)",
        "High-Speed (120 fps)" => "ハイスピード (120 fps)",
        "Square (1.0)" => "正方形 (1.0)",
        "HD Anamorphic (1.2)" => "HDアナモフィック (1.2)",
        "HDTV 1080 30fps (1920 x 1080)" => "HDTV 1080 30fps (1920 x 1080)",
        "4K UHD 60fps (3840 x 2160)" => "4K UHD 60fps (3840 x 2160)",
        "720p HD 24fps (1280 x 720)" => "720p HD 24fps (1280 x 720)",
        "Shorts / Reels (1080 x 1920)" => "ショート動画 / Reels (1080 x 1920)",
        "Instagram Feed (1080 x 1080)" => "Instagramフィード (1080 x 1080)",
        "Cycle through selected layers" => "選択したレイヤーを切り替える",
        "Start RAM preview" => "RAMプレビューを開始",
        "Create Solid Layer" => "平面レイヤーを作成",
        "Create Text Layer" => "テキストレイヤーを作成",
        "Create Null Layer" => "ヌルレイヤーを作成",
        "Create Adjustment Layer" => "調整レイヤーを作成",
        "Pre-compose selected layers" => "選択レイヤーをプリコンポーズ",
        "Toggle Motion Sketch" => "モーションスケッチを切り替え",
        "Fit selected layers in Viewer, or fit composition" => "選択レイヤーまたはコンポジションをビューアーに合わせる",
        "Fit composition in Viewer" => "コンポジションをビューアーに合わせる",
        "Return to previous composition" => "前のコンポジションに戻る",
        "Fit selected layer to composition" => "選択レイヤーをコンポジションに合わせる",
        "Split selected layer at playhead" => "再生ヘッド位置でレイヤーを分割",
        "Add or remove layer marker" => "レイヤーマーカーを追加／削除",
        "Add or remove composition marker" => "コンポジションマーカーを追加／削除",
        "Go to next composition marker" => "次のコンポジションマーカーへ移動",
        "Go to previous composition marker" => "前のコンポジションマーカーへ移動",
        "Reveal keyframed properties; press twice for modified properties" => "キーフレームのあるプロパティを表示。2回押すと変更済みのプロパティを表示",
        "Nudge selected layer (Shift for larger steps)" => "選択レイヤーを微調整 (Shiftで移動量を拡大)",
        "Step one frame backward / forward" => "1フレーム戻る／進む",
        "Zoom timeline in / out" => "タイムラインを拡大／縮小",
        "Deselect current selection" => "現在の選択を解除",
        "Rename selected layer" => "選択レイヤーの名前を変更",
        "Delete selected layers or keyframes" => "選択レイヤーまたはキーフレームを削除",
        "Open inline numeric editor" => "数値を直接編集",
        "Constrain rotation or movement" => "回転または移動を軸に固定",
        "Set work area in / out" => "ワークエリアの開始／終了を設定",
        "Add or remove audio fade" => "オーディオフェードを追加／削除",
        "Select keyframes with a marquee" => "ドラッグ範囲でキーフレームを選択",
        "Open keyframe actions" => "キーフレーム操作を開く",
        "Slip edit layer timing" => "レイヤーのタイミングをスリップ編集",
        "Scale selected layer" => "選択レイヤーを拡大／縮小",
        "Open Command Palette" => "コマンドパレットを開く",
        "Play / Stop RAM Preview" => "RAMプレビューを再生／停止",
        "Save Project" => "プロジェクトを保存",
        "New Composition" => "新規コンポジション",
        "Open Render Queue / Export" => "レンダーキュー／書き出しを開く",
        "Apply Easy Ease to selected keyframes" => "選択したキーフレームにイージーイーズを適用",
        "Duplicate Selected Layer" => "選択レイヤーを複製",
        "Selection Tool" => "選択ツール",
        "Hand / Pan Tool" => "手のひら／パンツール",
        "Zoom Tool" => "ズームツール",
        "Rotation Tool" => "回転ツール",
        "Pan Behind / Anchor Point Tool" => "アンカーポイントツール",
        "Shape Tool" => "シェイプツール",
        "Pen / Vector Path Tool" => "ペン／ベクターパスツール",
        "3D Camera Tool" => "3Dカメラツール",
        "Text Tool" => "テキストツール",
        "Puppet Pin Tool" => "パペットピンツール",
        "Jump to Previous Keyframe" => "前のキーフレームへ移動",
        "Jump to Next Keyframe" => "次のキーフレームへ移動",
        "Play Forward (press again to increase speed)" => "順方向に再生（再度押すと速度を上げる）",
        "Set Work Area Start" => "ワークエリアの開始点を設定",
        "Set Work Area End" => "ワークエリアの終了点を設定",
        "Jump to Layer In-Point" => "レイヤーのインポイントへ移動",
        "Jump to Layer Out-Point" => "レイヤーのアウトポイントへ移動",
        "Select Position Property" => "位置プロパティを選択",
        "Select Scale Property" => "スケールプロパティを選択",
        "Select Rotation Property" => "回転プロパティを選択",
        "Select Opacity Property" => "不透明度プロパティを選択",
        "Reveal Anchor Point Property" => "アンカーポイントプロパティを表示",
        "Display language" => "表示言語",
        "Frame cache budget:" => "RAMキャッシュ:",
        "Memory limit:" => "メモリ上限:",
        "Adaptive preview quality (auto-reduce while playing)" => "再生中の解像度自動低下（アダプティブプレビュー）",
        "Undo steps:" => "取り消し回数:",
        "Undo history storage:" => "履歴メモリ使用量:",
        "Interval:" => "保存間隔:",
        "Preview audio during playback" => "再生時に音声をプレビュー",
        "Maximum Disk Cache Size:" => "最大サイズ:",
        "Choose Cache Folder..." => "キャッシュフォルダーを選択...",
        "Empty Disk Cache..." => "ディスクキャッシュを消去...",
        "Save" => "保存",
        "Cancel" => "キャンセル",
        "System" => "システム設定",
        "Japanese" => "日本語",
        "Asset Library" => "素材ライブラリ",
        "Help" => "ヘルプ",
        "Search projects..." => "プロジェクトを検索…",
        "Simply turning imagination into video." => "シンプルに、想像を映像に。",
        "+ New Project" => "＋ 新規プロジェクト",
        "Recent Projects" => "最近開いたプロジェクト",
        "View All →" => "すべて表示 →",
        "Recent Work" => "最近の作業",
        "No recent projects" => "最近開いたプロジェクトはありません",
        "No matching projects found" => "一致するプロジェクトが見つかりません",
        "Rich VFX Effects" => "豊富なVFXエフェクト",
        "Ready-to-use animations" => "すぐに使える演出",
        "Video, Audio & Images" => "動画・音声・画像",
        "Learn how to use" => "使い方を学ぶ",
        "All Projects" => "すべてのプロジェクト",
        "No projects yet" => "プロジェクトがまだありません",
        "Create a project with \"+ New Project\" or open an existing project." => "「＋ 新規プロジェクト」から作成するか、既存のプロジェクトを開いてください。",
        "Easy Ease" => "イージーイーズ",
        "Easy Ease (F9)" => "イージーイーズ (F9)",
        "Smart Presets..." => "プリセット…",
        "Physics Spring" => "スプリング",
        "⏸ Hold" => "⏸ 停止",
        "Linear" => "リニア",
        "⏩ 2x Speed" => "⏩ 2倍速",
        "⏪ 0.5x Speed" => "⏪ 0.5倍速",
        "Render All Queue (Cmd+M)" => "すべてのキューを書き出し (Cmd+M)",
        "+ Add Active Comp" => "＋ 現在のコンポを追加",
        "Clear Queue" => "キューを消去",
        "↻ Retry Failed" => "↻ 失敗した項目を再試行",
        "Export Farm Job Manifest" => "レンダーファーム構成を書き出し",
        "Render Settings" => "レンダリング設定",
        "Export Composition Video" => "コンポジション動画の書き出し",
        "Output Path:" => "出力先:",
        "Browse..." => "参照...",
        "Target FPS:" => "フレームレート:",
        "Frame Range" => "フレーム範囲",
        "Start Async Render" => "バックグラウンドで書き出し開始",
        "Cancel Render" => "書き出しを中止",
        "Standard Ease" => "標準イーズ",
        "Fast Acceleration" => "急加速",
        "Smooth Deceleration" => "スムーズ減速",
        "Spring Overshoot" => "バウンスオーバーシュート",
        "Sine Wave" => "サイン波",
        "Fast Out" => "ファストアウト",
        "Slow In" => "スローイン",
        "Elastic" => "エラスティック",
        "Bounce" => "バウンス",
        "Cycle" => "サイクル",
        "Mirror" => "ミラー",
        "Quadratic In" => "2次加速",
        "Quadratic Out" => "2次減速",
        _ => english,
    }
    .to_owned()
}

pub fn tr_format(template: &str, arguments: &[&str]) -> String {
    let translated = tr(template);
    let mut result = String::with_capacity(translated.len() + 16);
    let mut current = translated.as_str();
    for argument in arguments {
        if let Some(index) = current.find("{}") {
            result.push_str(&current[..index]);
            result.push_str(argument);
            current = &current[index + 2..];
        } else {
            break;
        }
    }
    result.push_str(current);
    result
}
