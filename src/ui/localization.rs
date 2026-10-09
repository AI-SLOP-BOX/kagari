use std::sync::atomic::{AtomicU8, Ordering};

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
        "Select a layer to perform motion tracking." => "モーショントラッキングを実行するレイヤーを選択してください。",
        "No layer selected. Select a layer in timeline." => "レイヤーが選択されていません。タイムラインでレイヤーを選択してください。",
        "No layer selected" => "レイヤーが選択されていません",
        "Paint" => "ペイント",
        "Audio" => "オーディオ",
        "Audio Mixer" => "オーディオミキサー",
        "Export composition (Cmd+M)" => "コンポジションを書き出し (Cmd+M)",
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
