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
        "Location" => "保存場所",
        "Color Space" => "カラースペース",
        "Advanced Settings" => "詳細設定",
        "Project defaults and cache options" => "プロジェクトの初期値とキャッシュ設定",
        "Create Project" => "プロジェクトを作成",
        "Learning" => "チュートリアル",
        "Settings" => "設定",
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
        "Performance" => "パフォーマンス",
        "Cache" => "キャッシュ",
        "CPU" => "CPU",
        "Color Management" => "カラーマネジメント",
        "Auto-save" => "自動保存",
        "UI Appearance" => "外観・UI",
        "Keyboard Shortcuts" => "キーボードショートカット",
        "Plugins" => "プラグイン",
        "Language" => "言語",
        "Display language" => "表示言語",
        "Frame cache budget:" => "フレームキャッシュメモリ:",
        "Adaptive preview quality (auto-reduce while playing)" => "再生中の解像度自動低下（アダプティブプレビュー）",
        "Undo steps:" => "取り消し回数:",
        "Undo history storage:" => "履歴メモリ使用量:",
        "Interval:" => "保存間隔:",
        "Preview audio during playback" => "再生時に音声をプレビュー",
        "Maximum Disk Cache Size:" => "最大ディスクキャッシュサイズ:",
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
        "Tutorial" => "チュートリアル",
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
        "Close" => "閉じる",
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
