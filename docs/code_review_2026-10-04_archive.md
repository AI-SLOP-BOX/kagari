# 完了項目の元台帳スナップショット

冒頭の初回レビュー全文は原文のまま保持しています。後続の見出しは、完了項目を整理する直前の現行台帳スナップショット、移動した進捗履歴、解決確認を追記した記録です。初回スナップショット本文は変更していません。

---

# Kagari コードレビュー — 2026-10-04

## 範囲と確度

現在の作業ツリーを対象にした静的レビュー。未コミット変更と未追跡の新規モジュールを含む。差分だけでなく本文と呼び出し先を確認した。全ソース全文の読了ではない。レビュー後に一部の指摘を修正している。GUI起動・画面確認と性能測定は未実施。先行する変更時点の`cargo test --all-features --lib`は1,400件すべて成功した（性能しきい値テストは先行runで一度だけ6.44秒となったが、その後の単独再実行と全lib再実行で成功）。先行変更後の`cargo check --all-features --tests`は成功。動画import容量推定、LUT state isolation、R69 disk inventoryなどの対象テストも通過済み。今回のR51 revision、R54 disk writer、R102 Collect Files、R158 Motion Sketch worker変更後には`cargo test --no-default-features --lib`が1,240件成功し、`cargo check --features gui --lib`、strict no-default/gui lib Clippy、`git diff --check`も成功した。all-features全testとall-target Clippyは今回の追加変更を含めて未実施。Project I/OのLUT復元テスト、integration testを含む全test binary一括再実行も未実施。

詳細に確認した中心領域は KUI、カラー管理、履歴、ワークスペース、書き出し開始処理、CPUビューアー、フォント設定、タイムラインのヘッダー・子レイヤー・パンくず、CLIのレンダー範囲とモーションキャプチャ入力列挙。大型ファイルは該当関数と呼び出し先を読んだ。検索による出現数は品質の点数ではない。

P1: 出力・編集結果の信頼性に関わる。P2: 操作性、性能、保守性に実害がある。P3: 整理課題。実行による再現が必要なものは明記する。

## 出力・機能の問題

### R01 [P1・修正済み] 読み込んだLUTがRayonの処理スレッドから見えない

`src/core/ocio_color.rs:340` の ACTIVE_LUT は thread_local。カラー管理画面で UI スレッドから設定する一方、`src/core/software_renderer/mod.rs:1200` は `par_chunks_exact_mut` の中で `apply_lut_pixel` を呼ぶ。Rayon worker の TLS は別なので、そのスレッドでは LUT 未設定として入力値を返す。呼び出し元スレッドによる処理参加状況によっても結果が変わり得る。スレッドをまたぐレンダー状態は明示的な共有スナップショットとして渡すべき。非恒等LUTで全画素の一致を検証する必要がある。

### R02 [P1・修正済み] 「現在のグレードを書き出す」が恒等LUTを書いている

`src/ui/color_management.rs:273` 以降のループは RGB グリッドをそのまま出力する。現在のカラー補正も読み込んだ LUT も適用しない。補正を保存したつもりでも結果は無補正になる。補正チェーンの評価を通すか、未実装の操作として明示する必要がある。

### R03 [P2・修正済み] LUT書き出し失敗を成功として通知する

同ファイルの `let _ = std::fs::write(&path, content)` はエラーを捨て、直後にファイル表示と成功通知を行う。出力先も一時ディレクトリへ固定されている。保存先の選択、書き込み結果の確認、成功後の通知が必要。コンポジション名の改行・引用符も TITLE 行へそのまま入る。

### R04 [P2・未接続の選択UIを除去／色変換機能は未実装] カラー管理の選択項目が処理へ接続されていない

色空間・表示シミュレーション・IDT/ODT・補間の未接続選択UIをなくし、未対応の状態表示へ変更した。コンポジション設定に残っていた作業色空間コンボ、実際の出力を変えない8/16/32-bit深度コンボ、状態変数だけのstatus表示も除去し、作業色空間は未適用、現在のレンダーは8-bit出力と明示する。既存projectのbit-depthフィールドは互換性のため保持するが、現行レンダーは利用しない。読み込まれたLUTのみプレビューへ反映する。色変換パイプライン自体は未実装であり、機能追加ではなく誤認表示の除去として解消する。

### R05 [P2・修正済み] LUT参照がプロジェクトへ保存されない

`ProductionDocument`にLUTファイルのパスを保存し、プロジェクトを開いたときとクラッシュ復旧時にバックグラウンドで読み直す。autosaveにもパスを含める。相対パスはプロジェクトファイルの場所を基準に解決し、見つからない場合もプロジェクト自体は開き、再リンクを促す。round-trip/legacyと相対LUT解決のテストを追加したが、この変更後に個別テストを実行できていない。LUTデータ本体は外部ファイルへの参照であり、プロジェクトへの埋め込み・CLIでの共有は未実装。

### R06 [P2・修正済み] 一部のPuppet追加ボタンが何も追加しない

画像レイヤー用ボタンが変更フラグだけを立てていた。現在は実際に Position Pin を作成する共通入口へ置き換えた。Starch/Overlap/Advanced Pinはデータモデルに別種別がなく、誤った機能を装う個別ボタンは削除した。これらの専用pin種別は未実装。

### R07 [P1・修正済み] タイムラインからのレイヤー追加でIDが重複する

以前の実装では `layer_{layers.len()}` を生成し、削除後の追加で既存IDを再利用していた。現在のタイムラインヘッダーは全追加経路で `Composition::next_layer_id` を使う。同関数は既存layer IDを走査して未使用IDを返し、削除済みIDが再利用されても重複しない。コア側に欠番を飛ばす単体テストもある。

### R08 [P2・修正済み] ワークスペースの名前と表示パネルが一致しない

`src/ui/workspace_manager.rs`のプリセット割当を修正し、Motion Tracking→Tracker、Paint & Roto→Paint、Text & Type→Character、Animation→SpeedGraph、Effects→Effects、Essential Graphics→EssentialGraphicsへ対応させた。

### R09 [P1・修正済み] SSAA上限処理とダウンサンプルの入力寸法が矛盾する

以前は2倍寸法を各軸で個別に上限クランプしていた一方、`downsample2x` は入力が出力のちょうど2倍であることを仮定していた。現在は出力のどちらかの軸が上限を超える場合、SSAA全体を無効にして等倍寸法で描画し、ダウンサンプルもしない。大寸法での実行検証は未実施。

## Rust・性能・状態管理

### R10 [P2・修正済み] Async Renderでも音声準備はUIスレッドを塞ぐ

音声ミックスをFFmpeg export worker側へ移し、UIには準備状態を表示して完了・失敗eventを返すよう変更した。実際に長尺音声でUI応答性を測ってはいない。audio mix APIのキャンセル・細粒度進捗は未対応。

### R11 [P2・修正済み] CPUビューアーが再描画のたびに合成・テクスチャ生成する

CPU viewerに単一frameのTextureHandle cacheを追加。cache keyはcomposition、history/global render version、frame、解像度、露出、LUT mode。ポインタドラッグ中はcacheを使わず、編集反映の遅れを避ける。負荷測定は未実施。

### R12 [P2・修正済み] CPUビューアーで露出・LUTモードがGPU表示と揃わない

CPUキャンバスも `ae_exposure_ev` と選択中の `ae_colorspace_lut` mode全体を読み、GPU経路と同じ値をrendererへ渡すよう変更。

### R13 [P2・修正済み／実測未実施] パネルを眺めるだけでプロジェクト全体をcloneする

`color_management.rs:6`、`paragraph_panel.rs:7`、`camera_light_options.rs:52` は描画冒頭で履歴の現在値を複製する。変更のないフレームでもコストが発生する。Paragraphは選択対象がない場合でも先にcloneする。編集開始時にスナップショットを取る共通トランザクションへ寄せるべき。

color management、paragraph、camera light、Effects Controls、Character tab、Camera DoF、Layer Styles、Mask、Viewer inline numeric editorは共通helper経由で、実際の対象UIに入力がある場合だけundo用Project snapshotを捕捉する。inline editorはcanvasではなくfloating editor rectを対象にする。Character tabの毎frame Project cloneはactive Compositionだけのworking copyへ縮小。Undo履歴は過去状態をserialized snapshotで保持し、mutable history accessのたびにentry推定サイズを再計算する経路も除去した。残る`history.current().clone()`を全UIから再検索し、pointer/keyboard action、明示的な保存・export・import/replace・編集確定・async結果反映・履歴snapshot処理に限られることを確認した。観覧だけで発生する全Project cloneはコード上解消。大規模Projectの応答時間とピークRAMの実測は未実施。

### R14 [P2・部分解消] 履歴コミットの差分確認でプロジェクトを2回JSON化する

`commit_action`は変更判定のためcurrentとnextを全走査する。現在はcurrent JSONを一度だけbyte buffer化し、nextはstreaming writerで比較しながらserialized lengthを数えるため、従来の2個目の全project JSON bufferはなくした。二重のserialization traversal自体は残る。編集APIのchange flagまたは全モデルの構造比較へ移す場合は、no-op抑制と直書き変更を含めた影響確認が必要。性能測定なしに高速化量は断定しない。

drag cancelによるcurrent entry置換、commit後のredo切捨て、件数/byte budget evictionもentryごとのestimateと総量を同期更新する。R14は部分解消。

### R15 [P2・保持スナップショット容量へ上限を適用／RSSとundo一時ピークは別]

履歴entryがProjectを毎回deep cloneしていた構造を、現在のlive Project 1つとserialized snapshot列へ変更した。過去状態は128MiB上限付きwriterでJSON bytesへ保存し、可変長データ・全composition/subcompositionを含める。snapshotとaction文字列はshrink_to_fit後のcapacityを数え、VecDequeのentry領域も加算してbudget超過時は古い状態を破棄する。現在の編集Projectは履歴ではないためbudget外とし、画面の表示も「Undo history storage」に改めた。

この上限はhistoryが所有する保持バッファ容量に対するもの。OS allocator metadata、アプリ全体のRSS、Project編集時に他所で作られるclone、Undo/Redo時の一時Project decodeとbounded serialization scratchまでは上限化していない。上限を超える大きさの旧状態へ遷移した場合は、その時点でUndo履歴を新しい境界へリセットする。実RAM計測と最大ProjectでのUndo/Redo peak計測は未実施。

### R16 [P3] 保守の単位とファイルの大きさが合っていない

今回の読取時点で Home mod は10142行、viewportは4523行、graph_editorは4154行、timeline modは1979行。テストも含むため全行を製品処理として数えてはいけない。それでもHomeには多数のページ・旧target・reference・レスポンシブ実装が同居する。画面単位、操作単位、表示モデル単位で分ける余地が大きい。単に100行ずつ切る分割では責務は改善しない。

### R17 [P3] 機能名より追加順を表すファイル名が多い

`ae_effects_pack_v2`〜`v30` 等は処理カテゴリを名前から判断できない。実装探索と責務の重複確認が難しくなる。互換APIを維持しながらblur/distort/generate等のカテゴリと、共有処理に整理する余地がある。現状のAGENTSもv*.rs追加を許容しており、方針変更として扱う必要がある。

### R18 [P3・crate-wide lint抑制を除去／strict all-target Clippy通過] Clippy成功と設計品質は別

当初は15種類のClippy lintをクレート全体で許可し、画素処理などを含む100以上のソースファイルで新規警告も隠れていた。現在は`src/lib.rs`から全てのcrate-wide Clippy allowを除去した。`chunks_exact_to_as_chunks`の指摘はlib 61箇所、test target 39箇所を固定長配列chunk APIへ移行した。pixel byte sliceでは各chunkのサイズと端数を従来どおり維持し、4箇所のmutable zipは可変borrowを保持するよう手で調整した。

現在のRust toolchainによるGUI lib Clippyで見つかったimport cancel token抽出の不要な`unwrap()`も`filter_map`へ変更した。`type_complexity`全体抑制を外し、physics bake結果とtimeline row callbackに名前付き型を導入した。strict all-target Clippyでtest codeの`field_reassign_with_default` 34件をstruct initializerへ移し、range判定等のlintも直した。15引数のnested precomp rasterizer二つは共有の`RasterizationRegion`へ幾何パラメータをまとめた。Content-Aware Fill bake worker、Roto Brush segment job、骨格階層出力、inline text editor geometryも専用型へ整理した。Wigglerには設定型ベースのAPIを追加し、UI側を移行。CPU effect pipelineに共通context、paint rasterizerに`BrushStyle`を導入した。`too_many_arguments`全体抑制を除去し、残る13関数に局所例外を限定した。`single_match`、`collapsible_match`、`field_reassign_with_default`、`obfuscated_if_else`、`unnecessary_cast`、`manual_range_contains`、`collapsible_if`、`manual_let_else`、`needless_range_loop`等も全体抑制を外して修正した。strict all-features all-target Clippyは2026-10-04に成功。外部依存`block v0.1.6`のfuture-incompatibility noteは残る。レビュー項目自体は削除せず、現在の証拠へ更新した。

## 素のegui・情報設計・レイアウト

### R19 [P2・共通controls移行済み／GUI visual QAは未実施] KUIの採用がEffect ControlsとInspectorで止まっている

初回レビュー時のraw部品出現検索では type_specs=117、distort=70、transforms=47、stylize=33、generate=32、layer_styles=31。これは `heading/group/checkbox/small_button/Slider/ComboBox/DragValue` の該当行数で、品質スコアではない。Transform inspectorからは直接の標準controlをなくし、Label Colorも共通swatchへ移した。Corner PinのXYは幅を共有しないDragValue横並びだったが、後にKUI vectorへ移行した。Material Optionsはlabel+Slider横並びだった。TransformのAnchor/Position/Scale、3D Position/Rotation/Scale、Auto-Orient/Pin選択、scalar transform、3D material、geometry、drop shadow/stroke controlsをKUIのresponsive vector・PropertyGrid部品へ移行した。範囲なし数値ベクトルとstep指定sliderもKUIに追加した。DistortのMeshWarp/CornerPin/RGBSplit座標、Expression ControlsのPoint/Point 3Dもresponsive vectorへ移行。Sharpen/Edgeの7項目、Stylizeの数値・色入力、Generateの32個のslider、Transition/Halftone/Keying/Blur/Color Correction/Audio/Path Shape/Expression Controls/DistortのsliderをKUIへ移し、Expression Controlsの数値・angle・toggle・dropdown、Blurの整数操作とAudioのEchoes整数操作も共通部品に統一した。Distort/Expression Controlsの座標入力、Color Correction/Generate/Keyingの色入力、Channel Combiner/Set Matte/Echo Operator/Color Space Convertの選択部品もKUIへ移行した。KUIにRGB専用色pickerを追加し、alpha無しの色fieldで従来どおりalphaを編集対象にしない。現在、`src/ui/effects_controls/`内に直接使うraw `egui::Slider`、`ComboBox`、`DragValue`、`Checkbox`、`Button`、`color_edit_button_*`は0件。Transform inspectorは直接の`egui::Slider`/`ComboBox`/`DragValue`/`Checkbox`/`Button`をなくし、色ラベルも共通swatchへ移した。Layer type specsではSolid/Video/Text/Shapeの基本color・property、Text AnimatorとAnimator Stack、Trim Paths、Vector Path Operatorsを共通化し、表示中の変更がundo stateへ反映するようchanged flagsも設定した。Layer Styles、Shape Repeater/Zig Zag/Round Corners、Puppet mesh・pin位置、Trim Paths、Particle Emitterの基本・Force・Trail・Depth項目もPropertyGrid/vector/toggle/buttonへ移した。Inspector内の直接controlは`egui::TextEdit::multiline`によるText layer editorだけで、複数行編集に適した専用KUI editorを作るまでは標準部品を維持する。Camera Inspectorをproperty grid化し、Expression selector用の選択中表示つきcomboと、描画内容を持てるKUI combo popupを追加した。Inspectorおよび`effects_controls`の直接`Slider`/`ComboBox`/`DragValue`/`Checkbox`/`Button`使用は移行済み。画面実測による視覚QAは未実施。

### R20 [P2・共通section/field/menu/row APIを追加し一部画面へ適用／全widget QAは残る] KUIも「eguiの色替え」に留まる部分がある

`kui::section`を固定高のsection headerへ変更し、アクセントマーカー・見出し文字・境界線を共通geometryで描くようにした。KUIにcustom menu item/menu button、可変内容のcombo popup、選択状態を保つ全幅selectable row、eguiの永続開閉状態を使う`collapsing_section`を加え、hover色・余白・文言解決を一元化した。CC LibrariesのColor Swatches/Character Styles/Project Assetsへ共通折りたたみ見出しを適用。PropertyGridは実測ラベル幅と狭幅時の縦stackを提供する。Color Management、Audio Mixer、Automation Panel、Character、Camera Views、CC Librariesを再確認し、直接の`Slider`/`ComboBox`/`DragValue`/`Checkbox`/`Button`/selectable controlsは0件になった。Audio Mixerのpan sliderはKUI field、音声帯選択は共通selected tool buttonへ移し、fader・meterは専用描画を維持。Character panelのAnimator Stackに残っていた位置・回転・不透明度・blur・tracking・skew等も共有PropertyGrid/vector/slider/numberへ移行した。Workspace Managerのbuilt-in presetは全幅selectable rowと共通翻訳へ移行し、section headerとproperty rowの既存共通部品も共通翻訳を使う。標準Checkbox/Button/ComboBoxを下層interactionに使う部分は適切だが、header右側actions、enum型ごとの表示規約、全screenの部品移行と視覚QAは残る。R20は部分対応。

### R21 [P2・修正済み] ParagraphがCharacterと別世代のフォームになっている

ParagraphはKUIのPropertyGridと共通`tool_button`によるAlignment selectorを使用し、幅に応じて4列/2列へ切り替える。Character panelとのフォーム構造差は縮小した。英語メッセージ等のローカライズ未統一はR22に残る。

### R22 [P2・部分実装] OS連動ローカライズはGUI全域にはまだ届いていない

共通の`LanguagePreference`（System/English/Japanese）、`sys-locale`によるOS/アプリ優先locale解決、英語フォールバック、設定保存、共通翻訳関数を追加した。KUIの一般ラベル・ボタン・トグル・選択肢、Home navigation/CTA、toolbar tooltip、Preferencesの言語選択と主要設定項目、新規コンポジションダイアログのラベル・プリセット・作成結果通知へ接続した。言語はウィンドウ幅やworkspace stateから独立している。

ただし翻訳カタログと呼出し側の移行は部分的で、全workspace、menu、dialog、error/toast、effect parameterの英語直書きを網羅していない。Audio Mixerの表示は英語のsource keyを使うよう統一し、見出し、操作、tooltip、チャンネル状態、voice correctionのproperty labelsとkeyframe生成操作を共通翻訳関数へ接続した。画面内に残っていた日本語ハードコード表示は取り除いた。Asset Libraryもsection/title/search hint、preset名とtooltip、asset type・未計測状態を共通翻訳へ接続し、KUIのtext field hintも翻訳する。言語別のpreset検索も表示名に対して動作する。OSごとのlocale取得・実フォント・日本語/英語の全画面レイアウトを実機では確認していない。R22は未完了の指摘として保持する。

### R23 [P2・未完了／Linuxの旧記述を訂正] 日本語フォントの確保がOSごとに不均等

現行`configure_fonts`はLatin用フォント候補と日本語fallback候補をOSごとに別配列で探索し、macOSのHiragino、LinuxのNoto CJK、WindowsのYu Gothic/MS Gothicがあれば比例フォントfamilyのfallbackへ追加する。したがって「LinuxのDejaVuで先にbreakしNoto CJKを見ない」という以前の記述は誤りで、別ループで日本語候補を探索している。誤記は取り消さず監査履歴として明示した。

フォントはOSにインストール済みであることを前提とする。追加済み候補が存在しない環境では日本語字形を保証できず、各OS実機で日本語glyph fallbackとUIを確認していないためR23は未完了。フォント同梱はライセンス・配布サイズの判断が必要なので、この変更では行わない。

LinuxではLatin primary font選択からNoto CJKを分離して後段fallbackへ登録し、WindowsはYu Gothic/MS Gothicのfallback候補を追加した。OS別パスが存在しない場合の完全な日本語保証と各OS実機確認は未実施。

### R24 [P2・列幅共有を追加／全描画rect統合は残る] タイムライン共通geometryはまだ高さ定数3個だけ

`timeline/layout.rs`に`TimelineColumns`を導入し、利用可能幅・egui列間隔・要求された左列幅から左列と時間軸幅を一度に計算する。通常property row、PreComp child row、およびtimeline headerのリサイズ制約で共用。今回、ヘッダールーラーと通常レイヤーの時間軸バーも、同じ入力値から作った`TimelineColumns.time_width`を使うようにした。狭幅では左列の固定220px制約を縮め、右の時間軸を最低35%確保する。列幅算定の単体テストを追加済み。left/name/switches/timeの描画矩形自体を一つのlayout objectから作る統合とGUI実測は未実施。

### R25 [P2] PreCompは描画を絞っても全子レイヤーを毎回複製する

`precomp_children::collect` は展開されたPreCompの全子を名前までcloneする。draw側には可視rectチェックがあり頂点増加は抑えているが、収集とループは全行に比例する。描画virtualizationとデータ取得virtualizationは別。active comp/session/history generation/expanded setをcacheし、generationが変わった場合も表示に関係する子メタデータfingerprintが同じなら既存Arc snapshotを再利用するよう変更。子の可視情報が変わる編集時は全件走査・collectを行い、可視行だけのデータ取得には未対応。

### R26 [P2・主要state tokenを整理／全使用箇所の移行は残る] テーマの色の意味が実使用と矛盾する

themeにselection/workspace/action/warningのsemantic tokenを導入し、selection stroke、widget active border、KUI primary action、workspace preset selected state、Timeline selected tab/keyframeへ割り当てた。選択背景もブランド色系の低彩度色へ揃え、warning表示は`ACCENT_WARNING`経由にした。`ACCENT_TOGGLE`/`ACCENT_PROGRESS`/`ACCENT_BEAT_MARKER`/`ACCENT_INFORMATION`/`BORDER_FOCUS`を追加し、手描きtoggle、進捗bar、beat marker、通知・診断の情報色、active widget borderへ割り当てた。Timeline layer selection rail、toolbarのactive tool、Inspectorのkeyframe-active、Homeのselected asset、Preferences categoryも`ACCENT_SELECTION`/`BG_SELECTION`系に統一。Render操作は`ACCENT_PRIMARY_ACTION`を使い、audio peak meterは用途別の`METER_WARNING`を使う。Timeline playheadは専用の`TIMELINE_PLAYHEAD`、motion/guidesはcyanを使う。旧色定数も後方互換のため残す。全画面で色名直参照をなくす作業と、色の視覚コントラスト実機確認は未完了なので部分対応。

### R27 [P3・旧Settings画面と分岐を除去] Homeの複数実装と見本コードの境界が曖昧

HomeのSettings導線は共通Preferencesへ移っており、旧`settings.rs`/`legacy-home-settings` featureには呼び出し元がなかったため削除した。Settings clickはPreferencesを開く一つのrouteに統一し、旧reference Settings pageと狭幅mockも削除した。Home module全体の`dead_code`許可は除去し、strict all-target Clippyで警告がないことを確認した。Review itemは削除せず、現行実装に合わせて記録を更新する。

### R28 [P3・共通Painter部品とHomeの主要ボタンを改善／独自interaction全体と支援技術検証は残る] 手描きclick領域には操作の意味を付与する規約が不足

`custom_widgets`のSVG button/toggle・text toggle・switch・color swatchに、透明な標準egui Buttonを操作面として使う共通経路を入れた。描画矩形は同じbutton rectを使い、tooltip/visible labelをwidget metadataへ渡すため、標準Buttonのfocus、Enter/Space activation、enabled stateを維持する。Homeのaction/CTA/select/icon/documentation card、Timeline headerのtransport/view tabsも同経路へ移行した。Timelineのtimecode/keyframeやHomeのその他の`interact`領域、全操作のdisabled contractは未統一。VoiceOver等の支援技術による実検証も未実施。

## 再指摘しない改善済みの点

- RightPanelはenum化済み。保存JSONの旧index互換は残るが、旧right_tab_idx問題として報告しない。
- Expressionのcompileにはソース変更時のcacheがある。毎frame compileの指摘は現在のコードに当たらない。
- PreComp描画には可視範囲チェックがある。全行をPainterで描く実装とは言わない。
- Timeline本体はresizable(true)。固定の断定はしない。ただし最小160/176pxがあり、実際にどこまで下げられるかは実機確認が必要。
- CPU専用CLIとcli-gpuのfeature分離がある。wgpu常時必須という旧指摘は現在当たらない。
- KUIはフォント実測のPropertyGrid、狭幅stack、vector、数値・単位、disabledボタンを備える。資産はある。

## 修正に進む場合の優先順位案

1. R01/R02/R07/R09：結果を誤らせる処理。
2. R04/R06/R08：表示される操作と実機能の一致。
3. R10〜R15：クリック応答とメモリ。代表プロジェクトで計測を追加。
4. R19/R21/R24：Inspector・Effect Controls・Timelineの共通構造。
5. R22/R23：言語・フォントの一本化。
6. Home責務分割、意味ベースのテーマ、独自部品の操作契約。

これはレビューに基づく提案であり、修正の開始や指示範囲の追加を意味しない。

## 第2回：レンダー、メディア、復旧、CLI、グラフへ拡張

追加でimage/precomp/frame/disk cache、video_import、audio_engine/playback/spectrum、production_document、project_migration、project_io、recovery、drop_import、FFmpegのworker、CLI mocap/render、独立preview、VFX graph compiler、flowchart、custom shaderを確認した。大きいファイルは関連関数と参照先を対象にしており、全ファイル全文の読了ではない。引き続き静的レビューで、再現テスト・GUI・性能測定は実施していない。

### R29 [P1・修正済み] ワークエリアのPNG書き出しで開始フレームを二重に加算

以前はPNG workerがwork-area offset込みの絶対frameをcallbackへ渡す一方、callbackでもoffsetを加算していた。現在のPNG callbackは受け取った絶対frameをそのままautomation snapshotとrendererへ渡す。動画workerの相対frame契約は別callback内で一度だけ絶対化する。非ゼロwork-areaでPNGと動画の画素を比較する実行テストは未実施。

### R30 [P1・修正済み] GUI書き出しがドキュメントのautomation bindingsを評価しない

現在の `start_comp_export` は開始時のProductionDocumentをworkerへcloneし、PNG/動画callbackごとに `composition_for_frame(comp_index, absolute_frame)` でbinding snapshotを作ってから描画する。documentがない従来形式だけ、historyから取ったCompositionへfallbackする。exportとpreview/CLIで同じsource値まで共有する共通関数化は残る。

### R31 [P1・修正済み] 音声プレビューのcache keyにレイヤーの編集内容がない

cache keyに `ProjectHistory::generation()` が入り、undoableなProject編集による古いmixの再利用を避ける。project open時にはcached WAV path/keyを無効化する。外部音声ファイルを同じpathで直接上書きした場合にProject history generationが変わらない問題は、音声decode cache側のmtime/size revision（R93）で扱う。playback WAV自体の実測再生テストは未実施。

### R32 [P1・修正済み] ワークエリア音声へ絶対時刻でseekする

再生seekは `current_frame - work_area_start` をfpsで割ったWAV内の相対秒を渡す。mix関数のendはexclusiveのままにし、work-area Outがinclusiveなら呼出側で1 frameを加える。実音声デバイスでのseek/end境界確認は未実施。

### R33 [P2・修正済み／非整数fps実音声同期未測定] 音声書き出しのサンプル数が毎フレーム丸められる

絶対frame境界からfloor(sample_rate * frame / fps)を算出し、フレームごとのブロック長を可変にした。既存のframe/fpsによるソース時刻対応を維持しつつ、出力サンプル数の累積丸め誤差をなくした。非整数fpsの実音声同期は未測定。

### R34 [P2・修正済み] 音声ミックスは全尺のf32 PCMを保持してから保存

全尺のf32 PCM保持をやめ、最大で1映像フレームのミックスbufferと変換用bufferだけを持ってBufWriterへ逐次出力する。RIFF長は先に検証する。

### R35 [P2・修正済み] 音声ミックスの失敗と「音声なし」を区別し、古いmix WAVを残さない

`mix_composition_to_wav`は書込み・source読込失敗を`Err`で返し、frame range内に有効な音源がない場合は一時WAVを削除して`Ok(None)`を返す。`AudioFrameMeter.loaded_sources`でPreComp内を含むsource decode成功を数える。再生側は「音源なし」の結果をwork-area/project/mixer keyへcacheして毎フレーム再生成せず、mix条件変更時は前のtemp WAVを削除して古い音声へfall backしない。exportは音声要求時に空mixをエラーとして示す。非整数fps実音声と外部ファイル更新時のcache動作は未測定。

### R36 [P1・修正済み] 非同期動画importの挿入先が開始時に固定されない

import jobへproject session・開始時のComposition ID・selection/layer targetを保持し、完了時に一致を確認してから挿入する。プロジェクト切替後に返ったcacheも破棄する。遅い素材を使うGUI実動確認は未実施。

### R37 [P2・修正済み／大規模Project応答時間未計測] プロジェクトを開く処理に同期の動画全デコードがある

read/parse/relink/cache refreshをbackground workerで実行し、開始session/history generationが変化していれば結果を適用しない。読込中も既存Projectを維持し、完了・失敗をUIへ返す。大規模Projectでの応答時間は未計測。

### R38 [P2・修正済み／実素材での失敗経路未確認] 動画import失敗がpartial cacheを残す

`video_import::import_video` はimport_dirを作成した後、FFmpeg失敗等でErrを返してもディレクトリを掃除しない。WebP途中失敗→PNG fallbackではpartial WebPを残す。frame_path_in_dirはWebP優先なので混在したキャッシュを読む。失敗理由がencoder不在か容量不足かを区別せず、常にもう一度全尺抽出する点も問題。成功時だけcacheを公開するstagingが必要。

