//! Beginner-friendly Japanese Context Help, Workspace Layout Persistence, and
//! Transactional Undo/Redo Stack (coalescing drag/scrub edits into single steps).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HelpEntry {
    pub topic_id: &'static str,
    pub title_ja: &'static str,
    pub summary_ja: &'static str,
    pub steps_ja: &'static [&'static str],
    pub shortcut_hint: &'static str,
}

pub const JAPANESE_HELP_TOPICS: &[HelpEntry] = &[
    HelpEntry {
        topic_id: "timeline",
        title_ja: "タイムラインの基本操作",
        summary_ja: "レイヤーの並び順・表示タイミング・キーフレームを管理するメイン画面です。",
        steps_ja: &[
            "1. 左上の「+」またはドロップで画像・動画・テキストを追加します。",
            "2. バーの左右端をドラッグして表示開始・終了（IN/OUT）を調整します。",
            "3. プロパティ横のひし形アイコンを押すとキーフレームが打てます。",
        ],
        shortcut_hint: "Space: 再生/停止 | J/K/L: 逆再生/停止/倍速 | Cmd/Ctrl+Z: 元に戻す",
    },
    HelpEntry {
        topic_id: "roto_brush",
        title_ja: "ロトスコープ（被写体切り抜き）",
        summary_ja: "人物やオブジェクトをなぞるだけで背景から切り抜き、次フレームへ自動追従させます。",
        steps_ja: &[
            "1. 切り抜きたい被写体の内側をざっくりブラシでなぞります。",
            "2. はみ出した背景は Alt/Option ドラッグで「背景」として除外します。",
            "3. 髪の毛など境界が硬いときは「エッジ調整 (Refine Edge)」と「エッジ汚染除去」をオンにします。",
        ],
        shortcut_hint: "Alt/Option+ドラッグ: 背景指定 | [ / ]: ブラシサイズ変更",
    },
    HelpEntry {
        topic_id: "pro_keyer",
        title_ja: "キーイング（グリーン/ブルーバック抜き）",
        summary_ja: "背景色をワンクリックで透明化し、被写体の緑かぶり（スピル）も自動で消します。",
        steps_ja: &[
            "1. スポイトで背景の緑または青を選択します。",
            "2. 黒クリップ／白クリップで抜き残しと透けを調整します。",
            "3. 「スピル抑制 (Despill)」を 0.7〜0.9 にすると輪郭の緑かぶりが消えます。",
        ],
        shortcut_hint: "アルファ表示切替で抜き具合を確認しながら調整すると簡単です",
    },
    HelpEntry {
        topic_id: "aces_color",
        title_ja: "ACES カラーマネジメント",
        summary_ja: "Log 撮影素材や 3DCG・HDR 素材の色域を破綻させずに統一処理する映画基準の色管理です。",
        steps_ja: &[
            "1. 作業色空間を ACEScg（または Linear sRGB / Display P3）に設定します。",
            "2. 3-Way カラーホイールで Lift（暗部）/ Gamma（中間）/ Gain（明部）を整えます。",
            "3. 最後に書き出し先（sRGB / Rec.709 / Display P3）に合わせて出力します。",
        ],
        shortcut_hint: "露出オーバーでも ACES なら白飛びせず滑らかにロールオフします",
    },
    HelpEntry {
        topic_id: "node_graph",
        title_ja: "ノードグラフ合成",
        summary_ja: "エフェクトや合成の流れを「箱と線」でつなぎ、複雑な VFX を整理して組めます。",
        steps_ja: &[
            "1. MediaIn から ColorWheel・ProKeyer・SapphireGlow などを線で繋ぎます。",
            "2. 最後は必ず MediaOut ノードへ接続します。",
        ],
        shortcut_hint: "ノードの並びを入れ替えても元の素材データは破壊されません",
    },
    HelpEntry {
        topic_id: "audio_export",
        title_ja: "音声編集・書き出し",
        summary_ja: "BGM・効果音・セリフの音量調整とフェード、WAV/動画へのミックスダウンを行います。",
        steps_ja: &[
            "1. タイムラインに音声トラックを配置し、フェードイン/アウトを設定します。",
            "2. 書き出しパネルから MP4（音声付き）または WAV 書き出しを実行します。",
        ],
        shortcut_hint: "波形を見ながらタイミングを合わせられます",
    },
];

pub fn lookup_japanese_help(topic_id: &str) -> Option<&'static HelpEntry> {
    JAPANESE_HELP_TOPICS.iter().find(|e| e.topic_id == topic_id)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceLayout {
    pub name: String,
    pub show_project_panel: bool,
    pub show_effect_panel: bool,
    pub show_properties_panel: bool,
    pub show_timeline: bool,
    pub show_node_graph: bool,
    pub show_graph_editor: bool,
    pub left_sidebar_ratio: f32,
    pub right_sidebar_ratio: f32,
    pub bottom_panel_ratio: f32,
}

