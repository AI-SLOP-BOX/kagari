# Kagari VFX — Agent Onboarding & Architecture Guide

このリポジトリは Rust 製の映像合成・モーショングラフィックスエンジンおよびスタジオアプリです。初見の AI エージェントが迷わず開発・検証・拡張できるように、全体構成と必須ルールをここに集約しています。

## 1. バイナリとディレクトリ構成

- **`kagari` CLI** ([src/bin/cli.rs](src/bin/cli.rs), feature: `cli`):
  - ヘッドレス編集・レンダリング・QC 検査・OTIO/Lottie 変換・自動演出（`story`, `mv`）を提供する CLI。
  - **AI 向け自己記述**: `kagari schema`（または `kagari effects --json`, `kagari doctor --json`, `kagari presets --json`）を実行すると、全サブコマンド・引数・列挙値を JSON で確認できます。
- **`kagari-studio` GUI** ([src/bin/ame_studio.rs](src/bin/ame_studio.rs), default feature: `ame`):
  - AmeUI (`../AmeUI`) を UI エンジンに採用したデスクトップスタジオ。
  - `.ame` UI 定義: `src/ui/ame/`（エントリ: `src/ui/ame/kagari_studio.ame`、モジュール登録: `src/ui/ame_sources.rs`）
  - 多言語辞書: `src/ui/locales/*.json`（[build.rs](build.rs) がコンパイル時に `OUT_DIR` へ自動生成）
- **コアエンジン**: `src/core/`（タイムライン、エフェクト、OpenFX、オーディオ、ノードグラフ、ソフトウェア/GPU レンダー）

## 2. 厳守ルール

1. **800 行ルール**: `src/` 配下の全 `.rs` / `.ame` ファイルは **800 行以下** を維持する。
   ```bash
   find src -name '*.rs' -o -name '*.ame' | xargs wc -l | awk '$1 > 800 && $2 != "total"'
   ```
2. **Mac ローカルで重い `target/` ビルドを走らせない**: ビルド・テストは Linux リモートビルダー（`aislopbox@192.168.0.21`）で行う。
3. **UI 多言語対応**: 新言語の追加は `src/ui/locales/<locale>.json` を 1 ファイル増やすだけ（Rust や `.ame` 編集不要）。新しい翻訳キーを使うコンポーネントには原則 `If(ui_locale != "")` を確認し、`HitArea("固定ID", ...)` の第 1 引数は既存テスト互換のため変更しない。
4. **AmeUI のバグ／機能要望**: `../AmeUI/リクエスト.md` へ `AmeUI` 単体で再現可能な最小 `.ame`（`amec check` 通過確認済み）付きで記録する。

## 3. 必読スキル (`.agents/skills/`)

作業内容に応じて以下の `SKILL.md` を参照してください：

- [linux-builder](.agents/skills/linux-builder/SKILL.md): Linux リモートビルダーへの `rsync` / `cargo test` 手順
- [amec-parallel-probe](.agents/skills/amec-parallel-probe/SKILL.md): 16 コア並列で全画面・多言語・高DPI・最小幅を数秒で一斉検査する `scripts/run_studio_matrix.py`
- [ameui-pitfalls](.agents/skills/ameui-pitfalls/SKILL.md): AmeUI で踏みがちな罠（`Grow(1)`、`SettingsRow` 引数補間、`window` 必須項目等）と回避策
- [kagari-i18n](.agents/skills/kagari-i18n/SKILL.md): `src/ui/locales/*.json` と `build.rs` 自動生成の運用ルール
- [ameui-guide](.agents/skills/ameui-guide/SKILL.md): AmeUI 機能一覧と `リクエスト.md` 記載ルール
- [motion_ui_design](.agents/skills/motion_ui_design/SKILL.md): モーション UI 設計ガイドライン

## 4. 標準の検証コマンド（Linux リモートビルダー）

```bash
# 1. 同期 & 全12ケース並列 UI マトリクス計測（clipped_text_pixels: 0 / layout_overflows: 0 を確認）
rsync -az --delete --exclude 'target/' --exclude '.git/' ./ aislopbox@192.168.0.21:~/kagari-linux-check/
ssh aislopbox@192.168.0.21 '/home/aislopbox/AmeUI/target/debug/amec check ~/kagari-linux-check/src/ui/ame/kagari_studio.ame'
ssh aislopbox@192.168.0.21 'python3 - ~/kagari-linux-check/src/ui/ame/kagari_studio.ame' < .agents/skills/amec-parallel-probe/scripts/run_studio_matrix.py

# 2. スタジオ単体・E2E テスト
ssh aislopbox@192.168.0.21 'cd ~/kagari-linux-check && cargo test --bin kagari-studio --features ame engine_hub && cargo test --bin kagari-studio --features ame test_reload_ame_ui_runtime && cargo test --bin kagari-studio --features ame ui_render'

# 3. CLI テスト
ssh aislopbox@192.168.0.21 'cd ~/kagari-linux-check && cargo test --bin kagari --features cli'
```
