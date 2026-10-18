//! Data model and static step/chapter definitions for the interactive tutorial.

pub struct TutorialStep {
    pub chapter: TutorialChapterId,
    pub title: &'static str,
    pub body: &'static [&'static str],
    /// Actionable "try it now" hint shown in accent color
    pub hint: &'static str,
    /// Keyboard shortcut shown in a pill badge (empty = none)
    pub shortcut: TutorialShortcut,
    /// Extra tip shown only in Pro mode or as expandable text
    pub advanced_tip: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TutorialChapterId {
    Introduction,
    Composition,
    Animation,
    Effects,
    Advanced,
    Export,
}

#[derive(Clone, Copy)]
pub enum TutorialShortcut {
    None,
    Binding(crate::ui::shortcuts::ShortcutId),
    Bindings(&'static [crate::ui::shortcuts::ShortcutId]),
    Gesture(&'static str),
}

impl TutorialShortcut {
    pub fn label(self) -> Option<String> {
        use crate::ui::shortcuts::{display_shortcut, format_platform_shortcut_hint};
        match self {
            Self::None => None,
            Self::Binding(id) => Some(display_shortcut(id)),
            Self::Bindings(ids) => Some(
                ids.iter()
                    .map(|id| display_shortcut(*id))
                    .collect::<Vec<_>>()
                    .join(" / "),
            ),
            Self::Gesture(gesture) => Some(format_platform_shortcut_hint(gesture)),
        }
    }
}

pub struct TutorialChapter {
    pub id: TutorialChapterId,
    pub title: &'static str,
    pub description: &'static str,
    pub first_step: usize,
}

pub fn chapters() -> &'static [TutorialChapter] {
    const CHAPTERS: &[TutorialChapter] = &[
        TutorialChapter {
            id: TutorialChapterId::Introduction,
            title: "はじめに",
            description: "画面構成とツールの使い方",
            first_step: 0,
        },
        TutorialChapter {
            id: TutorialChapterId::Composition,
            title: "コンポジションとレイヤー",
            description: "映像の土台とレイヤー管理",
            first_step: 2,
        },
        TutorialChapter {
            id: TutorialChapterId::Animation,
            title: "アニメーション",
            description: "キーフレームと動きの調整",
            first_step: 4,
        },
        TutorialChapter {
            id: TutorialChapterId::Effects,
            title: "エフェクト",
            description: "エフェクトとエクスプレッション",
            first_step: 6,
        },
        TutorialChapter {
            id: TutorialChapterId::Advanced,
            title: "3Dとトラッキング",
            description: "3Dカメラと実映像の追跡",
            first_step: 8,
        },
        TutorialChapter {
            id: TutorialChapterId::Export,
            title: "書き出し",
            description: "形式を選んで映像を書き出す",
            first_step: 10,
        },
    ];
    CHAPTERS
}

pub fn chapter_name(id: TutorialChapterId) -> &'static str {
    chapters()
        .iter()
        .find(|chapter| chapter.id == id)
        .map(|chapter| chapter.title)
        .unwrap_or("チュートリアル")
}

