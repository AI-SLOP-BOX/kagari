# Kagari VFX システム総合解体新書 (System Reference & Capabilities)

Kagari VFX（かがり）は、Rustで構築されたAfter Effectsスタイルのレイヤー型モーショングラフィックス＆コンポジティングエンジンです。
GUI編集（`kagari-studio`）とヘッドレスCLI自動化（`kagari`）の双方を第一級市民として設計されています。

このドキュメントは、「この機能はあるのか？」「どこまで実装されているのか？」「どのファイルにあるのか？」を開発者およびAIエージェントが一目で把握できるようにまとめた包括的リファレンスです。

---

## 1. アーキテクチャ概要と設計哲学

```
                        ┌───────────────────────────────┐
                        │        Project JSON           │
                        │ (Composition, Layers, Params) │
                        └───────────────┬───────────────┘
                                        │
                  ┌─────────────────────┴─────────────────────┐
                  ▼                                           ▼
      ┌─────────────────────────┐               ┌─────────────────────────┐
      │     CPU Software        │               │       GPU (wgpu)        │
      │   Reference Pipeline    │               │    Preview Pipeline     │
      │ (100% Deterministic)    │               │ (Metal/Vulkan/DX12)     │
      └───────────┬─────────────┘               └───────────┬─────────────┘
                  │                                         │
        ┌─────────┴─────────┐                               │
        ▼                   ▼                               ▼
  [ Headless CLI ]   [ Viewport (Fallback) ]      [ Viewport (Hardware) ]
   PNG/EXR/MP4/GIF    Unsupported FX & Draft       Real-time 60fps Playback
```

### デュアルレンダリングパイプライン
1. **CPUソフトウェアレンダラー（基準実装・完全決定論）** (`src/core/software_renderer/`)
   - プラットフォーム・GPU環境に一切依存せず、1バイトの狂いもなく完全同一の画を出力するリファレンスレンダラー。
   - Rayonによるマルチスレッド並列レンダリング（MFR: Multi-Frame Rendering）に対応。
   - すべてのエフェクト（v1〜v30）、SDFシェイプ、テキストラスタライザ、OBJ 3Dモデル、調整レイヤー、トラックマットが動作。
2. **GPUプレビューレンダラー（wgpu）** (`src/core/renderer.rs`, `src/core/shader.wgsl`, `src/core/compute_blur.wgsl`)
   - ビューポート再生時の超高速リアルタイム表示用。macOSではMetal、WindowsではDX12/Vulkanで動作。
   - 未サポートの高度エフェクトや複雑なパス・パペット変形が含まれる場合は、安全にCPUプレビュー（`render_frame_to_pixels_preview`）へフォールバックします。

---

## 2. Cargo.toml & 依存クレート役割一覧

| クレート名 | バージョン | 役割と用途 |
|---|---|---|
| **AmeUI** | local path | `kagari-studio` の既定UI。画面定義は `src/ui/ame/kagari_studio.ame`、Rustホストは `src/bin/ame_studio.rs` |
| **eframe / egui** | 0.29 | 旧Studioの任意起動バイナリ `kagari-studio-egui`。コードは `src/legacy_egui/` に保持 |
| **wgpu** | 22 | Metal / Vulkan / DX12 バックエンドのGPUコンポジティング＆シェーダー実行 |
| **rhai** | 1.18 | AEライクなプロパティエクスプレッション言語（サンドボックス実行・安全なループ上限付き） |
| **rayon** | 1.8 | CPUエフェクト処理およびMFR（複数フレーム並列書き出し）のためのマルチコア並列スイープ |
| **symphonia** | 0.5.5 | 音声デコード（MP3, AAC, FLAC, WAV, OGG, AIFF, ALAC, CAF, MKV, MP4コンテナ） |
| **rodio** | 0.19 | GUIプレビュー時のリアルタイムオーディオ同期再生 |
| **ab_glyph** | 0.2 | テキストレイヤー用フォント読み込みおよびベクターグリフ・ラスタライズ |
| **image** | 0.25 | JPEG, PNG, WebP, EXR, TIFF などの静止画読み込み・プロキシキャッシュ |
| **flate2** | 1 | OpenEXRのZIP圧縮（VFXインターチェンジ書き出し用） |
| **naga** | 0.19 | ユーザー定義WGSLカスタムシェーダーの構文検査・型検証 |
| **libloading** | 0.8 | OpenFX（OFX）外部プラグインのダイナミックロード（Host ABI Probe） |
| **rfd** | 0.14 | OSネイティブのファイル選択・保存ダイアログ（macOS / Windows / Linux） |
| **clap** | 4 | ヘッドレスCLIコマンドライン引数パース |

