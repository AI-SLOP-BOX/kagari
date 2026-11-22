---
name: ameui-pitfalls
description: AmeUI (.ame DSL / ランタイム / amec CLI) で実際に踏みやすい罠と確実な回避パターン集。Kagari・Hirari 等で .ame UI を編集・デバッグするとき、または新しいコンポーネントや状態バインディングを追加するときに必ず参照する。
---

# AmeUI 実践落とし穴・回避マニュアル

Kagari Studio の開発および `amec` 並列検証で実際に踏んだ AmeUI 固有の罠と、確定している回避パターンをまとめたリファレンス。

---

## 1. `component_memo` の暗黙依存漏れ（`REQ-423`）

### 症状
引数なし・他バインディング非参照の `component` 内で `{t.key}`（`translations`）だけを使っている場合、`ui_locale` を動的に切り替えても、AmeUI の `component_memo` が古いロケールの `Arc<ViewNode>` を返し続けて表示が変わらない。

### 回避パターン
`{t.*}` を使うコンポーネントの本体を `If(ui_locale != "") { ... }` で囲み、`component_dependencies` に `ui_locale` を明示登録する。

```ame
// NG: 言語切替後にキャッシュが効いて更新されない
component HomeHeader() {
  VStack(spacing: 4) {
    Text("{t.home_title}")
  }
}

// OK: ui_locale への依存を明示
component HomeHeader() {
  If(ui_locale != "") {
    VStack(spacing: 4) {
      Text("{t.home_title}")
    }
  }
}
```

---

## 2. `HStack` 内の `VStack` + `Spacer` によるテキスト押し潰し

### 症状
カードやツールバー行で `HStack` の中に「アイコン・テキスト列・`Spacer`・右端ボタン」を並べると、中央の `VStack` が最小幅に縮み、`amec measure` で `layout_overflows`（特に英語や長文翻訳時）が発生する。

### 回避パターン
テキスト側の `VStack` を `Grow(1)` で包み、`Spacer` を取り除く。

```ame
// NG: 英語や長文で overflow_x が発生する
HStack(spacing: 14) {
  Svg("...", 44, 44)
  VStack(spacing: 3) {
    Text("{t.home_open_project}")
    Text("{t.home_open_project_sub}")
  }
  Spacer
  Button("{t.home_browse_btn}", action: open_project)
}

// OK: 利用可能幅いっぱいに広がり多言語でも溢れない
HStack(spacing: 14) {
  Svg("...", 44, 44)
  Grow(1) {
    VStack(spacing: 3) {
      Text("{t.home_open_project}")
      Text("{t.home_open_project_sub}")
    }
  }
  Button("{t.home_browse_btn}", action: open_project)
}
```

---

## 3. `If(...)` 分岐が裏で `ViewKind::Stack` を挟む構造（`REQ-375`）

### 症状
`If(cond) { Overlay { ... } }` のように書くと、`If` が中身を `ViewKind::Stack` で包むため、親コンテナ側が「直下の子が `Overlay` である」と判定できず、Overlay 高さが `0.0` に潰れることがある。

### 回避パターン
モーダルや `Overlay` を条件表示するときは、`Overlay` 自体を `ZStack` 直下に置くか、`If` の外側にサイズ確定できるラッパーを置く構成を検討し、必ず `amec measure` で `layout_overflows: 0` を確認する。

---

## 4. `window { ... }` 宣言の必須フィールド

最小再現 `.ame` や新規テストアプリを作るとき、`window` ブロックには **`title`・`size`・`min_size` の 3 つすべてが必須**。いずれかが欠けるとコンパイルエラーになる。

```ame
app MinimalProbe {
  window { title: "Probe" size: 640, 480 min_size: 320, 240 }
  view {
    VStack { Text("ok") }
  }
}
```

---

## 5. `RelativeFrame` と `.frame()` の単位違い

- `RelativeFrame` の `width` / `height` はピクセルではなく **`0.0..=4.0` の親に対する割合**。
- ピクセル固定サイズを指定したい場合は `.frame(height: 360)` または `Frame(...)` を使う。

---

## 6. テスト用ターゲット名 (`HitArea`) と表示テキストの分離

E2E テストやクリック操作は `HitArea("ホーム", action: go_home)` のような ID/ラベル文字列に依存している場合がある。
UI を多言語化・文言変更する際は、**`HitArea` の第1引数はそのまま維持し、内側の `Text("{t.nav_home}")` だけ差し替える** こと。

---

## 7. `amec` サブコマンドの成熟度・オプション仕様

| サブコマンド | 状態 | 注意点 |
|---|---|---|
| `amec check` | 安定 | 高速構文・型チェック。まず最初に通す |
| `amec measure` | 安定 | `--scale` を使う（`--scale-factor` ではない）。`--pinch-zoom` には先行する `--pointer X,Y` が必須 |
| `amec svg` | 安定 | ベクタースナップショットを SVG 出力（構造確認に有効） |
| `amec build --target bytecode` | 安定 | Kagari Studio 全体でも `.amec` 生成可能 |
| `amec bundle` | 安定 | `app.ame` + `manifest.json` の配布バンドル出力 |
| `amec build --target rust` | 未成熟 (`REQ-424`) | `Button` 等基本ウィジェットでも `unsupported widget kind` になるため本番利用不可 |

---

## 8. 最小幅 (`min_size`) レスポンシブ確認の鉄則

`window { min_size: 640, 480 }` を宣言している場合、デフォルト `1600×1000` だけでなく必ず `--width 640 --height 480` でも `amec measure` を実行する。`narrow` 分岐に入っていても固定文言が横溢れしやすい。