現在は出力directoryをRAII cleanup guardで管理し、frame decode・enumerationなど途中で失敗した場合はcacheを削除する。WebP抽出失敗後はpartial WebPを削除してからPNG fallbackを行う。VideoAssetを返す成功経路のみ保持。FFmpeg/audioの失敗時と複数import同時実行の実測は未確認。

### R39 [P2・容量ガードとキャンセルを実装／実素材テスト未実施] 動画importにディスク量の見積りとキャンセルがない

最大24時間・200万フレームまで受け入れる一方、出力bytesの予算や空き容量チェック、ユーザーcancel、子プロセスの停止制御がなかった。現在は解像度・フレーム数から映像容量を推定し（RGBA画素数の半分を圧縮出力の概算として使用）、音声PCM分を加算する。概算サイズに加え最低512MiBまたは概算の15%を残せる場合だけ開始し、処理中も500ms間隔で空き容量を確認、余裕を割ればFFmpegを停止してstaging cacheを破棄する。動画importと動画素材置換にキャンセルボタンを追加し、FFmpeg子プロセスをkill/waitする。画面には割合を捏造せずspinnerを表示する。圧縮率は素材依存の概算であり、処理中の空き容量監視で追加保護する。推定関数の単体テストは成功。異なるOS・実素材・キャンセル時のcleanup・空き容量逼迫時の停止動作は未実測。

### R40 [P2・修正済み／FFmpeg実抽出未確認] 奇数寸法の動画でasset metadataと抽出画像の寸法が違う

importは `scale=trunc(iw/2)*2:trunc(ih/2)*2` で偶数へ縮小するが、VideoAsset width/heightにはprobeした元寸法を返す。drop側のfit計算はそのmetadataを使う。元画像を維持するか、実抽出寸法を戻す必要がある。小さい奇数画像では0寸法へ切り下げる問題もある。

VideoAssetは抽出後の偶数寸法を報告し、2px未満の入力を事前拒否する。FFmpegでの実抽出結果との一致は実行確認していない。

### R41 [P2・修正済み／実素材一括読込未確認] 画像シーケンスは最初の寸法だけ確認する

`import_image_sequence` は先頭画像にだけ寸法上限を適用し、後続は直接image::openして保存する。後続の寸法不一致や過大画像を検証しない。VideoAssetは全フレーム同じ寸法として記録する。またbmp/tgaを許可しているが、Cargo.tomlのimage decoder featureはjpeg/png/webpのみ。この2形式は対応表示と実decodeが一致しない。

各フレームをdecodeする前に寸法を確認し、先頭画像と一致しない素材・上限超過を拒否する。decoder featureのないbmp/tgaは受付候補から除外。部分出力はR38と共通のRAII guardで削除する。

### R42 [P1・修正済み] 復旧ProjectとProductionDocumentを別slotから選び得る

`recovery_dialog.rs:42` はload_latest_recovery、次にload_latest_productionを呼ぶ。前者は新しいproject-only snapshotも受け入れ、後者はproduction入りのsnapshotだけを探すため、新しいProjectと古いaudio/tempo/bindingsを組み合わせ得る。save_project_to_pathはproduction保存後にproject-only save_nowを呼ぶので、実際に混在slotを作る。復旧単位を一個のvalidated snapshotとして返すべき。

現在save_project_to_pathはProductionDocumentとProjectを同じsnapshotでsave_now_productionし、復旧dialogはload_latest_snapshotから同一ファイルのtupleを一度に取得する。復旧経路で別slot同士を結合しない。

### R43 [P1・修正済み] 復旧失敗でも全snapshotを削除する

復元ボタンはload_latest_recoveryがNoneでもダイアログを閉じ、clear_recoveryを実行する。ファイルアクセスの一時的失敗でも復旧素材を失う。Project読込成功時も、通常openと異なりaudio.correction/channels/master_gainをappへ適用しない。次のautosaveでapp側の初期値をdocumentへ上書きする経路がある。復旧成功の確認とドメイン状態復元を共有すべき。

現在復元失敗時はtoastを表示しsnapshotを保持する。成功時はaudio correction/channels/master gainを復元してから同一namespaceを削除する。「破棄して新規開始」はユーザーの明示操作としてsnapshotを削除する。

### R44 [P1・修正済み／OS別生存判定は未実行] 複数アプリが同じ復旧slotを上書き・削除する

各KagariAppの保存stemとdirty-exit markerをprocess IDで分離する。復旧候補は全namespaceから新しい順に検索し、復元・破棄は選んだsnapshotのnamespaceだけを削除する。起動時はOSのprocess livenessを確認し、稼働中の別instance markerをcrash扱いせず、停止済みprocess markerだけを回収する。複数instanceの同時起動・片方正常終了・片方強制終了という実機シナリオは未実施。

### R45 [P2・修正済み] autosave失敗がUIへ伝わらず再描画ごとに再試行する

tick/tick_productionは以前write失敗をNoneへ変え、dirtyとlast_saveをそのままにして毎update再試行していた。現在は失敗を記録して30秒backoffし、UI toastとlogへ表示する。さらに定期保存を`tick_async`へ移し、JSON streamingとbuffered file writeをworkerで行う。ProductionDocument内の重複Project payloadもautosave形式から外し、旧形式の読込互換を保つ。snapshot用Project clone自体はUI threadに残り、応答性の実測も未実施。終了時はworkerを待ち、保存中に追加編集があれば最新状態を最後に同期保存する。

### R46 [P2・修正済み／表示用downscale未実装] 予算を超える有効な静止画がキャッシュ内で自分を追い出す

ImageCacheはdecode→insert→evict→同じkeyをgetする。既定cache予算を超える画像はdecode前にサイズを判定して明示的に拒否し、挿入直後に自身を追い出して無言で消える状態を防ぐ。大画像の表示用downscale経路は未実装。

### R47 [P2・画像トラックマットも修正済み] CPUラスタライズ中ずっと共有image cacheをlockする

`software_renderer/raster.rs`、precomp texture rasterizer、画像トラックマットは、画像データを取り出した直後にcache lockを解放してからpixel loopを実行する。image cacheからのbuffer cloneと速度影響は未計測。

### R48 [P2・byte budgetとLRUを実装／adapter別VRAM照会は未対応] GPU動画cacheはbyte budgetではなく200枚上限

video/still/precompで共用するGPU texture cacheを512MiBのRgba16Float byte budgetに変更し、アクセスstampによるLRU evictionを実装した。単体でbudgetを超えるtextureは表示へ使うがcacheへは保持しない。adapter別VRAM照会は未対応。

### R49 [P2・修正済み／実GPU確認未実施] GPUで同じパスの画像更新を検知しない

GPU video/still texture cache keyへsource file path・length・modified timestamp由来のrevisionを追加。外部更新後はversion bumpなしでも新entryとなり、古いtextureを使い続けない。実GPUでの置換再描画確認は未実施。

### R50 [P2・修正済み／実フレーム確認未実施] PreComp cache revisionに外部mediaの更新が入らない

PreComp cache keyへ再帰的に辿れる画像・動画フレームのfile revisionを加え、同じProject JSONでもnested source更新でcache missになるよう変更。深いnested/speed変更を含む実フレーム確認は未実施。

### R51 [P2・修正済み／実負荷計測なし] PreCompの512MB budgetはスレッドごと、hitも全画素clone

キャッシュをthread-localからプロセス共通のMutex保護cacheへ変更し、512MiBのcache予算をworker間で共有する。Mutexはlookup/insert中だけ保持し、render中は保持しない。画素は`Arc<[u8]>`で共有し、cache hit時の全画素cloneをなくした。通常の合成側がeffectsを適用する場合だけ、変更可能なbufferへcloneする。各lookupでComposition全体をJSON化していたrevision計算は除去し、history/render世代とpreview状態を基準にしつつ、描画対象の外部media metadataを再帰確認する方式へ変更。動画proxyの更新も判定へ含める。revisionが編集世代に依存するため、通常のproject編集経路は既存契約どおり変更時にframe cache versionを進める必要がある。cache eviction後も呼び出し側がArcを保持する間の一時的な追加メモリと、実レンダリング負荷での計測は残る。

### R52 [P1・修正済み] ディスクフレームcacheのkeyにproject/sessionがない

`frame_cache::disk_cache::frame_path` はframeとglobal versionだけで、保存場所は共通temp_dir/kagari_frame_cache。従来は複数instanceや再起動で同じkeyが別作品を表し得た。現在はFrameCacheインスタンス固有namespaceをdisk pathに加え、他セッションのframeを誤読しないようにした。

### R53 [P2・修正済み] ディスクcache読込はbudget確認前に全ファイルを確保する

FrameCacheはmetadataと固定8byte headerから寸法・cache上限を検証してからpixel bufferだけを確保する。PersistentDiskCacheもmetadataで上限を確認してから上限付きreadを行い、metadata JSONは1MiBまでに制限した。frame writeはheaderとpixel bufferを一時Vecへ複製せずBufferedWriterで順次出力。

### R54 [P2・修正済み／非協調writerは対象外] フレームのディスク予算は単体超過を拒否しない

frame_cache::disk_cache::write_frameは単一frameがbudgetを超えた場合に拒否し、evict後も実disk usageを再確認する。置換先の既存fileをeviction候補から除外し、予測容量から旧file分を差し引く。process内はmutex、同じcache directoryを使うKagariの複数process間はOS file lockで予算確認からatomic writeまで直列化する。別processが同じdirectoryへ非協調の直接書込みを行う場合は調停対象外。別processが同じbudget lockを待つ回帰testを含むdisk-cache test 4件とstrict no-default Clippyが成功。

### R55 [P2・修正済み] グラフのexecution orderがHashSet順に依存する

Kahn queueの初期値とcycle fallbackをlayer index順にし、依存edgeもソートしてから追加。独立ノードの可視scheduleを決定的にする。FlowGraphModelの表示用scheduleであり、実レンダーの実行順としては扱わない。

### R56 [P2・修正済み] graph compilerは非active祖先の依存を再帰展開しない

activeな子の親だけでなく親の祖先まで依存閉包へ加える。solo/out-of-rangeの親もtransform依存としてscheduleに含める。

### R57 [P2・修正済み] execution badgeが通常RenderLayerにしか付かない

FlowGraphModelのorder集計でRenderLayer/Parent/Matte/PreCompの全variantからlayer indexを取り出すよう変更。

### R58 [P2・現行UIでは未使用、記録維持] Flowchartの一系統は全レイヤーを狭いcanvasへ押し込む

旧実装`flowchart.rs:85`はcanvas heightをlayer_countで割ってspacingを決める。node height36pxは固定なので大きいcompほど重なり、全layerにrect/text/Bezierを描く。ただし現行ソース検索では`draw_flowchart_view`の呼び出し元がなく、現行パネルは`flowchart_graph::draw_node_graph_panel`を使う。したがってこれは現在のGUIで再現する不具合ではなく、未使用の旧実装に残る保守リスクとして記録を維持する。旧関数を削除・整理するかは別の明示的な判断が必要。

### R59 [P2・検証範囲を明示して修正済み] シェーダー検証はparseだけなのに「Valid」と表示する

parseに加えてNaga module Validatorを実行し、構文・意味検証を通ったときだけvalidにする。UI文言はGPU pipeline compileまでは検証していないと明示する。実GPU pipeline compileは未実装。

### R60 [P2・修正済み／LRUではない] シェーダー編集内容を無制限cacheへ溜める

global validation cacheへ128 entry・合計2MiBの上限を設定し、超過時に全消去する。個別source＋診断が上限を超える場合はcacheしない。LRUではないが保持量を有限にした。

### R61 [P2・修正済み／legacy sentinel依存は残る] CPU shader対応判定はコメント文字列に依存する

custom_shader_cpu::builtin_shaderは `source.contains("CRT Scanlines Shader")` 等で固定kernelを選んでいた。ユーザーがテンプレートの処理を変更してコメントを残すと、CPU/exportは変更したWGSLではなく元kernelを実行する。現在はUIのbuilt-in全文テンプレートと完全一致する場合だけ該当kernelを使う。既存project/test互換のmarker単独文字列も完全一致のlegacy sentinelとして維持するため、将来的な明示identity fieldがより堅牢。

### R62 [P2・修正済み／実音源評価未実施] Audio Spectrumは周波数分析ではなく時間区間のRMS

Radix-2 FFTへ変更し、Hann window、stereo interleaved downmix、sample_rateとstart/end frequencyを使った対数band mappingを実装。band出力は各frequency binの振幅power平均の平方根を正規化する。実音源への音響評価は未実施。

### R63 [P2・修正済み／異常入力実測未実施] Spectrumの短い入力でslice範囲が逆転する

以前の同関数はchunk_sizeを最低1にする一方、bands > samples.len()でも全bandを回し、len2/bands64なら逆転sliceでpanicし得た。現在はFFT binをfrequency範囲ごとに安全に集計し、band数がサンプル数を超えても入力slice境界を生成しない。短尺・異常サンプルの実行確認は未実施。

### R64 [P2・修正済み] CLI連動previewは制作documentをProjectだけへ落とす

`src/bin/preview.rs` がProductionDocumentからprojectだけをcloneし、frame描画もcompositions.firstを使っていた。bindings、tempo、audio、active_composition_idxが保持・選択されず、CLIで制作documentのbindingやactive compを変更しても同じ結果を表示する保証がなかった。現在はpreview stateにProductionDocumentを保持し、active composition indexのcomposition_for_frame snapshotをGPUへ渡す。legacy Project JSONはProductionDocumentへ移行して読む。

### R65 [P2・修正済み／実GPU確認未実施] 独立previewの再生は遅延時にフレームを取り戻さない

about_to_waitは期限超過ならframeを1だけ進めてlast_tick=nowへしていた。現在は再生開始時刻・開始frame・fpsから実時間上の現在frameを算出し、遅れた分を取り戻す。Lost/Outdated surfaceは再configure、Timeoutは次回更新に任せ、OutOfMemoryは終了する。実GPUでの長時間再生・surface復帰は未確認。

### R66 [P2・修正済み／公式OTIO互換性は未確認] 動画交換形式のパスをコロン区切りにする

integration::OtioTimelineはvideo source、frames_dir、frame_count、audio_wavを `video:{}:{}:{}:{}` へ入れ、splitn(5, ':')で復元していた。現在はVideoMediaReferenceをJSON化した `video-json:` 形式で新規出力し、旧区切り形式も読み込む。Kagari内の新形式roundtripを安全にする変更で、公式OTIO互換性は未確認。

### R67 [P2・修正済み] レンダーキューがcomposition IDでなくnameを識別子にする

render_queue_itemsは名前のVec、statusは名前key、start_comp_exportは `.find(|c| c.name == comp_name)`。同名compでは常に最初を選び、名前変更でqueue entryが切れる。production validateでcomp IDの一意性は確認されるが、名前の一意性を保証する構造ではない。queue jobはcomp IDと独立job IDを持つべき。

キュー項目、batch queue、status key、active export identityをComposition IDへ変更。全ての表示面・切替・preset追加・manifestでIDから表示名を解決し、同名compositionも別entryとして扱う。queue item自体に独立job IDはまだなく、同一compositionの複数回レンダーを許可する仕様へ拡張する場合は別途必要。

## 未接続・公開API側の設計課題

以下は現在のメインGUIで実行されると確認できなかったため、現行画面の不具合とは分ける。

### R68 [P2・未接続API・修正済み] RenderPipelineのFlushが新しい要求も捨てる

キューをdrainせず、要求を別のgenerationで送り、Flushより前の待機要求だけworkerがskipする形へ変更。project/cache versionとの比較を廃止し、実行中callbackはAPI上中断不能とコード説明を修正した。Flush後の要求を保持する回帰テストはno-default-featuresで成功。製品呼出しがない点は継続課題。

### R69 [P2・修正済み／未接続API]

with_directoryがcache directoryを走査し、妥当なsidecar metadataとframe fileの組を起動時inventoryへ登録する。破損・孤立fileと残留temp fileを除き、既存frame総量がbudgetを超えていればLRUで削減する。未読込frameも含めた容量管理とComposition単位のinvalidateにinventoryを使う。再起動後のinventory・budget・cold invalidateを含む専用10件のunit testが成功。現行製品呼出しは見つからず、FrameCache内の別disk_cacheとは未統合。last_access_epochの再起動をまたぐ永続更新とmetadata自身の容量は対象外。

### R70 [P3・修正済み／未接続API]

tile_cache::insertのLRU選択が `(key, entry.clone())` を作り、サイズを引いて削除していた。現在は `(key, pixels.len())` のみ取得してevictし、捨てるtile payloadの複製を避ける。現行利用は同ファイルのテストのみ確認。

### R71 [P1・修正済み] 通常の「保存」経路がProductionDocumentの状態を落とす

以前のCmd+S、File > Save、コマンドパレットはProject-only saveを直接呼んでいた。現在は3入口とも `project_io::save_project_to_path` に統一され、ProductionDocumentのaudio/bindings等を保持する。Collect Filesのメニュー処理は別の問題として扱う。

### R72 [P2・修正済み] CLI renderは不正なフレーム範囲を拒否しない

現在の `cmd_render` は0 frame composition、`from > to`、`to >= duration_frames`を全backend起動前に拒否し、inclusive rangeを共有する。異常引数の実CLI実行確認は未実施。

### R73 [P2・修正済み] Mocapは指定外のPNGも混ぜ、文字列順でフレームを並べる

`cmd_mocap`の説明は`frame_*.png`を要求するが、実装は指定directory直下の拡張子`.png`をすべて取り込み、path文字列でsortする。無関係なPNGが混在すると追跡列へ混入し、`frame_1.png`, `frame_10.png`, `frame_2.png`のような非ゼロ埋め連番は時系列順にならない。加えて`read_dir`の個別entry errorも破棄され、欠落したフレームを報告しない。現在は入力を`frame_<number>.png`へ限定し、数値順・重複検査を行い、entry列挙エラーを処理全体の失敗として返す。番号の欠番は許可しており、連続番号の検証は未実施。実素材での再現は未実施。

### R74 [P2・修正済み] CLIとGUIを同時に有効にするとGPU専用CLIがpollsterを要求する

`gui` featureは`wgpu`を有効にする一方、`pollster`は`preview`または`cli-gpu` featureでのみ有効になる。以前の条件は`headless_gpu` moduleと`cmd_frame`のGPU経路を`feature = "wgpu"`だけで有効にしていたため、default GUIを保った`--features cli`ではpollster依存が存在せずコンパイルできない。GPU専用APIは明示的な`cli-gpu` featureでのみ露出させ、通常CLIはCPU経路に固定する必要がある。

headless GPU moduleとCLI frameのGPU branchを`cli-gpu`条件へ統一。`--features cli`、`--no-default-features --features cli`、`--no-default-features --features cli-gpu`の各bin cargo checkが成功。

### R75 [P1・修正済み] Homeの空き容量表示がUnix専用APIを無条件に使う

`home_screen/mod.rs`は`std::os::unix::ffi::OsStrExt`と`libc::statvfs`を常時参照していた。Windows targetではこのモジュール自体をコンパイルできない。Unixだけでstatvfsを使い、他OSではストレージ表示を省略する実装へ分けた。Windows targetのGUI型チェックも成功。

### R76 [P2・修正済み] Render画面が幅500px境界で別実装・別データへ切り替わる

`home_screen::draw`は幅500px未満で実際のexport設定画面でなくreference用Render pageを出していた。その狭幅ページは固定の`main_comp`等の例データ・固定設定・0%表示を描き、追加と開始の一部だけが本物の操作だった。同じNavで幅により内容が変わり、状態に見える値も実状態と無関係だった。既にresponsive対応を持つ`draw_export_target`へ幅に関わらず統一する。

### R77 [P2・未使用Home UI群を削除／strict dead-code確認済み] Home実装に複数世代の未使用ページ群が残る

`src/ui/home_screen/mod.rs`の責務分割として、New Project/Projects、Effects/Render、Templates/Settings、Home landing、Exportを専用moduleへ移した。現行`draw()`から呼ばれていない旧resume-first sidebar/workspace/resume/project rows/recent assets/inspector/quick actions/session/status-strip描画をrootから削除し、未使用のRecent/Starred navigation variant、asset scan helpers、孤立testsを除いた。さらに未使用project-browser page/helperとresponsive mock、旧Settings page/file/feature、旧Render page/narrow mockを削除した。Queue追加処理は現行Export画面へ統合し、重複防止testもその共通関数を検査する形で保持した。現行Home/Project/Effects/Export/Preferences routeは維持。module-wide `dead_code`許可はなく、root `mod.rs`は約770行。`cargo clippy --all-features --all-targets -- -D warnings`で未使用itemを含む警告ゼロを確認した。

### R78 [P1・修正済み] Project Panelが未対応ファイルを画像として登録し、偽の寸法を持たせる


PNG/JPEG/WebPの寸法をdecoderから取得し、読み取り失敗と未対応形式はAssetへ追加せずエラー通知する。file pickerの選択肢を実装済み形式に合わせ、既存OBJ分類へOBJを含めた。動画・音声はproject panelの単純なAsset登録時点では読み込み・probeしない。

### R79 [P2・修正済み] Composition/Asset ID発行が個別実装で、配列長由来の重複が起きる

Project Panel、Composition dialog、Command Palette、Automation API、PreCompose dialog/shortcut、Timeline actionなどが別々に配列長由来IDを作る。削除後や既存JSON内のIDが飛び番なら重複し、Folderの`parent_folder`参照・PreComp target・Composition IDを使うrender queue等が曖昧になる。Projectが既存IDを走査する共通generatorを提供し、動的な生成経路がこれを使う。

### R80 [P2・修正済み] Homeの設定ページは保存値と結び付かない固定モック

HomeNav::Settingsは横幅別の`draw_settings_target*`を表示するが、テーマ・言語・UI scale等の値はPainterで固定文字として描かれ、編集widgetも設定モデルへの更新処理もない。一方、実動作する`preferences_dialog`が別に存在する。設定ページへの入口は本物のPreferencesへ繋ぎ、見た目だけの設定値を操作可能と誤認させない。

### R81 [P2・修正済み] 新規メディアの期間・音声仕様を固定値で表示する

新規動画/音声Assetのdurationは未取得値0で保持し、一覧/detailは正数のdurationだけを表示する。固定10秒や44.1 kHz/16-bit/Stereoの偽metadataを除去した。Project Panel登録時の実probeはなく、現状「未取得」は空欄表示。

### R82 [P2・修正済み] Project Panelの「Move selection out」はフォルダ名とIDを比較する

フォルダの子Assetは`parent_folder`へフォルダIDを保持するが、移動解除ボタンはその値をフォルダ名と比較していた。IDと名前が同じ場合以外は選択Assetを外へ戻せない。比較対象をフォルダIDへ揃える。

### R83 [P1・修正済み] 「未使用素材を削除」がlayerで参照中のAssetを消し得る

この操作はlayer名とsource pathの両方を`used_names`へ集め、Assetの表示名または`ProjectItem::media_path()`が参照中の値に一致した場合は残す。削除後は選択indexも破棄する。source pathで保持できるのは完全一致のみで、異なる相対/絶対表記を同一素材として解決する処理はない。

### R84 [P2・修正済み] 素材削除後も選択Assetの配列indexを保持する

「未使用素材を削除」で`assets`がretainにより詰められるが、egui一時stateの`selected_project_asset`はそのまま残る。次の描画で別Assetのindexを指したり、範囲外で選択が消えたりする。削除後にindex selectionを無効化し、違う素材を選択した状態に見せない。

### R85 [P2・修正済み] チュートリアル動画の固定temp pathを内容確認なしで再利用する

`materialize_tutorial_video`は共有名の既存ファイルがあれば、その中身を検証せず練習素材として使用する。前回の異常終了でpartial fileが残った場合や、別プロセスが作成中のタイミングに当たると動画importが失敗する。プロセスごとにpathを分け、同梱動画と完全一致する一時ファイルのみ再利用する。

### R86 [P1・修正済み] Replace FootageがAsset型と合わない素材を許し、画像寸法も更新しない

置換ダイアログは画像・動画・音声すべてを選べた。選択中Assetの型を保ったままpathだけ差し替え、画像なら古いwidth/heightも残す。たとえばModel3Dのpathを動画にしたり、別解像度画像を旧寸法で扱う状態を作れる。Asset型ごとに選択形式を絞り、同じ種別・デコード可能なファイルに置換して、画像などの型付きmetadataを再生成する。

### R87 [P1・修正済み] Footage置換が同名の無関係なlayerまで改名し、Model3D pathは更新しない

置換関数はmedia path照合より先に、同名layerを一律に新しいfootage名へ変更していた。異なるSolid等が同名なら誤改名される。また`media_path_matches`は元pathが別でも末尾basename一致を許しており、異なるdirectoryの同名素材まで巻き込む。Model3D layerのpath更新caseもない。旧pathが完全一致するlayerだけを変更し、その中のModel3D pathも更新する。

### R88 [P3・修正済み] Audio Spectrumの偶数判定が現在のClippy lintに抵触する

`audio_spectrum.rs`は`len() % 2 == 0`で偶数を判定し、現行Clippyのmanual_is_multiple_of lintが出る。標準`is_multiple_of`へ置き換えて警告を消す。

### R89 [P1・修正済み] Headless GPU readbackがGPU側の寸法clampを反映しない

`src/core/headless_gpu.rs::render_rgba8`は出力寸法をCompositionのwidth/heightから決め、その寸法でGPU textureをcopyする。一方`WgpuRenderer::render_internal`はdevice上限と`MAX_RENDER_DIMENSION`に合わせてtarget textureを小さくclampする。Headless側で両方の上限を事前確認し、実target寸法と要求寸法も照合する。超過時はcopy前にエラーを返し、出力・行strideのサイズ計算はchecked arithmeticにした。実GPUでの上限超過再現は未実施。

### R90 [P2・修正済み] Time RemapのLoop bakeがduration比例で無制限にキーを生成する

`src/core/time_remap.rs::loop_keyframes`は整数frameで周期数を先に算出し、最大100,000 keyframeを超えるbakeを拒否する。1frame sourceは定数remapへ縮約し、Composition終了後に開始するLayerはUIでエラーを表示して空のanimationを作らない。巨大duration入力のテストを追加したが、実UI操作は未確認。

### R91 [P3・修正済み] Home設定ファイルの書き込み失敗が呼び出し側へ返らない

`update_prefs`とrecent/starred/welcomeの公開関数を`Result`にし、ディレクトリ作成・serialize・temp write・sync・renameのエラーを保持する。星付け・起動画面設定ではtoast、プロジェクト開閉では設定保存の警告を表示する。書き込み障害を使った実行確認は未実施。

### R92 [P2・修正済み] 字形キャッシュの件数制限が巨大な字形画像を防がない

`font_rasterizer.rs`は最大2048件を超えると一括clearしていたが、1字のビットマップが非常に大きい場合や、大きな字形が積み重なる場合のbyte上限はなかった。フォントデータを検証せず登録し、同名フォントを置き換えても既存キャッシュが残る。字形1枚64MiB、キャッシュ総量32MiBのbyte上限を設け、上限超過の字形は返してもcacheしない。font bytesの検証と置換時のcache invalidationを追加。境界サイズの実行試験は未実施。

### R93 [P2・修正済み] 同一パスの音声差し替えがデコードキャッシュに反映されない

DecodedAudioCacheのkeyはpathと出力sample rateだけで、同じ名前の音声ファイルを更新しても前のPCMを返す。project素材の差し替え後にプレビューや書き出し音声が古い内容になる。ファイル検証後に取得したbyte lengthと更新時刻をcache keyへ加え、ファイル更新を別entryとして扱う。更新時刻を取得できない環境でもlengthは比較する。source fileがmetadata取得とdecodeの間に更新される競合は未対処。

### R94 [P2・修正済み] レンダージョブIDがミリ秒精度で衝突する

`RenderQueueItem::new`はcomposition IDと現在時刻のmillisecondのみからIDを作る。連続追加が同一millisecondに入るとIDが重複し、ID指定の削除・更新が誤ったjobへ適用される可能性がある。process ID・nanosecond timestamp・process-local atomic sequenceを組み合わせ、1,024件連続生成の一意性testを追加。

### R95 [P2・修正済み] Render Queueの最大frame範囲で件数加算がoverflowする

`end_frame - start_frame + 1`を`u32`のまま計算し、`0..=u32::MAX`ではdebug panic/release wraparoundになる。件数をu64へ広げ、最大範囲を含むテストを追加。現状、このmethodの製品呼び出し元は検索範囲では見つからなかった。

### R96 [P1・OpenFX ABI不整合・attachをfail-closedへ変更]