---

## 3. レイヤータイプ（LayerType）全一覧と対応状況

すべてのレイヤーは `src/core/timeline/layer.rs` の `LayerType` で定義されています。

| レイヤー種別 | 状態 | 担当モジュール | 特徴・詳細機能 |
|---|:---:|---|---|
| **Solid** | ✅ 完全 | `core/timeline`, `software_renderer` | 単色平面レイヤー。サイズ・カラー・不透明度・ブレンドモード完全対応。 |
| **Text** | ✅ 完全 | `core/text_layout`, `core/font_rasterizer`, `core/text_animator` | フォントファミリー、サイズ、文字詰め（Tracking）、行送り（Leading）、揃え、パス上テキスト（Text on Path）、文字単位の範囲セレクターアニメーション（Text Animator）。 |
| **Shape** | ✅ 完全 | `core/sdf`, `core/shape_repeater`, `core/shape_modifiers` | 矩形、楕円、星型、多角形、ベジェパス。SDF（Signed Distance Field）描画による超高品質アンチエイリアス、ブーリアン演算（MergePaths）、シェイプリピーター、押し出し（Extrusion）。 |
| **Footage (Image / Video)** | ✅ 完全 | `core/image_cache`, `core/video_import`, `core/proxy` | 静止画（PNG, JPG, WebP, TIFF, EXR）および動画（連番フレーム抽出・キャッシュ）。フレームブレンド（オプティカルフロー風補間）、プロキシ解像度対応。 |
| **PreComp (プリコンポ)** | ✅ 完全 | `core/software_renderer/precomp`, `core/precomp_cache` | ネストされた別コンポジションの再帰レンダリング。エッセンシャルプロパティによるパラメータオーバーライド、コラプス変換（Collapse Transformations）。 |
| **Adjustment Layer (調整レイヤー)** | ✅ 完全 | `core/software_renderer/composite` | 下位レイヤー全体に対してエフェクトスタックを適用（背景アルファを破壊しない非破壊合成）。 |
| **Camera3D / Light** | ✅ 完全 | `core/camera_dof`, `core/shadow_map` | 3D空間カメラ（画角、焦点距離、F値、被写界深度DOF、ボケ円形状）、平行光・ポイントライトによるシャドウマップ投影。 |
| **Model3D** | ✅ 完全 | `core/obj_loader`, `core/model3d_raster` | Wavefront OBJ 3Dモデルファイルのインポート、3Dパスペクティブ投影ラスタライズ。 |
| **Particle** | ✅ 完全 | `core/particle_system`, `core/advanced_particles_3d` | 3Dパーティクルエミッター（重力、風、乱流、引力フォース、ライフタイム、カラーグラデーション）。 |
| **Audio** | ✅ 完全 | `core/audio_engine`, `core/audio_dsp`, `core/audio_spectrum` | 波形プレビュー、Symphoniaデコード、EQ・リバーブ・コンプ、オーディオスペクトラムビジュアライザ。 |
| **Null** | ✅ 完全 | `core/timeline`, `core/parenting_engine` | 親子付けの基準アンカーとして機能する非描画レイヤー。 |

---

## 4. エフェクトシステム（VFX）の全貌

Kagariは標準で膨大なCPU/GPUエフェクト群を内蔵しています（`src/core/ae_effects_pack*.rs`）。

