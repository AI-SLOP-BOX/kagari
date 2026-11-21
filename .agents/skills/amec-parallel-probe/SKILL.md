---
name: amec-parallel-probe
description: Linux リモートビルダー (192.168.0.21, 16コア) で amec measure / check / svg / build を並列実行し、Kagari Studio や最小再現 .ame のレイアウトオーバーフロー・文字クリップ・スケール耐性・多言語表示を数秒で一斉検証するスキル。
---

# `amec` 超並列プローブ＆UI品質検証ガイド

GUI ウィンドウを開かずに、Linux リモートビルダーの 16 コアを使って複数画面・複数ロケール・複数解像度・エッジケースを並列検証する手順。

---

## 1. Kagari Studio 全画面マトリクス一斉計測（推奨フロー）

UI ファイル（`src/ui/ame/**/*.ame`）や翻訳（`src/ui/locales/*.json`）を変更したら、まず `rsync` で Linux へ同期したうえで同梱スクリプトをリモート実行する。

```bash
rsync -az --delete --exclude 'target/' --exclude '.git/' \
  /Users/dev/Desktop/AI-SLOP-Family/Kagari/ \
  aislopbox@192.168.0.21:~/kagari-linux-check/

ssh aislopbox@192.168.0.21 'python3 - ~/kagari-linux-check/src/ui/ame/kagari_studio.ame' \
  < .agents/skills/amec-parallel-probe/scripts/run_studio_matrix.py
```

### マトリクスに含まれる標準ケース
1. `home_ja_1600`: Home 日本語 (`1600×1000`)
2. `home_en_1600`: Home 英語 (`1600×1000`, `ui_locale=en`)
3. `editor_ja_1600`: Editor 日本語 (`1600×1000`, `home_page=false, editor_page=true`)
4. `settings_en_1600`: 環境設定 英語 (`1600×1000`, `ui_locale=en, settings_page=true`)
5. `export_en_1600`: 書き出し 英語 (`1600×1000`, `ui_locale=en, home_page=false, export_page=true`)
6. `scale_200_1600`: 高DPI 200% (`--scale 2.0`)
7. `scale_125_1600`: 高DPI 125% (`--scale 1.25`)
8. `theme_light_1600`: ライトテーマ (`--system-theme light`)
9. `min_size_640`: 最小ウィンドウ幅 (`640×480`)
10. `aot_bytecode`: `amec build --target bytecode` 検証

---

## 2. 合格基準と結果の見方

- **`clipped_text_pixels == 0`**: 文字の上下・左右の描画欠けが 0 であること（必須）。
- **`layout_overflows == 0`**: 親コンテナからはみ出す子要素が 0 であること。
  - もし `overflow_details` にテキストが出たら、`HStack` 内の `VStack` に `Grow(1)` を付け忘れていないか確認する（`ameui-pitfalls` 参照）。
- **スナップショットの目視確認**:
  異常や新規画面追加時は `--snapshot ~/amec_snaps/<name>.png` を付けて `scp` で取り、`read_file` で直接視覚確認する。

---

## 3. AmeUI の新しい穴を探すとき（鬼門並列プローブ）

新しい機能やバグ報告用の検証をするときは、Linux 上の `python3` + `concurrent.futures.ThreadPoolExecutor(max_workers=os.cpu_count())` で一時 `.ame` を生成し、以下を並列で叩く：
- **BiDi / RTL 混在**: アラビア語・ヘブライ語・日本語混在
- **複合絵文字 / ZWJ**: `👨‍👩‍👧‍👦🏳️‍🌈🇯🇵👍🏽` の `Text` / `TextField`
- **動的スケール切替**: `--scale 1.0 --change-scale 2.0`
- **入力＋ショートカット**: `--type 検索=日本語入力テスト --key-press 検索=Primary+A --assert-focused 検索`
- **AmeUI 側への報告**: 再現する最小 `.ame`（10〜30行・`amec check` 通過確認済み）を作り、`../AmeUI/リクエスト.md` のルール（`ameui-guide` スキル）に従って追記する。