/// Full 12-step walkthrough — from first launch to professional export.
pub fn steps() -> &'static [TutorialStep] {
    &[
        // ── Chapter 1: Interface ─────────────────────────────────────────────
        TutorialStep {
            chapter: TutorialChapterId::Introduction,
            title: "1/12 — 画面の読み方",
            body: &[
                "Kagari VFXは作業内容ごとにワークスペースを切り替えて使います。",
                "「編集」では、左に素材、中央にViewer、下にTimeline、右にInspectorが並びます。",
                "起動直後にコンポジションがない場合、ViewerとTimelineは空です。",
                "上部の「編集：コンポ」から編集ワークスペースへ移動し、パネルの位置を確認しましょう。",
            ],
            hint: "試してみよう: 上部のワークスペースから「編集」を開く",
            shortcut: TutorialShortcut::None,
            advanced_tip: "上部のワークスペースタブから作業画面を切り替えられます。",
        },
        TutorialStep {
            chapter: TutorialChapterId::Introduction,
            title: "2/12 — ツールバーとビューポート",
            body: &[
                "Viewerは、選択中のコンポジションを表示する作業領域です。",
                "コンポジションがない間は、先に作成する必要があります。",
                "Viewerのツールバーには選択・手のひら・ズームなどの操作があります。",
                "表示倍率はViewerの表示メニューから変更できます。",
            ],
            hint: "試してみよう: コンポジションを作成してViewerに表示する",
            shortcut: TutorialShortcut::Binding(crate::ui::shortcuts::ShortcutId::SelectionTool),
            advanced_tip: "Viewerの表示倍率を変更しても、コンポジションの出力解像度は変わりません。",
        },
        // ── Chapter 2: Composition & Layers ──────────────────────────────────
        TutorialStep {
            chapter: TutorialChapterId::Composition,
            title: "3/12 — コンポジションを作る",
            body: &[
                "コンポジションは、映像の解像度・フレームレート・長さを決める作業の土台です。",
                "  • 解像度: 1920×1080 (Full HD) など",
                "  • フレームレート: 24fps、30fps、60fps など",
                "  • デュレーション: コンポジションの長さ",
                "を設定して作成します。",
                "上部メニューの Composition > New Composition から作成します。",
                "複数のコンポジションはプリコンポーズして整理できます。",
            ],
            hint: "試してみよう: Composition > New Composition で1920×1080 / 30fps / 10秒を作成",
            shortcut: TutorialShortcut::Binding(crate::ui::shortcuts::ShortcutId::NewComposition),
            advanced_tip: "ネストされたコンポは Composition パネルでダブルクリックして編集可能。タイムライン上に親コンポと子コンポが同時に見えます。",
        },
        TutorialStep {
            chapter: TutorialChapterId::Composition,
            title: "4/12 — レイヤーの追加と管理",
            body: &[
                "レイヤーは素材の積み重ね。上のレイヤーが前面に表示されます。",
                "追加できるレイヤーの種類:",
                "  Text — テキスト (Layer > New > Text)",
                "  Solid — 無地の背景色 (Layer > New > Solid)",
                "  Video — 動画ファイルをドロップ",
                "  Audio — WAVファイルをドロップ",
                "  Light — 3D照明ソース (Layer > New > Light)",
                "  Camera — 3Dカメラ (Layer > New > Camera)",
                "タイムライン上でドラッグして順番を入れ替えられます。",
            ],
            hint: "試してみよう: Layer > New > Text で「Kagari」と入力してみよう",
            shortcut: TutorialShortcut::Binding(crate::ui::shortcuts::ShortcutId::TextTool),
            advanced_tip: "レイヤーを複数選択 (Shift+クリック) して Cmd+Shift+C でプリコンポーズ。複雑なアニメをグループ化できます。",
        },
        // ── Chapter 3: Animation & Keyframes ─────────────────────────────────
        TutorialStep {
            chapter: TutorialChapterId::Animation,
            title: "5/12 — キーフレームの基本",
            body: &[
                "アニメーションの仕組み:",
                "  タイムラインで「フレーム 0」に再生ヘッドを移動",
                "  2. インスペクターの「Position」でキーフレーム記録を有効にする",
                "     → キーフレーム記録開始",
                "  3. 再生ヘッドを「フレーム 60」(2秒後)に移動",
                "  4. Viewerでレイヤーをドラッグして位置を変更",
                "  5. Spaceキーでプレビュー再生し、動きを確認",
                "キーフレームの形で補間方式を見分けられます。",
            ],
            hint: "試してみよう: テキストレイヤーの Position に2つKFを打って動かす",
            shortcut: TutorialShortcut::Bindings(&[
                crate::ui::shortcuts::ShortcutId::TogglePlayback,
                crate::ui::shortcuts::ShortcutId::RevealPosition,
            ]),
            advanced_tip: "タイムライン下部の「Graph Editor」ボタンで速度グラフを開くと、動きの緩急を曲線で細かく制御できます。「Easy Ease」はF9で一発適用。",
        },
        TutorialStep {
            chapter: TutorialChapterId::Animation,
            title: "6/12 — イージングとグラフエディター",
            body: &[
                "「イージング」とは動きの緩急のこと。自然な動きの鍵です。",
                "",
                "  • Easy Ease [F9]  — 入りと出しを両方なめらかに",
                "  • Easy Ease In   — 終端だけゆっくり止まる",
                "  • Easy Ease Out  — 最初だけゆっくり出発",
                "  • Linear         — 一定速度の機械的な動き",
                "",
                "グラフエディター (タイムライン > Graph Editor) では",
                "ベジェハンドルをドラッグして完全カスタムの緩急を作れます。",
                "「Animation > The Smoother…」ではキーフレームを簡略化できます。",
            ],
            hint: "試してみよう: キーフレームを選択して F9 (Easy Ease) を適用しプレビュー",
            shortcut: TutorialShortcut::Binding(crate::ui::shortcuts::ShortcutId::EasyEase),
            advanced_tip: "19種類のイーズプリセットが内蔵されています。「Bounce」「Elastic」「Back」などを試してダイナミックな動きを作りましょう。",
        },
        // ── Chapter 4: Effects ────────────────────────────────────────────────
        TutorialStep {
            chapter: TutorialChapterId::Effects,
            title: "7/12 — 実映像にエフェクトを適用",
            body: &[
                "この実習を選ぶと、同梱の短い映像を読み込みます。",
                "読み込み後、映像レイヤーがタイムラインに追加され、Color Tintが適用されます。",
                "映像をViewerで確認できたら、再生しながら色の変化を調整します。",
                "",
                "Space キーで再生・停止し、Inspector で Color Tint の強さを変えます。",
                "再生しながら調整すると、動く映像に対する色の変化を確認できます。",
                "Inspector の有効スイッチで効果を切り替え、適用前後を比べます。",
            ],
            hint: "動く映像にColor Tintを適用して色味を調整",
            shortcut: TutorialShortcut::Binding(crate::ui::shortcuts::ShortcutId::TogglePlayback),
            advanced_tip: "Color Tint の強さにキーフレームを設定すると、映像の途中で色調を変化させられます。",
        },
        TutorialStep {
            chapter: TutorialChapterId::Effects,
            title: "8/12 — エクスプレッション",
            body: &[
                "エクスプレッションはパラメータに「式」を書いてアニメを自動化する機能。",
                "",
                "よく使う式 (インスペクターでプロパティを Alt+クリック):",
                "",
                "  wiggle(3, 20)",
                "  → 毎秒3回、±20px ランダムにゆれる",
                "",
                "  loopOut(\"cycle\")",
                "  → 設定したキーフレームを無限ループ",
                "",
                "  time * 180",
                "  → 秒数×180°で永遠に回転",
                "",
                "  smooth(0.2, 5)",
                "  → ガタつく動きをなめらかに平滑化",
                "",
                "エクスプレッションは Rhai スクリプトで書かれており、",
                "JavaScript ライクな構文で複雑なロジックも記述可能。",
            ],
            hint: "試してみよう: Rotation に「time * 90」を入力して永遠に回転",
            shortcut: TutorialShortcut::Gesture("Alt+クリック"),
            advanced_tip: "エクスプレッション対応プロパティの式欄で、式を編集して結果を確認できます。",
        },
        // ── Chapter 5: 3D & Advanced ──────────────────────────────────────────
        TutorialStep {
            chapter: TutorialChapterId::Advanced,
            title: "9/12 — 3Dレイヤーとカメラ",
            body: &[
                "レイヤーの「3D」スイッチをオンにすると3Dレイヤーになります。",
                "",
                "3D レイヤーでできること:",
                "  • X/Y/Z 軸の Position・Rotation・Scale",
                "  • Layer > New > Camera でカメラを追加",
                "  • Layer > New > Light でライトを追加",
                "  • Inspector の Material Options で設定:",
                "    - Casts Shadows   → 他レイヤーに影を落とす",
                "    - Accepts Shadows → 影を受け取る",
                "    - Depth of Field  → ボケ(被写界深度)の計算",
                "",
                "試してみよう: 3Dスイッチを有効にし、X/Y/Zの位置を変えてViewerで奥行きを確認します。",
            ],
            hint: "試してみよう: Layer > New > Camera でカメラを追加し、3Dレイヤーを配置する",
            shortcut: TutorialShortcut::None,
            advanced_tip: "カメラを null レイヤーにペアレントして、カメラリグ（クレーンショットやドリーズーム）を簡単に作れます。",
        },
        TutorialStep {
            chapter: TutorialChapterId::Advanced,
            title: "10/12 — トラッキングから3Dカメラを作る",
            body: &[
                "このレッスンでは、実映像上の特徴点を追跡し、3Dカメラの動きを推定します。",
                "",
                "  1. 動画を読み込み、タイムラインで動画レイヤーを選択",
                "  2. トラッカーパネルで追跡点を追加し、映像中の目立つ特徴に置く",
                "  3. 作業範囲を決めて「前方を解析」を実行",
                "  4. 別の特徴にも追跡点を置き、複数点の動きを確認",
                "  5. 「3Dカメラを追跡・ソルブ」でカメラ軌道を生成",
                "",
                "追跡が不安定なときは、コントラストの高い模様を選び、作業範囲を短くします。",
                "追跡点が少ない、または同じ平面上に偏ると、カメラ推定に失敗することがあります。",
            ],
            hint: "試してみよう: 動画レイヤーに追跡点を置き、前方解析を実行する",
            shortcut: TutorialShortcut::None,
            advanced_tip: "3Dカメラソルブは、追跡点に複数フレーム分のキーフレームがある場合に軌道を推定します。",
        },
        // ── Chapter 6: Export & Release ───────────────────────────────────────
        TutorialStep {
            chapter: TutorialChapterId::Export,
            title: "11/12 — 書き出し方法",
            body: &[
                "上部メニューの Composition > Add to Render Queue (Cmd+M) で書き出し設定を開きます:",
                "",
                "  MP4 (H.264)    — Web/SNS 向け汎用フォーマット",
                "  ProRes 422     — 編集用高品質マスター",
                "  ProRes 4444    — アルファ付き合成素材",
                "  Lottie JSON    — Web アニメーション (互換)",
                "  GIF            — ループアニメーション",
                "  MLT XML        — Kdenlive / Shotcut との連携",
                "",
                "形式・範囲・出力先を確認してから書き出します。",
            ],
            hint: "試してみよう: Composition > Add to Render Queue を開き、出力先と形式を選んで書き出す",
            shortcut: TutorialShortcut::Binding(crate::ui::shortcuts::ShortcutId::OpenRenderQueue),
            advanced_tip: "CLIツール (kagari コマンド) を使えばスクリプトやCI/CDからヘッドレスレンダリングが可能。例: kagari frame --project p.json --frame 30 --output out.png",
        },
        TutorialStep {
            chapter: TutorialChapterId::Export,
            title: "12/12 — 次のステップ",
            body: &[
                "チュートリアル完了です！おめでとうございます！",
                "",
                "さらに深く学ぶために:",
                "",
                "  Help > Keyboard Shortcuts Reference...",
                "     → 全キーボードショートカット一覧",
                "",
                "  Animation メニュー",
                "     → The Smoother、The Wiggler、キーフレーム補助",
                "",
                "  GitHub: github.com/AI-SLOP-BOX/kagarivfx",
                "     → Issues / Discussions / Pull Requests 大歓迎!",
                "",
                "短いコンポジションを作り、素材を入れて、動きを付けて書き出す流れを試してください。",
            ],
            hint: "Help > Start Guided Tutorial... からいつでも再開できます",
            shortcut: TutorialShortcut::None,
            advanced_tip: "コントリビュート歓迎! cargo test --all-features がゼロ失敗なら PR を送ってください。",
        },
    ]
}