### 主要エフェクトカテゴリ
- **ブラー＆シャープ**: GaussianBlur, FastBoxBlur, DirectionalBlur, RadialBlur, CameraLensBlur, BilateralBlur, ChannelBlur, UnsharpMask
- **ディストーション＆変形**: DisplacementMap, TurbulentDisplace, Twirl, Ripple, Spherize, PolarCoordinates, WaveWarp, Mirror, CornerPin, Transform, MotionTile, Offset, BendWarp, PageTurn, Bulge, OpticsCompensation
- **グリッチ＆シグナル**: BadTV, GlitchDisplacement, VenetianBlinds, LinearWipe, Scanlines, VHSArtifacts, Pixelate, Posterize, Dither
- **カラー＆トーン**: Levels, Curves, ColorBalance, HueSaturation, Vibrance, Tint, Invert, Threshold, Posterize, Colorama, Tritone, Exposure, BlackAndWhite, PhotoFilter, LUT (3D .cube)
- **スタイル＆ジェネレーター**: DropShadow, Glow, OuterGlow, Stroke, Fill, Checkerboard, GradientRamp, 4ColorGradient, FractalNoise, CellNoise, LightRays, LensFlare, OpticalFlare, LightningBeam
- **キーイング＆マット**: ChromaKey, ColorKey, LinearColorKey, DifferenceMatte, MatteChoker, SimpleChoker, SetMatte, AlphaCleaner
- **時間＆シネマ**: Echo, PosterizeTime, Timewarp (Optical Flow), FrameBlending, FilmGrain, Vignette, CCBloom

---

## 5. 高度なプロフェッショナル機能＆GUI専用パネル

「こんなプロ機能まであるのか」と驚かれる、GUIおよびコアに接続済みの専門パネル群です。

### ① アニメーション＆エクスプレッション
- **スプライングラフエディタ** (`src/legacy_egui/graph_editor.rs`):
  - ベジェ曲線の速度グラフ（Speed Graph）と値グラフ（Value Graph）の直接編集。
  - イージングプリセット（Easy Ease / F9, Ease In, Ease Out, Exponential）。
- **Rhaiエクスプレッションエンジン & スクリプティングコンソール** (`src/core/expression_engine/`, `src/legacy_egui/scripting_console.rs`):
  - `time`, `value`, `wiggle(freq, amp)`, `loopIn()`, `loopOut()`, `linear()`, `ease()` などのAE互換関数群。
  - スクリプティングコンソール（ExpressionテストおよびAutomationバッチ実行）。
- **The Wiggler & The Smoother パネル** (`src/legacy_egui/the_wiggler_panel.rs`, `src/legacy_egui/the_smoother_panel.rs`):
  - キーフレームに対するジッター付加とRDPアルゴリズムによる不要キーフレームの間引き・平滑化。
- **モーションスケッチ (Motion Sketch)** (`src/legacy_egui/motion_sketch_panel.rs`):
  - ビューポート上でマウスをフリーハンドドラッグすると、再生時間と同期して直接座標キーフレームがリアルタイム記録される。
- **2D物理演算＆バウンサー (Physics & Dynamics)** (`src/legacy_egui/physics_panel.rs`, `src/core/physics/`):
  - 重力・反発係数（Bounciness）・摩擦・床との衝突判定シミュレーションを行い、レイヤーのPositionキーフレームに一括ベイク。

### ② ロトスコープ・ペイント・変形
- **Roto Brush Engine** (`src/core/roto_brush_engine/`, `src/legacy_egui/viewport/roto_brush.rs`):
  - 前景・背景ストロークによるオブジェクト自動切り抜き、オプティカルフローによる時間伝播（Propagation）。
- **Puppet Warp (パペットツール)** (`src/core/puppet_engine.rs`, `src/core/puppet_warp.rs`):
  - ドロネー三角形分割メッシュ生成、変形ピン（Deform Pins）による2Dキャラクタアニメーション。
- **ペイント＆クローンスタンプ** (`src/core/paint_engine_pro.rs`, `src/legacy_egui/paint_panel.rs`):
  - レイヤー上への直接ブラシペイント、消しゴム、オフセットサンプリングクローン。
- **Content-Aware Fill** (`src/core/content_aware_fill.rs`, `src/legacy_egui/content_aware_fill.rs`):
  - 不要オブジェクトをマスクで囲み、周囲のテクスチャや時間方向の情報を利用して自動除去。

### ③ トラッキング＆カメラ
- **2D Point / Quad / Planar Tracker** (`src/core/tracker_engine/`, `src/legacy_egui/tracker_panel/`):
  - パターンマッチング追従、Corner Pin（コーナーピン）へのキーフレームベイク、手ブレ補正（Stabilize）。
