# AmeUI 機能・カラー・CLI リファレンス（Kagari 用クイックガイド）

## 1. `amec` CLI（ヘッドレス検証・スクショ・フォーマット）
Mac/Linux の CLI から `.ame` の構文チェック・レイアウト計測・PNG スナップショット撮影が可能。

```bash
# 構文・バインディング・レイアウト計測 + PNG スナップショット撮影
cargo run --manifest-path ../AmeUI/Cargo.toml --bin amec -- measure src/ui/ame/kagari_studio.ame --snapshot /tmp/kagari_snapshot.png

# フォーマットチェック
cargo run --manifest-path ../AmeUI/Cargo.toml --bin amec -- fmt src/ui/ame/kagari_studio.ame --check
```

- `measure` は以下を自動検証して JSON で返す：
  - `layout_assertions`（はみ出し・重なり）
  - `text_clip_assertions`（テキスト見切れ）
  - アクション名・バインディング名・マテリアル名の解決チェック

---

## 2. マテリアル（`MaterialKind`）
`app` ブロック内で `material <Name> { kind: <kind> ... }` を定義し、ウィジェット末尾に `.material(<Name>)` で適用する。

| `kind` | 用途・効果 | 主なプロパティ |
| :--- | :--- | :--- |
| `liquid_glass` | すりガラス（グラスモーフィズム） | `blur: 24`（整数）, `refraction: 35`（整数） |
| `bloom` | ロゴ・アクセント要素の発光 | - |
| `gradient_border` | グラデーション枠線 | - |
| `mesh_gradient` | メッシュグラデーション背景 | - |
| `neumorphic` | ニューモーフィズム | - |
| `iridescent` | 虹色・玉虫色光沢 | - |
| `liquid_shape` | 有機的リキッド形状 | - |
| `grain` | フィルムグレイン質感 | - |

> **Kagari での使い分け方針**:
> - **編集画面（Editor）**: 色判断を狂わせないため `Card` や派手な透過を避け、ニュートラルダークのフラット面＋1px 境界で統一。
> - **ホーム画面・オーバーレイ**: `liquid_glass` / `bloom` / `gradient_border` / `mesh_gradient` を活用。

---

## 3. Kagari / AmeUI ダークテーマ基準カラー（RGB）
- **ビューポート周囲背景**: `rgb(8, 9, 11)`（`viewport.rs`）
- **タイムライン背景**: `rgb(13, 15, 17)`（`timeline_drawing.rs`）
- **AmeUI ダークベース背景**: `rgb(20, 21, 23)`
- **タイムラインプレイヘッド（DaVinci 風赤）**: `rgb(235, 68, 54)`
- **Kagari アクセントオレンジ**: `#f58c4b` / `rgb(245, 140, 75)`
- **ヘアライン境界・L字コーナーブラケット**: `rgba(255, 255, 255, 24..55)`

---

## 4. AmeUI コア機能モジュール（`../AmeUI/src/` 静的観測）
- **ドッキング・レイアウト**: `docking/`（ドッキングペイン）, `layout.rs`, `lazy_list.rs`（仮想リスト）, `SplitPane`
- **モーション・アニメーション**: `motion/`（スプリング・イージング・トランジション）
- **アプリ基盤**: `command.rs`（コマンドパレット/ディスパッチ）, `undo.rs`, `selection.rs`, `navigation.rs`, `notification.rs`, `clipboard.rs`, `form.rs`
- **描画・アセット**: `canvas.rs`, `svg.rs`, `image.rs`, `icons.rs`, `font.rs`, `theme.rs`
- **テスト・計測**: `testing/`（ヘッドレスハーネス・レイアウト/クリップ検証）

## 5. `.ame` パーサー対応ウィジェット一覧（`../AmeUI/src/lang/parser/widgets/`）
- **レイアウト・構造**: `VStack`, `HStack`, `ZStack`, `Positioned`, `Wrap`, `Grid`, `AdaptiveGrid`, `GridSpan`, `SplitPane` (`Horizontal`/`Vertical`), `MultiSplit`, `Center`, `Card`/`Surface`, `Overlay`, `Padding`, `Indent`, `MaxWidth`, `MinWidth`, `Grow`, `Align`, `Spacer`, `Divider`, `ScrollView`, `HorizontalScrollView`, `SyncScroll`
- **表示・コンテンツ**: `Text`/`Title`/`Caption`, `SelectableText`, `EditableText`, `Markdown`, `Svg`, `Diff`, `EmptyState`, `AdaptiveLabel`, `ProgressBar`, `Timeline`
- **コントロール・入力**: `Button`, `IconButton`, `AttachedButton`, `TextField`, `TextArea`, `FormField`, `CompositeInput`, `Select`/`DropdownButton`, `Slider`, `ValueSlider`, `DragValue`, `UnitPicker`, `ColorPicker`, `ColorSwatches`, `DatePicker`, `ShortcutEditor`, `CurveEditor`, `GradientPicker`, `FontPicker`
- **データ・ナビゲーション**: `ForEach`, `Tree`, `Table`, `VirtualList`, `HorizontalVirtualList`, `Tabs`, `Accordion`, `Pagination`, `Stepper`, `CommandPalette`, `NavigationItem`, `SettingsSection`, `SettingsCard`, `SettingsRow`, `DragSource`, `DropTarget`
- **制御フロー・オーバーレイ**: `If`, `Responsive`/`Adaptive`, `Async`/`AsyncBoundary`, `Modal`, `ManagedOverlay`

## 6. `.ame` モディファイア一覧（`../AmeUI/src/lang/parser/modifiers.rs`）
- **レイアウト・サイズ**: `.padding(...)`, `.frame(width: ..., height: ...)`, `.relative_width(...)`, `.relative_frame(...)`, `.max_width(...)`, `.min_width(...)`, `.grow(...)`, `.align(start|center|end|stretch|baseline)`, `.offset(...)`, `.card(...)`
- **テキスト・配色・マテリアル**: `.font_size(...)`, `.font_family(...)`, `.weight(regular|medium|semibold|bold)`, `.letter_spacing(...)`, `.contrast(high|medium|low)`, `.secondary(...)`, `.accent(...)`, `.opacity(...)`, `.material(...)`
- **インタラクション・フォーカス・メニュー**: `.context_menu() { Item("...", action: ...) }`, `.shortcut(...)`, `.on_hover(enter: ..., exit: ..., exit_delay_ms: ..., group: ...)`, `.focus_scope(trap: ...)`, `.transformable(handles: ...)`, `.pointer_lighting(...)`
- **アニメーション・演出**: `.shared_element(...)`, `.motion_blur`, `.page_curl`, `.chain`, `.collide`, `.flock`, `.height_reveal`, `.minimize`, `.progress_morph`, `.chart_line`（イージング: `linear`, `ease_in`, `ease_out`, `ease_in_out`, `ease_in_out_cubic`, `ease_out_cubic`, `ease_out_back`, `spring`, `spring_physics`）

