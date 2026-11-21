---
name: kagari-i18n
description: Kagari Studio の多言語対応（i18n）ルールと新規言語・新規翻訳キー追加手順。src/ui/locales/*.json、build.rs 自動生成、locale 切り替えアクション、テスト規約を扱うときに使用。
---

# Kagari Studio 多言語対応（i18n）開発ガイド

Kagari Studio の UI 翻訳は `src/ui/locales/*.json` を唯一の正本（Single Source of Truth）とし、`build.rs` がコンパイル時に `.ame` モジュールと Rust メタデータを自動生成する仕組みになっている。

---

## 1. ファイル構成と自動生成の仕組み

- **正本**: `src/ui/locales/<locale>.json`（例: `en.json`, `ja.json`）
- **ビルド時生成 (`build.rs` → `OUT_DIR`)**:
  - `i18n_strings.ame` → `ame_sources.rs` 経由で `"state/strings.ame"` として提供
  - `i18n_locale_select.ame` → `"components/settings/locale_select.ame"` (`LocaleSelect()`) として提供
  - `i18n_locales.rs` → `ame_sources::i18n::{SUPPORTED_LOCALES, LOCALE_SEARCH_KEYWORDS, is_supported_locale}`
  - `i18n_kagari_studio.ame` → `kagari_studio.ame` の `// <generated-locale-commands>` 〜 `// </generated-locale-commands>` 間を自動置換
- **単体 `amec check` 互換用スタブ**:
  - `src/ui/ame/state/strings.ame` と `src/ui/ame/components/settings/locale_select.ame` は、Linux 上で `amec check src/ui/ame/kagari_studio.ame` を単体実行するときだけ参照されるため、削除せず `en` / `ja` の基本定義を残す。

---

## 2. 新しい言語を追加する手順

Rust コードも `.ame` ファイルも編集不要。

1. `src/ui/locales/en.json` をコピーして `src/ui/locales/<code >.json`（例: `ko.json`, `zh-CN.json`）を作成
2. `_meta` オブジェクトを設定：
   ```json
   "_meta": {
     "label": "한국어 (Korean)",
     "keywords": "korean hangul 韓国語",
     "command": "言語 / Language: 한국어 (Korean)",
     "order": 30
   }
   ```
3. 各翻訳キーの値を訳す（未訳キーは自動的に `"en"` へフォールバック）。
4. Linux リモートビルダーで `cargo test --bin kagari-studio --features ame ui_render` を実行し、`supported_locales_are_loaded_from_json_catalogs` を含む全テスト通過を確認。

---

## 3. 新しい翻訳キーを UI に追加する手順

1. `src/ui/locales/en.json` と `src/ui/locales/ja.json`（および他言語 JSON）に同じキーを追加する。
2. `.ame` 側で `{t.your_key}` を使う。
3. **必須チェックリスト（罠回避）**:
   - [ ] その `component` に `If(ui_locale != "") { ... }` があるか（`ameui-pitfalls` の `REQ-423` 回避）
   - [ ] 長い訳文（英語・ドイツ語など）になったときに `HStack` 内で潰れないよう `Grow(1)` を使っているか
   - [ ] `HitArea("既存の固定ID", ...)` を壊していないか
   - [ ] `amec measure --bindings ui_locale=en` で `clipped_text_pixels: 0` と `layout_overflows` を確認したか
   - [ ] テスト（`ui_render_e2e.rs`）で日本語⇔英語の切替と復元を確認したか

---

## 4. テスト時の注意

- `preferences.rs` の `set_ui_locale` は `write_preference("ui_locale", ...)` でディスク設定を書き換えるため、テスト内で `"en"` へ切り替えた場合は必ず同じテスト内で `"ja"` に戻すこと。