- **3D Camera Tracker & Blender連携** (`src/core/camera_track.rs`, `src/legacy_egui/menu/file_edit.rs`):
  - 特徴点検出とカメラ動きの3D空間推定。
  - **Blender Camera Track (.json) インポート機能**: Blenderでトラッキングしたカメラデータをコンポジションの3Dカメラキーフレームとして直接ベイク読み込み可能。

### ④ カラーマネジメント＆Lumetriカラー
- **Lumetri Color 統合パネル** (`src/legacy_egui/lumetri_color/`):
  - Basic Correction（Input LUT, 3-wayカラーホイール/DragValue）、Curves、Color Wheels、HSLセカンダリ、Vignette。
- **OCIO / ACES / ICC** (`src/core/ocio_color.rs`, `src/core/aces.rs`, `src/core/icc_color_engine.rs`):
  - 32bit浮動小数点（Float32）リニアカラースペース、ACEScg、sRGB/Rec.709変換。
  - 波形モニター（Waveform）、RGBパレード、ベクトルスコープ（Vectorscope）、ヒストグラム。

### ⑤ コンポジション構造・インターチェンジ
- **フローチャートビュー (Comp & Layer Flowchart View / Shift+F3)** (`src/legacy_egui/flowchart.rs`):
  - コンポジション階層、ネストPreComp、トラックマット関係、循環参照（Dependency Cycle）の検出をノードグラフ形式で可視化・パン＆ズーム操作。
- **OpenTimelineIO (.otio) & MLT XML インポート/エクスポート** (`src/legacy_egui/menu/file_edit.rs`):
  - Kdenlive, Shotcut, Premiere, DaVinci Resolve とのタイムライン構造インターチェンジ。
- **字幕・キャプションインポート/エクスポート** (`src/legacy_egui/menu/file_edit.rs`, `src/legacy_egui/menu/composition.rs`):
  - `.srt` / `.vtt` ファイルを読み込み、タイムコードに沿ったテキストレイヤーを自動一括生成。逆にコンポジション内のキャプションをSRTとして書き出し。
- **ファイル収集 (Collect Files)** (`src/legacy_egui/menu/file_edit.rs`):
  - AEの「ファイルの収集」と同様、プロジェクトが参照しているすべての映像・音声・画像・3Dモデルを特定フォルダに集約コピーし、ポータブルなプロジェクトバンドルを生成。
- **プロジェクトの整理 (Remove Unused Footage / Reduce Project)** (`src/legacy_egui/menu/file_edit.rs`):
  - 使用されていないフッテージの一括パージ、アクティブなコンポジションとその依存関係のみを残すプロジェクト縮小。

---

## 6. 「すでに実装されている機能」完全判定表

「これ絶対ないだろう」と思って再開発してしまうのを防ぐための詳細判定表です。

