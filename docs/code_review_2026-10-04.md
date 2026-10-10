# Kagari コードレビュー — 2026-10-04

> 完了済み項目の本文と履歴は[完了項目アーカイブ](code_review_2026-10-04_archive.md)に全文保存しています。ここには未完了・部分対応・確認待ちの指摘を残しています。

## 今回の区切り

初回レビュー範囲の実装確認・修正・記録整理は今回ここで一区切りとする。以下の10件（R14、R15、R16、R17、R22、R24、R28、R98、R103、R123）は未解決または部分対応のフォロー項目として残し、修正済みとは扱わない。GUIの画面確認、OS別フォント表示、長時間処理の性能測定、外部OTIO相互運用などの環境依存検証も残る。完了項目の本文と修正履歴はアーカイブに保存した。

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

ただし翻訳カタログと呼出し側の移行は部分的で、全workspace、menu、dialog、error/toast、effect parameterの英語直書きを網羅していない。Audio Mixerの表示は英語のsource keyを使うよう統一し、見出し、操作、tooltip、チャンネル状態、voice correctionのproperty labelsとkeyframe生成操作を共通翻訳関数へ接続した。画面内に残っていた日本語ハードコード表示は取り除いた。Asset Libraryもsection/title/search hint、preset名とtooltip、asset type・未計測状態を共通翻訳へ接続し、KUIのtext field hintも翻訳する。言語別のpreset検索も表示名に対して動作する。Color Managementではラベル、未対応状態、ツールチップ、LUT読み書き通知を共通カタログへ接続し、日本語直書きをなくした。Custom Shader画面の見出し、テンプレート、検証状態、CPU fallback説明、ファイル操作、パラメーター表示と、Export Dialogの出力設定、進捗、完了・失敗通知も共通翻訳へ接続した。今回Render Queueの操作、状態、進捗、時間範囲、設定表示とツールチップを共通カタログへ追加し、幅の狭い場合は操作ボタンを折り返す。さらにFrame QCの見出し、操作、計測値ラベル、診断状態、既知のエラーと通知をカタログへ追加した。Metadataパネルもコンポジション情報、レイヤー統計、レンダー設定、書き出し情報の項目名を共通カタログへ接続した。Vectorscopeのウィンドウ名、表示モード、空データ状態も共通翻訳へ接続した。Time Remapパネルの操作、モード、説明、通知とループ生成エラーも翻訳カタログへ接続した。Proxy設定の状態、解像度、MFR説明、RAMキャッシュ操作と通知も翻訳カタログへ接続した。Markerパネルの追加・移動・削除操作、一覧、初期ラベルと通知も共通翻訳へ接続した。今回Trackerの見出し、トラッカーポイント操作、マーカーレス設定、既知の解析状態・結果通知も翻訳カタログへ接続し、TrackerPointの保存名や内部IDは変更していない。レンダージョブmanifestのstatus/schema等の内部値は英語のままにして表示翻訳と分離した。codec名やFFmpeg由来の動的診断文など技術固有の文字列は原文表示が残る。OSごとのlocale取得・実フォント・日本語/英語の全画面レイアウトを実機では確認していない。全workspace・menu等の未移行が残るためR22は未完了。

今回、Trackerの3Dカメラ・平面・4点トラッキングと適用操作の表示、ツールチップ、通知も共通カタログへ接続した。TrackerPointの保存名や内部IDは変更していない。

Content-Aware Fill画面も方法・範囲・進捗・操作・完了通知と、既知の静的エラーを共通カタログへ接続した。フレーム番号やOS I/O由来の動的診断は原文になる場合がある。

### R24 [P2・可視幅共有とproperty/precomp行の矩形共通化／通常layer row・GUI実測は残る] タイムライン共通geometryはまだ高さ定数3個だけ

`timeline/layout.rs`に`TimelineColumns`を導入し、利用可能幅・egui列間隔・要求された左列幅から左列と時間軸幅を一度に計算する。通常property row、PreComp child row、およびtimeline headerのリサイズ制約で共用。ヘッダールーラーと通常レイヤーの時間軸バーも同じ計算結果を使う。今回、Timeline rowの名前欄でローカルに計算していたswitch控除幅と40–220pxの名前幅をlayout moduleへ集約し、極端に狭い場合に最小幅40pxがavailable領域からはみ出す動作を修正した。さらにproperty rowとPreComp child rowは左右それぞれの利用可能矩形を共通の`allocate_columns`から取得する。列幅と名前欄算定のunit testsは既存。通常layer rowのname/switch/time partitionは異なる入れ子構造のままで、同じ矩形モデルには未統合。GUI実測も未実施。

### R28 [P3・共通Painter部品とHomeの主要ボタンを改善／独自interaction全体と支援技術検証は残る] 手描きclick領域には操作の意味を付与する規約が不足

`custom_widgets`のSVG button/toggle・text toggle・switch・color swatchに、透明な標準egui Buttonを操作面として使う共通経路を入れた。描画矩形は同じbutton rectを使い、tooltip/visible labelをwidget metadataへ渡すため、標準Buttonのfocus、Enter/Space activation、enabled stateを維持する。Homeのaction/CTA/select/icon/documentation card、Timeline headerのtransport/view tabsも同経路へ移行した。Timelineのtimecode/keyframeやHomeのその他の`interact`領域、全操作のdisabled contractは未統一。VoiceOver等の支援技術による実検証も未実施。

## 再指摘しない改善済みの点

- RightPanelはenum化済み。保存JSONの旧index互換は残るが、旧right_tab_idx問題として報告しない。
- Expressionのcompileにはソース変更時のcacheがある。毎frame compileの指摘は現在のコードに当たらない。
- PreComp描画には可視範囲チェックがある。全行をPainterで描く実装とは言わない。
- Timeline本体はresizable(true)。固定の断定はしない。ただし最小160/176pxがあり、実際にどこまで下げられるかは実機確認が必要。
- CPU専用CLIとcli-gpuのfeature分離がある。wgpu常時必須という旧指摘は現在当たらない。
- KUIはフォント実測のPropertyGrid、狭幅stack、vector、数値・単位、disabledボタンを備える。資産はある。

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

## 履歴

完了項目の実装進捗と検証記録は[アーカイブ](code_review_2026-10-04_archive.md)へ移動した。未完了の項目と残課題は上記の個別指摘に記載する。

## 保留中の追加指摘（初回範囲の完了後に扱う）

以下は初回レビュー範囲を完了する前に私が追加してしまった項目。記録は保持し、初回範囲R01〜R161の未完了分を先に扱う。

### R164 [P2] GPU computeはGUI rendererと別deviceを作り、各effect後にCPU readbackで同期する

`src/core/renderer.rs` のGUI rendererはeframeから受け取ったdevice/queueを共有する。一方、`src/core/compute_pipeline.rs::GpuComputeContext::new` は独自Instanceからadapter/device/queueをもう一組要求し、CPU pixel bufferをGPUへ送るcompute経路でdispatch後に `Maintain::Wait`、staging bufferのmap、全pixelのCPUコピーを行う。`src/core/cpu_effects.rs` はeffect単位でこの経路を呼ぶため、複数GPU effectを積んだ場合はGPU↔CPU往復と同期がeffectごとに入り、GUI previewの主レンダーともdevice資源が分離する。GPU演算自体がCPU fallbackより速くても、同期・転送を含む全体が速いとは限らない。opt-inで既定無効という緩和はあるが、現状を「GUIとGPU effectが一つのGPU pipelineを共有」とは扱えない。GUI device再利用、複数effectをまとめたdispatch、readback回数、実機のend-to-end計測を検討したい。