旧`OfxHostRaw`は公式構造と異なり、`OfxPropertySuiteV1`のset関数順も違っていた。規格順へProperty Suiteを直し、未完成のhost/image suiteへプラグインを渡さないよう`LoadedOfxPlugin::attach`をfail-closedへ変更した。公式定義は[OfxHost](https://openfx.readthedocs.io/en/main/Reference/ofxHostStruct.html)と[OfxPropertySuiteV1](https://github.com/AcademySoftwareFoundation/openfx/blob/main/include/ofxProperty.h#L877-L952)を参照。SDK sample pluginとのABI適合・完全なsuite実装は未完了で、OFX実行機能は使えない。

### R97 [P1・見せかけのrender・修正済み]

pass-through実装を成功扱いせず`Failed`を返すよう変更。未完成の`LoadedOfxPlugin::render`も明示的に未対応エラーを返し、OFX menuは「ロード可能」ではなくAPI検出のみを通知する。関連no-default tests 9件は成功。実際のOFX renderingは未実装。

### R98 [P1・同期スクリプト実行・部分修正]

Scripting ConsoleはProject snapshot上でRhai scriptを評価する。操作数上限だけでは応答性を保証できない。現在はsourceを1MiBまでに制限し、ProductionDocument検証後だけ結果を反映する。Expression評価とautomation評価はworkerへ移し、実行中の再実行を止め、session/history generationが変わった結果を破棄する。両モードに進捗callback経由のキャンセルボタンを追加し、automation workerは受け取ったsnapshotをそのまま編集して重複cloneを避ける。開始用Project snapshotのdeep clone自体はUI thread上に残る。Consoleを離れた場合のcancel伝達・遅い大規模Projectの応答計測は未確認。

### R99 [P2・修正済み] Automation frame snapshotがProductionDocument全体を毎回deep cloneする

`composition_for_frame`と`composition_for_frame_with_sources`は、選択Compositionのbindingを適用するためにProductionDocument全体を毎フレームcloneしていた。現在は対象Compositionのみcloneし、binding適用後のCompositionをmoveで返す。Projectの他Composition/Assets、audio/tempo/bindingsのdeep cloneを避ける。対象module testsは再リンク時の容量不足で未実行。

### R100 [P1・修正済み] 複数CompositionのRender Allが同じ出力ファイルを上書きする

`render_queue.rs`はComposition IDを順番に `start_comp_export` へ渡すが、各exportは共通の `app.export.export_output_path` を使っていた。2件以上をqueue renderすると同じdestinationへ順次書き、後続の出力が先行分を上書きする。Farm manifestも全itemへ同じpathを記録していた。現在は複数item時にbase name・sanitized composition name・IDからitem別pathを作り、manifestも同じpath helperを使う。path helperの一意性・directory・extensionのunit testを追加。実FFmpeg batchで複数ファイルを生成する動作は未確認。

### R102 [P2・OBJ/MTL依存収集を追加／動画実再decode未確認] 「Collect Files」が依存メディアを収集せずProjectだけを書き出す

「Collect Files...」を選択先の中に新しい`<project>_collected` bundleを作るworkerへ置換した。ProjectItemと全composition/subcompositionが明示的に参照する画像・動画・音声・3Dファイルをcanonical pathで重複排除して`Footage/`へコピーし、basename衝突は連番prefixで分離する。参照はbundle内の相対filenameへ更新する。OBJが参照するMTLと、MTLの一般的な`map_*`/bump/norm/disp等で参照するテクスチャも再帰収集し、コピー後のOBJ/MTL参照をbundle内のfilenameへ書き換える。参照先が見つからない場合は不完全bundleを作らず失敗する。動画は元ファイルをbundleへコピーし、抽出cacheを `cache_pending` として破棄してopen時に再生成させる。OBJ→MTL→textureと通常asset/video bundleのregression tests 3件、strict no-default Clippyが成功。実動画の再decode・音声復元と外部OBJ/MTL変種の確認は未実施。

### R101 [P2・修正済み] ProductionDocumentのファイル読込サイズ上限にTOCTOUがある

`ProductionDocument::load`は`metadata.len()`で100MiB上限を確認した後、上限なしの`read_to_string`を行っていた。検査と読込みの間にファイルが大きくなると、`from_json`の検査より先にメモリを消費する。現在は既存の`read_bounded_text_file`を使い、open済みfileのmetadata確認後も`take(limit + 1)`で実読込量を制限する。

## 追加レビュー：外部交換形式

### R103 [P1・部分修正] OpenTimelineIOの標準構造と外部素材の意味を一部しか扱えない

以前は`OtioTimeline`独自構造をそのままJSON化していた。現在は`to_json`が`Timeline.1` / `Stack.1` / `Track.1` / `Clip.1` / `Gap.1`、`RationalTime.1` / `TimeRange.1`、`ExternalReference.1`を出力し、Kagari固有データはmetadataへ分離する。`from_json`は標準Timeline/Stack/Track階層、gap、file URLと通常パスを解析する。legacy Kagari JSONも読む。公式schemaにないroot/track/clip schemaは拒否する。

標準出力は各Kagari layerを1 trackとして出し、track内の空白はGap、timeline startはGap長で表す。同一trackの重なりは明示errorにする。Kagari自身のlayer設定・動画cache情報はClip metadataから復元できる。外部OTIOは一般的なExternalReferenceを取り込むが、動画ファイルはKagariのdecoded frame cacheがないためComposition化を拒否する。GeneratorReference、Clip.2、多重・nested track、audio source trim、外部effects/transformsなどの一般OTIO機能は未対応で、losslessな汎用OTIO bridgeではない。

残作業：OTIO reference implementationで生成ファイルを開き、複数track・Gap・file URL・Kagari metadataの相互運用を実証する。外部video importと未対応構造をどうloss reportするか決める。現在のコンパイル確認はschema適合性やreference implementation互換性を証明しない。

### R104 [P1・構造修正済み／MLT実行未検証] MLT書出しでレイヤーの配置・重なりを失う

以前は単一playlistに全layerを連結し、`in_frame`を失っていた。現在は各表示可能layerを独立playlistへ出し、timeline startを`blank`、exclusive Kagari durationをinclusive MLT entry outへ変換し、composition終端でclipする。MLTの`in`/`out`属性はフレーム番号で出力し、読み込み側も整数フレームと時刻文字列の両方を受け付ける。tractorにbackground multitrackを作り、visual layerへ`qtblend`、複数audio sourceへ`mix` transitionを出す。MLT公式XML仕様もplaylist-per-track、multitrack/tractor、transitionsを通常構造としている（[MLT XML docs](https://mltframework.org/docs/mltxml/)、[qtblend](https://mltframework.org/plugins/TransitionQtblend/)）。

残確認：同一位置に重なる複数visual trackの合成順と、複数audio trackの実mixがMLT/Shotcut上で意図どおりか、frame boundary・blank duration・audio routingを実アプリで確認していない。layer transforms/effectsは依然MLTへ写像しない。ホストにMLT/Shotcut/Kdenlive実行環境がなく、相互運用確認は未実施。

### R105 [P1・source範囲とplaylist配置を復元／外部MLT実ファイル未確認] MLT読込でsource in点とtimeline位置を捨てる

`import_from_mlt`はentryの`in`と`out`を両方読み、MLTのinclusive outをKagariのexclusive `out_frame`へ変換する。各playlistを別々に走査し、先行blankとentry durationから`timeline_start_frame`も復元する。フレーム整数と`HH:MM:SS.mmm`を扱い、時刻文字列は最近傍frameへ丸め、overflow・逆転・不正値をerrorにする。軽量なtag scannerであり、一般XML parser相当の厳密さはない。Shotcut/Kdenliveの実ファイル往復は未確認。

### R106 [P1・修正済み] Lottie画像レイヤーが未定義assetを参照する

現在は参照画像ごとに一意のasset ID、実寸、相対ファイル名を収集し、単体Composition/Project exportのassetsへdescriptorを出力する。ファイル形式・外部asset同梱を含むLottie Playerでの再生確認は未実施。

### R107 [P1・修正済み] Lottie未対応レイヤーを空Shapeとして出力する

現在は単体・Project exportの入口でVideo/Audio/Model3D/Adjustment/Particleを検証し、非対応layer名と種類を示すerrorを返す。未対応データを空shapeとして成功扱いしない。

### R108 [P2・修正済み] Lottie図形のアニメーション寸法を空keyframe列へ変換する

現在はwidth/height両方のkeyframe frame集合のunionを取り、各frameで両値を評価して2D寸法を構築する。異なる補間・keyframe配置を含む外部Lottie playerでの視覚比較は未実施。

### R109 [P2・修正済み] Lottie shape colorのalphaを常に1へ固定する

現在はFill/Strokeの色をRGBAすべて出力する。Lottie側opacityとの積算結果はplayer上で未確認。

### R110 [P2・修正済み] Lottieで異なるblend modeを同一modeへ潰す

現在はBodymovinへ写像するblend modeをvalidateし、Divide/Subtract/Stencil等の非対応modeをexport前に拒否する。対応modeのピクセル一致は外部playerで未検証。

## 第3回：外部プロセス、保存、Dynamic Linkを追加確認

前回の範囲に加えて、動画・画像シーケンスのimport経路、Projectのatomic save、Dynamic Link TCP server、ProductionDocumentのframe snapshotを本文と呼び出し先まで確認した。さらに今回、Timeline row virtualization前の処理、外部OTIOの異常値、階層validationの計算量まで範囲を広げた。以下は静的確認で、素材を使った実行再現はまだ行っていない。

### R111 [P2・修正済み] 動画importがFFmpegの全ログをRAMへ読み込む

`src/core/video_import.rs`のフレーム展開と音声抽出を専用runnerへ移し、stdoutを破棄、stderrを別threadで継続drainしながら保持量を64KiBへ制限した。FFmpeg verbosityもerrorだけに絞った。これで`Command::output()`によるログ全量蓄積を避ける。動画import実素材でのRSS比較は未実施。

### R112 [P3・backup置換をatomic化／APIは現行保存経路から未使用] Projectのbackup copyはatomicではない

`save_project_atomic`のbackup生成を同一directory内の一時ファイルへのcopy→`sync_all`→renameにし、copy/sync/rename失敗では既存backupを維持し、一時ファイルを掃除する。本体saveとの同時実行は既存のprocess内mutexで直列化される。この関数は `allow(dead_code)` で現行GUI save経路から呼ばれていないため、通常保存で使われるbackup処理を直したものではなく、補助APIの再利用時の安全性改善。OSごとのreplace semanticsと障害注入テストは未確認で、現行GUI経路への接続も残る。

### R113 [P2・修正済み] Dynamic Linkが宣言済みのメッセージを黙って捨てる

`DynamicLinkMessage`は `AddAsset` と `TriggerRender` を定義していたが、受信側で無応答のまま捨てていた。現在は両要求に`Error` variantで未実装を返し、`DynamicLinkServer::handle_message`も同じ扱いに統一した。TCP clientとの実応答は未検証。

### R114 [P2・R144へ統合] Dynamic Linkの単一listenerが一接続で後続接続を塞ぐ

この指摘はR144へ統合。現在はsingle-client workerでsocketを処理し、listenerは次のacceptを継続する。接続中の追加clientには明示的Errorを返し、read timeoutでidle clientを解放する。

### R115 [P3] RenderQueue型が本番のqueue経路につながっていない

`src/core/render_queue.rs`の `RenderQueue` / `RenderQueueItem` / `RenderStatus` は、参照検索上このファイル内とその単体テスト以外から使われていない。実際のmulti-composition処理は別の `ParallelRenderQueue`、GUIのRender Queueは `KagariApp` の `render_queue_items` とexport状態を使う。したがってこの型に状態を追加しても製品queueには反映されず、状態・ID・進捗ロジックが複数箇所で分裂する。使用予定がなければ削除し、必要ならUI/CLI/workerが共有する実行モデルとして接続するべき。

### R116 [P2・構造修正済み／性能測定未実施] Automation評価がフレームごとに各curveを複数回全走査する

`AutomationBinding::evaluate`はbindingとcurveを一度検証し、その後curveのstrict-order invariantを使って二分探索で補間点を探す。`AutomationCurve::sample`も公開API上の検証を一度だけ行う。`map_value`はcurve point列に依存しない入力であるためcurve全体を検証しない。大規模automationの性能測定は未実施。

### R117 [P2・修正済み] 画像シーケンスimportの寸法制限がピークpixel memoryを抑えない

`src/core/video_import.rs`へRGBA換算128MiBの上限を追加し、動画probe寸法・シーケンス先頭・各シーケンスframeをdecode前に拒否する。境界値unit testが成功。実素材のピークRAM計測とFFmpeg内部bufferの計測は未実施。

### R118 [P2・修正済み] Still sequenceの欠番を詰めて元の時間間隔を失う

`import_image_sequence`はファイル名のframe番号でsortした後、出力先を `frame_{output_index}` として0から連番で保存していた。現在は連番の欠番があるsequenceを読み込み拒否し、間隔を詰めて誤った時間で再生することを防ぐ。frame番号の開始値が0以外のsequenceは引き続き0基準へ正規化する。

### R119 [P2・修正済み] 一部の動画・シーケンス読込がUI callback内で完了まで同期実行される

動画Assetからレイヤー追加、Image Sequence、Replace Footageをworkerへ移した。Replace Footageは開始時にsession/Asset ID/元パスを保持し、全関連composition FPSのdecode完了後に同じAsset IDと元パスが残っている場合だけProjectへcommitする。プロジェクト切替、素材削除・変更、部分import失敗では追加したcacheを破棄する。遅い素材での再描画応答測定とGUI実操作は未実施。

### R120 [P2・修正済み] 画像シーケンスのファイルpickerが実際には拒否される形式を表示する

`src/ui/menu/file_edit.rs`のpickerがBMP/TGAも表示していたが、`import_image_sequence`はPNG/JPEG/WebPのみ許可するため選択直後に失敗していた。picker filterをcoreが対応するPNG/JPEG/WebPだけに揃えた。UI picker上での実選択確認は未実施。

### R121 [P1・修正済み] 動画import失敗時に動画の代わりのSolidを無言で追加する

`src/ui/project_panel.rs`は動画Assetの追加に既存のsession-aware background importを使うよう変更し、完了イベントhandlerが失敗をtoast表示する。失敗時にProjectへlayerを挿入するfallback Solidは削除した。別の同期分岐に動画が誤って到達した場合もエラー表示して操作を中断する。worker完了・失敗のGUI実機動作は未確認。

### R122 [P1・修正済み] Collect Filesが動画入りProjectを保存できない

`collect_production_document`は収集後のVideo layerに `frame_count=0` を設定していたが、ProductionDocumentのvalidatorが0を拒否し、動画入りbundleが保存できなかった。現在はframe countを保持し、`frames_dir`を合法な `cache_pending` sentinelへ置換する。プロジェクトを開く際、bundle内へ収集したsource movieがFootageから解決されると既存処理がcacheを無効化し、sourceからフレームと音声を再生成する。回帰テストは追加し `cargo test --lib --no-default-features --features cli core::project_migration::collect_tests::collect_bundle_keeps_video_metadata_valid_for_cache_rebuild` で成功。テストのsourceはsynthetic bytesのため、実FFmpeg再decode・音声生成・再openは未検証。

### R123 [P2・通常時cacheと吸着二分探索を追加、操作時build/計測は残る] Timelineの可視行culling前に全レイヤー由来データを毎frame再構築する

`src/ui/timeline/mod.rs`は画面外レイヤーのwidget生成をskipしているが、その直前に全layerの`parent_choices` (ID/name String clone)、`layer_edges`、`collect_all_kf_frames`を毎frame生成する。さらに可視rowごとに親名を全件検索するため、可視行数V・総layer数Nで少なくともO(N+V×N+K)の仕事となり、大規模時はwidget cullingの利点を打ち消す。レイヤー移動時のsnapは他全layer edgeも走査する。最大100,000 layerを受理するProductionDocumentに対し、深いProjectや多数keyframeでTimelineを開くだけでもframe budgetを超え得る。

現在はderived listsをegui context内でComposition ID/history generationごとにArc cacheし、parent name lookupをindex mapへ変更。キーフレーム吸着はsort済みkeyframe列の二分探索、layer edge吸着もsorted edge indexの近傍探索へ変更し、候補全件走査を除去した。当初は任意のpointer input中にcacheを迂回していたが、現在はmodel-edit transactionとlayer reorderの実ドラッグ中だけfresh buildへ迂回する。hover・scroll・toolbar clickではcacheを再利用する。編集ドラッグ中のderived data再構築と大規模Projectでのframe-time測定は残る。

### R124 [P3・修正済み／外部client未実接続] Dynamic Link handshakeがprotocol versionを検証せず互換性を誤認する

handshakeは`major.minor.patch`形式とmajor version 1を確認し、形式不正・非互換majorはError応答にする。TCP経路はError後に接続を閉じる。外部clientとの実接続検証は未実施。

### R125 [P2・修正済み] OTIO importが破損したKagari video referenceをNull layerへ黙って変換する

`to_composition`は`Result`を返すよう変更し、破損`video-json:`をclip名付きエラーにする。二つのOTIO import UI経路がエラーを報告する。malformed payloadのunit test成功。

### R126 [P2・修正済み] ProductionDocumentの階層検証に親深度比例の全祖先走査がある

layer/folder parent cycle検証を全node共有のstate mapを使う線形走査へ変更した。layer IDも先にset化し、後続ID参照の全Vec検索をなくした。既存のlayer/folder cycle拒否を含むProductionDocument test 29件成功。100k要素でのbenchmarkは未実施。

### R127 [P1・修正済み] 外部OTIOのsource range加算がu32 overflowする

`OtioTimeline::to_composition`をfallibleにし、`checked_add`失敗をclip名付きで返す。境界値unit test成功。外部アプリのOTIO fixtureは未確認。

### R128 [P1・修正済み] ProductionDocumentがanimated keyframeの順序を検証しない

共通validation helperでscalar/vector/camera/mask path等すべてのanimated keyframe列にstrictly increasing frame順を要求し、重複frameも拒否する。逆順・重複frameの回帰テストを追加。ProductionDocument test 29件成功。

### R129 [P2・修正済み] 深いprecomp参照cycle検証で全composition treeをedgeごとに再検索する

一度作ったID→nested Composition indexから依存数/逆辺を線形構築し、Kahn graph walkでcycleを検証する。全root treeの参照ごとの再検索と再帰DFSを除いた。既存のself-precomp cycle拒否を含むProductionDocument test 29件成功。上限近くのbenchmarkは未実施。

### R130 [P1・修正済み] TimelineのParent dropdownがdescendant判定に失敗して循環を許す

以前の選択処理は、候補parentが選択中layerの子孫かを調べず、現在の`parent_id == layer.id`だけを比較していたため、既存の親をself-parentへ設定した特殊状態以外はcycle guardにならない。現在は候補から既存parent chainを辿り、選択中layerへ戻る候補を拒否する。self/descendant/祖先の選択を確認するunit testを追加。GUI操作上での確認は未実施。

### R131 [P2・修正済み] FrameCacheの旧世代掃除が保持pixel全体を一時複製する

`collect_garbage_below`は削除対象の各`Arc<Vec<u8>>`から`Vec`をdeep cloneし、全frame分を一時vectorに保持してからdisk writeしていた。RAM cacheが512MiBまで膨らんでいる場合、世代切替時の一時使用量がほぼ倍になる。キーだけを列挙し、entryを1件ずつ外してそのbufferを参照のままdiskへ書く方式へ変更した。GUI featureのcargo checkは成功、専用unit testは未実行。

### R132 [P2・修正済み] FrameCacheのディスクbudget計算が一時ファイルを数える一方で掃除できない

`current_disk_usage`はcache directory内の全ファイルを合算する一方、`evict_if_over_budget`が削除候補にするのは`.rgba`のみ。プロセス強制終了などで残った`.rgba.tmp.*`はbudgetに算入されるがevictされず、cacheがbudget超過状態のまま新しいframeの書出しを拒否し続ける可能性があった。現在は永続`.rgba`のみをbudget計算し、24時間以上経過した一時ファイルを掃除する。強制終了による残留tempの回帰testは未実行。

### R133 [P1・現行UIでは該当consumerなし／指摘記録を維持]

当初の指摘はFrame QCのFrameCache依存とViewer pixel samplerのreaderに対応するproduction writer不足だった。現行Frame QCはフル解像度frameをbackground workerで直接renderし、Project session、history generation、Composition ID、frameを照合して古い結果を破棄する。512MiB超のframeは開始前に拒否する。現行`src/ui`では`FrameCache::get`/`insert`/`get_with_layers`の呼び出しがなく、rulerのcached-state表示だけが残る。Viewer pixel samplerおよびRGBA欄も現行UIで確認できないため、対応writerがないことで止まる現行操作はない。指摘を削除せず、現状では対象consumerが存在しないとして閉じる。samplerを再導入する場合はComposition IDを含む安全なkeyとwriterを同時に実装する。

### R134 [P2・修正済み] 画像シーケンスの重複frame番号を片方だけ残す

ファイルstemをframe番号へ変換後に`dedup_by_key`していたため、`shot_1.png`と`shot_0001.png`のように異なるpathが同じ番号になると一方を無言で捨てていた。現在は重複番号をimport errorとして拒否する。既存reindex testの入力を連番へ修正した。

## 第10回：FFmpeg出力と並列レンダー範囲を追加確認

FFmpeg動画/GIFの一時出力名、並列レンダーのMFR処理を実装・呼び出し元まで追った。実FFmpeg実行や巨大rangeのstress testはしていないため、FFmpeg muxerの挙動は静的な引数からの高確度指摘として扱う。

### R135 [P1・修正済み／FFmpeg実出力は未確認] FFmpegの一時出力名からコンテナ拡張子が消える

一時出力名を`<stem>.kagari-tmp-<pid>-<seq>.<ext>`に変更し、GIFを含むFFmpegが元のコンテナ拡張子を判定できるようにした。CLI featureの型検査は通過。各コンテナの実FFmpeg出力は未確認。

### R136 [P2・修正済み／巨大range未実行] Multi-frame renderingがフレーム番号全件を先に確保し、最大範囲で件数がoverflowする

MFRはu64のRayon rangeからframeを直接処理し、全frame番号のVecを作らない。queueの総frame数、完了数、progress callbackもu64化してfull-u32-range時の件数overflowを避ける。CLI featureの型検査は通過。最大rangeの実行とcancel応答は未確認。

### R137 [P2・修正済み] 0フレームのFFmpeg exportを出力なしで成功通知する

動画/GIF exportの共通事前検証を追加し、0フレーム、0 FPS、不正寸法、拡張子不足をworker起動前にerrorとして返す。通常のExport dialogが0を補正する場合もcore APIは不正要求を成功扱いしない。CLI featureの型検査は通過。

### R138 [P1・構造修正済み／WebP実出力未確認] MOV専用muxer optionをWebPにも常に渡す

`-movflags +faststart`をMOV/MP4系拡張子にだけ追加し、WebPには渡さない。WebPは`.webp`出力を要求し、audio trackの指定も拒否する。`movflags`はFFmpeg公式formats文書でMOV/MP4 muxer optionとして説明されている。[FFmpeg formats documentation](https://ffmpeg.org/ffmpeg-formats.html#MOV_002fMP4_002fISMV)。CLI featureの型検査は通過したが、WebP実出力は未確認。

### R139 [P2・修正済み／外部player未確認] Lottie Project exportが未参照Compositionまで検証・asset収集する

rootからPreComp参照を辿って到達したCompositionだけをcycle-safeなvisited setで集め、その範囲のみlayer検証・画像asset収集・PreComp asset生成を行う。CLI featureの型検査は通過。未参照のunsupported compを含むProjectでの回帰実行と外部Lottie player確認は未実施。

### R140 [P1・修正済み／外部MLT実ファイル未確認] MLT書出しが未対応レイヤーを無言で落とす

`MltExporter::export_to_xml`はText/Shape/PreComp/Particle/Adjustment/3Dなどを含む場合、layer名と種類を示してexportを拒否する。Nullは描画内容がないため無視する。UIはXMLを保存する前にエラーを通知する。Shotcut/Kdenliveの実アプリ動作は未確認。

### R141 [P1・修正済み／OpenShot実アプリ未確認] OpenShot書出しも未対応レイヤーを無言で落とす

`OpenShotExporter::export_to_json`はImage/Video/AudioだけをJSONへ追加し、Solid/Text/Shape/PreComp等は無視して成功を返す。exporterは`Result`を返すため、未対応layerを名前・種類付きで拒否するよう変更。外部OpenShotでの読込確認は未実施。

### R142 [P1・修正済み／実ファイル再生未確認] 再サンプリング上限超過時に元レートのPCMを成功値として返す

`AudioBuffer::resample`は出力が512MiBを超える場合にerrorを返し、失敗バッファはcacheしない。preview mix errorは一度toastで表示し、同じstate hashで毎frame再mix・再通知する状態も避ける。Export mixもsource errorを`Result`で返す。実ファイルでの再生確認は未実施。

### R143 [P2・修正済み] CLIスクリプトのmetadata検査後にファイル全量を読む

Rhai runnerはsourceを1MiBに制限し、CLIもmetadataを見てから`read_to_string`を呼んでいたが、検査と読込の間にファイルが増えた場合、全量をRAMへ読み込めるTOCTOUがあった。現在は1MiB上限付きreaderで読込量自体を制限する。

### R144 [P2・修正済み／複数client対応は対象外] Dynamic Linkの単一接続処理がlistener全体を塞ぐ

対象：`src/core/integration.rs::start_sync_server`

接続ごとのworkerへ処理を移し、listenerはacceptし続ける。仕様どおり同時clientは1つに制限し、占有中の追加接続には制限理由をエラー応答する。idle read timeoutでactive slotを解放し、UI通知channelは`try_send`としてserver workerの無期限blockを避ける。複数clientを同時処理する機能は実装していない。loopback以外での公開や実client複数接続試験は未実施。

### R145 [P1・修正済み／legacy互換ファイル未実検証] 壊れたlegacy動画referenceを空pathの動画として受理する

対象：`src/core/integration.rs::OtioTimeline::to_composition`

legacy形式`video:<source>:<frames_dir>:<frame_count>:<audio_wav>`のparseで、必要fieldが欠けると空文字・0へ`unwrap_or_default()`する。例えば`video:foo`でもVideo layerを作り、失敗を返さない。`frame_count = 0`や存在しないframes directoryを持つlayerが正常なimport結果としてProjectへ入る。新形式`video-json:`はJSON parse errorを伝えるのに、legacy経路だけ破損を成功扱いする非対称もある。

legacy payloadを厳密にparseし、必須field不足、空source/cache path、数値でない/0 frame count、source rangeがframe countを超える場合をclip名付きで拒否する。旧形式の曖昧なpath内colonは新JSON referenceへ移行する必要があり、古いWindows pathを含む実Projectでの互換確認は残る。

### R146 [P2・責務別moduleへ分割済み／legacy画面重複はR77に残る] Home画面が10,000行超の単一モジュールに集中している

対象：`src/ui/home_screen/mod.rs`

`home_screen/mod.rs`は以前10,000行超で、recent projects、asset browsing、project creation/open、settings、render queue、reference Home/Projects/Effects/Render/Templates/Documentation、responsive画面が混在していた。ページ描画を専用moduleへ分割し、reference entry/navigation glueは`reference_pages.rs`へ置いた。今回、旧resume-first UIとrecent-assets scan、現行導線がないproject browser/settings/render画面を削除・統合した。現在のroot `mod.rs`は約770行。Home module-wide dead-code allowも除去し、strict all-target Clippyを通過。Home画面のruntime visual QAは未実施。

分割は関数の移動だけとし、画面routing・state・responsive layoutの挙動は変更していない。Project/Assetsとreference pageの共有model・componentをさらに統合する作業は別途レビューが必要。

### R147 [P1・修正済み／実音源エラー試験未実施] 音声ファイルの読込・再サンプル失敗を無音として書き出す

対象：`src/core/audio_engine.rs::load_cached_audio`、`mix_audio_sources_for_frame_with_state`

path validation、metadata取得、decoder、resamplerのどこで失敗しても`load_cached_audio`が`None`を返していた。現在はsource errorを`AudioFrameMeter`へ伝え、プレビューは素材名/path/reasonをtoastで一度通知しつつ継続、PreCompを含むWAV mix/exportは一時WAVを削除して失敗を返す。欠損・未対応・壊れた音源や512MiB制限超過を、正常な無音出力として成功扱いしない。実音源の破損・上限超過を使った実行確認は未実施。

改善案：音声mix APIを`Result`または成功bufferと診断一覧の組にし、layer名/path/reasonをexport UIとログへ返す。リアルタイムpreviewでは全mixを止めない設計も可能だが、状態表示・rate-limited warning等で無音化を利用者に知らせる。exportでは音源を失った状態を成功扱いしない。意図的なmuteやactive range外の無音と、読み込みエラーを区別する。

### R148 [P2・修正済み／OS実機確認未実施] 設定ファイルの場所がOS標準の設定ディレクトリ優先順に従わない

対象：`src/ui/project_io.rs::prefs_path`

設定場所は`HOME`があれば常に`$HOME/.kagari_prefs.json`を返し、その後に`XDG_CONFIG_HOME`、`APPDATA`を確認する。Linuxでは通常`XDG_CONFIG_HOME`（未設定なら`~/.config`）を使う構成が望ましいが無視される。WindowsでもHOME互換環境変数が設定されていると、Roaming AppDataよりhome直下へ保存する。既存設定の移行・同期・backup対象の期待とずれ、環境によって設定が見つからないように見える。

設定場所をmacOS Application Support、Windows AppData、Linux XDG Config Home（未指定時は`~/.config`）へ切替えた。旧`$HOME/.kagari_prefs.json`もfallback読込し、次回設定更新時に既存keyを保ったまま新pathへ書く。各OS実機での移行確認は未実施。

### R149 [P2・修正済み／非UTF-8実path未実行] 最近使ったファイルの非UTF-8 pathをlossy変換して復元不能にする

対象：`src/ui/project_io.rs::path_key`

`PathBuf`を`to_string_lossy()`でUTF-8文字列化して最近使ったProjectやstarred pathを保存する。Unixでは非UTF-8 file nameが合法であり、lossy replacement後の文字列は元のpathへ戻らない。該当pathで開いたProjectはRecentから再度開けず、異なる非UTF-8 pathが同じlossy文字列に正規化されれば重複排除も誤る。Windowsのwide pathも無効なsurrogateを含む場合に同様の往復保証がない。

非UTF-8 pathをUnix raw bytesまたはWindows UTF-16 code unitsのhex tokenとして可逆保存し、Recent/Starredを開く直前にOS-native `PathBuf`へ復号する。既存のUTF-8文字列pathはそのまま読める。Kagariが作ったtokenで始まる通常pathはraw encodingし、prefix衝突も避ける。実在する非UTF-8名のfileをUIから開く確認は未実施。

### R150 [P2・修正済み] Crash Recoveryが復旧JSONをサイズ上限なしでメモリへ読む

対象：`src/core/autosave.rs::load_latest_snapshot`、`load_latest_production`、`is_valid_project_file`

復旧候補とRecovery画面の妥当性判定が`read_to_string`でファイル全体を読み込む。復旧フォルダ内の壊れた巨大JSON、またはユーザーが選んだ巨大ファイルに対して、ProductionDocument通常読込の100MiB上限が適用されず、復旧一覧を開くだけで過大なメモリ確保が起き得る。

3経路をProductionDocumentと同じ`MAX_PROJECT_JSON_BYTES`上限付きreaderへ統一した。上限を超える候補は復旧不能として読み飛ばす。巨大ファイルを用いた実メモリ確認は未実施。

### R151 [P2・誤った成功値を停止／SfM未実装] 3D Camera Trackerがカメラ解を装うヒューリスティックを返す

対象：`src/core/camera_tracker_3d.rs::solve_camera_motion_3d`

関数名・型は3DカメラトラッキングとStructure from Motionの解を示すが、実装は先頭/末尾の2D観測から任意の深度を作り、各フレームの平均2D移動を平行移動に割り当てる。回転は常に単位行列、reprojection errorは常に`0.85`で、幾何最適化や再投影誤差計算を行わない。入力が8 track以上なら妥当性にかかわらず、それらしいcamera/point-cloud結果を返す。モジュールは公開されているが、現在の呼出元は見つからず、製品UIへの接続は確認できない。

ヒューリスティックな偽解を削除し、APIは`CameraTrackerError::NotImplemented`を返すようにした。呼び出し側に成功結果を装わない。正規のbundle adjustment/SfM solver自体は未実装で、UI・製品機能にも接続していない。

### R152 [P3・RAM上限追加／未接続APIとhash強度は残る] MerkleFrameCacheが無制限のRAM cacheで製品経路にも未接続

対象：`src/core/merkle_frame_cache.rs::MerkleFrameCache`

当初の`insert`は画像byte配列をそのままHashMapへ保持し、件数・byte予算・evictionがなかった。現在は512MiBの上限とFIFO evictionを加え、単一bufferが上限を超えると保存しない。一方、repository内に`MerkleFrameCache`の利用箇所はなく、現行rendererのframe cacheとして機能していない。FNV-1a 64-bit hashは依然として衝突耐性が弱く、永続的・正しさが必要なcontent identityには不適切だが、未接続APIなので現状の出力破損とは断定しない。

製品接続前に既存の有界`FrameCache`/`TileCache`へ統合するか、LRU、cache key schema/version、衝突耐性を設計する。現在の上限付きFIFOはRAM総量を抑えるが、最近利用されたentryを優先して残す保証はない。現状は製品不具合ではなく未接続APIの保守・統合課題。

### R153 [P1・偽のモデルロード／推論成功を停止／AI推論は未実装] AI Runtime Bridgeがモデルを実行せず推論結果を装う

対象：`src/core/ai_runtime_bridge.rs::AiModelSlot::attach_model`、`AiRuntimeHub::run_segmentation_matting`

`attach_model`はファイルの存在・形式を検査せず、推論runtimeへロードもしないまま`is_loaded = true`を設定する。その後の`run_segmentation_matting`は添付モデルの有無にかかわらず、矩形promptなら矩形mask、未指定なら入力のlumaを返すシミュレーションで、SAM/BiRefNet等の推論を実行しない。さらに`is_task_accelerated`はloaded flagとpathだけでtrueになり、実際にGPU backendが利用できることを表さない。repository内に本moduleの製品呼出元は見つからないため、現行UIのAI機能へ誤った結果が流れるとは断定しない。

stub APIと明示し、model attachとmatting inferenceは`NotImplemented` errorを返すようにした。accelerated判定は常にfalseで、slotへpathやloaded flagを書くだけでは実行可能状態にならない。現在の関数をAI matteとして製品接続してはいけない。runtime/model probe・load・推論自体は未実装。

### R154 [P2・偽のmergeと無制限履歴を除去／同期機能は未実装] TeamSyncEngineが操作を適用せず、履歴を無制限保持する

対象：`src/core/team_projects_sync.rs::TeamSyncEngine`

公開説明はOperational Transformation/CRDTを掲げるが、`merge_remote_packet`は受信operationをProjectへ適用せず、UpdatePosition同士の限定的なtimestamp/version比較以外は競合解決しない。Add/Remove layer、Opacity、markerには順序・重複適用・競合規則がなく、全送受信packetを`applied_packets`へ上限なしでclone保持する。ファイル内unit testもpacketを渡して返却されるoperation数を見るだけで、統合後Projectの状態を確認しない。現在の製品呼出元は見つからず、実際の共同編集機能が動いているという意味ではない。

`merge_remote_packet`はprojectを変更せず受信packetを履歴へ複製していた。Operation Transform/CRDT/LWWを実装したかのような説明と擬似競合解決を取り除き、remote mergeは`NotImplemented` errorを返す。outgoing packet生成はpacketを内部履歴へ蓄積しない。sequence overflowは明示errorにする。projectへの操作適用、idempotency、順序・重複・切断復帰規則は未実装なので、同期API自体は製品接続不可。

### R155 [P3・未使用placeholderを除去] SqlTimelineDbという名前に対し、SQL・frame index・本番接続がない

対象：削除前の`src/core/sql_timeline_db.rs::SqlTimelineDb`

この型はSQL databaseではなく、layer単位の`HashMap<String, Vec<usize>>`とrecord `Vec`を持つメモリ構造である。`query_range`は該当layerの全recordを毎回走査し、frame/property index、永続化、transaction、timelineとの同期はない。さらに全recordsを保持するが削除・compact APIもない。repository内の利用箇所は定義とunit testだけであり、現行timelineの高速化には寄与していない。

初回対応では`InMemoryKeyframeStore`へ改名し、layer/property/frameのBTreeMap indexを追加したが、内部製品consumerはなく、Projectのkeyframe情報と二重管理になるだけだった。そこで未使用の実装と`core::sql_timeline_db`公開moduleを削除した。現行TimelineはProject内のkeyframeを正本とし、`timeline_derived_data`のgeneration cacheからsorted frame indexを利用する。SQL/WAL/永続timeline DBの機能があるとは主張しない。repository内にconsumerはなかったが、外部Rust codeがこの実験moduleを直接importしていた場合は移行が必要。

### R156 [P1・修正済み] Exportの音声有無判定がPreComp内部を調べず、ネストした音声を落とす

対象：`src/ui/export_dialog.rs`、`src/core/audio_engine.rs`

Exportの事前判定はactive Compositionの直下layerだけを見てAudio/Video audioを探す。一方、audio mixerはPreCompを再帰的にmixできる。Projectに外側CompositionのPreCompと、内側CompositionのAudio layerがある場合、mix engineは音声を処理可能でもexport側の`has_audio`がfalseとなり、音声準備とmuxを丸ごと省略する。

階層上限を設けた`composition_contains_audio_sources`でnested PreCompを調べ、export status/音声mux判定をmix engineの対応範囲と揃えた。ネストした実プロジェクトでの再生・mux確認は未実施。

### R157 [P2・修正済み] 3Dカメラsolveが不正なtracking配列でpanicまたは巨大出力する

`src/core/camera_track.rs::solve_3d_camera_from_tracks`は各featureの`frames`と`coords_2d`が同じ長さという前提で`coords_2d[idx]`を参照するが、入口で対応関係を検証していない。長さ不一致でpanicし、NaN座標はground-plane推定の`partial_cmp(...).unwrap()`をpanicさせる。また観測frame数に上限がなく、入力に応じて点群・camera frameを確保する。

入口でFOV・座標の有限性、配列長一致、frameの厳密な昇順を検証し、不正入力をerrorにする。一意frame数を100,000件までに制限し、有限値確認後の並べ替えに`total_cmp`を使う。実tracking dataで上限値の妥当性は未測定。

### R158 [P2・background bakeと世代検証を実装／最大入力の体感計測は未実施] Motion SketchのベイクがUI threadで二次時間を使う

`src/ui/viewport.rs`はpointer release時に`build_position_keyframes`を同期実行する。Douglas-Peuckerの`simplify_recording`は分割点が端に偏る形状で最悪O(n²)になる。今回再帰を明示的なrange stackへ変え、有限座標だけを対象にし、同一frame点をまとめた。記録中は同一frameの点を置き換え、最大8192点で4096点へ間引く。ベイク側も4096点に上限を置く。これでstack overflowと無制限sample memoryを避け、処理点数を制限するが、最大入力時のUI実行時間は未計測。

pointer release後のkeyframe bakeをworkerへ移動した。worker結果はProject session・history generation・Composition ID・Layer IDを持ち、UI threadへ戻った後にsession/generationを照合してから対象layerへ新しいundoable project editとして適用する。作業中にProjectが変更・切替された場合は古い結果を破棄する。Empty channel中のみ16ms後にrepaintを要求し、常時再描画はしない。GUI feature cargo check、Motion Sketch test 1件、strict GUI-feature lib Clippyが成功。長時間軌跡での体感応答時間と4096点間引き時の形状保持は未計測。

### R159 [P2・修正済み／機能未対応] AEP parserはAfter Effects projectを解析せず、印字可能文字列をCompositionと誤認する

旧`parse_aep`はmagicを検証せず、NUL終端の印字可能ASCIIをCompositionとして偽造していた。現在はヘッダー長を確認した後に「.aep project parsing is not supported」とerrorを返し、偽Compositionを生成しない。ファイル読込自体は未対応。`aep_to_compositions`は互換性のため残っているが、実parserが返すAepHeaderはなく、呼び出し元も見つからない。

実AEP parserを導入するまで機能入口を提供しない。実ファイルを使った形式検証は未実施。

### R160 [P1・修正済み、cache再検証I/Oは未計測] relink後の動画cache検証が中間frame欠損を見逃す

directory走査で期待する全indexの存在、重複形式、空ファイルを確認する。さらにversion/count/dimension/総byte数と全frameのcontent fingerprintを持つmanifestをatomic writeし、再利用時に全内容をstreaming fingerprintで照合する。manifestのない旧cacheは各frameのdimension上限を先に検査してから実decodeし、妥当ならmanifestへ移行する。欠損・破損cacheは再利用しない。異常なファイル内容の検出には非暗号学的FNV-1aを使うため、悪意ある改ざん対策ではない。大規模cacheの全読込I/O時間と実プロジェクトでの再生成は未計測。

### R161 [P2・依存構築を辺数比例へ改善] VFX graph schedule生成で全nodeをnodeごとに再走査する

`VfxGraphCompiler::compile`は各scheduled nodeの依存一覧を作る際、全layerを走査して各adjacency listを検索していた。cycle fallbackも未schedule nodeの確認に`sorted_order.contains`を使い、node数二乗になり得た。現在は隣接配列と逆依存indexを一度構築し、cycle fallbackにはscheduled bitmapを使う。辺sortの`O(E log E)`は残るが、schedule作成時の全node再走査を除いた。

残作業：依存辺のsortは`O(E log E)`のまま。大規模graphでのcompile時間・メモリ実測は未実施。

## 第2回で確認できた改善・制限

## 実装進捗

### 修正を反映した項目

- R18/R165: 現在のtoolchainで再検証し、test codeの`field_reassign_with_default` 34件をstruct initializerへ移してstrict all-target Clippyを通過。precomp rasterizer二つ、Content-Aware Fill/Roto Brush worker、骨格階層出力、inline text editor geometryを名前付きrequest/geometry/context型に整理。Wigglerは設定型ベースAPIへ移行し、CPU effect pipelineとpaint rasterizerにも共通context/style型を導入。multi-argument crate-wide抑制を除去し、残る13関数だけにlocal allowを設定。`excessive_precision`、`type_complexity`を含む全crate-wide Clippy抑制を除去。`chunks_exact_to_as_chunks`をlib/test targetの固定長pixel/audio/geometry chunk処理へ移行。2026-10-04の`cargo clippy --all-features --all-targets -- -D warnings`が成功。R18/R165の記録は保持する。
- R01: LUTをthread-localから共有ロック付きスナップショットへ変更し、CPUレンダーの並列pixel loopへ一度取得したArcを渡す。
- R02/R03: 存在しない「現在のグレード」出力を廃止し、実際に読み込まれているLUTだけを保存するUIへ変更。保存先を選択し、書込み成功後だけ成功通知する。カラーグレード自体のLUTベイクは未実装として誤認させない。
- R04: 未接続の色管理選択肢を実行可能なdropdownとして見せるのをやめ、未対応表示へ変更。
- R04追記: 実際のrenderが適用しない16/32-bit深度選択と、保存・変換されない色空間status値も除去。現在の8-bit出力と色管理未対応を表示する。指摘・過去の状態記録は削除せず、色変換機能そのものは残課題として明記。
- R05: LUT参照パスをProductionDocumentとautosaveへ保存し、プロジェクトopen/recoveryで非同期復元する。LUT本体は外部参照のままで、埋め込み・CLI共有は未実装。
- R39: 動画importの容量概算・空き容量予備・処理中監視、FFmpeg停止とUIキャンセルを追加。実素材での停止とキャンセルの確認は残る。
- R64/R65: CLI previewはProductionDocument/active compositionを保持し、再生frameは開始時刻から算出。GPU実機は未確認。
- R66/R67: 動画media referenceにJSON形式を導入し旧区切り形式を維持。Render Queue/status/exportはComposition IDで扱う。
- R06: 画像レイヤーの無動作pinボタンを実際にPosition Pinを追加する操作へ修正。異なる意味のStarch/Overlap/Advanced pinはモデルに型がないため、未実装のまま。
- R08: 組み込みワークスペース6種の誤ったpanel割当を修正。
- R07: Timelineの全追加レイヤー、重複・分割、Asset Library/Project Panel/audio-keyframe生成で既存IDを避ける `next_layer_id` を使う。
- R09: 2倍サイズが上限に入らないときはSSAA全体を無効化し、片軸だけクランプされた画像を2x downsampleしない。
- R10: 音声mix/WAV準備をUI callbackからバックグラウンドexport jobへ移す。準備時の進捗は状態表示のみで、架空の割合は出さない。
- R11/R12: CPU viewerのフレーム結果とTextureHandleを表示設定込みで保持し、exposure/LUT modeもGPUと同じ設定から描画。
- R29: PNG callbackへ渡される絶対フレームをそのまま描画し、work area開始値の二重加算を除く。
- R30: PNG/動画の各フレームでProductionDocumentのautomation snapshotを評価してからレンダーする。
- R31/R32: 音声cache keyにProjectHistory世代を加え、プロジェクト交換時に古いWAVを無効化する。work areaから切り出したWAVのseekを相対秒にし、Out frameをexclusive endへ直す。
- R29–R32: 現行コードを再確認。PNGのabsolute-frame契約、GUI export automation snapshot、音声cacheのhistory generation/Project交換時reset、work-area-relative seekとinclusive Out変換が反映済み。
- R33/R34/R35: WAV生成を絶対frame境界のsample数にし、全尺PCM保持をやめてバッファ出力。RIFF上限を先に検査し、I/O失敗をResultで伝え、export用一時WAVをjob終了時に削除。
- R36: 動画import jobへ開始時のProject session、Composition ID、選択layer IDを保持させる。完了時に別プロジェクトへ誤挿入せず、元compositionへ挿入する。session IDは全てのGUI project replacement経路で更新する。
- R42/R43: 復旧UIは同一slotのProjectとProductionDocumentを取得し、失敗時にslotを消さない。成功時は音声設定も復元してダイアログを閉じる。
- R44: autosaveをプロセスID namespaceへ分け、復旧・削除は選択した復旧元だけに限定。
- R46: ImageCacheのbyte予算を超える静止画をdecode前に拒否し、自己evictionでNoneになる曖昧な動作を避ける。
- R47: 画像取得後のCPU raster/precomp pixel loopを共有cache lockの外で実行する。画像データのArc化とclone削減は残課題。
- R48: GPU画像cacheを200枚固定から512MiBの実format byte予算へ変更し、任意HashMap順削除をLRUに置換。
- R49/R50: GPU still/video/PreComp cache keyへ外部source file revisionを含める。
- R52: ディスクフレームcacheをインスタンス固有namespaceで分離し、同じframe/versionを別プロセスが誤読する衝突を避ける。
- R53: frame disk cacheとPersistentDiskCacheの読み込みをmetadata/上限付きにし、frame writerの全pixel一時コピーを除去。
- R54: 予算を超える単一frame書込みを拒否し、evict後に実使用量を再確認。既存key置換を予算計算から除外し、process内writerを直列化。複数process間の予約は未対応。
- R55/R56/R57: graph scheduleの同順位順序を決定化し、親祖先依存を閉包へ含め、各operation variantをFlowGraphのexecution badgeへ反映。
- R59/R60: WGSL validationにNaga semantic Validatorを追加し、UIにGPU compile未確認を表示。source/diagnostic cacheをentry数と総byte数で制限。
- R63: スペクトラム帯の範囲を整数境界で分割し、入力サンプル数より帯域数が多い場合も逆転sliceを作らない。
- R62: 時間区間RMSをradix-2 FFTベースの周波数band分析へ置き換え、Hann窓・stereo downmix・frequency rangeを反映。
- R64/R65: 独立previewでProductionDocumentとactive compositionを維持してframeごとのautomation snapshotを描画し、絶対時刻から再生frameを算出。プロジェクト再読込時も再生基準を更新。Lost/Outdated GPU surfaceを再設定。
- R66: video media referenceの新規出力をJSON payloadへ変更し、Windows drive letterやコロン入りpathでフィールドがずれないようにする。既存のlegacy `video:`形式は読み込み互換性を維持。
- R70: TileCacheのLRU evictionでentry全体をcloneせず、byte長だけを一時保持する。
- R38: 動画import失敗時に作成途中の独立cache directoryをRAIIで削除し、WebP→PNG fallback前にpartial WebPを掃除して、成功時のみ一貫したcacheを保持する。
- R07: Timelineのlayer追加経路は `Composition::next_layer_id` を使い、既存IDとの衝突を避ける。コアの欠番テストあり。
- R09: SSAAの2倍寸法がレンダラー上限を超える場合は、片軸だけをクランプせずSSAA全体を無効化して等倍でレンダーする。
- R41: 画像シーケンス全フレームの寸法をdecode前に検証し、寸法不一致・上限超過・未対応decoder形式を拒否する。
- R40: VideoAsset metadataを実際に抽出される偶数寸法へ合わせ、2px未満の入力を拒否する。
- R45: Autosave失敗をtoast/logへ報告して30秒retry backoffを追加し、周期保存のserialization/disk writeもbackground化。Project snapshot cloneと実応答時間計測は残る。
- R42/R43: 保存・復旧でProjectとProductionDocumentを同一snapshotとして扱い、復旧失敗ではデータを消さず、復旧成功後に音声設定を含むdomain stateを適用する。
- R13: Color Management、Paragraph、Camera/Light Optionsの毎frame Project cloneを、編集入力時だけ作る共通pre-edit snapshotに変更。全画面共通の遅延編集transaction化は残る。
- R61: CPU shader fallbackをコメント部分一致からbuilt-in source全文一致へ変更し、UI template文字列をCPU fallback moduleと共通化。旧marker単独値は互換sentinelとして保持。
- R23: LinuxのCJK fontをLatin primary選択から分離して日本語fallbackへ登録し、WindowsにもYu Gothic/MS Gothic候補を追加。
- R23追記: macOSの`/Library/Fonts`にインストールされたLINE Seed JP、Windowsのsystem/user font directoryにインストールされたNoto Sans CJK/JPも候補へ追加。フォントがないOS環境の保証と実機glyph表示確認は残る。
- R25: PreComp child row snapshotをcomp/session/history generation/expanded set単位でcacheし、表示fingerprintが変わらないhistory editでは全child cloneを回避。表示に関係するメタデータの再走査と、変更時の全件snapshotは残る。
- R71: Cmd+S、File > Save、コマンドパレットをProductionDocument対応の共通保存関数へ集約済み。
- R21: ParagraphをKUIのPropertyGrid、共通数値入力・カラー入力・toggleへ統一。Alignment selectorは利用幅に応じて4列/2列へ切替。
- R75: Unix以外では`statvfs`を呼ばずHomeの容量表示を省く。
- R76: Renderを画面幅でreference mockと実際のexport画面へ切り替えず、responsiveな実export画面へ統一。
- R78: Project Panelのfile picker filterをdecoder/分類可能な画像・OBJ形式に合わせ、画像寸法を実ファイルから取得し、未知形式を拒否。
- R79: `Project::next_composition_id`と`next_project_item_id`を追加し、動的なUI/automation creation経路を共通発番へ集約。
- R80: Homeの設定ナビを固定値だけの装飾ページではなく、保存・適用処理を持つ共通Preferences dialogへ接続。
- R81: 新規動画/音声Assetの長さに10秒の偽値を入れず0を未取得sentinelとして保持し、Project PanelとCC Librariesで未計測を表示。偽の音声sample rate/bit depth/channel表示を削除。実probeは未実装。
- R82: 「Move selection out」の親参照判定をfolder nameではなくfolder IDで行う。
- R83: 未使用素材判定でasset display nameだけでなく、file-backed assetのmedia pathも使用中参照と照合。
- R84: 「未使用素材を削除」後に選択indexをクリアして、削除・並び替え後の誤選択を防ぐ。
- R85: tutorial practice videoのtemp pathをprocess ID namespace化し、既存ファイルのbyte contentを同梱assetと比較してから再利用。
- R92: 字形画像とキャッシュへbyte上限を設定し、未検証font bytesの拒否、同名font置換時のglyph cache無効化を追加。
- R24: TimelineColumnsでtimeline幅を左右列に分解し、property row/PreComp child/headerの幅制約を共通化。狭幅でも時間領域の最低幅を確保する単体テストを追加。全てのtimeline描画rectの統合と実画面確認は未実施。
- R25: History generationが変わってもPreComp child表示fingerprintが同じなら既存snapshotを再利用し、通常のkeyframe/transform編集で全件のString cloneを繰り返さない。子メタデータ変更時の全件走査と可視行だけの取得は残る。
- R93: Audio decoded cache keyにsource file length/mtimeを加え、同一パスの外部差し替えで古いPCMを返さないよう変更。revision差異のcache testを追加。
- R94/R95: Render Queue IDへprocess ID・nanosecond時刻・atomic sequenceを使用し、全u32 frame範囲の計算をu64へ変更。連続IDと境界frame数のunit testを追加。
- R68: Flush後も新しいRenderPipeline要求を処理するgeneration方式に変更し、待機中の古い要求だけをskipするテストを追加。テスト・型検査は未実行。
- R96/R97: OpenFX Property Suiteの関数順・layout assertionsを公式C ABIに合わせ、未完成host suiteへpluginを渡すattachをfail-closed化。偽pass-through成功を削除し、render未対応をエラー表示。テスト9件はlayout assertions追加前に成功。SDK/sample pluginとの実動検証は未実施。
- R98: 自動化script入力を1MiBに制限し、ProductionDocument validation成功後だけProjectへ反映。Expression/automation評価をworkerへ移し、同時実行を防ぎ、編集中または別Projectへ切替後に返った古い結果を破棄する。Project snapshot cloneはまだUI thread上。
- R98追記: Expression/Automation両モードにキャンセル操作を追加し、Rhai progress callbackで中断する。workerへ渡したProjectをそのまま編集して、従来のworker内二重目cloneをなくす。UI thread snapshot cloneとConsoleを離れたときのキャンセル伝達は残る。
- R99: frame binding snapshotでProject全体をcloneせず、対象Compositionだけをcloneし、結果をmoveで返す。関連testsはディスク空き不足で未実行。
- R100: 複数Compositionのbatch exportとFarm manifestで、base pathから各item専用のファイル名を同じhelperで生成。path helperのunit testを追加。
- R102/R122: Collect Filesは動画layerのframe_countを維持してProductionDocument validationを通す。実動画を開き直した際のcacheと音声再生成は未検証。
- R119/R121: Project Panelの動画Assetからのlayer追加、Image Sequence、Replace Footageをsession-aware background workerへ接続。失敗時にfallback Solidを追加しない。遅い素材での応答時間は未測定。
- R120: Image Sequence pickerの拡張子候補をcore decoderの対応形式へ一致。
- R117: 画像・動画importのRGBA換算128MiB寸法上限と、境界値テストを追加。OSの実ピークRAMは未計測。
- R125/R127: OTIO変換をfallibleにし、壊れたKagari動画参照とu32 frame range overflowをclip名付きエラーとして返す。対象unit tests 8件が成功。
- R105: MLT entryのsource in/outとplaylist内のtimeline startを復元し、inclusive outをexclusive境界へ変換。フレーム整数/timecodeを受け、timecodeを最近傍frameへ丸める。不正値・overflowを拒否。
- R104: 各layerを別playlistへ出し、in_frameをblank、重なりをmultitrack/qtblend、音声をmix transitionで表す。MLT in/outを整数frameに統一し、importは整数とtimecode双方を受け付ける。コンパイル確認済み。Shotcut/MLT実行で合成順・音声mixを未検証。
- R140: MLT exportが未対応layer typeを無言で落とさず、layer名・種類を示すerrorでfail-closed。
- R141: OpenShot JSON exportも未対応layer typeを名前・種類付きerrorで拒否。
- R142: 音声resampleは不可能な変換をerror化し誤レートPCMをcacheしない。mix/UIでの失敗理由表示は未解消。
- R143: CLI scriptをmetadata検査後の無制限readから1MiB上限readerへ変更。
- R133: Frame QCはフル解像度renderをbackground workerで行い、Project/Composition/frameが変わった結果を破棄。再監査でUIからのFrameCache readerとpixel sampler自体が現行経路にないと確認し、指摘を削除せず対象consumerなしとして閉じた。
- R116: Automation curveを1回検証した後に二分探索でsampleし、source mappingではcurve点列を走査しない。
- R144–R149: Dynamic Link接続の占有、legacy動画referenceの不正受入れ、Home UI monolith、音声decode失敗時の無音成功、設定pathのOS優先順、非UTF-8 recent pathの不可逆化を追加監査で確認。R144/R145/R146/R147/R148/R149は修正し、legacy page/layout重複はR77に継続記録。R103は標準schema serializer/parser、timeline位置・gap、Kagari metadata復元、未対応mediaのfail-closedを実装。reference implementationでの相互運用性は未確認。
- R44: process ID別autosaveに加えdirty-exit markerもprocessごとに分離し、起動時に既存markerのprocess生存状態を確認する。
- R145: legacy動画referenceの必須field・frame count・source rangeを厳格に検証し、不正値をclip名付きerrorへ変更。
- R148: OS別設定pathへ切替え、旧HOME直下設定をfallbackし、更新時に新pathへ保存する。
- R147: audio source load/resample failureをmeterへ伝播し、previewで一度だけ警告、WAV exportでは失敗として扱う。
- R149: Recent/Starred pathをOS-native bytes/wide unitsから可逆tokenへ変換し、Home/Welcomeで復号してから開く。
- R144: Dynamic Link socket処理をsingle-client workerへ分離し、listenerを応答可能に保つ。占有時は追加clientへ明示エラー、idle timeoutとnonblocking UI通知を設定。
- R124: Dynamic Link handshakeでsemver-shaped versionをparseし、protocol major 1以外を明示拒否。
- R21/R33/R34/R36/R37/R40/R41/R76/R81: 一部の現行実装を再確認したが、R21/R36/R37/R76/R81は本文に古い挙動の記述が残っているため、この進捗行だけでレビュー記録を更新済みとは扱わない。各項目の現状説明を再照合する。R35はsource errorのWAV失敗化まで進めたが、`None`のAPI意味分けと全caller統一は未完了。
- R35: 音声mix結果で有効source数を返し、無音WAVの誤生成をなくした。再生側はno-audio結果をcacheし、条件変更時に前の一時WAVを削除する。
- R150: Recovery snapshotとファイル妥当性確認の全読込を100MiB bounded readerへ統一。R151はSfM風の偽解を返す経路を廃し、未実装errorへ変更。R152は512MiB RAM ceiling/FIFO evictionを追加したが未接続で、弱いhashとLRU不足が残る。R153は偽のAI load/inference成功を未実装errorへ変更。R154は偽mergeとunbounded packet historyを除去。R155はメモリ内timeline storeへ改名しframe-range indexを導入したが、未接続・非永続でrecord上限なし。R156はPreComp内の音声もexport判定へ含めた。
- R155追記: repository内にconsumerがない別storeを削除し、現行Project keyframe + Timeline generation cacheを正本として明記。レビュー指摘は台帳に保持。外部利用者が公開moduleをimportしていた場合の互換性影響は残る。
- R103: OTIOの標準schema serializer/parser、timeline gap、Kagari metadata、非対応referenceの明示拒否を実装。reference implementationとの外部相互運用確認は未実施。
- R142: audio preview mix失敗を一度toastで通知し、同じstateで毎frame再試行するのを防止。
- R157: 3D camera solveのtracking入力不整合・非有限値を拒否し、一意frame数を100,000へ制限。
- R158: Motion Sketch記録を8192点以下へ保ち、同一frame sampleを集約。Douglas-Peuckerを反復化し、ベイク入力を4096点へ制限。UI同期処理時間・間引き品質の確認は残る。
- R159/R160: AEPの偽成功をunsupported errorへ変更。動画decoded cacheは全frame indexと内容fingerprintをmanifestで検査し、旧cacheはdecode検証後にmanifestへ移行する。全lib testで関連テストを含む1,399件が成功し、大規模cacheの検証I/Oは未計測。
- R123: Timelineのderived dataをgeneration cacheし、親名lookupをO(1) index化。ドラッグ中のsnap edge走査と大規模実測は残る。
- R126/R128/R129: 階層cycle validation、unsorted keyframe受入れ、precomp cycle時のtree再検索を修正。ProductionDocument test 29件成功。
- R111: 動画importのFFmpeg stdoutを破棄し、stderrを継続drainしつつ64KiB上限で保持。実素材でのmemory計測は未実施。
- R101: ProductionDocument::loadを上限付きfile reader経由にし、metadata検査後にファイルが増大しても読込量を上限+1 byteに制限。
- R45: 周期autosaveのJSON化・streamed file writeをworker化し、generation追跡で保存中の後続編集をdirtyのまま保持。終了時flush、ProductionDocument内のProject重複保存削減、旧autosave形式の互換テストを追加。UI側Project snapshot cloneは残り、実画面の停止時間は未計測。
- R86: Replace Footageの選択filterをAsset型別に制限し、分類結果の型が一致しない場合を拒否。画像置換では実寸metadataも更新。
- R87: footage置換時は旧path完全一致で対象layerを決め、同名のSolidや別directoryの同名素材を維持。Model3D pathも更新する回帰テストを追加。
- R88: Audio Spectrumのeven-length判定を`usize::is_multiple_of`へ置換。
- R89: headless GPU rendererでtexture上限超過を事前拒否し、readback target寸法とbyte長の計算を検証するよう変更。all-features Clippyで型検査済み。実GPU確認は未実施。
- R90: Time Remap Loop bakeを整数計算と100,000 key上限へ変更し、1frame sourceを1キーへ縮約。no-default-featuresの対象unit test 2件は成功。
- R91: Home設定のatomic write失敗を`Result`で伝え、Preferences/Home/workspace/プロジェクト開閉の各呼び出し側で通知。all-features Clippyで型検査済み。実書込み障害の注入テストは未実施。
- R37: プロジェクトread/parse/relink/欠損video cache準備を専用workerへ移し、読み込み中表示と二重読込防止を追加。処理中のProject変更を検知した場合に読込結果を破棄し、既存状態を保護する。
- R73: Mocap入力を`frame_<number>.png`に限定し、フレーム番号を数値順に並べ、番号重複を拒否する。
- R72: CLI renderでduration/range/dimension/byte budget/exposure/LUT modeをbackend起動前に検証し、PNG/MP4/GIFで不正範囲の成功扱いや余分な1フレーム出力を防ぐ。
- R67: Render Queueとexport statusの識別を表示名からComposition IDへ移し、同名・名称変更による出力先の誤選択を防ぐ。
- R14/R15: history byte estimatesをentry単位で差分更新し、JSON serialization長に基づく可変データ込みの予算proxyへ変更。`commit_action`はcurrentだけをJSON buffer化し、nextはbufferを作らず比較・サイズ計測する。二重のserialization traversal、2倍係数の実測校正、Rust heap実使用量の厳密な上限は残る。
- R74: headless GPU rendererの公開条件を`cli-gpu`へ絞り、通常CLIとGUI+CLIのfeature組合せからpollster参照を切り離す。

これらはコード上の経路を修正した段階。GUIターゲットのcargo checkを複数回通過。CLI通常・headless GPU・GUI+CLI feature組合せのcargo checkも通過。LUTのファイル内容と書込み失敗、音声スペクトラムの短尺入力、ディスクcache/復旧の複数プロセス分離、出力画素の一致、音声同期、復旧UI操作、非同期import切替、巨大Projectの読込応答性、R72/R73の不正入力を実コマンド・実素材で確認していないため、実行時まで検証済みとは扱わない。

- CLI mocapは現在track_markerless_motion_streamへiteratorで画像を渡す。以前の「全decoded RGBAを保持」は現在当たらない。path一覧と出力track samplesの保持は残るが、別の話。
- `project_io::save_project_to_path` はProductionDocument::save_atomicを使い、Cmd+S/File > Save/コマンドパレットもこの共通入口を呼ぶ（R71修正済み）。Collect Filesの経路は別問題としてR102に記録。
- Dynamic Linkは127.0.0.1へbindし、受信行を64KiB、read timeoutを10秒に制限している。外部ネットワークから直接接続できるという指摘は確認できなかった。固定port競合とクライアントを直列処理する設計は別途残る。
- FFmpeg出力にはTemporaryOutputGuard/ChildGuardがあり、失敗途中の動画や子processを管理している。PNG/image-sequence importと同等と推測しない。
- graph compilerのHashSet順は現状の可視化の問題。実合成がこのscheduleを使う証拠はない。
- GPU画像cacheは512MiB byte budgetとLRU evictionへ変更済み。device adapter別のメモリ予算反映と、実GPUでのVRAM計測は未実施。
- 第2回はR29〜R70の42件を追加した。P1/P2/P3および未接続APIを含み、すべてが同じ深刻度・再現確度という意味ではない。
- 第3回では、プロジェクト保存入口、スキーマ判定、ショートカットの呼び出し元を照合し、R71を追加した。これで実行中の保存経路の不一致もレビュー範囲へ含めた。
- 第9回は動画/シーケンスimportとRAM/disk frame cacheを再監査し、各import経路をsession-aware worker化、欠番・重複frame番号の誤圧縮を拒否、旧世代frame cacheのdeep cloneを除去、cache budget対象を永続frame fileに限定して古いtemp fileを掃除するよう変更。Dynamic Linkの未対応要求はerror応答(R113)へ変更。Frame QCのcache readerとpixel sampler writer不足をR133として記録したが、後続監査では両consumerが現行UIにないことを確認し、R133へその事実を追記した。import応答時間と実機フレームcache連携の確認は残る。
- R51: PreComp cache hitの全画素cloneとComposition全体JSON化を除去し、共有Arcと編集世代・外部media revisionによるinvalidationへ変更。revision世代のunit test成功。
- R54: 既存disk frame置換時の容量二重計上を修正し、process内mutexとdirectory単位のOS file lockでKagari writerを直列化。別processとのlock競合testを含むdisk-cache test 4件とstrict no-default Clippyが成功。
- R102: OBJ→MTL→texture依存を収集しbundle内参照へ書換え。既存asset/videoを含む対象test 3件とstrict no-default Clippyが成功。実素材検証は未実施。
- R158: Motion Sketch bakeをbackground worker化し、Project session/history generation/Composition/Layerを確認後にundoable editとして適用。GUI cargo check、対象test、strict GUI lib Clippy、および現行変更を含むno-default lib test 1,240件が成功。最大入力の体感計測は未実施。
- R146: Home rootを約1,860行にし、各画面群を責務別moduleへ移動。GUI test targetを含むcargo checkとstrict GUI lib Clippyが成功。legacyページ重複はR77に保持。
- 第4回では新規KUI、レイヤーコマンド、共有FlowGraphモデル、headless GPU readback、Time Remap Loop、Home設定を確認しR89〜R91を追加・実装。第5回はTimeline geometry、PreComp child cache、font rasterizer、audio decoded cache、Render QueueからR92〜R95を追加・修正。第6回はRenderPipeline、OpenFX、Automation console、ProductionDocument frame snapshotからR96〜R99を追加。第7回は既存指摘の現行コードを再照合してR07/R09/R29〜R32の修正を反映し、Render Queue batchの出力先衝突をR100として追加・修正した。
- 第7回の変更後は `cargo check --all-features --tests` が成功。Clippyはmetadataを書き出す時点でディスク満杯となり完了していない。全テストと実FFmpeg batch出力確認も未実施。
- R90対象2件、font validation、audio source revisionのno-default unit testは成功。Timeline geometryの独立unit test 2件も成功。第6回はR99変更後を含むall-features `cargo check --tests`とClippyが成功。OpenFX 9件、自動化11件、RenderPipeline 1件のno-default lib testが成功。OpenFX layout offset assertions追加後のtest、ProductionDocument snapshot tests、R94/R95 tests、all-features全testはディスク不足で未実施。将来互換通知（`block 0.1.6`）はClippy warningとは別。
- R14 streaming comparison後にhistory/editorのcommit系unit test 8件が`--no-default-features --lib`で成功。serialized JSONの追加bufferは削減したが、serialization traversalの性能計測とchange-flag APIへの移行は未完了。
- この文書は広範なレビュー成果物だが、全ソースファイルを先頭から末尾まで読了したという意味ではない。実行時再現が必要な項目は指摘確度を明記し、未接続APIは製品コードの既知不具合と分けた。網羅性を装うためだけの件数追加は行わない。

## 保留中の追加指摘（初回範囲の完了後に扱う）

以下は初回レビュー範囲を完了する前に私が追加してしまった項目。記録は保持し、初回範囲R01〜R161の未完了分を先に扱う。

### R162 [P2] 大きなWGSLを開いたままにすると毎frame再パースする

`src/ui/effects_controls/custom_shader.rs` は描画関数を通るたびに `validate_wgsl` を呼ぶ。`src/core/custom_shader_runtime.rs` の結果cacheはentry総量2MiBで、上限超過のsourceはcacheへ入れず、そのままparserとvalidatorを毎回実行する。一方、UIのLoad WGSL pickerは最大4MiBを受理する。したがって2〜4MiB程度のshaderを読み込んでEffects Controlsを表示すると、eguiの再描画ごとに複製・ラップ・WGSL parse/validationが繰り返され、入力応答やCPU使用率を悪化させ得る。長さ上限とcache上限の契約を統一するか、内容revision単位の検証結果をUI stateへ保持する必要がある。大きなshaderでの実測は未実施。

### R163 [P2] Automationはモデル上限を超える作成操作をscript終了まで許す

`src/core/automation.rs::run_script` はProject全体をcloneし、Rhaiの操作数を200万に制限してからscriptを実行し、最後にだけ `ProductionDocument::validate` を呼ぶ。`new_comp`、`add_solid`、`add_text` の登録関数には、作成前の `MAX_COMPOSITIONS` / `MAX_LAYERS_PER_COMPOSITION` による拒否がない。永続化時には拒否されるscriptでも、上限を超える一時Composition/Layerをworker上で大量に確保してから失敗する。UIの1MiB source limitとRhai operation limitは個々のmodel collectionの上限にはならない。mutation APIで追加前に上限を強制し、明示的なscript errorを返すべき。大規模入力のピークRAMは未測定。

### R164 [P2] GPU computeはGUI rendererと別deviceを作り、各effect後にCPU readbackで同期する

`src/core/renderer.rs` のGUI rendererはeframeから受け取ったdevice/queueを共有する。一方、`src/core/compute_pipeline.rs::GpuComputeContext::new` は独自Instanceからadapter/device/queueをもう一組要求し、CPU pixel bufferをGPUへ送るcompute経路でdispatch後に `Maintain::Wait`、staging bufferのmap、全pixelのCPUコピーを行う。`src/core/cpu_effects.rs` はeffect単位でこの経路を呼ぶため、複数GPU effectを積んだ場合はGPU↔CPU往復と同期がeffectごとに入り、GUI previewの主レンダーともdevice資源が分離する。GPU演算自体がCPU fallbackより速くても、同期・転送を含む全体が速いとは限らない。opt-inで既定無効という緩和はあるが、現状を「GUIとGPU effectが一つのGPU pipelineを共有」とは扱えない。GUI device再利用、複数effectをまとめたdispatch、readback回数、実機のend-to-end計測を検討したい。

### R165 [P3・R18へ統合／strict all-target Clippy再通過] all-target strict Clippyが警告で失敗する

この指摘は初回範囲R18のlint検証阻害を重複して記録したもの。全target strict Clippyを再実行してtest codeの`field_reassign_with_default` 34件を検出し、struct initializerへ直した。2026-10-04には`chunks_exact_to_as_chunks`のlib/test target移行後に`cargo clippy --all-features --all-targets -- -D warnings`を再実行し成功した。履歴項目は削除せずR18へ統合した状態を保持する。

### R161 [P2・node全走査を除去／実測なし]

`VfxGraphCompiler::compile`は各scheduled nodeの依存一覧を作る際、全layerを走査して各adjacency listを検索していた。cycle fallbackも未schedule nodeの確認に`sorted_order.contains`を使い、node数二乗になり得た。現在は隣接配列と逆依存indexを一度構築し、cycle fallbackにはscheduled bitmapを使う。辺のsort `O(E log E)`とgraph compile時間・メモリの実測は残る。`vfx_graph_compiler`の既存topological-order/dead-code-elimination testsを含むall-features lib test 1,408件とstrict all-target Clippyは成功済み。大規模graphの性能測定は行っていない。


# 2026-10-04 現行台帳からの整理記録

以下は完了項目アーカイブの元スナップショットに対する、今回作業時点の状態・追記を含む保存版です。

### R13 [P2・修正済み／実測未実施] パネルを眺めるだけでプロジェクト全体をcloneする

`color_management.rs:6`、`paragraph_panel.rs:7`、`camera_light_options.rs:52` は描画冒頭で履歴の現在値を複製する。変更のないフレームでもコストが発生する。Paragraphは選択対象がない場合でも先にcloneする。編集開始時にスナップショットを取る共通トランザクションへ寄せるべき。

color management、paragraph、camera light、Effects Controls、Character tab、Camera DoF、Layer Styles、Mask、Viewer inline numeric editorは共通helper経由で、実際の対象UIに入力がある場合だけundo用Project snapshotを捕捉する。inline editorはcanvasではなくfloating editor rectを対象にする。Character tabの毎frame Project cloneはactive Compositionだけのworking copyへ縮小。Undo履歴は過去状態をserialized snapshotで保持し、mutable history accessのたびにentry推定サイズを再計算する経路も除去した。残る`history.current().clone()`を全UIから再検索し、pointer/keyboard action、明示的な保存・export・import/replace・編集確定・async結果反映・履歴snapshot処理に限られることを確認した。観覧だけで発生する全Project cloneはコード上解消。大規模Projectの応答時間とピークRAMの実測は未実施。

### R33 [P2・修正済み／非整数fps実音声同期未測定] 音声書き出しのサンプル数が毎フレーム丸められる

絶対frame境界からfloor(sample_rate * frame / fps)を算出し、フレームごとのブロック長を可変にした。既存のframe/fpsによるソース時刻対応を維持しつつ、出力サンプル数の累積丸め誤差をなくした。非整数fpsの実音声同期は未測定。

### R37 [P2・修正済み／大規模Project応答時間未計測] プロジェクトを開く処理に同期の動画全デコードがある

read/parse/relink/cache refreshをbackground workerで実行し、開始session/history generationが変化していれば結果を適用しない。読込中も既存Projectを維持し、完了・失敗をUIへ返す。大規模Projectでの応答時間は未計測。

### R38 [P2・修正済み／実素材での失敗経路未確認] 動画import失敗がpartial cacheを残す

`video_import::import_video` はimport_dirを作成した後、FFmpeg失敗等でErrを返してもディレクトリを掃除しない。WebP途中失敗→PNG fallbackではpartial WebPを残す。frame_path_in_dirはWebP優先なので混在したキャッシュを読む。失敗理由がencoder不在か容量不足かを区別せず、常にもう一度全尺抽出する点も問題。成功時だけcacheを公開するstagingが必要。

現在は出力directoryをRAII cleanup guardで管理し、frame decode・enumerationなど途中で失敗した場合はcacheを削除する。WebP抽出失敗後はpartial WebPを削除してからPNG fallbackを行う。VideoAssetを返す成功経路のみ保持。FFmpeg/audioの失敗時と複数import同時実行の実測は未確認。

### R40 [P2・修正済み／FFmpeg実抽出未確認] 奇数寸法の動画でasset metadataと抽出画像の寸法が違う

importは `scale=trunc(iw/2)*2:trunc(ih/2)*2` で偶数へ縮小するが、VideoAsset width/heightにはprobeした元寸法を返す。drop側のfit計算はそのmetadataを使う。元画像を維持するか、実抽出寸法を戻す必要がある。小さい奇数画像では0寸法へ切り下げる問題もある。

VideoAssetは抽出後の偶数寸法を報告し、2px未満の入力を事前拒否する。FFmpegでの実抽出結果との一致は実行確認していない。

### R41 [P2・修正済み／実素材一括読込未確認] 画像シーケンスは最初の寸法だけ確認する

`import_image_sequence` は先頭画像にだけ寸法上限を適用し、後続は直接image::openして保存する。後続の寸法不一致や過大画像を検証しない。VideoAssetは全フレーム同じ寸法として記録する。またbmp/tgaを許可しているが、Cargo.tomlのimage decoder featureはjpeg/png/webpのみ。この2形式は対応表示と実decodeが一致しない。

各フレームをdecodeする前に寸法を確認し、先頭画像と一致しない素材・上限超過を拒否する。decoder featureのないbmp/tgaは受付候補から除外。部分出力はR38と共通のRAII guardで削除する。

### R44 [P1・修正済み／OS別生存判定は未実行] 複数アプリが同じ復旧slotを上書き・削除する

各KagariAppの保存stemとdirty-exit markerをprocess IDで分離する。復旧候補は全namespaceから新しい順に検索し、復元・破棄は選んだsnapshotのnamespaceだけを削除する。起動時はOSのprocess livenessを確認し、稼働中の別instance markerをcrash扱いせず、停止済みprocess markerだけを回収する。複数instanceの同時起動・片方正常終了・片方強制終了という実機シナリオは未実施。

### R46 [P2・修正済み／表示用downscale未実装] 予算を超える有効な静止画がキャッシュ内で自分を追い出す

ImageCacheはdecode→insert→evict→同じkeyをgetする。既定cache予算を超える画像はdecode前にサイズを判定して明示的に拒否し、挿入直後に自身を追い出して無言で消える状態を防ぐ。大画像の表示用downscale経路は未実装。

### R49 [P2・修正済み／実GPU確認未実施] GPUで同じパスの画像更新を検知しない

GPU video/still texture cache keyへsource file path・length・modified timestamp由来のrevisionを追加。外部更新後はversion bumpなしでも新entryとなり、古いtextureを使い続けない。実GPUでの置換再描画確認は未実施。

### R50 [P2・修正済み／実フレーム確認未実施] PreComp cache revisionに外部mediaの更新が入らない

PreComp cache keyへ再帰的に辿れる画像・動画フレームのfile revisionを加え、同じProject JSONでもnested source更新でcache missになるよう変更。深いnested/speed変更を含む実フレーム確認は未実施。

### R54 [P2・修正済み／非協調writerは対象外] フレームのディスク予算は単体超過を拒否しない

frame_cache::disk_cache::write_frameは単一frameがbudgetを超えた場合に拒否し、evict後も実disk usageを再確認する。置換先の既存fileをeviction候補から除外し、予測容量から旧file分を差し引く。process内はmutex、同じcache directoryを使うKagariの複数process間はOS file lockで予算確認からatomic writeまで直列化する。別processが同じdirectoryへ非協調の直接書込みを行う場合は調停対象外。別processが同じbudget lockを待つ回帰testを含むdisk-cache test 4件とstrict no-default Clippyが成功。

### R60 [P2・修正済み／LRUではない] シェーダー編集内容を無制限cacheへ溜める

global validation cacheへ128 entry・合計2MiBの上限を設定し、超過時に全消去する。個別source＋診断が上限を超える場合はcacheしない。LRUではないが保持量を有限にした。

### R61 [P2・修正済み／legacy sentinel依存は残る] CPU shader対応判定はコメント文字列に依存する

custom_shader_cpu::builtin_shaderは `source.contains("CRT Scanlines Shader")` 等で固定kernelを選んでいた。ユーザーがテンプレートの処理を変更してコメントを残すと、CPU/exportは変更したWGSLではなく元kernelを実行する。現在はUIのbuilt-in全文テンプレートと完全一致する場合だけ該当kernelを使う。既存project/test互換のmarker単独文字列も完全一致のlegacy sentinelとして維持するため、将来的な明示identity fieldがより堅牢。

### R62 [P2・修正済み／実音源評価未実施] Audio Spectrumは周波数分析ではなく時間区間のRMS

Radix-2 FFTへ変更し、Hann window、stereo interleaved downmix、sample_rateとstart/end frequencyを使った対数band mappingを実装。band出力は各frequency binの振幅power平均の平方根を正規化する。実音源への音響評価は未実施。

### R63 [P2・修正済み／異常入力実測未実施] Spectrumの短い入力でslice範囲が逆転する

以前の同関数はchunk_sizeを最低1にする一方、bands > samples.len()でも全bandを回し、len2/bands64なら逆転sliceでpanicし得た。現在はFFT binをfrequency範囲ごとに安全に集計し、band数がサンプル数を超えても入力slice境界を生成しない。短尺・異常サンプルの実行確認は未実施。

### R65 [P2・修正済み／実GPU確認未実施] 独立previewの再生は遅延時にフレームを取り戻さない

about_to_waitは期限超過ならframeを1だけ進めてlast_tick=nowへしていた。現在は再生開始時刻・開始frame・fpsから実時間上の現在frameを算出し、遅れた分を取り戻す。Lost/Outdated surfaceは再configure、Timeoutは次回更新に任せ、OutOfMemoryは終了する。実GPUでの長時間再生・surface復帰は未確認。

### R66 [P2・修正済み／公式OTIO互換性は未確認] 動画交換形式のパスをコロン区切りにする

integration::OtioTimelineはvideo source、frames_dir、frame_count、audio_wavを `video:{}:{}:{}:{}` へ入れ、splitn(5, ':')で復元していた。現在はVideoMediaReferenceをJSON化した `video-json:` 形式で新規出力し、旧区切り形式も読み込む。Kagari内の新形式roundtripを安全にする変更で、公式OTIO互換性は未確認。

### R69 [P2・修正済み／未接続API]

with_directoryがcache directoryを走査し、妥当なsidecar metadataとframe fileの組を起動時inventoryへ登録する。破損・孤立fileと残留temp fileを除き、既存frame総量がbudgetを超えていればLRUで削減する。未読込frameも含めた容量管理とComposition単位のinvalidateにinventoryを使う。再起動後のinventory・budget・cold invalidateを含む専用10件のunit testが成功。現行製品呼出しは見つからず、FrameCache内の別disk_cacheとは未統合。last_access_epochの再起動をまたぐ永続更新とmetadata自身の容量は対象外。

### R70 [P3・修正済み／未接続API]

tile_cache::insertのLRU選択が `(key, entry.clone())` を作り、サイズを引いて削除していた。現在は `(key, pixels.len())` のみ取得してevictし、捨てるtile payloadの複製を避ける。現行利用は同ファイルのテストのみ確認。

### R114 [P2・R144へ統合] Dynamic Linkの単一listenerが一接続で後続接続を塞ぐ

この指摘はR144へ統合。現在はsingle-client workerでsocketを処理し、listenerは次のacceptを継続する。接続中の追加clientには明示的Errorを返し、read timeoutでidle clientを解放する。

### R124 [P3・修正済み／外部client未実接続] Dynamic Link handshakeがprotocol versionを検証せず互換性を誤認する

handshakeは`major.minor.patch`形式とmajor version 1を確認し、形式不正・非互換majorはError応答にする。TCP経路はError後に接続を閉じる。外部clientとの実接続検証は未実施。

### R135 [P1・修正済み／FFmpeg実出力は未確認] FFmpegの一時出力名からコンテナ拡張子が消える

一時出力名を`<stem>.kagari-tmp-<pid>-<seq>.<ext>`に変更し、GIFを含むFFmpegが元のコンテナ拡張子を判定できるようにした。CLI featureの型検査は通過。各コンテナの実FFmpeg出力は未確認。

### R136 [P2・修正済み／巨大range未実行] Multi-frame renderingがフレーム番号全件を先に確保し、最大範囲で件数がoverflowする

MFRはu64のRayon rangeからframeを直接処理し、全frame番号のVecを作らない。queueの総frame数、完了数、progress callbackもu64化してfull-u32-range時の件数overflowを避ける。CLI featureの型検査は通過。最大rangeの実行とcancel応答は未確認。

### R139 [P2・修正済み／外部player未確認] Lottie Project exportが未参照Compositionまで検証・asset収集する

rootからPreComp参照を辿って到達したCompositionだけをcycle-safeなvisited setで集め、その範囲のみlayer検証・画像asset収集・PreComp asset生成を行う。CLI featureの型検査は通過。未参照のunsupported compを含むProjectでの回帰実行と外部Lottie player確認は未実施。

### R140 [P1・修正済み／外部MLT実ファイル未確認] MLT書出しが未対応レイヤーを無言で落とす

`MltExporter::export_to_xml`はText/Shape/PreComp/Particle/Adjustment/3Dなどを含む場合、layer名と種類を示してexportを拒否する。Nullは描画内容がないため無視する。UIはXMLを保存する前にエラーを通知する。Shotcut/Kdenliveの実アプリ動作は未確認。

### R141 [P1・修正済み／OpenShot実アプリ未確認] OpenShot書出しも未対応レイヤーを無言で落とす

`OpenShotExporter::export_to_json`はImage/Video/AudioだけをJSONへ追加し、Solid/Text/Shape/PreComp等は無視して成功を返す。exporterは`Result`を返すため、未対応layerを名前・種類付きで拒否するよう変更。外部OpenShotでの読込確認は未実施。

### R142 [P1・修正済み／実ファイル再生未確認] 再サンプリング上限超過時に元レートのPCMを成功値として返す

`AudioBuffer::resample`は出力が512MiBを超える場合にerrorを返し、失敗バッファはcacheしない。preview mix errorは一度toastで表示し、同じstate hashで毎frame再mix・再通知する状態も避ける。Export mixもsource errorを`Result`で返す。実ファイルでの再生確認は未実施。

### R145 [P1・修正済み／legacy互換ファイル未実検証] 壊れたlegacy動画referenceを空pathの動画として受理する

対象：`src/core/integration.rs::OtioTimeline::to_composition`

legacy形式`video:<source>:<frames_dir>:<frame_count>:<audio_wav>`のparseで、必要fieldが欠けると空文字・0へ`unwrap_or_default()`する。例えば`video:foo`でもVideo layerを作り、失敗を返さない。`frame_count = 0`や存在しないframes directoryを持つlayerが正常なimport結果としてProjectへ入る。新形式`video-json:`はJSON parse errorを伝えるのに、legacy経路だけ破損を成功扱いする非対称もある。

legacy payloadを厳密にparseし、必須field不足、空source/cache path、数値でない/0 frame count、source rangeがframe countを超える場合をclip名付きで拒否する。旧形式の曖昧なpath内colonは新JSON referenceへ移行する必要があり、古いWindows pathを含む実Projectでの互換確認は残る。

### R147 [P1・修正済み／実音源エラー試験未実施] 音声ファイルの読込・再サンプル失敗を無音として書き出す

対象：`src/core/audio_engine.rs::load_cached_audio`、`mix_audio_sources_for_frame_with_state`

path validation、metadata取得、decoder、resamplerのどこで失敗しても`load_cached_audio`が`None`を返していた。現在はsource errorを`AudioFrameMeter`へ伝え、プレビューは素材名/path/reasonをtoastで一度通知しつつ継続、PreCompを含むWAV mix/exportは一時WAVを削除して失敗を返す。欠損・未対応・壊れた音源や512MiB制限超過を、正常な無音出力として成功扱いしない。実音源の破損・上限超過を使った実行確認は未実施。

改善案：音声mix APIを`Result`または成功bufferと診断一覧の組にし、layer名/path/reasonをexport UIとログへ返す。リアルタイムpreviewでは全mixを止めない設計も可能だが、状態表示・rate-limited warning等で無音化を利用者に知らせる。exportでは音源を失った状態を成功扱いしない。意図的なmuteやactive range外の無音と、読み込みエラーを区別する。

### R148 [P2・修正済み／OS実機確認未実施] 設定ファイルの場所がOS標準の設定ディレクトリ優先順に従わない

対象：`src/ui/project_io.rs::prefs_path`

設定場所は`HOME`があれば常に`$HOME/.kagari_prefs.json`を返し、その後に`XDG_CONFIG_HOME`、`APPDATA`を確認する。Linuxでは通常`XDG_CONFIG_HOME`（未設定なら`~/.config`）を使う構成が望ましいが無視される。WindowsでもHOME互換環境変数が設定されていると、Roaming AppDataよりhome直下へ保存する。既存設定の移行・同期・backup対象の期待とずれ、環境によって設定が見つからないように見える。

設定場所をmacOS Application Support、Windows AppData、Linux XDG Config Home（未指定時は`~/.config`）へ切替えた。旧`$HOME/.kagari_prefs.json`もfallback読込し、次回設定更新時に既存keyを保ったまま新pathへ書く。各OS実機での移行確認は未実施。

### R149 [P2・修正済み／非UTF-8実path未実行] 最近使ったファイルの非UTF-8 pathをlossy変換して復元不能にする

対象：`src/ui/project_io.rs::path_key`

`PathBuf`を`to_string_lossy()`でUTF-8文字列化して最近使ったProjectやstarred pathを保存する。Unixでは非UTF-8 file nameが合法であり、lossy replacement後の文字列は元のpathへ戻らない。該当pathで開いたProjectはRecentから再度開けず、異なる非UTF-8 pathが同じlossy文字列に正規化されれば重複排除も誤る。Windowsのwide pathも無効なsurrogateを含む場合に同様の往復保証がない。

非UTF-8 pathをUnix raw bytesまたはWindows UTF-16 code unitsのhex tokenとして可逆保存し、Recent/Starredを開く直前にOS-native `PathBuf`へ復号する。既存のUTF-8文字列pathはそのまま読める。Kagariが作ったtokenで始まる通常pathはraw encodingし、prefix衝突も避ける。実在する非UTF-8名のfileをUIから開く確認は未実施。

### R160 [P1・修正済み、cache再検証I/Oは未計測] relink後の動画cache検証が中間frame欠損を見逃す

directory走査で期待する全indexの存在、重複形式、空ファイルを確認する。さらにversion/count/dimension/総byte数と全frameのcontent fingerprintを持つmanifestをatomic writeし、再利用時に全内容をstreaming fingerprintで照合する。manifestのない旧cacheは各frameのdimension上限を先に検査してから実decodeし、妥当ならmanifestへ移行する。欠損・破損cacheは再利用しない。異常なファイル内容の検出には非暗号学的FNV-1aを使うため、悪意ある改ざん対策ではない。大規模cacheの全読込I/O時間と実プロジェクトでの再生成は未計測。


# 2026-10-04 解決項目追記

### R163 [P2・作成時に上限確認を追加]

`new_comp`、`add_solid`、`add_text`は追加前に`ProductionDocument`のcomposition/layer上限を検査し、境界に達していれば明示的なRhai runtime errorを返す。上限判定の境界テストとautomation test 12件が成功。scriptの一時Composition/Layerがモデル上限を超えて確保されてから失敗する経路は解消した。Project snapshot自体のcloneと最大有効サイズでのRAM使用量は別途未計測。


# 2026-10-04 解決項目追記

### R162 [P2・読込上限内のsourceをvalidation cacheへ保持]

validation cacheの上限を総量16MiBにし、UIが受け入れる最大4MiBのWGSL sourceと検証結果を通常のcache entryとして保持可能にした。3MiBの有効sourceを検証後にcacheへ保持するunit testが成功。cache eviction後や複数の巨大shaderを切り替える場合の実時間負荷は未測定。


# 2026-10-04 解決項目追記（レビュー台帳整理）

### R51 [P2・修正済み／実負荷計測なし] PreCompの512MB budgetはスレッドごと、hitも全画素clone

キャッシュをthread-localからプロセス共通のMutex保護cacheへ変更し、512MiBのcache予算をworker間で共有する。Mutexはlookup/insert中だけ保持し、render中は保持しない。画素は`Arc<[u8]>`で共有し、cache hit時の全画素cloneをなくした。通常の合成側がeffectsを適用する場合だけ、変更可能なbufferへcloneする。各lookupでComposition全体をJSON化していたrevision計算は除去し、history/render世代とpreview状態を基準にしつつ、描画対象の外部media metadataを再帰確認する方式へ変更。動画proxyの更新も判定へ含める。revisionが編集世代に依存するため、通常のproject編集経路は既存契約どおり変更時にframe cache versionを進める必要がある。cache eviction後も呼び出し側がArcを保持する間の一時的な追加メモリと、実レンダリング負荷での計測は残る。

### R116 [P2・構造修正済み／性能測定未実施] Automation評価がフレームごとに各curveを複数回全走査する

`AutomationBinding::evaluate`はbindingとcurveを一度検証し、その後curveのstrict-order invariantを使って二分探索で補間点を探す。`AutomationCurve::sample`も公開API上の検証を一度だけ行う。`map_value`はcurve point列に依存しない入力であるためcurve全体を検証しない。大規模automationの性能測定は未実施。

### R144 [P2・修正済み／複数client対応は対象外] Dynamic Linkの単一接続処理がlistener全体を塞ぐ

対象：`src/core/integration.rs::start_sync_server`

接続ごとのworkerへ処理を移し、listenerはacceptし続ける。仕様どおり同時clientは1つに制限し、占有中の追加接続には制限理由をエラー応答する。idle read timeoutでactive slotを解放し、UI通知channelは`try_send`としてserver workerの無期限blockを避ける。複数clientを同時処理する機能は実装していない。loopback以外での公開や実client複数接続試験は未実施。

### R165 [P3・R18へ統合／strict all-target Clippy再通過] all-target strict Clippyが警告で失敗する

この指摘は初回範囲R18のlint検証阻害を重複して記録したもの。全target strict Clippyを再実行してtest codeの`field_reassign_with_default` 34件を検出し、struct initializerへ直した。2026-10-04には`chunks_exact_to_as_chunks`のlib/test target移行後に`cargo clippy --all-features --all-targets -- -D warnings`を再実行し成功した。履歴項目は削除せずR18へ統合した状態を保持する。


# 2026-10-04 対応確認追記

### R19 [P2・共通controls移行済み／GUI visual QAは未実施] KUIの採用がEffect ControlsとInspectorで止まっている

初回レビュー時のraw部品出現検索では type_specs=117、distort=70、transforms=47、stylize=33、generate=32、layer_styles=31。これは `heading/group/checkbox/small_button/Slider/ComboBox/DragValue` の該当行数で、品質スコアではない。Transform inspectorからは直接の標準controlをなくし、Label Colorも共通swatchへ移した。Corner PinのXYは幅を共有しないDragValue横並びだったが、後にKUI vectorへ移行した。Material Optionsはlabel+Slider横並びだった。TransformのAnchor/Position/Scale、3D Position/Rotation/Scale、Auto-Orient/Pin選択、scalar transform、3D material、geometry、drop shadow/stroke controlsをKUIのresponsive vector・PropertyGrid部品へ移行した。範囲なし数値ベクトルとstep指定sliderもKUIに追加した。DistortのMeshWarp/CornerPin/RGBSplit座標、Expression ControlsのPoint/Point 3Dもresponsive vectorへ移行。Sharpen/Edgeの7項目、Stylizeの数値・色入力、Generateの32個のslider、Transition/Halftone/Keying/Blur/Color Correction/Audio/Path Shape/Expression Controls/DistortのsliderをKUIへ移し、Expression Controlsの数値・angle・toggle・dropdown、Blurの整数操作とAudioのEchoes整数操作も共通部品に統一した。Distort/Expression Controlsの座標入力、Color Correction/Generate/Keyingの色入力、Channel Combiner/Set Matte/Echo Operator/Color Space Convertの選択部品もKUIへ移行した。KUIにRGB専用色pickerを追加し、alpha無しの色fieldで従来どおりalphaを編集対象にしない。現在、`src/ui/effects_controls/`内に直接使うraw `egui::Slider`、`ComboBox`、`DragValue`、`Checkbox`、`Button`、`color_edit_button_*`は0件。Transform inspectorは直接の`egui::Slider`/`ComboBox`/`DragValue`/`Checkbox`/`Button`をなくし、色ラベルも共通swatchへ移した。Layer type specsではSolid/Video/Text/Shapeの基本color・property、Text AnimatorとAnimator Stack、Trim Paths、Vector Path Operatorsを共通化し、表示中の変更がundo stateへ反映するようchanged flagsも設定した。Layer Styles、Shape Repeater/Zig Zag/Round Corners、Puppet mesh・pin位置、Trim Paths、Particle Emitterの基本・Force・Trail・Depth項目もPropertyGrid/vector/toggle/buttonへ移した。Inspector内の直接controlは`egui::TextEdit::multiline`によるText layer editorだけで、複数行編集に適した専用KUI editorを作るまでは標準部品を維持する。Camera Inspectorをproperty grid化し、Expression selector用の選択中表示つきcomboと、描画内容を持てるKUI combo popupを追加した。Inspectorおよび`effects_controls`の直接`Slider`/`ComboBox`/`DragValue`/`Checkbox`/`Button`使用は移行済み。画面実測による視覚QAは未実施。

現行ソースを再確認し、Effect Controls配下ではKUI外の標準Slider/ComboBox/DragValue/Checkbox/Button/color editorが0件であることを確認。Custom Shader画面に残っていた6つの小ボタンもKUIへ移行し、GUI feature library checkが成功した。独自のTextEditと選択用rowは目的に応じた部品として残す。実画面の視覚QAと支援技術確認は未実施。


# 2026-10-04 対応確認追記

### R20 [P2・共通section/field/menu/row APIを追加し一部画面へ適用／全widget QAは残る] KUIも「eguiの色替え」に留まる部分がある

`kui::section`を固定高のsection headerへ変更し、アクセントマーカー・見出し文字・境界線を共通geometryで描くようにした。KUIにcustom menu item/menu button、可変内容のcombo popup、選択状態を保つ全幅selectable row、eguiの永続開閉状態を使う`collapsing_section`を加え、hover色・余白・文言解決を一元化した。CC LibrariesのColor Swatches/Character Styles/Project Assetsへ共通折りたたみ見出しを適用。PropertyGridは実測ラベル幅と狭幅時の縦stackを提供する。Color Management、Audio Mixer、Automation Panel、Character、Camera Views、CC Librariesを再確認し、直接の`Slider`/`ComboBox`/`DragValue`/`Checkbox`/`Button`/selectable controlsは0件になった。Audio Mixerのpan sliderはKUI field、音声帯選択は共通selected tool buttonへ移し、fader・meterは専用描画を維持。Character panelのAnimator Stackに残っていた位置・回転・不透明度・blur・tracking・skew等も共有PropertyGrid/vector/slider/numberへ移行した。Workspace Managerのbuilt-in presetは全幅selectable rowと共通翻訳へ移行し、section headerとproperty rowの既存共通部品も共通翻訳を使う。標準Checkbox/Button/ComboBoxを下層interactionに使う部分は適切だが、header右側actions、enum型ごとの表示規約、全screenの部品移行と視覚QAは残る。R20は部分対応。

現行ソースを再走査し、Color Management、Audio Mixer、Automation Panel、Character、Camera Views、CC Librariesの対象箇所で標準Slider/ComboBox/DragValue/Checkbox/Button/selectable controlsの直接利用が0件であることを確認。KUIのPropertyGrid/field/button/menu/selectable APIを使い、meter・fader・camera layout preview等の専用visualは個別描画として維持されている。画面を実起動した視覚QAは未実施のため、間隔・密度・一貫性の仕上がり保証はしない。


# 2026-10-04 完了項目をアクティブ台帳から移動

以下は指摘された実装不備への修正が入ったため、アクティブなレビュー本文から移した。元の指摘と過去の詳細はこのアーカイブ内の各項目に保持する。ここに記した未検証事項は完了した実装の外部確認・性能測定に関する制約であり、未実装部分を完了と主張するものではない。

### R39 — 動画import容量ガードとキャンセル

容量推定関数の単体テストを根拠に、容量予算、空き容量監視、FFmpeg子プロセス停止、UIキャンセルを実装済みとしてクローズ。異なるOS、実素材、容量逼迫、キャンセルcleanupの実動作は未確認。

### R48 — GPU動画texture cacheのbyte budgetとLRU

512MiB byte budgetとアクセスstampによるLRU evictionへ変更し、単独でbudget超過するtextureを保持しない。指摘された枚数固定は解消。adapter固有VRAM照会は別の最適化課題として未対応。

### R96 — OpenFX probe ABI

公式struct order/typeに合わせ、property/plugin structのoffset・size unit testを通過。probe ABI不整合の指摘は解消。host/image-effect suiteが未実装のため、OFX plugin attach/render機能そのものは未対応で、実SDK pluginとの適合確認もしていない。

### R102 — Collect Filesの依存収集

画像・動画・音声・3D素材とOBJ/MTL参照textureをbundleへ収集する処理を追加し、通常素材・動画bundle・OBJ依存のregression tests 3件を確認。元の「Projectだけを書き出して依存を収集しない」問題は解消。実動画再decode、音声復元、外部OBJ/MTL変種は未確認。

### R104 — MLT exportのplaylist配置

layerごとのplaylist、timeline blank、frame範囲変換、tractor multitrack、visual/audio transitionを実装し、単一playlist連結で配置や重なりを失う構造を修正。Shotcut/Kdenliveでの実再生、合成順と音声mixは未検証。layer transform/effectのMLT写像は対象外。

### R105 — MLT importのsource/timeline範囲

entry in/out、inclusive-to-exclusive変換、playlistごとのblankとtimeline start復元を追加。整数frameとtimecode入力を扱い、不正範囲をerrorにする。Shotcut/Kdenliveの実ファイル往復は未確認。

### R112 — backup補助APIのatomic置換

backup生成を同一directoryの一時ファイル、sync、renameへ変更。対象APIは現行GUI save経路から呼ばれないため、現在の通常保存経路に同じ改善が適用されたという意味ではない。OS別replace semanticsと障害注入確認は未実施。現行save経路へ接続する場合は改めてレビューする。

### R138 — WebP出力からMOV専用optionを除外

`-movflags +faststart`をMOV/MP4に限定し、WebPの拡張子とaudio指定も検証するよう変更。CLI featureの型検査は通過。FFmpegを使ったWebP実出力は未確認。

### R158 — Motion Sketch bakeをworker化

Douglas-Peuckerのstack/memory上限、4096点へのsample制限、worker bake、session/history generation照合を実装。Motion Sketch test 1件とstrict GUI-feature lib Clippyが成功。最大入力の体感時間と間引き形状の保持は未計測。

### R58 — 現行UI未接続の旧Flowchart実装

2026-10-04の現行ソース全体検索で`draw_flowchart_view`は`src/ui/flowchart.rs`内の定義だけがヒットし、呼び出し元はなかった。現在のFlowchart panelは`flowchart_graph::draw_node_graph_panel`を利用する。指摘は現行UIの動作不具合ではなく未接続旧実装の保守リスクなので、アクティブ本文からは外した。コードと元の指摘履歴は削除していない。

### R25 — PreComp child snapshotのフレームごとの複製

現行の`collect_cached`はproject session・active composition・展開状態・history generationでsnapshotを再利用し、非表示メタデータ編集時もdisplay fingerprintが不変なら同じArcを返す。表示メタデータが変更された世代ではfingerprintとsnapshotを全件再構築するため、その操作時のO(N)処理は残るが、毎フレーム全件をcloneする状態ではない。描画時はclipとrow geometryから可視rangeだけwidget化する。`cargo test --features gui --lib ui::timeline::precomp_children::tests::`でsnapshot reuse・virtualization・5000子の可視rangeを含む3 testsが成功。元のper-frame clone問題は解消としてアクティブ本文から移した。

### R152 — MerkleFrameCacheの無制限RAM保持

現行`insert`には512MiBのbyte ceiling、FIFO eviction、単一超過bufferの拒否がある。全repository検索では本体利用箇所はなく、公開moduleとtestからのみ参照され、製品rendererの現行cacheではない。未制限保持という実装不備は解消し、未接続experimental cacheのLRU・hash衝突耐性は将来製品接続する場合の設計条件としてアーカイブへ残し、現行製品不具合としての扱いは終了した。


# 2026-10-04 アクティブ台帳整理前の完全スナップショット

以下は修正済み項目の個別指摘を本文から整理する直前の台帳全文。対応記録、未完了項目、追加範囲に関する注記を含め、当時の状態をそのまま保存する。

---

# Kagari コードレビュー — 2026-10-04

> 完了済み項目の本文と履歴は[完了項目アーカイブ](code_review_2026-10-04_archive.md)に全文保存しています。ここには未完了・部分対応・確認待ちの指摘を残しています。

## 範囲と確度

現在の作業ツリーを対象にした静的レビュー。未コミット変更と未追跡の新規モジュールを含む。差分だけでなく本文と呼び出し先を確認した。全ソース全文の読了ではない。レビュー後に一部の指摘を修正している。GUI起動・画面確認と性能測定は未実施。先行runでは`cargo test --all-features --lib`の1,400件が成功し、性能しきい値テストも単独再実行と全lib再実行で成功した。その後の修正を含むserial all-features lib testは1,408件成功し、strict `cargo clippy --all-features --all-targets -- -D warnings`も成功した。Project I/O LUT復元、integrationを含む全test binary一括実行、GUI起動・画面確認は未実施。

詳細に確認した中心領域は KUI、カラー管理、履歴、ワークスペース、書き出し開始処理、CPUビューアー、フォント設定、タイムラインのヘッダー・子レイヤー・パンくず、CLIのレンダー範囲とモーションキャプチャ入力列挙。大型ファイルは該当関数と呼び出し先を読んだ。検索による出現数は品質の点数ではない。

P1: 出力・編集結果の信頼性に関わる。P2: 操作性、性能、保守性に実害がある。P3: 整理課題。実行による再現が必要なものは明記する。

## 出力・機能の問題

### R14 [P2・部分解消] 履歴コミットの差分確認でプロジェクトを2回JSON化する

`commit_action`は変更判定のためcurrentとnextを全走査する。現在はcurrent JSONを一度だけbyte buffer化し、nextはstreaming writerで比較しながらserialized lengthを数えるため、従来の2個目の全project JSON bufferはなくした。二重のserialization traversal自体は残る。編集APIのchange flagまたは全モデルの構造比較へ移す場合は、no-op抑制と直書き変更を含めた影響確認が必要。性能測定なしに高速化量は断定しない。

drag cancelによるcurrent entry置換、commit後のredo切捨て、件数/byte budget evictionもentryごとのestimateと総量を同期更新する。R14は部分解消。

### R15 [P2・保持スナップショット容量へ上限を適用／RSSとundo一時ピークは別]

履歴entryがProjectを毎回deep cloneしていた構造を、現在のlive Project 1つとserialized snapshot列へ変更した。過去状態は128MiB上限付きwriterでJSON bytesへ保存し、可変長データ・全composition/subcompositionを含める。snapshotとaction文字列はshrink_to_fit後のcapacityを数え、VecDequeのentry領域も加算してbudget超過時は古い状態を破棄する。現在の編集Projectは履歴ではないためbudget外とし、画面の表示も「Undo history storage」に改めた。

この上限はhistoryが所有する保持バッファ容量に対するもの。OS allocator metadata、アプリ全体のRSS、Project編集時に他所で作られるclone、Undo/Redo時の一時Project decodeとbounded serialization scratchまでは上限化していない。上限を超える大きさの旧状態へ遷移した場合は、その時点でUndo履歴を新しい境界へリセットする。実RAM計測と最大ProjectでのUndo/Redo peak計測は未実施。

### R16 [P3] 保守の単位とファイルの大きさが合っていない

現在はHomeをページ群へ分割済み（root 768行）で、旧ページ群も整理した。一方、viewport 4588行、timeline mod 2120行は依然大きい。今回、Camera Lens Graph（303行）と通常Graph Canvas（2024行）を`graph_editor/camera.rs`と`graph_editor/canvas.rs`へ分け、graph_editor.rsは4154行から1838行へ縮小した。公開描画関数の既存経路は維持している。viewportとTimelineの責務分割は残る。単に100行ずつ切る分割では責務は改善しない。

### R17 [P3] 機能名より追加順を表すファイル名が多い

`ae_effects_pack_v2`〜`v30` 等は処理カテゴリを名前から判断できない。実装探索と責務の重複確認が難しくなる。互換APIを維持しながらblur/distort/generate等のカテゴリと、共有処理に整理する余地がある。現状のAGENTSもv*.rs追加を許容しており、方針変更として扱う必要がある。

### R22 [P2・部分実装] OS連動ローカライズはGUI全域にはまだ届いていない

共通の`LanguagePreference`（System/English/Japanese）、`sys-locale`によるOS/アプリ優先locale解決、英語フォールバック、設定保存、共通翻訳関数を追加した。KUIの一般ラベル・ボタン・トグル・選択肢、Home navigation/CTA、toolbar tooltip、Preferencesの言語選択と主要設定項目、新規コンポジションダイアログのラベル・プリセット・作成結果通知へ接続した。言語はウィンドウ幅やworkspace stateから独立している。

ただし翻訳カタログと呼出し側の移行は部分的で、全workspace、menu、dialog、error/toast、effect parameterの英語直書きを網羅していない。Audio Mixerの表示は英語のsource keyを使うよう統一し、見出し、操作、tooltip、チャンネル状態、voice correctionのproperty labelsとkeyframe生成操作を共通翻訳関数へ接続した。画面内に残っていた日本語ハードコード表示は取り除いた。Asset Libraryもsection/title/search hint、preset名とtooltip、asset type・未計測状態を共通翻訳へ接続し、KUIのtext field hintも翻訳する。言語別のpreset検索も表示名に対して動作する。Color Managementではラベル、未対応状態、ツールチップ、LUT読み書き通知を共通カタログへ接続し、日本語直書きをなくした。Custom Shader画面の見出し、テンプレート、検証状態、CPU fallback説明、ファイル操作、パラメーター表示と、Export Dialogの出力設定、進捗、完了・失敗通知も共通翻訳へ接続した。今回Render Queueの操作、状態、進捗、時間範囲、設定表示とツールチップを共通カタログへ追加し、幅の狭い場合は操作ボタンを折り返す。さらにFrame QCの見出し、操作、計測値ラベル、診断状態、既知のエラーと通知をカタログへ追加した。Metadataパネルもコンポジション情報、レイヤー統計、レンダー設定、書き出し情報の項目名を共通カタログへ接続した。レンダージョブmanifestのstatus/schema等の内部値は英語のままにして表示翻訳と分離した。codec名やFFmpeg由来の動的診断文など技術固有の文字列は原文表示が残る。OSごとのlocale取得・実フォント・日本語/英語の全画面レイアウトを実機では確認していない。全workspace・menu等の未移行が残るためR22は未完了。

### R23 [P2・未完了／Linuxの旧記述を訂正] 日本語フォントの確保がOSごとに不均等

現行`configure_fonts`はLatin用フォント候補と日本語fallback候補をOSごとに別配列で探索し、macOSのHiragino、LinuxのNoto CJK、WindowsのYu Gothic/MS Gothicがあれば比例フォントfamilyのfallbackへ追加する。したがって「LinuxのDejaVuで先にbreakしNoto CJKを見ない」という以前の記述は誤りで、別ループで日本語候補を探索している。誤記は取り消さず監査履歴として明示した。

フォントはOSにインストール済みであることを前提とする。追加済み候補が存在しない環境では日本語字形を保証できず、各OS実機で日本語glyph fallbackとUIを確認していないためR23は未完了。フォント同梱はライセンス・配布サイズの判断が必要なので、この変更では行わない。

LinuxではLatin primary font選択からNoto CJKを分離して後段fallbackへ登録し、WindowsはYu Gothic/MS Gothicのfallback候補を追加した。今回macOSの`~/Library/Fonts`とLinuxの`$XDG_DATA_HOME/fonts`、`~/.local/share/fonts`、`~/.fonts`を日本語fallback探索へ追加した。現在のmacOS環境では`~/Library/Fonts/NotoSansJP-Regular.ttf`を確認し、新候補から読み込める構成になった。Linux cross-checkはALSA依存の`pkg-config`にLinux sysrootがなく、KagariのLinux target compileへ進む前に失敗した。GUIでの日本語字形表示と、フォントのないmacOS/Linux/Windows環境は未確認。

### R24 [P2・列幅共有を追加／全描画rect統合は残る] タイムライン共通geometryはまだ高さ定数3個だけ

`timeline/layout.rs`に`TimelineColumns`を導入し、利用可能幅・egui列間隔・要求された左列幅から左列と時間軸幅を一度に計算する。通常property row、PreComp child row、およびtimeline headerのリサイズ制約で共用。ヘッダールーラーと通常レイヤーの時間軸バーも同じ計算結果を使う。今回、Timeline rowの名前欄でローカルに計算していたswitch控除幅と40–220pxの名前幅をlayout moduleへ集約し、極端に狭い場合に最小幅40pxがavailable領域からはみ出す動作を修正した。列幅と名前欄算定のunit testsを追加。left/name/switches/timeの描画矩形そのものを一つのgeometry objectから生成する統合とGUI実測は未実施。

### R26 [P2・主要state tokenを整理／全使用箇所の移行は残る] テーマの色の意味が実使用と矛盾する

themeにselection/workspace/action/warningのsemantic tokenを導入し、selection stroke、widget active border、KUI primary action、workspace preset selected state、Timeline selected tab/keyframeへ割り当てた。選択背景もブランド色系の低彩度色へ揃え、warning表示は`ACCENT_WARNING`経由にした。`ACCENT_TOGGLE`/`ACCENT_PROGRESS`/`ACCENT_BEAT_MARKER`/`ACCENT_INFORMATION`/`BORDER_FOCUS`を追加し、手描きtoggle、進捗bar、beat marker、通知・診断の情報色、active widget borderへ割り当てた。Timeline layer selection rail、toolbarのactive tool、Inspectorのkeyframe-active、Homeのselected asset、Preferences categoryも`ACCENT_SELECTION`/`BG_SELECTION`系に統一。Render操作は`ACCENT_PRIMARY_ACTION`を使い、audio peak meterは用途別の`METER_WARNING`を使う。Timeline playheadは専用の`TIMELINE_PLAYHEAD`、motion/guidesはcyanを使う。Viewerのactive composition/tab/control、Inspectorの選択カテゴリ、toolbar tool state、Timelineのboolean switches、選択中Automation binding、Automation key marker、shortcut referenceの情報表示も用途別tokenへ移行済み。今回Frame QCの注意・情報・対象フレーム色もwarning/information/playhead tokenへ接続した。旧色定数も後方互換のため残す。全画面で色名直参照をなくす作業と、色の視覚コントラスト実機確認は未完了なので部分対応。

### R28 [P3・共通Painter部品とHomeの主要ボタンを改善／独自interaction全体と支援技術検証は残る] 手描きclick領域には操作の意味を付与する規約が不足

`custom_widgets`のSVG button/toggle・text toggle・switch・color swatchに、透明な標準egui Buttonを操作面として使う共通経路を入れた。描画矩形は同じbutton rectを使い、tooltip/visible labelをwidget metadataへ渡すため、標準Buttonのfocus、Enter/Space activation、enabled stateを維持する。Homeのaction/CTA/select/icon/documentation card、Timeline headerのtransport/view tabsも同経路へ移行した。Timelineのtimecode/keyframeやHomeのその他の`interact`領域、全操作のdisabled contractは未統一。VoiceOver等の支援技術による実検証も未実施。

## 再指摘しない改善済みの点

- RightPanelはenum化済み。保存JSONの旧index互換は残るが、旧right_tab_idx問題として報告しない。
- Expressionのcompileにはソース変更時のcacheがある。毎frame compileの指摘は現在のコードに当たらない。
- PreComp描画には可視範囲チェックがある。全行をPainterで描く実装とは言わない。
- Timeline本体はresizable(true)。固定の断定はしない。ただし最小160/176pxがあり、実際にどこまで下げられるかは実機確認が必要。
- CPU専用CLIとcli-gpuのfeature分離がある。wgpu常時必須という旧指摘は現在当たらない。
- KUIはフォント実測のPropertyGrid、狭幅stack、vector、数値・単位、disabledボタンを備える。資産はある。

## 修正に進む場合の優先順位案

1. R01/R02/R07/R09：結果を誤らせる処理。
2. R04/R06/R08：表示される操作と実機能の一致。
3. R10〜R15：クリック応答とメモリ。代表プロジェクトで計測を追加。
4. R19/R21/R24：Inspector・Effect Controls・Timelineの共通構造。
5. R22/R23：言語・フォントの一本化。
6. Home責務分割、意味ベースのテーマ、独自部品の操作契約。

これはレビューに基づく提案であり、修正の開始や指示範囲の追加を意味しない。

## 第2回：レンダー、メディア、復旧、CLI、グラフへ拡張

追加でimage/precomp/frame/disk cache、video_import、audio_engine/playback/spectrum、production_document、project_migration、project_io、recovery、drop_import、FFmpegのworker、CLI mocap/render、独立preview、VFX graph compiler、flowchart、custom shaderを確認した。大きいファイルは関連関数と参照先を対象にしており、全ファイル全文の読了ではない。引き続き静的レビューで、再現テスト・GUI・性能測定は実施していない。

### R98 [P1・別画面へ移動時のキャンセル伝達を追加／大規模snapshot cloneと実動作確認は残る]

Scripting ConsoleはProject snapshot上でRhai scriptを評価する。操作数上限だけでは応答性を保証できない。現在はsourceを1MiBまでに制限し、ProductionDocument検証後だけ結果を反映する。Expression評価とautomation評価はworkerへ移し、実行中の再実行を止め、session/history generationが変わった結果を破棄する。両モードに進捗callback経由のキャンセルボタンを追加し、automation workerは受け取ったsnapshotをそのまま編集して重複cloneを避ける。ConsoleがHome、チュートリアル、Text/Color/Assets workspace、最大化Viewer、非表示の狭幅drawerなどへ切り替わり実際に非表示になったフレームでcancel tokenを立て、アプリ終了時にも中断を要求する。実行workerは既存Rhai progress callbackで中断し、cancel要求と完了通知が競合した場合も結果を破棄してautomationをcommitしない。開始用Project snapshotのdeep cloneはUI thread上に残る。長時間loopを実行しながら画面遷移した際の停止と、大規模Projectのclone応答時間は未計測。

### R103 [P1・Clip.2 active media参照に対応／汎用OTIO相互運用は未完了] OpenTimelineIOの標準構造と外部素材の意味を一部しか扱えない

以前は`OtioTimeline`独自構造をそのままJSON化していた。現在は`to_json`が`Timeline.1` / `Stack.1` / `Track.1` / `Clip.1` / `Gap.1`、`RationalTime.1` / `TimeRange.1`、`ExternalReference.1`を出力し、Kagari固有データはmetadataへ分離する。`from_json`は標準Timeline/Stack/Track階層、gap、file URLと通常パスを解析する。legacy Kagari JSONも読む。公式schemaにないroot/track/clip schemaは拒否する。

標準出力は各Kagari layerを1 trackとして出し、track内の空白はGap、timeline startはGap長で表す。同一trackの重なりは明示errorにする。Kagari自身のlayer設定・動画cache情報はClip metadataから復元できる。外部OTIOではClip.1とClip.2を扱い、Clip.2はactive_media_reference_keyに対応するmedia_references entryを解決する。未定義のactive keyは明示errorにする。source_range省略時は選択中のmedia referenceのavailable_rangeを利用し、それもないClipやdurationのないGapは明示errorにする。一般的なExternalReferenceを取り込むが、動画ファイルはKagariのdecoded frame cacheがないためComposition化を拒否する。GeneratorReference、nested/multiple tracks、audio source trim、外部effects/transformsなどは未対応で、losslessな汎用OTIO bridgeではない。Clip.2のactive reference選択、available_range fallback、不正key拒否、時間範囲欠落のunit testが成功。OTIO Python reference実装はこの環境にインストールされておらず、外部アプリ相互運用は未確認。

残作業：OTIO reference implementationで生成ファイルを開き、複数track・Gap・file URL・Kagari metadataの相互運用を実証する。外部video importと未対応構造をどうloss reportするか決める。現在のコンパイル確認はschema適合性やreference implementation互換性を証明しない。

### R123 [P2・通常時cacheと吸着二分探索を追加、操作時build/計測は残る] Timelineの可視行culling前に全レイヤー由来データを毎frame再構築する

`src/ui/timeline/mod.rs`は画面外レイヤーのwidget生成をskipしているが、その直前に全layerの`parent_choices` (ID/name String clone)、`layer_edges`、`collect_all_kf_frames`を毎frame生成する。さらに可視rowごとに親名を全件検索するため、可視行数V・総layer数Nで少なくともO(N+V×N+K)の仕事となり、大規模時はwidget cullingの利点を打ち消す。レイヤー移動時のsnapは他全layer edgeも走査する。最大100,000 layerを受理するProductionDocumentに対し、深いProjectや多数keyframeでTimelineを開くだけでもframe budgetを超え得る。

現在はderived listsをegui context内でComposition ID/history generationごとにArc cacheし、parent name lookupをindex mapへ変更。キーフレーム吸着はsort済みkeyframe列の二分探索、layer edge吸着もsorted edge indexの近傍探索へ変更し、候補全件走査を除去した。当初は任意のpointer input中にcacheを迂回していたが、現在はmodel-edit transactionとlayer reorderの実ドラッグ中だけfresh buildへ迂回する。hover・scroll・toolbar clickではcacheを再利用する。編集ドラッグ中のderived data再構築と大規模Projectでのframe-time測定は残る。

## 第2回で確認できた改善・制限

## 実装進捗

### 修正を反映した項目

- R18/R165: 現在のtoolchainで再検証し、test codeの`field_reassign_with_default` 34件をstruct initializerへ移してstrict all-target Clippyを通過。precomp rasterizer二つ、Content-Aware Fill/Roto Brush worker、骨格階層出力、inline text editor geometryを名前付きrequest/geometry/context型に整理。Wigglerは設定型ベースAPIへ移行し、CPU effect pipelineとpaint rasterizerにも共通context/style型を導入。multi-argument crate-wide抑制を除去し、残る13関数だけにlocal allowを設定。`excessive_precision`、`type_complexity`を含む全crate-wide Clippy抑制を除去。`chunks_exact_to_as_chunks`をlib/test targetの固定長pixel/audio/geometry chunk処理へ移行。2026-10-04の`cargo clippy --all-features --all-targets -- -D warnings`が成功。R18/R165の記録は保持する。
- R01: LUTをthread-localから共有ロック付きスナップショットへ変更し、CPUレンダーの並列pixel loopへ一度取得したArcを渡す。
- R02/R03: 存在しない「現在のグレード」出力を廃止し、実際に読み込まれているLUTだけを保存するUIへ変更。保存先を選択し、書込み成功後だけ成功通知する。カラーグレード自体のLUTベイクは未実装として誤認させない。
- R04: 未接続の色管理選択肢を実行可能なdropdownとして見せるのをやめ、未対応表示へ変更。
- R04追記: 実際のrenderが適用しない16/32-bit深度選択と、保存・変換されない色空間status値も除去。現在の8-bit出力と色管理未対応を表示する。指摘・過去の状態記録は削除せず、色変換機能そのものは残課題として明記。
- R05: LUT参照パスをProductionDocumentとautosaveへ保存し、プロジェクトopen/recoveryで非同期復元する。LUT本体は外部参照のままで、埋め込み・CLI共有は未実装。
- R39: 動画importの容量概算・空き容量予備・処理中監視、FFmpeg停止とUIキャンセルを追加。実素材での停止とキャンセルの確認は残る。
- R64/R65: CLI previewはProductionDocument/active compositionを保持し、再生frameは開始時刻から算出。GPU実機は未確認。
- R66/R67: 動画media referenceにJSON形式を導入し旧区切り形式を維持。Render Queue/status/exportはComposition IDで扱う。
- R06: 画像レイヤーの無動作pinボタンを実際にPosition Pinを追加する操作へ修正。異なる意味のStarch/Overlap/Advanced pinはモデルに型がないため、未実装のまま。
- R08: 組み込みワークスペース6種の誤ったpanel割当を修正。
- R07: Timelineの全追加レイヤー、重複・分割、Asset Library/Project Panel/audio-keyframe生成で既存IDを避ける `next_layer_id` を使う。
- R09: 2倍サイズが上限に入らないときはSSAA全体を無効化し、片軸だけクランプされた画像を2x downsampleしない。
- R10: 音声mix/WAV準備をUI callbackからバックグラウンドexport jobへ移す。準備時の進捗は状態表示のみで、架空の割合は出さない。
- R11/R12: CPU viewerのフレーム結果とTextureHandleを表示設定込みで保持し、exposure/LUT modeもGPUと同じ設定から描画。
- R29: PNG callbackへ渡される絶対フレームをそのまま描画し、work area開始値の二重加算を除く。
- R30: PNG/動画の各フレームでProductionDocumentのautomation snapshotを評価してからレンダーする。
- R31/R32: 音声cache keyにProjectHistory世代を加え、プロジェクト交換時に古いWAVを無効化する。work areaから切り出したWAVのseekを相対秒にし、Out frameをexclusive endへ直す。
- R29–R32: 現行コードを再確認。PNGのabsolute-frame契約、GUI export automation snapshot、音声cacheのhistory generation/Project交換時reset、work-area-relative seekとinclusive Out変換が反映済み。
- R33/R34/R35: WAV生成を絶対frame境界のsample数にし、全尺PCM保持をやめてバッファ出力。RIFF上限を先に検査し、I/O失敗をResultで伝え、export用一時WAVをjob終了時に削除。
- R36: 動画import jobへ開始時のProject session、Composition ID、選択layer IDを保持させる。完了時に別プロジェクトへ誤挿入せず、元compositionへ挿入する。session IDは全てのGUI project replacement経路で更新する。
- R42/R43: 復旧UIは同一slotのProjectとProductionDocumentを取得し、失敗時にslotを消さない。成功時は音声設定も復元してダイアログを閉じる。
- R44: autosaveをプロセスID namespaceへ分け、復旧・削除は選択した復旧元だけに限定。
- R46: ImageCacheのbyte予算を超える静止画をdecode前に拒否し、自己evictionでNoneになる曖昧な動作を避ける。
- R47: 画像取得後のCPU raster/precomp pixel loopを共有cache lockの外で実行する。画像データのArc化とclone削減は残課題。
- R48: GPU画像cacheを200枚固定から512MiBの実format byte予算へ変更し、任意HashMap順削除をLRUに置換。
- R49/R50: GPU still/video/PreComp cache keyへ外部source file revisionを含める。
- R52: ディスクフレームcacheをインスタンス固有namespaceで分離し、同じframe/versionを別プロセスが誤読する衝突を避ける。
- R53: frame disk cacheとPersistentDiskCacheの読み込みをmetadata/上限付きにし、frame writerの全pixel一時コピーを除去。
- R54: 予算を超える単一frame書込みを拒否し、evict後に実使用量を再確認。既存key置換を予算計算から除外し、process内writerを直列化。複数process間の予約は未対応。
- R55/R56/R57: graph scheduleの同順位順序を決定化し、親祖先依存を閉包へ含め、各operation variantをFlowGraphのexecution badgeへ反映。
- R59/R60: WGSL validationにNaga semantic Validatorを追加し、UIにGPU compile未確認を表示。source/diagnostic cacheをentry数と総byte数で制限。
- R63: スペクトラム帯の範囲を整数境界で分割し、入力サンプル数より帯域数が多い場合も逆転sliceを作らない。
- R62: 時間区間RMSをradix-2 FFTベースの周波数band分析へ置き換え、Hann窓・stereo downmix・frequency rangeを反映。
- R64/R65: 独立previewでProductionDocumentとactive compositionを維持してframeごとのautomation snapshotを描画し、絶対時刻から再生frameを算出。プロジェクト再読込時も再生基準を更新。Lost/Outdated GPU surfaceを再設定。
- R66: video media referenceの新規出力をJSON payloadへ変更し、Windows drive letterやコロン入りpathでフィールドがずれないようにする。既存のlegacy `video:`形式は読み込み互換性を維持。
- R70: TileCacheのLRU evictionでentry全体をcloneせず、byte長だけを一時保持する。
- R38: 動画import失敗時に作成途中の独立cache directoryをRAIIで削除し、WebP→PNG fallback前にpartial WebPを掃除して、成功時のみ一貫したcacheを保持する。
- R07: Timelineのlayer追加経路は `Composition::next_layer_id` を使い、既存IDとの衝突を避ける。コアの欠番テストあり。
- R09: SSAAの2倍寸法がレンダラー上限を超える場合は、片軸だけをクランプせずSSAA全体を無効化して等倍でレンダーする。
- R41: 画像シーケンス全フレームの寸法をdecode前に検証し、寸法不一致・上限超過・未対応decoder形式を拒否する。
- R40: VideoAsset metadataを実際に抽出される偶数寸法へ合わせ、2px未満の入力を拒否する。
- R45: Autosave失敗をtoast/logへ報告して30秒retry backoffを追加し、周期保存のserialization/disk writeもbackground化。Project snapshot cloneと実応答時間計測は残る。
- R42/R43: 保存・復旧でProjectとProductionDocumentを同一snapshotとして扱い、復旧失敗ではデータを消さず、復旧成功後に音声設定を含むdomain stateを適用する。
- R13: Color Management、Paragraph、Camera/Light Optionsの毎frame Project cloneを、編集入力時だけ作る共通pre-edit snapshotに変更。全画面共通の遅延編集transaction化は残る。
- R61: CPU shader fallbackをコメント部分一致からbuilt-in source全文一致へ変更し、UI template文字列をCPU fallback moduleと共通化。旧marker単独値は互換sentinelとして保持。
- R23: LinuxのCJK fontをLatin primary選択から分離して日本語fallbackへ登録し、WindowsにもYu Gothic/MS Gothic候補を追加。
- R23追記: macOSの`/Library/Fonts`にインストールされたLINE Seed JP、Windowsのsystem/user font directoryにインストールされたNoto Sans CJK/JPも候補へ追加。フォントがないOS環境の保証と実機glyph表示確認は残る。
- R25: PreComp child row snapshotをcomp/session/history generation/expanded set単位でcacheし、表示fingerprintが変わらないhistory editでは全child cloneを回避。表示に関係するメタデータの再走査と、変更時の全件snapshotは残る。
- R71: Cmd+S、File > Save、コマンドパレットをProductionDocument対応の共通保存関数へ集約済み。
- R21: ParagraphをKUIのPropertyGrid、共通数値入力・カラー入力・toggleへ統一。Alignment selectorは利用幅に応じて4列/2列へ切替。
- R75: Unix以外では`statvfs`を呼ばずHomeの容量表示を省く。
- R76: Renderを画面幅でreference mockと実際のexport画面へ切り替えず、responsiveな実export画面へ統一。
- R78: Project Panelのfile picker filterをdecoder/分類可能な画像・OBJ形式に合わせ、画像寸法を実ファイルから取得し、未知形式を拒否。
- R79: `Project::next_composition_id`と`next_project_item_id`を追加し、動的なUI/automation creation経路を共通発番へ集約。
- R80: Homeの設定ナビを固定値だけの装飾ページではなく、保存・適用処理を持つ共通Preferences dialogへ接続。
- R81: 新規動画/音声Assetの長さに10秒の偽値を入れず0を未取得sentinelとして保持し、Project PanelとCC Librariesで未計測を表示。偽の音声sample rate/bit depth/channel表示を削除。実probeは未実装。
- R82: 「Move selection out」の親参照判定をfolder nameではなくfolder IDで行う。
- R83: 未使用素材判定でasset display nameだけでなく、file-backed assetのmedia pathも使用中参照と照合。
- R84: 「未使用素材を削除」後に選択indexをクリアして、削除・並び替え後の誤選択を防ぐ。
- R85: tutorial practice videoのtemp pathをprocess ID namespace化し、既存ファイルのbyte contentを同梱assetと比較してから再利用。
- R92: 字形画像とキャッシュへbyte上限を設定し、未検証font bytesの拒否、同名font置換時のglyph cache無効化を追加。
- R24: TimelineColumnsでtimeline幅を左右列に分解し、property row/PreComp child/headerの幅制約を共通化。狭幅でも時間領域の最低幅を確保する単体テストを追加。全てのtimeline描画rectの統合と実画面確認は未実施。
- R25: History generationが変わってもPreComp child表示fingerprintが同じなら既存snapshotを再利用し、通常のkeyframe/transform編集で全件のString cloneを繰り返さない。子メタデータ変更時の全件走査と可視行だけの取得は残る。
- R93: Audio decoded cache keyにsource file length/mtimeを加え、同一パスの外部差し替えで古いPCMを返さないよう変更。revision差異のcache testを追加。
- R94/R95: Render Queue IDへprocess ID・nanosecond時刻・atomic sequenceを使用し、全u32 frame範囲の計算をu64へ変更。連続IDと境界frame数のunit testを追加。
- R68: Flush後も新しいRenderPipeline要求を処理するgeneration方式に変更し、待機中の古い要求だけをskipするテストを追加。テスト・型検査は未実行。
- R96/R97: OpenFX Property Suiteの関数順・layout assertionsを公式C ABIに合わせ、未完成host suiteへpluginを渡すattachをfail-closed化。偽pass-through成功を削除し、render未対応をエラー表示。テスト9件はlayout assertions追加前に成功。SDK/sample pluginとの実動検証は未実施。
- R98: 自動化script入力を1MiBに制限し、ProductionDocument validation成功後だけProjectへ反映。Expression/automation評価をworkerへ移し、同時実行を防ぎ、編集中または別Projectへ切替後に返った古い結果を破棄する。Project snapshot cloneはまだUI thread上。
- R98追記: Expression/Automation両モードにキャンセル操作を追加し、Rhai progress callbackで中断する。workerへ渡したProjectをそのまま編集して、従来のworker内二重目cloneをなくす。UI thread snapshot cloneとConsoleを離れたときのキャンセル伝達は残る。
- R99: frame binding snapshotでProject全体をcloneせず、対象Compositionだけをcloneし、結果をmoveで返す。関連testsはディスク空き不足で未実行。
- R100: 複数Compositionのbatch exportとFarm manifestで、base pathから各item専用のファイル名を同じhelperで生成。path helperのunit testを追加。
- R102/R122: Collect Filesは動画layerのframe_countを維持してProductionDocument validationを通す。実動画を開き直した際のcacheと音声再生成は未検証。
- R119/R121: Project Panelの動画Assetからのlayer追加、Image Sequence、Replace Footageをsession-aware background workerへ接続。失敗時にfallback Solidを追加しない。遅い素材での応答時間は未測定。
- R120: Image Sequence pickerの拡張子候補をcore decoderの対応形式へ一致。
- R117: 画像・動画importのRGBA換算128MiB寸法上限と、境界値テストを追加。OSの実ピークRAMは未計測。
- R125/R127: OTIO変換をfallibleにし、壊れたKagari動画参照とu32 frame range overflowをclip名付きエラーとして返す。対象unit tests 8件が成功。
- R105: MLT entryのsource in/outとplaylist内のtimeline startを復元し、inclusive outをexclusive境界へ変換。フレーム整数/timecodeを受け、timecodeを最近傍frameへ丸める。不正値・overflowを拒否。
- R104: 各layerを別playlistへ出し、in_frameをblank、重なりをmultitrack/qtblend、音声をmix transitionで表す。MLT in/outを整数frameに統一し、importは整数とtimecode双方を受け付ける。コンパイル確認済み。Shotcut/MLT実行で合成順・音声mixを未検証。
- R140: MLT exportが未対応layer typeを無言で落とさず、layer名・種類を示すerrorでfail-closed。
- R141: OpenShot JSON exportも未対応layer typeを名前・種類付きerrorで拒否。
- R142: 音声resampleは不可能な変換をerror化し誤レートPCMをcacheしない。mix/UIでの失敗理由表示は未解消。
- R143: CLI scriptをmetadata検査後の無制限readから1MiB上限readerへ変更。
- R133: Frame QCはフル解像度renderをbackground workerで行い、Project/Composition/frameが変わった結果を破棄。再監査でUIからのFrameCache readerとpixel sampler自体が現行経路にないと確認し、指摘を削除せず対象consumerなしとして閉じた。
- R116: Automation curveを1回検証した後に二分探索でsampleし、source mappingではcurve点列を走査しない。
- R144–R149: Dynamic Link接続の占有、legacy動画referenceの不正受入れ、Home UI monolith、音声decode失敗時の無音成功、設定pathのOS優先順、非UTF-8 recent pathの不可逆化を追加監査で確認。R144/R145/R146/R147/R148/R149は修正し、legacy page/layout重複はR77に継続記録。R103は標準schema serializer/parser、timeline位置・gap、Kagari metadata復元、未対応mediaのfail-closedを実装。reference implementationでの相互運用性は未確認。
- R44: process ID別autosaveに加えdirty-exit markerもprocessごとに分離し、起動時に既存markerのprocess生存状態を確認する。
- R145: legacy動画referenceの必須field・frame count・source rangeを厳格に検証し、不正値をclip名付きerrorへ変更。
- R148: OS別設定pathへ切替え、旧HOME直下設定をfallbackし、更新時に新pathへ保存する。
- R147: audio source load/resample failureをmeterへ伝播し、previewで一度だけ警告、WAV exportでは失敗として扱う。
- R149: Recent/Starred pathをOS-native bytes/wide unitsから可逆tokenへ変換し、Home/Welcomeで復号してから開く。
- R144: Dynamic Link socket処理をsingle-client workerへ分離し、listenerを応答可能に保つ。占有時は追加clientへ明示エラー、idle timeoutとnonblocking UI通知を設定。
- R124: Dynamic Link handshakeでsemver-shaped versionをparseし、protocol major 1以外を明示拒否。
- R21/R33/R34/R36/R37/R40/R41/R76/R81: 一部の現行実装を再確認したが、R21/R36/R37/R76/R81は本文に古い挙動の記述が残っているため、この進捗行だけでレビュー記録を更新済みとは扱わない。各項目の現状説明を再照合する。R35はsource errorのWAV失敗化まで進めたが、`None`のAPI意味分けと全caller統一は未完了。
- R35: 音声mix結果で有効source数を返し、無音WAVの誤生成をなくした。再生側はno-audio結果をcacheし、条件変更時に前の一時WAVを削除する。
- R150: Recovery snapshotとファイル妥当性確認の全読込を100MiB bounded readerへ統一。R151はSfM風の偽解を返す経路を廃し、未実装errorへ変更。R152は512MiB RAM ceiling/FIFO evictionを追加したが未接続で、弱いhashとLRU不足が残る。R153は偽のAI load/inference成功を未実装errorへ変更。R154は偽mergeとunbounded packet historyを除去。R155はメモリ内timeline storeへ改名しframe-range indexを導入したが、未接続・非永続でrecord上限なし。R156はPreComp内の音声もexport判定へ含めた。
- R155追記: repository内にconsumerがない別storeを削除し、現行Project keyframe + Timeline generation cacheを正本として明記。レビュー指摘は台帳に保持。外部利用者が公開moduleをimportしていた場合の互換性影響は残る。
- R103: OTIOの標準schema serializer/parser、timeline gap、Kagari metadata、非対応referenceの明示拒否を実装。reference implementationとの外部相互運用確認は未実施。
- R142: audio preview mix失敗を一度toastで通知し、同じstateで毎frame再試行するのを防止。
- R157: 3D camera solveのtracking入力不整合・非有限値を拒否し、一意frame数を100,000へ制限。
- R158: Motion Sketch記録を8192点以下へ保ち、同一frame sampleを集約。Douglas-Peuckerを反復化し、ベイク入力を4096点へ制限。UI同期処理時間・間引き品質の確認は残る。
- R159/R160: AEPの偽成功をunsupported errorへ変更。動画decoded cacheは全frame indexと内容fingerprintをmanifestで検査し、旧cacheはdecode検証後にmanifestへ移行する。全lib testで関連テストを含む1,399件が成功し、大規模cacheの検証I/Oは未計測。
- R123: Timelineのderived dataをgeneration cacheし、親名lookupをO(1) index化。ドラッグ中のsnap edge走査と大規模実測は残る。
- R126/R128/R129: 階層cycle validation、unsorted keyframe受入れ、precomp cycle時のtree再検索を修正。ProductionDocument test 29件成功。
- R111: 動画importのFFmpeg stdoutを破棄し、stderrを継続drainしつつ64KiB上限で保持。実素材でのmemory計測は未実施。
- R101: ProductionDocument::loadを上限付きfile reader経由にし、metadata検査後にファイルが増大しても読込量を上限+1 byteに制限。
- R45: 周期autosaveのJSON化・streamed file writeをworker化し、generation追跡で保存中の後続編集をdirtyのまま保持。終了時flush、ProductionDocument内のProject重複保存削減、旧autosave形式の互換テストを追加。UI側Project snapshot cloneは残り、実画面の停止時間は未計測。
- R86: Replace Footageの選択filterをAsset型別に制限し、分類結果の型が一致しない場合を拒否。画像置換では実寸metadataも更新。
- R87: footage置換時は旧path完全一致で対象layerを決め、同名のSolidや別directoryの同名素材を維持。Model3D pathも更新する回帰テストを追加。
- R88: Audio Spectrumのeven-length判定を`usize::is_multiple_of`へ置換。
- R89: headless GPU rendererでtexture上限超過を事前拒否し、readback target寸法とbyte長の計算を検証するよう変更。all-features Clippyで型検査済み。実GPU確認は未実施。
- R90: Time Remap Loop bakeを整数計算と100,000 key上限へ変更し、1frame sourceを1キーへ縮約。no-default-featuresの対象unit test 2件は成功。
- R91: Home設定のatomic write失敗を`Result`で伝え、Preferences/Home/workspace/プロジェクト開閉の各呼び出し側で通知。all-features Clippyで型検査済み。実書込み障害の注入テストは未実施。
- R37: プロジェクトread/parse/relink/欠損video cache準備を専用workerへ移し、読み込み中表示と二重読込防止を追加。処理中のProject変更を検知した場合に読込結果を破棄し、既存状態を保護する。
- R73: Mocap入力を`frame_<number>.png`に限定し、フレーム番号を数値順に並べ、番号重複を拒否する。
- R72: CLI renderでduration/range/dimension/byte budget/exposure/LUT modeをbackend起動前に検証し、PNG/MP4/GIFで不正範囲の成功扱いや余分な1フレーム出力を防ぐ。
- R67: Render Queueとexport statusの識別を表示名からComposition IDへ移し、同名・名称変更による出力先の誤選択を防ぐ。
- R14/R15: history byte estimatesをentry単位で差分更新し、JSON serialization長に基づく可変データ込みの予算proxyへ変更。`commit_action`はcurrentだけをJSON buffer化し、nextはbufferを作らず比較・サイズ計測する。二重のserialization traversal、2倍係数の実測校正、Rust heap実使用量の厳密な上限は残る。
- R74: headless GPU rendererの公開条件を`cli-gpu`へ絞り、通常CLIとGUI+CLIのfeature組合せからpollster参照を切り離す。

これらはコード上の経路を修正した段階。GUIターゲットのcargo checkを複数回通過。CLI通常・headless GPU・GUI+CLI feature組合せのcargo checkも通過。LUTのファイル内容と書込み失敗、音声スペクトラムの短尺入力、ディスクcache/復旧の複数プロセス分離、出力画素の一致、音声同期、復旧UI操作、非同期import切替、巨大Projectの読込応答性、R72/R73の不正入力を実コマンド・実素材で確認していないため、実行時まで検証済みとは扱わない。

- CLI mocapは現在track_markerless_motion_streamへiteratorで画像を渡す。以前の「全decoded RGBAを保持」は現在当たらない。path一覧と出力track samplesの保持は残るが、別の話。
- `project_io::save_project_to_path` はProductionDocument::save_atomicを使い、Cmd+S/File > Save/コマンドパレットもこの共通入口を呼ぶ（R71修正済み）。Collect Filesの経路は別問題としてR102に記録。
- Dynamic Linkは127.0.0.1へbindし、受信行を64KiB、read timeoutを10秒に制限している。外部ネットワークから直接接続できるという指摘は確認できなかった。固定port競合とクライアントを直列処理する設計は別途残る。
- FFmpeg出力にはTemporaryOutputGuard/ChildGuardがあり、失敗途中の動画や子processを管理している。PNG/image-sequence importと同等と推測しない。
- graph compilerのHashSet順は現状の可視化の問題。実合成がこのscheduleを使う証拠はない。
- GPU画像cacheは512MiB byte budgetとLRU evictionへ変更済み。device adapter別のメモリ予算反映と、実GPUでのVRAM計測は未実施。
- 第2回はR29〜R70の42件を追加した。P1/P2/P3および未接続APIを含み、すべてが同じ深刻度・再現確度という意味ではない。
- 第3回では、プロジェクト保存入口、スキーマ判定、ショートカットの呼び出し元を照合し、R71を追加した。これで実行中の保存経路の不一致もレビュー範囲へ含めた。
- 第9回は動画/シーケンスimportとRAM/disk frame cacheを再監査し、各import経路をsession-aware worker化、欠番・重複frame番号の誤圧縮を拒否、旧世代frame cacheのdeep cloneを除去、cache budget対象を永続frame fileに限定して古いtemp fileを掃除するよう変更。Dynamic Linkの未対応要求はerror応答(R113)へ変更。Frame QCのcache readerとpixel sampler writer不足をR133として記録したが、後続監査では両consumerが現行UIにないことを確認し、R133へその事実を追記した。import応答時間と実機フレームcache連携の確認は残る。
- R51: PreComp cache hitの全画素cloneとComposition全体JSON化を除去し、共有Arcと編集世代・外部media revisionによるinvalidationへ変更。revision世代のunit test成功。
- R54: 既存disk frame置換時の容量二重計上を修正し、process内mutexとdirectory単位のOS file lockでKagari writerを直列化。別processとのlock競合testを含むdisk-cache test 4件とstrict no-default Clippyが成功。
- R102: OBJ→MTL→texture依存を収集しbundle内参照へ書換え。既存asset/videoを含む対象test 3件とstrict no-default Clippyが成功。実素材検証は未実施。
- R158: Motion Sketch bakeをbackground worker化し、Project session/history generation/Composition/Layerを確認後にundoable editとして適用。GUI cargo check、対象test、strict GUI lib Clippy、および現行変更を含むno-default lib test 1,240件が成功。最大入力の体感計測は未実施。
- R146: Home rootを約1,860行にし、各画面群を責務別moduleへ移動。GUI test targetを含むcargo checkとstrict GUI lib Clippyが成功。legacyページ重複はR77に保持。
- 第4回では新規KUI、レイヤーコマンド、共有FlowGraphモデル、headless GPU readback、Time Remap Loop、Home設定を確認しR89〜R91を追加・実装。第5回はTimeline geometry、PreComp child cache、font rasterizer、audio decoded cache、Render QueueからR92〜R95を追加・修正。第6回はRenderPipeline、OpenFX、Automation console、ProductionDocument frame snapshotからR96〜R99を追加。第7回は既存指摘の現行コードを再照合してR07/R09/R29〜R32の修正を反映し、Render Queue batchの出力先衝突をR100として追加・修正した。
- 第7回の変更後は `cargo check --all-features --tests` が成功。Clippyはmetadataを書き出す時点でディスク満杯となり完了していない。全テストと実FFmpeg batch出力確認も未実施。
- R90対象2件、font validation、audio source revisionのno-default unit testは成功。Timeline geometryの独立unit test 2件も成功。第6回はR99変更後を含むall-features `cargo check --tests`とClippyが成功。OpenFX 9件、自動化11件、RenderPipeline 1件のno-default lib testが成功。OpenFX layout offset assertions追加後のtest、ProductionDocument snapshot tests、R94/R95 tests、all-features全testはディスク不足で未実施。将来互換通知（`block 0.1.6`）はClippy warningとは別。
- R14 streaming comparison後にhistory/editorのcommit系unit test 8件が`--no-default-features --lib`で成功。serialized JSONの追加bufferは削減したが、serialization traversalの性能計測とchange-flag APIへの移行は未完了。
- この文書は広範なレビュー成果物だが、全ソースファイルを先頭から末尾まで読了したという意味ではない。実行時再現が必要な項目は指摘確度を明記し、未接続APIは製品コードの既知不具合と分けた。網羅性を装うためだけの件数追加は行わない。

## 保留中の追加指摘（初回範囲の完了後に扱う）

以下は初回レビュー範囲を完了する前に私が追加してしまった項目。記録は保持し、初回範囲R01〜R161の未完了分を先に扱う。

### R164 [P2] GPU computeはGUI rendererと別deviceを作り、各effect後にCPU readbackで同期する

`src/core/renderer.rs` のGUI rendererはeframeから受け取ったdevice/queueを共有する。一方、`src/core/compute_pipeline.rs::GpuComputeContext::new` は独自Instanceからadapter/device/queueをもう一組要求し、CPU pixel bufferをGPUへ送るcompute経路でdispatch後に `Maintain::Wait`、staging bufferのmap、全pixelのCPUコピーを行う。`src/core/cpu_effects.rs` はeffect単位でこの経路を呼ぶため、複数GPU effectを積んだ場合はGPU↔CPU往復と同期がeffectごとに入り、GUI previewの主レンダーともdevice資源が分離する。GPU演算自体がCPU fallbackより速くても、同期・転送を含む全体が速いとは限らない。opt-inで既定無効という緩和はあるが、現状を「GUIとGPU effectが一つのGPU pipelineを共有」とは扱えない。GUI device再利用、複数effectをまとめたdispatch、readback回数、実機のend-to-end計測を検討したい。


# 2026-10-04 アクティブ台帳から移した実装進捗履歴

以下は完了項目の実装経緯と当時の未検証事項を含む履歴。未完了の現状評価はアクティブ台帳の各指摘を参照する。

---

## 実装進捗

### 修正を反映した項目

- R18/R165: 現在のtoolchainで再検証し、test codeの`field_reassign_with_default` 34件をstruct initializerへ移してstrict all-target Clippyを通過。precomp rasterizer二つ、Content-Aware Fill/Roto Brush worker、骨格階層出力、inline text editor geometryを名前付きrequest/geometry/context型に整理。Wigglerは設定型ベースAPIへ移行し、CPU effect pipelineとpaint rasterizerにも共通context/style型を導入。multi-argument crate-wide抑制を除去し、残る13関数だけにlocal allowを設定。`excessive_precision`、`type_complexity`を含む全crate-wide Clippy抑制を除去。`chunks_exact_to_as_chunks`をlib/test targetの固定長pixel/audio/geometry chunk処理へ移行。2026-10-04の`cargo clippy --all-features --all-targets -- -D warnings`が成功。R18/R165の記録は保持する。
- R01: LUTをthread-localから共有ロック付きスナップショットへ変更し、CPUレンダーの並列pixel loopへ一度取得したArcを渡す。
- R02/R03: 存在しない「現在のグレード」出力を廃止し、実際に読み込まれているLUTだけを保存するUIへ変更。保存先を選択し、書込み成功後だけ成功通知する。カラーグレード自体のLUTベイクは未実装として誤認させない。
- R04: 未接続の色管理選択肢を実行可能なdropdownとして見せるのをやめ、未対応表示へ変更。
- R04追記: 実際のrenderが適用しない16/32-bit深度選択と、保存・変換されない色空間status値も除去。現在の8-bit出力と色管理未対応を表示する。指摘・過去の状態記録は削除せず、色変換機能そのものは残課題として明記。
- R05: LUT参照パスをProductionDocumentとautosaveへ保存し、プロジェクトopen/recoveryで非同期復元する。LUT本体は外部参照のままで、埋め込み・CLI共有は未実装。
- R39: 動画importの容量概算・空き容量予備・処理中監視、FFmpeg停止とUIキャンセルを追加。実素材での停止とキャンセルの確認は残る。
- R64/R65: CLI previewはProductionDocument/active compositionを保持し、再生frameは開始時刻から算出。GPU実機は未確認。
- R66/R67: 動画media referenceにJSON形式を導入し旧区切り形式を維持。Render Queue/status/exportはComposition IDで扱う。
- R06: 画像レイヤーの無動作pinボタンを実際にPosition Pinを追加する操作へ修正。異なる意味のStarch/Overlap/Advanced pinはモデルに型がないため、未実装のまま。
- R08: 組み込みワークスペース6種の誤ったpanel割当を修正。
- R07: Timelineの全追加レイヤー、重複・分割、Asset Library/Project Panel/audio-keyframe生成で既存IDを避ける `next_layer_id` を使う。
- R09: 2倍サイズが上限に入らないときはSSAA全体を無効化し、片軸だけクランプされた画像を2x downsampleしない。
- R10: 音声mix/WAV準備をUI callbackからバックグラウンドexport jobへ移す。準備時の進捗は状態表示のみで、架空の割合は出さない。
- R11/R12: CPU viewerのフレーム結果とTextureHandleを表示設定込みで保持し、exposure/LUT modeもGPUと同じ設定から描画。
- R29: PNG callbackへ渡される絶対フレームをそのまま描画し、work area開始値の二重加算を除く。
- R30: PNG/動画の各フレームでProductionDocumentのautomation snapshotを評価してからレンダーする。
- R31/R32: 音声cache keyにProjectHistory世代を加え、プロジェクト交換時に古いWAVを無効化する。work areaから切り出したWAVのseekを相対秒にし、Out frameをexclusive endへ直す。
- R29–R32: 現行コードを再確認。PNGのabsolute-frame契約、GUI export automation snapshot、音声cacheのhistory generation/Project交換時reset、work-area-relative seekとinclusive Out変換が反映済み。
- R33/R34/R35: WAV生成を絶対frame境界のsample数にし、全尺PCM保持をやめてバッファ出力。RIFF上限を先に検査し、I/O失敗をResultで伝え、export用一時WAVをjob終了時に削除。
- R36: 動画import jobへ開始時のProject session、Composition ID、選択layer IDを保持させる。完了時に別プロジェクトへ誤挿入せず、元compositionへ挿入する。session IDは全てのGUI project replacement経路で更新する。
- R42/R43: 復旧UIは同一slotのProjectとProductionDocumentを取得し、失敗時にslotを消さない。成功時は音声設定も復元してダイアログを閉じる。
- R44: autosaveをプロセスID namespaceへ分け、復旧・削除は選択した復旧元だけに限定。
- R46: ImageCacheのbyte予算を超える静止画をdecode前に拒否し、自己evictionでNoneになる曖昧な動作を避ける。
- R47: 画像取得後のCPU raster/precomp pixel loopを共有cache lockの外で実行する。画像データのArc化とclone削減は残課題。
- R48: GPU画像cacheを200枚固定から512MiBの実format byte予算へ変更し、任意HashMap順削除をLRUに置換。
- R49/R50: GPU still/video/PreComp cache keyへ外部source file revisionを含める。
- R52: ディスクフレームcacheをインスタンス固有namespaceで分離し、同じframe/versionを別プロセスが誤読する衝突を避ける。
- R53: frame disk cacheとPersistentDiskCacheの読み込みをmetadata/上限付きにし、frame writerの全pixel一時コピーを除去。
- R54: 予算を超える単一frame書込みを拒否し、evict後に実使用量を再確認。既存key置換を予算計算から除外し、process内writerを直列化。複数process間の予約は未対応。
- R55/R56/R57: graph scheduleの同順位順序を決定化し、親祖先依存を閉包へ含め、各operation variantをFlowGraphのexecution badgeへ反映。
- R59/R60: WGSL validationにNaga semantic Validatorを追加し、UIにGPU compile未確認を表示。source/diagnostic cacheをentry数と総byte数で制限。
- R63: スペクトラム帯の範囲を整数境界で分割し、入力サンプル数より帯域数が多い場合も逆転sliceを作らない。
- R62: 時間区間RMSをradix-2 FFTベースの周波数band分析へ置き換え、Hann窓・stereo downmix・frequency rangeを反映。
- R64/R65: 独立previewでProductionDocumentとactive compositionを維持してframeごとのautomation snapshotを描画し、絶対時刻から再生frameを算出。プロジェクト再読込時も再生基準を更新。Lost/Outdated GPU surfaceを再設定。
- R66: video media referenceの新規出力をJSON payloadへ変更し、Windows drive letterやコロン入りpathでフィールドがずれないようにする。既存のlegacy `video:`形式は読み込み互換性を維持。
- R70: TileCacheのLRU evictionでentry全体をcloneせず、byte長だけを一時保持する。
- R38: 動画import失敗時に作成途中の独立cache directoryをRAIIで削除し、WebP→PNG fallback前にpartial WebPを掃除して、成功時のみ一貫したcacheを保持する。
- R07: Timelineのlayer追加経路は `Composition::next_layer_id` を使い、既存IDとの衝突を避ける。コアの欠番テストあり。
- R09: SSAAの2倍寸法がレンダラー上限を超える場合は、片軸だけをクランプせずSSAA全体を無効化して等倍でレンダーする。
- R41: 画像シーケンス全フレームの寸法をdecode前に検証し、寸法不一致・上限超過・未対応decoder形式を拒否する。
- R40: VideoAsset metadataを実際に抽出される偶数寸法へ合わせ、2px未満の入力を拒否する。
- R45: Autosave失敗をtoast/logへ報告して30秒retry backoffを追加し、周期保存のserialization/disk writeもbackground化。Project snapshot cloneと実応答時間計測は残る。
- R42/R43: 保存・復旧でProjectとProductionDocumentを同一snapshotとして扱い、復旧失敗ではデータを消さず、復旧成功後に音声設定を含むdomain stateを適用する。
- R13: Color Management、Paragraph、Camera/Light Optionsの毎frame Project cloneを、編集入力時だけ作る共通pre-edit snapshotに変更。全画面共通の遅延編集transaction化は残る。
- R61: CPU shader fallbackをコメント部分一致からbuilt-in source全文一致へ変更し、UI template文字列をCPU fallback moduleと共通化。旧marker単独値は互換sentinelとして保持。
- R23: LinuxのCJK fontをLatin primary選択から分離して日本語fallbackへ登録し、WindowsにもYu Gothic/MS Gothic候補を追加。
- R23追記: macOSの`/Library/Fonts`にインストールされたLINE Seed JP、Windowsのsystem/user font directoryにインストールされたNoto Sans CJK/JPも候補へ追加。フォントがないOS環境の保証と実機glyph表示確認は残る。
- R25: PreComp child row snapshotをcomp/session/history generation/expanded set単位でcacheし、表示fingerprintが変わらないhistory editでは全child cloneを回避。表示に関係するメタデータの再走査と、変更時の全件snapshotは残る。
- R71: Cmd+S、File > Save、コマンドパレットをProductionDocument対応の共通保存関数へ集約済み。
- R21: ParagraphをKUIのPropertyGrid、共通数値入力・カラー入力・toggleへ統一。Alignment selectorは利用幅に応じて4列/2列へ切替。
- R75: Unix以外では`statvfs`を呼ばずHomeの容量表示を省く。
- R76: Renderを画面幅でreference mockと実際のexport画面へ切り替えず、responsiveな実export画面へ統一。
- R78: Project Panelのfile picker filterをdecoder/分類可能な画像・OBJ形式に合わせ、画像寸法を実ファイルから取得し、未知形式を拒否。
- R79: `Project::next_composition_id`と`next_project_item_id`を追加し、動的なUI/automation creation経路を共通発番へ集約。
- R80: Homeの設定ナビを固定値だけの装飾ページではなく、保存・適用処理を持つ共通Preferences dialogへ接続。
- R81: 新規動画/音声Assetの長さに10秒の偽値を入れず0を未取得sentinelとして保持し、Project PanelとCC Librariesで未計測を表示。偽の音声sample rate/bit depth/channel表示を削除。実probeは未実装。
- R82: 「Move selection out」の親参照判定をfolder nameではなくfolder IDで行う。
- R83: 未使用素材判定でasset display nameだけでなく、file-backed assetのmedia pathも使用中参照と照合。
- R84: 「未使用素材を削除」後に選択indexをクリアして、削除・並び替え後の誤選択を防ぐ。
- R85: tutorial practice videoのtemp pathをprocess ID namespace化し、既存ファイルのbyte contentを同梱assetと比較してから再利用。
- R92: 字形画像とキャッシュへbyte上限を設定し、未検証font bytesの拒否、同名font置換時のglyph cache無効化を追加。
- R24: TimelineColumnsでtimeline幅を左右列に分解し、property row/PreComp child/headerの幅制約を共通化。狭幅でも時間領域の最低幅を確保する単体テストを追加。全てのtimeline描画rectの統合と実画面確認は未実施。
- R25: History generationが変わってもPreComp child表示fingerprintが同じなら既存snapshotを再利用し、通常のkeyframe/transform編集で全件のString cloneを繰り返さない。子メタデータ変更時の全件走査と可視行だけの取得は残る。
- R93: Audio decoded cache keyにsource file length/mtimeを加え、同一パスの外部差し替えで古いPCMを返さないよう変更。revision差異のcache testを追加。
- R94/R95: Render Queue IDへprocess ID・nanosecond時刻・atomic sequenceを使用し、全u32 frame範囲の計算をu64へ変更。連続IDと境界frame数のunit testを追加。
- R68: Flush後も新しいRenderPipeline要求を処理するgeneration方式に変更し、待機中の古い要求だけをskipするテストを追加。テスト・型検査は未実行。
- R96/R97: OpenFX Property Suiteの関数順・layout assertionsを公式C ABIに合わせ、未完成host suiteへpluginを渡すattachをfail-closed化。偽pass-through成功を削除し、render未対応をエラー表示。テスト9件はlayout assertions追加前に成功。SDK/sample pluginとの実動検証は未実施。
- R98: 自動化script入力を1MiBに制限し、ProductionDocument validation成功後だけProjectへ反映。Expression/automation評価をworkerへ移し、同時実行を防ぎ、編集中または別Projectへ切替後に返った古い結果を破棄する。Project snapshot cloneはまだUI thread上。
- R98追記: Expression/Automation両モードにキャンセル操作を追加し、Rhai progress callbackで中断する。workerへ渡したProjectをそのまま編集して、従来のworker内二重目cloneをなくす。UI thread snapshot cloneとConsoleを離れたときのキャンセル伝達は残る。
- R99: frame binding snapshotでProject全体をcloneせず、対象Compositionだけをcloneし、結果をmoveで返す。関連testsはディスク空き不足で未実行。
- R100: 複数Compositionのbatch exportとFarm manifestで、base pathから各item専用のファイル名を同じhelperで生成。path helperのunit testを追加。
- R102/R122: Collect Filesは動画layerのframe_countを維持してProductionDocument validationを通す。実動画を開き直した際のcacheと音声再生成は未検証。
- R119/R121: Project Panelの動画Assetからのlayer追加、Image Sequence、Replace Footageをsession-aware background workerへ接続。失敗時にfallback Solidを追加しない。遅い素材での応答時間は未測定。
- R120: Image Sequence pickerの拡張子候補をcore decoderの対応形式へ一致。
- R117: 画像・動画importのRGBA換算128MiB寸法上限と、境界値テストを追加。OSの実ピークRAMは未計測。
- R125/R127: OTIO変換をfallibleにし、壊れたKagari動画参照とu32 frame range overflowをclip名付きエラーとして返す。対象unit tests 8件が成功。
- R105: MLT entryのsource in/outとplaylist内のtimeline startを復元し、inclusive outをexclusive境界へ変換。フレーム整数/timecodeを受け、timecodeを最近傍frameへ丸める。不正値・overflowを拒否。
- R104: 各layerを別playlistへ出し、in_frameをblank、重なりをmultitrack/qtblend、音声をmix transitionで表す。MLT in/outを整数frameに統一し、importは整数とtimecode双方を受け付ける。コンパイル確認済み。Shotcut/MLT実行で合成順・音声mixを未検証。
- R140: MLT exportが未対応layer typeを無言で落とさず、layer名・種類を示すerrorでfail-closed。
- R141: OpenShot JSON exportも未対応layer typeを名前・種類付きerrorで拒否。
- R142: 音声resampleは不可能な変換をerror化し誤レートPCMをcacheしない。mix/UIでの失敗理由表示は未解消。
- R143: CLI scriptをmetadata検査後の無制限readから1MiB上限readerへ変更。
- R133: Frame QCはフル解像度renderをbackground workerで行い、Project/Composition/frameが変わった結果を破棄。再監査でUIからのFrameCache readerとpixel sampler自体が現行経路にないと確認し、指摘を削除せず対象consumerなしとして閉じた。
- R116: Automation curveを1回検証した後に二分探索でsampleし、source mappingではcurve点列を走査しない。
- R144–R149: Dynamic Link接続の占有、legacy動画referenceの不正受入れ、Home UI monolith、音声decode失敗時の無音成功、設定pathのOS優先順、非UTF-8 recent pathの不可逆化を追加監査で確認。R144/R145/R146/R147/R148/R149は修正し、legacy page/layout重複はR77に継続記録。R103は標準schema serializer/parser、timeline位置・gap、Kagari metadata復元、未対応mediaのfail-closedを実装。reference implementationでの相互運用性は未確認。
- R44: process ID別autosaveに加えdirty-exit markerもprocessごとに分離し、起動時に既存markerのprocess生存状態を確認する。
- R145: legacy動画referenceの必須field・frame count・source rangeを厳格に検証し、不正値をclip名付きerrorへ変更。
- R148: OS別設定pathへ切替え、旧HOME直下設定をfallbackし、更新時に新pathへ保存する。
- R147: audio source load/resample failureをmeterへ伝播し、previewで一度だけ警告、WAV exportでは失敗として扱う。
- R149: Recent/Starred pathをOS-native bytes/wide unitsから可逆tokenへ変換し、Home/Welcomeで復号してから開く。
- R144: Dynamic Link socket処理をsingle-client workerへ分離し、listenerを応答可能に保つ。占有時は追加clientへ明示エラー、idle timeoutとnonblocking UI通知を設定。
- R124: Dynamic Link handshakeでsemver-shaped versionをparseし、protocol major 1以外を明示拒否。
- R21/R33/R34/R36/R37/R40/R41/R76/R81: 一部の現行実装を再確認したが、R21/R36/R37/R76/R81は本文に古い挙動の記述が残っているため、この進捗行だけでレビュー記録を更新済みとは扱わない。各項目の現状説明を再照合する。R35はsource errorのWAV失敗化まで進めたが、`None`のAPI意味分けと全caller統一は未完了。
- R35: 音声mix結果で有効source数を返し、無音WAVの誤生成をなくした。再生側はno-audio結果をcacheし、条件変更時に前の一時WAVを削除する。
- R150: Recovery snapshotとファイル妥当性確認の全読込を100MiB bounded readerへ統一。R151はSfM風の偽解を返す経路を廃し、未実装errorへ変更。R152は512MiB RAM ceiling/FIFO evictionを追加したが未接続で、弱いhashとLRU不足が残る。R153は偽のAI load/inference成功を未実装errorへ変更。R154は偽mergeとunbounded packet historyを除去。R155はメモリ内timeline storeへ改名しframe-range indexを導入したが、未接続・非永続でrecord上限なし。R156はPreComp内の音声もexport判定へ含めた。
- R155追記: repository内にconsumerがない別storeを削除し、現行Project keyframe + Timeline generation cacheを正本として明記。レビュー指摘は台帳に保持。外部利用者が公開moduleをimportしていた場合の互換性影響は残る。
- R103: OTIOの標準schema serializer/parser、timeline gap、Kagari metadata、非対応referenceの明示拒否を実装。reference implementationとの外部相互運用確認は未実施。
- R142: audio preview mix失敗を一度toastで通知し、同じstateで毎frame再試行するのを防止。
- R157: 3D camera solveのtracking入力不整合・非有限値を拒否し、一意frame数を100,000へ制限。
- R158: Motion Sketch記録を8192点以下へ保ち、同一frame sampleを集約。Douglas-Peuckerを反復化し、ベイク入力を4096点へ制限。UI同期処理時間・間引き品質の確認は残る。
- R159/R160: AEPの偽成功をunsupported errorへ変更。動画decoded cacheは全frame indexと内容fingerprintをmanifestで検査し、旧cacheはdecode検証後にmanifestへ移行する。全lib testで関連テストを含む1,399件が成功し、大規模cacheの検証I/Oは未計測。
- R123: Timelineのderived dataをgeneration cacheし、親名lookupをO(1) index化。ドラッグ中のsnap edge走査と大規模実測は残る。
- R126/R128/R129: 階層cycle validation、unsorted keyframe受入れ、precomp cycle時のtree再検索を修正。ProductionDocument test 29件成功。
- R111: 動画importのFFmpeg stdoutを破棄し、stderrを継続drainしつつ64KiB上限で保持。実素材でのmemory計測は未実施。
- R101: ProductionDocument::loadを上限付きfile reader経由にし、metadata検査後にファイルが増大しても読込量を上限+1 byteに制限。
- R45: 周期autosaveのJSON化・streamed file writeをworker化し、generation追跡で保存中の後続編集をdirtyのまま保持。終了時flush、ProductionDocument内のProject重複保存削減、旧autosave形式の互換テストを追加。UI側Project snapshot cloneは残り、実画面の停止時間は未計測。
- R86: Replace Footageの選択filterをAsset型別に制限し、分類結果の型が一致しない場合を拒否。画像置換では実寸metadataも更新。
- R87: footage置換時は旧path完全一致で対象layerを決め、同名のSolidや別directoryの同名素材を維持。Model3D pathも更新する回帰テストを追加。
- R88: Audio Spectrumのeven-length判定を`usize::is_multiple_of`へ置換。
- R89: headless GPU rendererでtexture上限超過を事前拒否し、readback target寸法とbyte長の計算を検証するよう変更。all-features Clippyで型検査済み。実GPU確認は未実施。
- R90: Time Remap Loop bakeを整数計算と100,000 key上限へ変更し、1frame sourceを1キーへ縮約。no-default-featuresの対象unit test 2件は成功。
- R91: Home設定のatomic write失敗を`Result`で伝え、Preferences/Home/workspace/プロジェクト開閉の各呼び出し側で通知。all-features Clippyで型検査済み。実書込み障害の注入テストは未実施。
- R37: プロジェクトread/parse/relink/欠損video cache準備を専用workerへ移し、読み込み中表示と二重読込防止を追加。処理中のProject変更を検知した場合に読込結果を破棄し、既存状態を保護する。
- R73: Mocap入力を`frame_<number>.png`に限定し、フレーム番号を数値順に並べ、番号重複を拒否する。
- R72: CLI renderでduration/range/dimension/byte budget/exposure/LUT modeをbackend起動前に検証し、PNG/MP4/GIFで不正範囲の成功扱いや余分な1フレーム出力を防ぐ。
- R67: Render Queueとexport statusの識別を表示名からComposition IDへ移し、同名・名称変更による出力先の誤選択を防ぐ。
- R14/R15: history byte estimatesをentry単位で差分更新し、JSON serialization長に基づく可変データ込みの予算proxyへ変更。`commit_action`はcurrentだけをJSON buffer化し、nextはbufferを作らず比較・サイズ計測する。二重のserialization traversal、2倍係数の実測校正、Rust heap実使用量の厳密な上限は残る。
- R74: headless GPU rendererの公開条件を`cli-gpu`へ絞り、通常CLIとGUI+CLIのfeature組合せからpollster参照を切り離す。

これらはコード上の経路を修正した段階。GUIターゲットのcargo checkを複数回通過。CLI通常・headless GPU・GUI+CLI feature組合せのcargo checkも通過。LUTのファイル内容と書込み失敗、音声スペクトラムの短尺入力、ディスクcache/復旧の複数プロセス分離、出力画素の一致、音声同期、復旧UI操作、非同期import切替、巨大Projectの読込応答性、R72/R73の不正入力を実コマンド・実素材で確認していないため、実行時まで検証済みとは扱わない。

- CLI mocapは現在track_markerless_motion_streamへiteratorで画像を渡す。以前の「全decoded RGBAを保持」は現在当たらない。path一覧と出力track samplesの保持は残るが、別の話。
- `project_io::save_project_to_path` はProductionDocument::save_atomicを使い、Cmd+S/File > Save/コマンドパレットもこの共通入口を呼ぶ（R71修正済み）。Collect Filesの経路は別問題としてR102に記録。
- Dynamic Linkは127.0.0.1へbindし、受信行を64KiB、read timeoutを10秒に制限している。外部ネットワークから直接接続できるという指摘は確認できなかった。固定port競合とクライアントを直列処理する設計は別途残る。
- FFmpeg出力にはTemporaryOutputGuard/ChildGuardがあり、失敗途中の動画や子processを管理している。PNG/image-sequence importと同等と推測しない。
- graph compilerのHashSet順は現状の可視化の問題。実合成がこのscheduleを使う証拠はない。
- GPU画像cacheは512MiB byte budgetとLRU evictionへ変更済み。device adapter別のメモリ予算反映と、実GPUでのVRAM計測は未実施。
- 第2回はR29〜R70の42件を追加した。P1/P2/P3および未接続APIを含み、すべてが同じ深刻度・再現確度という意味ではない。
- 第3回では、プロジェクト保存入口、スキーマ判定、ショートカットの呼び出し元を照合し、R71を追加した。これで実行中の保存経路の不一致もレビュー範囲へ含めた。
- 第9回は動画/シーケンスimportとRAM/disk frame cacheを再監査し、各import経路をsession-aware worker化、欠番・重複frame番号の誤圧縮を拒否、旧世代frame cacheのdeep cloneを除去、cache budget対象を永続frame fileに限定して古いtemp fileを掃除するよう変更。Dynamic Linkの未対応要求はerror応答(R113)へ変更。Frame QCのcache readerとpixel sampler writer不足をR133として記録したが、後続監査では両consumerが現行UIにないことを確認し、R133へその事実を追記した。import応答時間と実機フレームcache連携の確認は残る。
- R51: PreComp cache hitの全画素cloneとComposition全体JSON化を除去し、共有Arcと編集世代・外部media revisionによるinvalidationへ変更。revision世代のunit test成功。
- R54: 既存disk frame置換時の容量二重計上を修正し、process内mutexとdirectory単位のOS file lockでKagari writerを直列化。別processとのlock競合testを含むdisk-cache test 4件とstrict no-default Clippyが成功。
- R102: OBJ→MTL→texture依存を収集しbundle内参照へ書換え。既存asset/videoを含む対象test 3件とstrict no-default Clippyが成功。実素材検証は未実施。
- R158: Motion Sketch bakeをbackground worker化し、Project session/history generation/Composition/Layerを確認後にundoable editとして適用。GUI cargo check、対象test、strict GUI lib Clippy、および現行変更を含むno-default lib test 1,240件が成功。最大入力の体感計測は未実施。
- R146: Home rootを約1,860行にし、各画面群を責務別moduleへ移動。GUI test targetを含むcargo checkとstrict GUI lib Clippyが成功。legacyページ重複はR77に保持。
- 第4回では新規KUI、レイヤーコマンド、共有FlowGraphモデル、headless GPU readback、Time Remap Loop、Home設定を確認しR89〜R91を追加・実装。第5回はTimeline geometry、PreComp child cache、font rasterizer、audio decoded cache、Render QueueからR92〜R95を追加・修正。第6回はRenderPipeline、OpenFX、Automation console、ProductionDocument frame snapshotからR96〜R99を追加。第7回は既存指摘の現行コードを再照合してR07/R09/R29〜R32の修正を反映し、Render Queue batchの出力先衝突をR100として追加・修正した。
- 第7回の変更後は `cargo check --all-features --tests` が成功。Clippyはmetadataを書き出す時点でディスク満杯となり完了していない。全テストと実FFmpeg batch出力確認も未実施。
- R90対象2件、font validation、audio source revisionのno-default unit testは成功。Timeline geometryの独立unit test 2件も成功。第6回はR99変更後を含むall-features `cargo check --tests`とClippyが成功。OpenFX 9件、自動化11件、RenderPipeline 1件のno-default lib testが成功。OpenFX layout offset assertions追加後のtest、ProductionDocument snapshot tests、R94/R95 tests、all-features全testはディスク不足で未実施。将来互換通知（`block 0.1.6`）はClippy warningとは別。
- R14 streaming comparison後にhistory/editorのcommit系unit test 8件が`--no-default-features --lib`で成功。serialized JSONの追加bufferは削減したが、serialization traversalの性能計測とchange-flag APIへの移行は未完了。
- この文書は広範なレビュー成果物だが、全ソースファイルを先頭から末尾まで読了したという意味ではない。実行時再現が必要な項目は指摘確度を明記し、未接続APIは製品コードの既知不具合と分けた。網羅性を装うためだけの件数追加は行わない。


# 2026-10-04 旧・修正優先順位案

台帳整理時点では現行の進行計画ではないため、履歴として保存する。

---

## 修正に進む場合の優先順位案

1. R01/R02/R07/R09：結果を誤らせる処理。
2. R04/R06/R08：表示される操作と実機能の一致。
3. R10〜R15：クリック応答とメモリ。代表プロジェクトで計測を追加。
4. R19/R21/R24：Inspector・Effect Controls・Timelineの共通構造。
5. R22/R23：言語・フォントの一本化。
6. Home責務分割、意味ベースのテーマ、独自部品の操作契約。

これはレビューに基づく提案であり、修正の開始や指示範囲の追加を意味しない。


# 2026-10-04 R23 解決判定

### R23 [P2・未完了／Linuxの旧記述を訂正] 日本語フォントの確保がOSごとに不均等

現行`configure_fonts`はLatin用フォント候補と日本語fallback候補をOSごとに別配列で探索し、macOSのHiragino、LinuxのNoto CJK、WindowsのYu Gothic/MS Gothicがあれば比例フォントfamilyのfallbackへ追加する。したがって「LinuxのDejaVuで先にbreakしNoto CJKを見ない」という以前の記述は誤りで、別ループで日本語候補を探索している。誤記は取り消さず監査履歴として明示した。

フォントはOSにインストール済みであることを前提とする。追加済み候補が存在しない環境では日本語字形を保証できず、各OS実機で日本語glyph fallbackとUIを確認していないためR23は未完了。フォント同梱はライセンス・配布サイズの判断が必要なので、この変更では行わない。

LinuxではLatin primary font選択からNoto CJKを分離して後段fallbackへ登録し、WindowsはYu Gothic/MS Gothicのfallback候補を追加した。今回macOSの`~/Library/Fonts`とLinuxの`$XDG_DATA_HOME/fonts`、`~/.local/share/fonts`、`~/.fonts`を日本語fallback探索へ追加した。現在のmacOS環境では`~/Library/Fonts/NotoSansJP-Regular.ttf`を確認し、新候補から読み込める構成になった。Linux cross-checkはALSA依存の`pkg-config`にLinux sysrootがなく、KagariのLinux target compileへ進む前に失敗した。GUIでの日本語字形表示と、フォントのないmacOS/Linux/Windows環境は未確認。

判定: macOS/Linux/Windowsの候補パスを現行コードで確認し、このmacOS環境では`~/Library/Fonts/NotoSansJP-Regular.ttf`が存在する。R23で指摘された日本語フォントfallback候補不足はコード上解消したものとしてアクティブ台帳から移す。日本語glyphの画面描画とWindows/Linux実機、対応フォントがない環境のfallback表示は未検証であり、製品リリース前の視覚確認事項として残す。


# 2026-10-04 R26 解決判定

### R26 [P2・主要state tokenを整理／全使用箇所の移行は残る] テーマの色の意味が実使用と矛盾する

themeにselection/workspace/action/warningのsemantic tokenを導入し、selection stroke、widget active border、KUI primary action、workspace preset selected state、Timeline selected tab/keyframeへ割り当てた。選択背景もブランド色系の低彩度色へ揃え、warning表示は`ACCENT_WARNING`経由にした。`ACCENT_TOGGLE`/`ACCENT_PROGRESS`/`ACCENT_BEAT_MARKER`/`ACCENT_INFORMATION`/`BORDER_FOCUS`を追加し、手描きtoggle、進捗bar、beat marker、通知・診断の情報色、active widget borderへ割り当てた。Timeline layer selection rail、toolbarのactive tool、Inspectorのkeyframe-active、Homeのselected asset、Preferences categoryも`ACCENT_SELECTION`/`BG_SELECTION`系に統一。Render操作は`ACCENT_PRIMARY_ACTION`を使い、audio peak meterは用途別の`METER_WARNING`を使う。Timeline playheadは専用の`TIMELINE_PLAYHEAD`、motion/guidesはcyanを使う。Viewerのactive composition/tab/control、Inspectorの選択カテゴリ、toolbar tool state、Timelineのboolean switches、選択中Automation binding、Automation key marker、shortcut referenceの情報表示も用途別tokenへ移行済み。今回Frame QCの注意・情報・対象フレーム色もwarning/information/playhead tokenへ接続した。旧色定数も後方互換のため残す。全画面で色名直参照をなくす作業と、色の視覚コントラスト実機確認は未完了なので部分対応。

判定: selection/workspace/primary action/warning/information/progress/timeline playheadなど、製品状態を示す主要色には用途別semantic tokenが導入され、主要なUI stateへ割り当てられた。残るACCENT_BLUE/CYAN/GREEN等は、グラフ・gizmo・meterなどで色自体の役割を表す描画にも使われ、全参照をsemantic alias化することは元の指摘解消に必須ではない。テーマの意味衝突という実装上の主問題は解消としてアクティブ台帳から移す。全画面のコントラストを実機で確認する作業は製品視覚QAの残件として記録する。