| 機能 | コア実装状況 | UI接続・実用状況 | 該当ソースファイル |
|---|:---:|:---:|---|
| **オーディオからキーフレーム生成** | ✅ 完全 | ✅ メニュー（Animation > Keyframe Assistant）から実行可（マルチバンドBass/Mid/Treble分離対応） | `core/audio_to_keyframes.rs`, `ui/menu/effect_anim.rs` |
| **テキストのアウトライン化 (Create Shapes from Text)** | ✅ 完全 | ✅ レイヤーメニューから1クリックでベジェシェイプレイヤーに変換 | `core/text_to_shapes.rs`, `ui/menu/layer.rs` |
| **自動トレース (Auto Trace)** | ✅ 完全 | ✅ Editメニューからアルファ輪郭をFreeformBezierシェイプに変換 | `core/auto_trace.rs`, `ui/menu/file_edit.rs` |
| **Blenderカメラトラッキングインポート** | ✅ 完全 | ✅ File > Import Blender Camera Track (.json) から3Dカメラにキーフレームベイク | `core/camera_track.rs`, `ui/menu/file_edit.rs` |
| **字幕（SRT/VTT）インポート＆エクスポート** | ✅ 完全 | ✅ File > Import Subtitles / Composition > Export Captions | `core/subtitles.rs`, `ui/menu/file_edit.rs` |
| **ファイルの収集 (Collect Files)** | ✅ 完全 | ✅ File > Collect Files... バックグラウンド並列集約 | `core/project_migration/`, `ui/menu/file_edit.rs` |
| **フローチャート（ノード構造グラフ）** | ✅ 完全 | ✅ Shift+F3 またはメニューからパン＆ズーム付きノードグラフ表示 | `ui/flowchart.rs`, `ui/flow_graph_model.rs` |
| **物理演算シミュレーション＆ベイク** | ✅ 完全 | ✅ Animation > Physics & Dynamics パネルから重力・衝突・反発をキーフレームベイク | `core/physics/`, `ui/physics_panel.rs` |
| **モーションスケッチ (リアルタイムマウス記録)** | ✅ 完全 | ✅ Animation > Motion Sketch からビューポートドラッグでリアルタイムパス記録 | `core/motion_sketch.rs`, `ui/motion_sketch_panel.rs` |
| **The Wiggler (ノイズ付加) / The Smoother (間引き)** | ✅ 完全 | ✅ Animation メニューから専用パネルを開いて適用可能 | `core/the_wiggler.rs`, `core/the_smoother.rs`, `ui/` |
| **OpenTimelineIO (.otio) 入出力** | ✅ 完全 | ✅ File > Import/Export OpenTimelineIO | `core/integration.rs`, `ui/menu/file_edit.rs` |
| **MLT XML (Shotcut/Kdenlive) 出力** | 🟡 サブセット（Transforms未対応） | ✅ File > Export MLT XML | `core/mlt_export.rs`, `ui/menu/file_edit.rs` |
| **Lottie JSON書き出し** | ✅ 完全 | ✅ CLI & UIからベクターWebアニメーション書き出し | `core/lottie_exporter.rs`, `bin/cli/` |
| **SVGパス取り込み (SVG to Mask)** | ✅ 完全 | ✅ File > Import SVG as Mask Layer からベジェマスクとして読み込み | `core/svg_ai_importer/`, `ui/menu/file_edit.rs` |
| **MOGRT（エッセンシャルグラフィックス）** | ✅ 完全 | ✅ Essential Properties パネルからパラメータ公開・オーバーライド | `core/mogrt_engine.rs`, `ui/essential_properties_panel.rs` |
| **OpenFX (OFX) プラグイン** | 🟡 ホストABIプローブ（実験的） | 🟡 Effect > OpenFX Plugins > Scan Standard Paths から検出プローブ可能（レンダリング統合は進行中） | `core/ofx_host.rs`, `core/openfx_bridge.rs`, `ui/menu/effect_anim.rs` |
| **Adobe After Effects SDK (AEPP/AEGP)** | 🟡 互換ヘッダ定義 | 🔴 外部.pluginバイナリの直接実行は未完 | `core/ae_sdk_host.rs`, `core/aegp_sdk_bridge.rs` |
| **Team Projects同期 (共同編集)** | 🟡 パケットモデル定義 | 🔴 ネットワークサーバー同期はパケット生成のみ実装 | `core/team_projects_sync.rs` |
| **C4D / OBJ 3Dメッシュインポート** | ✅ 完全 | ✅ File > Import OBJ 3D Model... から3Dレイヤーとして取り込み | `core/obj_loader.rs`, `ui/menu/file_edit.rs` |

---

## 7. CLI コマンド早見表

GUIを開かずにすべてのレンダリングと検査をターミナル・CIから実行できます。

```bash
# 1. コンポジションのフレーム範囲レンダリング (PNG連番)
cargo run --features cli --bin kagari -- render \
  --project project.json --comp "Main" --from 0 --to 120 --format png --output ./out

# 2. 単一フレームの高解像度レンダリング
cargo run --features cli --bin kagari -- frame \
  --project project.json --comp "Main" --frame 60 --output frame_060.png

# 3. Lottie JSON へのベクターアニメーション変換
cargo run --features cli --bin kagari -- lottie \
  --project project.json --output motion.json

# 4. プロジェクト健全性検査 (循環参照・欠損メディア・NaN検証)
cargo run --features cli --bin kagari -- validate --project project.json

# 5. 内蔵エフェクト一覧表示
cargo run --features cli --bin kagari -- effects
```