impl WorkspaceLayout {
    pub fn preset_beginner_simple() -> Self {
        Self {
            name: "初心者シンプル".to_string(),
            show_project_panel: true,
            show_effect_panel: false,
            show_properties_panel: true,
            show_timeline: true,
            show_node_graph: false,
            show_graph_editor: false,
            left_sidebar_ratio: 0.18,
            right_sidebar_ratio: 0.24,
            bottom_panel_ratio: 0.30,
        }
    }

    pub fn preset_vfx_compositing() -> Self {
        Self {
            name: "VFX ノード合成".to_string(),
            show_project_panel: true,
            show_effect_panel: true,
            show_properties_panel: true,
            show_timeline: true,
            show_node_graph: true,
            show_graph_editor: false,
            left_sidebar_ratio: 0.16,
            right_sidebar_ratio: 0.22,
            bottom_panel_ratio: 0.38,
        }
    }

    pub fn preset_color_grading() -> Self {
        Self {
            name: "カラーグレーディング".to_string(),
            show_project_panel: false,
            show_effect_panel: true,
            show_properties_panel: true,
            show_timeline: true,
            show_node_graph: false,
            show_graph_editor: false,
            left_sidebar_ratio: 0.18,
            right_sidebar_ratio: 0.28,
            bottom_panel_ratio: 0.26,
        }
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("layout serialize error: {e}"))
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let mut layout: Self =
            serde_json::from_str(json).map_err(|e| format!("layout parse error: {e}"))?;
        layout.left_sidebar_ratio = layout.left_sidebar_ratio.clamp(0.10, 0.40);
        layout.right_sidebar_ratio = layout.right_sidebar_ratio.clamp(0.10, 0.45);
        layout.bottom_panel_ratio = layout.bottom_panel_ratio.clamp(0.15, 0.60);
        Ok(layout)
    }
}

#[derive(Debug, Clone)]
struct UndoEntry<T> {
    label_ja: String,
    before: T,
    after: T,
}

/// Transactional Undo/Redo stack supporting drag coalescing (`begin_group` / `commit_group`).
#[derive(Debug)]
pub struct UndoTransactionStack<T: Clone> {
    undo_stack: Vec<UndoEntry<T>>,
    redo_stack: Vec<UndoEntry<T>>,
    active_group: Option<(String, T)>,
    max_entries: usize,
}

impl<T: Clone> UndoTransactionStack<T> {
    pub fn new(max_entries: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            active_group: None,
            max_entries: max_entries.max(1),
        }
    }

    pub fn push(&mut self, label_ja: impl Into<String>, before: T, after: T) {
        self.redo_stack.clear();
        if self.undo_stack.len() >= self.max_entries {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(UndoEntry {
            label_ja: label_ja.into(),
            before,
            after,
        });
    }

    pub fn begin_group(&mut self, label_ja: impl Into<String>, initial_state: T) {
        if self.active_group.is_none() {
            self.active_group = Some((label_ja.into(), initial_state));
        }
    }

    pub fn commit_group(&mut self, final_state: T) {
        if let Some((label, before)) = self.active_group.take() {
            self.push(label, before, final_state);
        }
    }

    pub fn undo(&mut self) -> Option<(&str, T)> {
        let entry = self.undo_stack.pop()?;
        let state = entry.before.clone();
        self.redo_stack.push(entry);
        let label = self.redo_stack.last()?.label_ja.as_str();
        Some((label, state))
    }

    pub fn redo(&mut self) -> Option<(&str, T)> {
        let entry = self.redo_stack.pop()?;
        let state = entry.after.clone();
        self.undo_stack.push(entry);
        let label = self.undo_stack.last()?.label_ja.as_str();
        Some((label, state))
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_and_workspace_layout_roundtrip() {
        let h = lookup_japanese_help("roto_brush").expect("roto help");
        assert!(h.title_ja.contains("ロトスコープ"));
        assert!(!h.steps_ja.is_empty());

        let layout = WorkspaceLayout::preset_vfx_compositing();
        let json = layout.to_json().expect("serialize");
        let parsed = WorkspaceLayout::from_json(&json).expect("deserialize");
        assert_eq!(layout, parsed);
    }

    #[test]
    fn undo_stack_coalesces_drag_group_and_undoes_cleanly() {
        let mut stack = UndoTransactionStack::new(64);
        stack.begin_group("位置移動", 10);
        // Many intermediate slider drags happen without pushing separate undo entries
        stack.commit_group(42);
        assert!(stack.can_undo());
        let (lbl, restored) = stack.undo().expect("undo");
        assert_eq!(lbl, "位置移動");
        assert_eq!(restored, 10);
        let (_, redone) = stack.redo().expect("redo");
        assert_eq!(redone, 42);
    }
}
