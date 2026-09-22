# ddq-revision（タグ付け・改訂履歴）の実現性検討と設計

- 日付: 2026-09-22
- 対象: テンプレート 2.2.2（`ddq` + `design-doc.lua` + `lib.typ` + ddq-table-editor 2.2.2）
- 位置づけ: 実装前の検討書。実装後は `docs/cli-impl/` / 拡張ごとの実装仕様書（`docs/table-editor-impl/` と同じ形）へ昇格する
- 経緯: 2026-09-22 の一問一答（Q1〜Q19）で決めた前提を §2 にまとめ、それに基づく設計を §3 以降に記す。同日に**検証 V1〜V5 をすべて実機で行い**、結果を §8 に記した。V1 で見つかった HTML のリンク切れは 2.2.2 として先行修正済み（PR #97）。`lib.typ` の `_xref-page`（V2）は ddq-revision 実装時に入れる

---

## 1. 結論

| 観点 | 評価 |
|---|---|
| 実現の可否 | **可能**。要素技術（行頭パターン走査・`git show` による旧版取得・Custom Editor・差分エディタ・`.tbl` 生成）は V1〜V5 で実機確認済みで、新規の難所は無い |
| 難易度 | **中**。CLI 側は「論理文書の展開 → ユニット抽出 → 差分帰属」が中核で、mermaid/PlantUML 対応より小さく、`.tbl` 自動分割より易しい。拡張側は Webview の表 UI 2 画面 + Custom Editor で、ddq-table-editor の基盤（React + vite + esbuild）を流用できる |
| 構成 | **コアは `ddq` CLI（Rust）、VSCode 拡張は UI 殻**。`ddq-diff` のような別 CLI は作らず、`ddq tag …` / `ddq rev …` のサブコマンドとして既存 CLI に追加する（§3.1） |
| 現行実装への影響 | **小〜中**。`design-doc.lua` は無変更。`lib.typ` にページ番号参照 `_xref-page`（16 行）、`postprocess-html.js` に別章への `@tbl-` リンク修正（16 行。既存バグの修正を兼ねる）。`extension/` → `extensions/ddq-table-editor/` への移動と `release.rs` の複数 VSIX 対応。scaffold の変更を伴うので **template の版上げ（2.3.0）** が必要 |
| 最大のリスク | 技術リスクは V1〜V5 の検証で解消した（§8）。残るのは運用面: (1) ラベル未付与文書では機能 2 が使えない（機能 1 の一括適用が前提。§4.6）(2) 人がタグを打ち忘れると次回の base 候補が出ない（Q17 で当面は人手と決めた）(3) CLI と拡張をまたぐ操作感は触ってみないと分からない（段階 3・5 で実文書に当てて確かめる） |

---

## 2. 決定事項（一問一答の結果）

| # | 論点 | 決定 |
|---|---|---|
| Q1 | タグの正体 | **Quarto ラベル**（`{#sec-x}` / `label="tbl-x"` / `{#fig-x}`）そのもの。改訂履歴のキーであり、本文・改訂履歴表のリンクにもそのまま使う |
| Q2 | 未付与時の命名 | 機械が候補を生成し、書き戻し前に人が一覧上で修正できる |
| Q3 | 候補の生成元 | **内容由来の決定的ハッシュ**（`sha1(相対パス + 種別 + 名称)` 先頭 6 桁、衝突時は桁を伸ばす）。初回付与の候補にのみ使い、付与後は不変 |
| Q4 | 対象範囲 | 見出し（全レベル）+ `.tbl` / `.ipo` ブロックの `label=` + `::: {#fig-}` ブロック + 素の pipe table のキャプション行 `: … {#tbl-x}`。裸の `![]()` や裸の `` ```{mermaid} `` は**包み直さない**（警告表示のみ） |
| Q5 | 改訂履歴の単位 | 変更は**最も深い単位に一意に帰属**（地の文 → 直近上位見出し、表・図 → 自身）。親見出しには伝播させない。改訂履歴作成は対象範囲すべてにラベルがあることが前提 |
| Q6 | 比較対象 | 基準 = 任意の Git ref（UI の既定は前回の改訂タグ `rev-<記号>`）、現在 = **作業ツリー**（未コミット含む） |
| Q7 | 改訂記号 | 自由入力。既定は前回タグの記号 +1。方式は `alpha`（`-`, `A`, `B`, …）を既定、`numeric` も選択可 |
| Q8 | 正本 | **機械可読な YAML**。そこから改訂履歴表（`.tbl`）を `ddq rev build` で `revisions/history.qmd` に生成し、`index.qmd` の前付けから include |
| Q9 | 分担 | コアは `ddq` CLI、拡張は UI 殻。新拡張は ddq-table-editor とは**別拡張** |
| Q10 | UI | TreeView は使わず **Webview の表形式**で統一。改訂履歴側は `revisions/*.yml` の **Custom Editor**。差分は VSCode 標準の差分エディタで表示 |
| Q11 | ファイル構成 | **改訂ごとに 1 ファイル** `revisions/rev-<記号>.yml`。`ddq rev build` はフォルダを読んで累積の表を生成。note 空は**警告して載せる** |
| Q12 | 差分判定 | ユニット本文の文字列比較。**空白・改行のみの差は無視**（`--strict` で有効化）。`renamed` は名称のみ変更。ファイル間移動は履歴に載せない。帰属不能な変更は疑似ラベル `(文書全体)`（既定オフ）。ラベル無しは `unlabeled` として警告 |
| Q13 | 書き戻し | CLI は**編集指示（ファイル・位置・挿入文字列）を JSON で返す**。VSCode では拡張が `WorkspaceEdit` で適用（Undo 可）。CLI 単体では `ddq tag apply` が自分で書く |
| Q14 | 走査対象 | `_quarto.yml` の `book.chapters`（part 内含む）+ そこから `{{< include >}}` で**再帰的に辿れるファイル**。`_quarto.yml` が無ければ再帰走査にフォールバック |
| Q15/16 | 箇所列 | `@sec-x` 等のリンクのみ（名称は載せない）。PDF のみ**頁列**を追加（V2 で検証済み。採用） |
| Q17 | 確定フロー | 「確定」= `ddq rev build` + Git コマンドの案内表示のみ。**タグ付け・コミットは人**が行う。`fixed` 判定は `rev-<記号>` タグの有無 |
| Q18 | 配置 | `extensions/ddq-table-editor/`・`extensions/ddq-revision/` に並べる（既存 `extension/` を移動）。名称は **ddq-revision** |
| Q19 | 成果物 | 本書（単一 Markdown）。実機検証は後で行う |

---

## 3. アーキテクチャ

### 3.1 責務分担

```
┌──────────────────────────── VSCode 拡張 ddq-revision ───────────────────────────┐
│  タグ付け画面 (Webview 表)        改訂履歴 Custom Editor (Webview 表, revisions/*.yml) │
│   ├ ddq tag list --json           ├ ddq rev diff --base <ref> --json                  │
│   ├ 候補ラベルの修正・チェック      ├ 差分ボタン → vscode.diff (git: URI)               │
│   └ WorkspaceEdit で書き戻し       ├ note 編集 → yml 保存 (VSCode が dirty/undo 管理)   │
│                                    └ 確定 → ddq rev build + git コマンド案内            │
└───────────────┬──────────────────────────────┬────────────────────────────────────┘
                │ spawn (JSON in/out)           │
┌───────────────▼──────────────────────────────▼────────────────────────────────────┐
│  ddq (Rust, 既存 CLI にサブコマンド追加)                                            │
│   tag list / tag apply         rev diff / rev build / rev next                      │
│   ├ scan: _quarto.yml → chapters → include 展開 → 論理文書                          │
│   ├ units: 見出し / .tbl / .ipo / #fig- / pipe table caption の抽出                 │
│   ├ label: ハッシュ候補生成, 編集指示 (file, line, col, insert)                     │
│   ├ git: `git show <ref>:<path>` で旧版の論理文書を同様に構築                        │
│   ├ diff: ユニット単位の正規化比較 → changed/added/removed/renamed                    │
│   └ build: revisions/*.yml → revisions/history.qmd (.tbl。index.qmd から include)    │
└───────────────────────────────────────────────────────────────────────────────────┘
                                          │
                              ddq pdf / ddq html (既存。生成物はただの .qmd)
```

- 拡張は **qmd を解釈しない**。表示するもの・編集するものはすべて ddq の JSON と yml。
- ddq は **VSCode を知らない**。同じコマンドを人・CI・他エディタ・AI エージェントが使える。
- 別 CLI（`ddq-diff`）にしない理由: 配布経路（単一 exe、`ddq release`、`update-doc`）と `_quarto.yml`/include の解釈コードを共有できる。Rust と Node に CLI が割れる不利益の方が大きい。

### 3.2 拡張が ddq を見つける方法

- 設定 `ddqRevision.ddqPath`（既定: 空 = `PATH` の `ddq`）。scaffold の `.vscode/settings.json` に項目を追加しておく。
- 起動時に `ddq --version` を呼び、拡張の版（= `template/VERSION`）と不一致なら警告（ddq-table-editor と同じ版同期ルール）。
- 実行環境: 執筆者の PC には ddq が配られている前提（利用マニュアルの役割分担どおり）。

---

## 4. 機能 1: タグ付け

### 4.1 検出規則（行頭パターン。フルパーサ不要）

| 種別 | 検出 | ラベル有 | ラベル無 → 編集指示 |
|---|---|---|---|
| 見出し | `^(#{1,6})\s+(.+?)(\s*\{[^}]*\})?\s*$`（コードフェンス内は除外） | `{…#sec-x…}` を含む | 行末に ` {#sec-<hash>}` を挿入。既に `{.unnumbered}` 等の属性があれば `{` の直後に `#sec-<hash> ` を挿入 |
| 表（統一） | `^:::+\s*\{[^}]*\.tbl[^}]*\}` | `label="tbl-x"` | `caption=` があれば `}` の直前に ` label="tbl-<hash>"` を挿入。**`caption` も `.unnumbered` も無ければ対象外**（番号無し表。一覧に「キャプション無し」で表示し、UI でキャプションを入力した場合のみ `caption=` と `label=` を同時挿入） |
| IPO 図 | `^:::+\s*\{[^}]*\.ipo[^}]*\}` | `label="tbl-x"` | `.tbl` と同じ（IPO は表番号を持つ） |
| 図（ブロック） | `^:::+\s*\{[^}]*#fig-` | 常に有（この記法は ID が必須） | — |
| 図（キャプション付き画像） | `^!\[.+\]\(.+\)\{[^}]*\}` / `^!\[.+\]\(.+\)$` | `{#fig-x}` を含む | `{#fig-<hash>}` を行末に付与（`::: {#fig-}` で包み直さない） |
| 表（pipe table） | pipe table ブロック直後（または直前）の `^:\s+(.+?)(\s*\{[^}]*\})?\s*$` | `{#tbl-x}` | 行末に ` {#tbl-<hash>}` |
| 裸の図 | `` ```{mermaid} `` / `` ```plantuml `` / 属性無し `![]()` で上記に該当しないもの | — | **自動付与しない**。一覧に「ラベル不可（要手動）」で警告のみ |

- 見出しレベルは `#`〜`######` すべて対象。`{.unnumbered}` の見出しも対象（改訂履歴のキーは必要）。
- **`.tbl` / `.ipo` / `::: {#fig-}` ブロックの内側にある見出しは対象外**。IPO 図は `## <機能名>` `### 入力` `### 処理` `### 出力` という見出しで中身を書く記法で、これは文書の節ではない（examples/docs では 9 件ある）。ラベルを付けず、改訂の帰属もそのブロック（表・図）のラベルに寄せる。
- `.md` も同じ規則（Quarto の `.md` 章は `.qmd` と同じ記法）。
- YAML front matter（`---` 〜 `---`）と fenced code block の内側は走査しない。

### 4.2 候補ラベルの生成

```
hash = sha1(relpath + "\n" + kind + "\n" + normalize(title)).hex[0..6]
label = { heading: "sec-", tbl/ipo/pipe: "tbl-", fig: "fig-" } + hash
```

- `normalize` は空白の圧縮と trim。`relpath` は執筆フォルダ基準の物理ファイルパス。
- 同一実行内で既存ラベル・他候補と衝突したら 8 桁、10 桁と伸ばす。
- 同じ状態で何度実行しても同じ候補になる（UI の再読込・CLI/拡張間で揺れない）。
- 人が候補を書き換えたときは `^(sec|tbl|fig)-[A-Za-z0-9_-]+$` と一意性を検証してから適用。

### 4.3 CLI インタフェース

```
ddq tag list  <執筆フォルダ> [--json] [--unlabeled]
ddq tag apply <執筆フォルダ> (--all | --from <edits.json>) [--dry-run]
```

`tag list --json` の出力（拡張はこれをそのまま表に流す）:

```json
{
  "folder": "docs",
  "order": ["index.qmd", "chapters/01-overview/index.qmd", "chapters/01-overview/01-purpose.qmd", "..."],
  "items": [
    { "kind": "heading", "level": 1, "file": "chapters/01-overview/index.qmd", "line": 1,
      "title": "概要", "label": "sec-overview", "suggested": null, "edit": null },
    { "kind": "heading", "level": 2, "file": "chapters/01-overview/01-purpose.qmd", "line": 1,
      "title": "目的", "label": null, "suggested": "sec-3f9a1c",
      "edit": { "file": "chapters/01-overview/01-purpose.qmd", "line": 1, "col": 6, "insert": " {#sec-3f9a1c}" } },
    { "kind": "tbl", "file": "chapters/03-design-conditions/index.qmd", "line": 6,
      "title": "設計条件", "label": "tbl-cond", "suggested": null, "edit": null },
    { "kind": "tbl", "file": "chapters/01-overview/02-features.qmd", "line": 27,
      "title": null, "label": null, "suggested": null, "edit": null, "warning": "no-caption" },
    { "kind": "fig", "file": "chapters/04-system/01-hardware/01-config.qmd", "line": 12,
      "title": null, "label": null, "suggested": null, "edit": null, "warning": "bare-mermaid" }
  ],
  "warnings": [ { "file": "chapters/99-draft.qmd", "message": "chapters に未登録（走査対象外）" } ]
}
```

- `edit` は **1 行内の挿入**だけで表現できる（行の追加・削除を伴わない）。`col` は UTF-8 バイトではなく**文字（コードポイント）位置**で返し、拡張側で VSCode の `Position` に変換する（qmd は日本語が多いので明記）。
- `tag apply --from edits.json` は `items[].edit` の配列（人が `insert` を書き換えたもの）を受け取り、行番号が大きい順に適用してずれを防ぐ。適用前に対象行が `list` 時と同一かを検査し、違えばそのファイルをスキップして報告する。

### 4.4 拡張側（タグ付け画面）

- コマンド `ddqRevision.tags`（コマンドパレット／エクスプローラの執筆フォルダ右クリック）。
- Webview 表: ファイル ／ 種別 ／ 名称 ／ 現ラベル ／ 候補ラベル（編集可）／ 適用 ☑。未付与行が既定でチェック済み。ファイル単位で折りたたみ。
- 行クリック → `vscode.window.showTextDocument(file, {selection: line})` でジャンプ。
- 「書き戻す」→ チェック行の `edit` を `WorkspaceEdit` にまとめて `applyEdit`（1 回の Undo で戻る）。未保存バッファにも当たるので保存の強制は不要。適用前に対象行の内容が `list` 時と同じかをエディタ上で再検査する。
- 適用後に `tag list` を再実行して表を更新。

### 4.5 未決・注意

- 見出し行末に `{#sec-…}` を付けると、ddq-table-editor など他ツールの見出し検出に影響しないか確認（Quarto 標準記法なので問題無い想定）。
- `index.qmd`（book のトップ）の見出しは Quarto が特別扱いするが、ラベル付与自体は無害。

### 4.6 運用上の前提

機能 2 は「対象範囲すべてにラベルがある」ことが前提なので、既存文書に対しては**最初に機能 1 を一括適用してコミットし、その状態に最初の改訂タグ（初版 `rev--` 相当、記号は `-`）を打つ**ところから運用を始める。この一括適用コミット自体は改訂ではない（本文が変わらないため）。

---

## 5. 機能 2: 改訂履歴

### 5.1 論理文書の構築

1. `_quarto.yml` の `book.chapters` を読む（`part:` の `chapters:` も再帰）。
2. 各章ファイルから `{{< include path >}}` を再帰展開し、展開位置に差し込む。展開後の並びが「文書順」。**パスの基準は「`chapters:` に並べた章ファイルのあるディレクトリ」で、入れ子の include でも変わらない**（include 元ファイルの位置ではない。`_quarto.yml` の注意書きのとおりで、V4 で実際に踏んだ）。例: `chapters/04-system/index.qmd` が `01-hardware/index.qmd` を include し、その中の include は `01-hardware/01-config.qmd` と書く（`01-config.qmd` ではない）。
3. 各行は `(物理ファイル, 行番号, 内容)` を保持する（ジャンプ・書き戻し用）。
4. include 先が `chapters` にも直接列挙されていれば重複警告。循環 include はエラー。
5. `_quarto.yml` が無い場合はフォルダ配下の `.qmd`/`.md` をファイル名順に並べる（`_book/`, `_freeze/`, `node_modules/`, `.git/` を除外）。

旧版側は `git show <base_commit>:<執筆フォルダ>/<path>` で `_quarto.yml` と各ファイルを取得し、同じ手順で構築する（作業ツリーの `_quarto.yml` ではなく**旧版の `_quarto.yml`** を使う。章の追加・削除も正しく検出するため）。旧版に存在するファイルは先に `git -c core.quotepath=off ls-tree -r --name-only <base_commit> -- <執筆フォルダ>` で一覧を取り、その中にあるものだけ `git show` する（無いファイルの `git show` は `fatal: path … does not exist` で失敗するため、エラー文字列の解析に頼らない）。

### 5.2 ユニットの抽出

論理文書を上から走査し、次のユニットに分割する。

| ユニット | 範囲 | 除外 |
|---|---|---|
| 見出し `sec-x` | 見出し行から、次の「同レベル以上の見出し」の直前まで | 配下の子見出しユニットの範囲、配下のラベル付き表・図ブロックの範囲 |
| （対象外） | `.tbl` / `.ipo` / `#fig-` ブロックの内側の見出し | ブロック自身のユニットに含める |
| 表 `tbl-x` | `::: {.tbl …}` 〜 対応する `:::`（`.ipo` も同じ）／ pipe table ブロック + キャプション行 | — |
| 図 `fig-x` | `::: {#fig-x}` 〜 対応する `:::`／キャプション付き画像 1 行 | — |
| `(文書全体)` | 最初の見出しより前の行（front matter 除く）、`_quarto.yml` | — |

- ラベル無しの表・図ブロックはユニットにせず、**包含する見出しユニットの本文に含める**（差分は見出しに帰属し、`unlabeled` 警告を併記）。
- 表・図の `title` は `caption=` / キャプション行 / `::: {#fig-}` 直後の画像キャプション（無ければ最初の行）から取る。
- fenced div の対応は `:::` の個数（`::::` 入れ子）で判定。既存 `design-doc.lua` と同じ緩さで良い（厳密な Pandoc 互換は不要。誤対応時は警告して見出しに帰属）。

### 5.3 差分判定

```
for each label in old ∪ new:
  old_unit = old[label], new_unit = new[label]
  if old_unit is None            → added
  elif new_unit is None          → removed  (title は旧版の名称)
  else:
    body_changed  = norm(old.body)  != norm(new.body)
    title_changed = old.title != new.title
    if body_changed                → changed  (title_changed なら title_old を併記)
    elif title_changed             → renamed
    else                           → (変更なし。出力しない)
    moved_from = old.file if old.file != new.file
```

- `norm`: ユニット本文から**見出し行／キャプション行を除き**、各行を trim、連続空白を 1 つに圧縮、空行を除去して結合。`--strict` は「行内の空白と空行の違いを無視しない」モードで、**改行コードの正規化だけは `--strict` でも必ず行う**。`core.autocrlf=true`（Windows の既定でよくある設定）の環境では作業ツリーが CRLF、`git show` の出力が LF になり、素の比較では全ユニットが changed になってしまう（V5 で実測）。
- ラベルは文書中で一意でなければならない。重複があれば `rev diff` はエラー（`tag list` でも重複警告）。
- 同一改訂内で「削除して同じラベルで別の場所に追加」は `changed` + `moved_from` になる（意図どおり）。

### 5.4 CLI インタフェース

```
ddq rev diff  <執筆フォルダ> --base <ref> [--json] [--strict] [--include-global]
ddq rev build <執筆フォルダ> [--check]          # revisions/*.yml → revisions/history.qmd
ddq rev next  <執筆フォルダ> [--json]           # 次の改訂記号と base 候補（最新 rev-* タグ）を返す
```

`rev diff --json`:

```json
{
  "base": "rev-B", "base_commit": "3c1f…", "strict": false,
  "entries": [
    { "label": "sec-purpose", "kind": "changed", "title": "目的", "title_old": "目的",
      "file": "chapters/01-overview/01-purpose.qmd", "line": 1, "moved_from": null },
    { "label": "tbl-a3f9c1", "kind": "added", "title": "設計条件", "file": "chapters/03-design-conditions/index.qmd", "line": 6 },
    { "label": "fig-net", "kind": "removed", "title": "ネットワーク構成", "file_old": "chapters/04-system/03-network/02-connections.qmd" }
  ],
  "global": { "changed": true, "files": ["index.qmd", "_quarto.yml"] },
  "unlabeled": [ { "kind": "heading", "file": "chapters/07-performance/index.qmd", "line": 14, "title": "測定条件" } ],
  "errors": []
}
```

- `entries` の順序は**新版の文書順**（removed は旧版で直前にあったラベルの後ろに挿す）。
- Git 呼び出しは `git` 子プロセス（`rev-parse`, `show`, `ls-tree`, `tag --list 'rev-*'`）。libgit2 は使わない（ddq の依存を増やさない。執筆者は Git を持っている前提）。
- **パスを出力する git コマンドには必ず `-c core.quotepath=off` を付ける**。既定では非 ASCII のパスが `"\346\226\207\346\233\270/..."` と 8 進エスケープされる（V5 で実測。`ls-tree` / `diff --name-only` / `status` が該当。`git show <ref>:<path>` の**中身**は影響を受けず常に UTF-8）。
- 日本語パスは Git 側では問題なく、絵文字も通る。制約は従来どおり ddq 側の `writing_folder::ensure_encodable`（CP932 に無い文字は Lua/Quarto が扱えない）。

### 5.5 YAML スキーマ（`revisions/rev-<記号>.yml`）

```yaml
rev: C                     # 改訂記号（ファイル名と一致。拡張が同期）
date: 2026-09-22
base: rev-B                # 人が選んだ比較基準の ref（タグ名・ブランチ名・コミット ID いずれも可）
base_commit: 3c1f8a2e9d4b7c105f3a2b8e6d7c419a0f5e3b21   # rev diff が解決した SHA。再取得はこれを使う
scheme: alpha              # alpha | numeric（次の記号の提案に使う。省略時 alpha）
entries:
  - label: sec-purpose
    kind: changed          # changed | added | removed | renamed | global
    title: 目的            # 表示用。build 時に最新の名称へ同期（removed は旧名を保持）
    file: chapters/01-overview/01-purpose.qmd
    note: |
      対象システムに○○を追加
  - label: fig-net
    kind: removed
    title: ネットワーク構成
    note: 図を統合
```

- `status` は持たない。**`rev-<記号>` タグが存在すれば fixed**（読み取り専用）、無ければ draft。
- **`base` と `base_commit` は両方持つ**（`base` は人が選んだ名前、`base_commit` はそれを `git rev-parse` で解決した SHA）。タグは付け替え・削除ができるため名前だけでは「同じ yml を再取得すると違う差分が出る」ことがあり、逆に SHA だけではそのコミットが到達不能になったとき（GC・shallow clone）に復元できない。タグは到達性を保つ錨でもあるので、名前と解決結果の両方を残す（ロックファイルが指定と解決済み版の両方を持つのと同じ）。
  - 再取得は `base_commit` で行う。`base` が今も解決でき、かつ `base_commit` と食い違うときは「`rev-B` が別のコミットを指しています（タグの付け替え？）」と警告し、固定のままか新しい方に更新するかを人が選ぶ。
  - `base_commit` が到達不能なら `base` にフォールバックして警告。`base` を省略して SHA を直接指定した場合は `base_commit` のみ。
  - 新版側（作業ツリー）のコミット ID は `rev build` の時点では存在しないので持たない。タグ付けが人手である限り後から書き戻せないが、**次の改訂の `base_commit` がその値になる**ため履歴を遡れば確定できる。
- `rev diff` の再取得時は `label` をキーに既存 `note` を引き継ぐ。差分から消えたエントリ（変更を戻した等）は note があれば残して `stale: true` を付け、UI で灰色表示（人が消す）。
- `global` エントリは `--include-global` を付けたときだけ生成（既定オフ）。

### 5.6 生成物（`ddq rev build`）

テンプレートは前付け（改正履歴・用語）を `index.qmd`（`{.unnumbered}`）に書く規約で、`lib.typ` も「目次より前の改訂履歴ページ」を想定済み（`design-toc`）。そこで**独立した章は作らず**、生成物を `revisions/history.qmd` に出し、人が管理する `index.qmd` から include する:

```markdown
## 改訂履歴 {.unnumbered}

{{< include revisions/history.qmd >}}
```

`revisions/history.qmd`（生成物。scaffold に空の雛形と上記 include を入れておく）:

```markdown
<!-- ddq rev build が生成する。手で編集しない（revisions/*.yml を直す）。 -->
::: {.content-visible when-format="typst"}
::: {.tbl widths="14,8,22,8,48"}
| 改訂日 | 記号 | 箇所 | 頁 | 修正内容 |
|---|---|---|---|---|
| 2026-09-22 | C | @sec-purpose | `#_xref-page("sec-purpose")`{=typst} | 対象システムに○○を追加 |
| 2026-09-22 | C | @tbl-a3f9c1 | `#_xref-page("tbl-a3f9c1")`{=typst} | 新規追加 |
| 2026-09-22 | C | ネットワーク構成（図・削除） | – | 図を統合 |
:::
:::

::: {.content-visible unless-format="typst"}
::: {.tbl widths="16,8,26,50"}
| 改訂日 | 記号 | 箇所 | 修正内容 |
|---|---|---|---|
| … |
:::
:::
```

- **caption/label は付けない**（番号無し表）。前付けは採番されない章なので、キャプションを付けると「表 0-1」になる（テンプレートの既知の制約）。差分対象からは `revisions/` 配下を除外する。
- **`merge-cols` は使わない**。V2 で確認したところ、改訂日・記号を縦結合すると自動分割で次ページに続いた部分の結合セルが空欄になり、どの改訂か読めない。全行に改訂日・記号を繰り返す方が改ページに強い。
- 全 `revisions/*.yml` を改訂記号順（scheme に従う）、その中は文書順で並べる。
- note が空 → 警告を出して空セルで載せる。`--check` は警告があれば非 0 終了（CI 用）。
- `.tbl` の自動分割（長い表のページ跨ぎ）は既存機構に任せる（V2: 300 行・17 ページで問題なし、見出し行は各ページに繰り返される）。
- `index.qmd` に include が無ければ警告（生成はする）。
- HTML では `@sec-x` が章を指すとき Quarto の既定で「14  統一テーブル .tbl の記法例」のように**番号＋章題**が出る（節以下は「1.2節」）。列幅を食うが `postprocess-html.js` の `@sec-` 後処理の範囲外なので v1 では許容し、気になれば後処理に「章タイトルを落とす」分岐を足す。

### 5.7 PDF のページ番号（検証済み・採用）

`template/lib.typ` の `_xref` は `query(label(...))` で参照先の `location()` を取っている。同じ `loc` でページカウンタを読む `_xref-page` を `_xref` の直後に追加した（作業ツリーに変更あり）:

```typst
#let _xref-page(name) = context {
  let sn = query(label("sn-" + name))
  let hit = if sn.len() > 0 { sn } else { query(label(name)) }
  if hit.len() > 0 {
    let loc = hit.first().location()
    link(loc, [#counter(page).at(loc).first()])
  } else {
    text(fill: red)[?]
  }
}
```

見出し（Quarto の `<sec-x>`）・`.tbl`/`.ipo`（`<sn-tbl-x>`）・素の pipe table（`<tbl-x>`）・図（`<fig-x>`）の 5 種すべてで、印字されたページ番号が実際の掲載ページと一致することを確認した（§8 V2）。`_page-start`（開始ページ番号）はカウンタ更新済みなので表示ページと一致する。300 行の表（17 ページに自動分割）でも Typst は警告なしに収束し、ビルド時間も変わらない（約 6 秒）。

### 5.8 拡張側（改訂履歴 Custom Editor）

- `contributes.customEditors`: `viewType: ddqRevision.editor`, `selector: [{ filenamePattern: "**/revisions/rev-*.yml" }]`, `priority: default`（テキストで開きたいときは「Reopen With」）。
- 画面構成:
  - ヘッダ: 改訂記号（編集可、変更でファイル名をリネーム）／ 改訂日 ／ 基準 ref（既定は `rev next` の候補、任意入力可）／ [差分を再取得] [章を生成] [確定]
  - 表: 種別 ／ 箇所（ラベル + 名称、クリックで該当行へジャンプ）／ [差分] ／ 修正内容（複数行 textarea）／ stale 行は灰色
  - フッタ: `unlabeled` があれば「ラベル未付与の見出し・表・図があります → タグ付け画面へ」、note 空の件数
- [差分] → `vscode.commands.executeCommand('vscode.diff', 左, 右, title, { selection })`（V3 で実機確認。行範囲へのスクロールは `selection` で渡す）。左右の URI は種別で使い分ける:

  | 種別 | 左（旧版） | 右（新版） |
  |---|---|---|
  | changed / renamed | `api.toGitUri(fileUri, base_commit)` | 作業ツリーの `file:` URI |
  | added | `ddq-rev:<名前>?empty`（自前スキーム。空） | 作業ツリーの `file:` URI |
  | removed | `api.toGitUri(fileUri, base_commit)` | `ddq-rev:<名前>?empty` |

  `api` は内蔵 Git 拡張（`vscode.extensions.getExtension('vscode.git')` → `activate()` → `getAPI(1)`）。**旧版に存在しないファイルの `git:` URI は `openTextDocument` が例外になる**（空文書にはならない。V3 で実測）ので、added の左ペインは自前の `TextDocumentContentProvider`（スキーム `ddq-rev`、空文字列を返す）で出す。removed の右ペインも同じ。
- 基準に**コミット SHA を渡しても `toGitUri` は機能する**ので、`base_commit` をそのまま使える（タグ名である必要はない）。
- 編集モデル: Webview 内の状態 ↔ `CustomTextEditorProvider` の `TextDocument`（YAML 文字列）。textarea の変更は `WorkspaceEdit` で yml 全体を書き換える（ddq-table-editor の `applyTable` と同じ発想。YAML の直列化は拡張側で `js-yaml`、キー順・複数行 `|` を固定して差分を汚さない）。保存・dirty・Undo・外部変更検知は VSCode 任せ。
- fixed（タグ有）のファイルは表を読み取り専用にし、ヘッダに「rev-C タグ済み。直すには `git tag -d rev-C`」と表示。
- 「新しい改訂を開始」コマンド: `ddq rev next --json` で記号と base を決め、`rev diff` の結果から `revisions/rev-<記号>.yml` を生成して開く。draft が既にあれば警告して開く。
- 「確定」: note 空を確認 → `ddq rev build` → 通知に `git add docs/revisions && git commit -m "rev C" && git tag rev-C` を表示（コピー可）。実行はしない（Q17）。

---

## 6. リポジトリ構成の変更

```
extensions/
  ddq-table-editor/      # extension/ から移動（内容は変更なし）
  ddq-revision/          # 新規。ddq-table-editor と同じ esbuild + vite + React + vitest 構成
    src/extension.ts     # コマンド登録, CustomTextEditorProvider, ddq 起動
    src/ddq/             # spawn と JSON 型定義 (tag list / rev diff / rev next)
    src/revision/        # yml ⇄ モデル, note 引き継ぎ, 記号の +1
    src/webview/         # TagTable.tsx, RevisionEditor.tsx, 共通 Grid
cli/src/commands/
  tag.rs                 # list / apply
  rev.rs                 # diff / build / next
cli/src/doc/             # 新規モジュール: 論理文書 (scan.rs), units.rs, labels.rs, gitsrc.rs, diff.rs
template/
  lib.typ                # _xref-page 追加（済・未コミット）
  postprocess-html.js    # 別章の .tbl/.ipo への @tbl- リンクを相対パスに直す（済・未コミット）
  scaffold/content/revisions/history.qmd            # 生成物の雛形（空表）
  scaffold/content/index.qmd                        # 「## 改訂履歴 {.unnumbered}」+ include
  scaffold/repo/vscode/settings.json                # ddqRevision.ddqPath
  VERSION                # 2.3.0
docs/
  revision-study.md      # 本書
```

- `cli/src/commands/release.rs` の `EXTENSION_DIR`/`EXTENSION_NAME` を配列にし、`extensions/*/` を順に `npm ci && npm run package` して VSIX を 2 本同梱する。版一致検査は両方に適用。
- `.github` の CI・`docs/README.md`・実装仕様書・`AGENT-GUIDE.md` の `extension/` 参照を `extensions/ddq-table-editor/` に直す。
- 共通 Webview 部品（Grid）は当面 `ddq-revision` 内に置き、二重化が実害になった時点で `extensions/shared/` に切り出す（先に抽象化しない）。

---

## 7. 実現性評価

| 要素 | 評価 | 根拠・備考 |
|---|---|---|
| 行頭パターンによる見出し・表・図の検出 | ◎ 検証済（V4） | 試作で examples/docs の見出し 112・表 22・IPO 3・図 19 を検出。既存 `design-doc.lua` も同程度の緩さで扱っている |
| include の再帰展開 | ◎ 検証済（V4） | パスの基準は「章ファイルのディレクトリ」で入れ子でも不変（§5.1）。examples/docs の 71 ファイルを正しく展開できた |
| 旧版の取得 | ◎ | `git show <ref>:<path>` のみ。サブモジュール・LFS は対象外 |
| 差分帰属（Q5/Q12 の規則） | ○ | ユニット化して文字列比較するだけ。行単位 diff は不要（差分の**中身**は VSCode の差分エディタが見せる） |
| 1 行内挿入による書き戻し | ◎ | 行の増減が無いので `WorkspaceEdit`/CLI どちらも単純 |
| Custom Editor（YAML） | ◎ | VSCode 標準 API。ddq-table-editor の Webview 基盤を流用 |
| `vscode.diff` で `git:` URI | ◎ 検証済（V3） | 内蔵 Git 拡張の `toGitUri` で可能（タグ・SHA とも）。日本語パスも可。added だけ自前スキームが要る |
| `.tbl` セル内の `@sec-`/`@tbl-`/`@fig-` | ◎ 検証済（V1） | PDF は 5 種すべて番号リンクになった。HTML は `.tbl`/`.ipo` への参照が別章だとリンク切れになる既存バグがあり、`postprocess-html.js` を直して解消した |
| PDF 頁列（`_xref-page`） | ◎ 検証済（V2） | 5 種すべて正しいページを指し、300 行でも収束 |
| `content-visible` で表の出し分け | ◎ 検証済（V1） | `.tbl` を `content-visible` の Div で包んでも PDF/HTML とも動いた |
| `_quarto.yml` に独自キーを足す | × 回避 | Quarto はプロジェクト設定を検証するため、v1 では**パスを規約固定**（`revisions/rev-*.yml`, `revisions/history.qmd`）にし設定を持たない |

---

## 8. 検証項目

| # | 内容 | 方法 | 結果 |
|---|---|---|---|
| V1 | `.tbl` セル内の `@sec-x` / `@tbl-x` / `@fig-x` が PDF・HTML で番号付きリンクになる。`content-visible` の Div で包んだ `.tbl` が動く | `examples/docs/index.qmd` から `revisions/history.qmd` を include し、見出し・`.tbl`・`.ipo`・素の pipe table・図の 5 種を参照する表を置いて `ddq pdf` / `ddq html` | **合格（2026-09-22）**。PDF: 「14 章」「表 3-1」「表 6.4.1.3-1」「表 4.1.2-1」「図 1.2-1」。HTML: 5 種とも番号リンク。ただし `.tbl`/`.ipo` への参照は `href="#tbl-cond"` のみで**別章からはリンク切れ**（既存バグ。`chapters/07-performance` → `@tbl-cond` も同じ）→ `postprocess-html.js` のパス 1 で id→章ファイルを控え、パス 2 で相対パスを補う修正を入れて解消（`href="chapters/03-design-conditions/index.html#tbl-cond"`）。`content-visible` の入れ子も両形式で動作 |
| V2 | `_xref-page` で頁が出る。300 行の改訂履歴表 + 自動分割で収束する | `lib.typ` に `_xref-page` を足して ddq を再ビルドし、6 行の表と 300 行の表で `ddq pdf` | **合格**。6 行: 5 種とも印字ページ（40, 7, 18, 9, 5）が実際の掲載ページと一致。300 行: 4〜20 ページに自動分割、8 ラベル × 300 行すべて正しいページ、Typst の警告なし、ビルド約 6 秒（変化なし）。副産物: `merge-cols="1,2"` は改ページ後の結合セルが空欄になるため不採用（§5.6） |
| V3 | `git:` URI の `vscode.diff` で旧版（タグ）と作業ツリーの差分が開き、行範囲へスクロールできる | 最小拡張を作り、インストール済み VSCode を `--extensionDevelopmentPath` / `--extensionTestsPath` / 専用 `--user-data-dir` で起動して実拡張ホストで実行（検証ワークスペースは V5 の日本語パス・リポジトリ） | **合格（2026-09-22）**。`toGitUri` で旧版を読め、差分タブが `TabInputTextDiff`（左 `git:` / 右 `file:`）で開く。タグでも SHA でも可。日本語パスも可。**added のファイルは `git:` URI が例外**になるため自前スキーム（空）で代替し、removed の右ペインも同じ方法で開けることを確認。なお `--user-data-dir` を新規にすると workspace trust で内蔵 Git 拡張が読み込まれないため `--disable-workspace-trust` が要る（検証手順上の注意） |
| V4 | 見出し行末 `{#sec-x}` が ddq-table-editor・Quarto プレビュー・HTML 目次に副作用を出さない | `ddq tag list/apply` の試作（Python）で examples/docs の全見出し 112 件＋ラベル未付与の表 2 件に一括付与し、付与前後で PDF・HTML を比較 | **合格（2026-09-22）**。PDF は 50 ページ・全ページのテキストが**完全一致**。HTML も 15 ファイルすべて本文テキストが一致、図表番号 164 件の並びも一致、内部リンク 163 件にリンク切れ無し。アンカーは `#本書について` → `#sec-02a174` に変わり、目次のリンクも追随。ddq-table-editor の単体テスト 168 件も通る（表の検出は `:::` と `\|` しか見ないため見出し属性の影響を受けない）。**副産物**: include の解決基準と、IPO 内側の見出しを除外する規則が判明（§5.1・§4.1） |
| V5 | 日本語パス・CP932 環境で `git show` の出力が正しく UTF-8 として読める | `設計書リポジトリ/文書/chapters/01-概要/index.qmd` という日本語パスのリポジトリを作り、タグ・作業ツリー・削除・追加を用意して実測 | **合格（2026-09-22）**。`git show <ref>:<path>` の**中身は常に UTF-8**。パスを**出力する**コマンド（`ls-tree` / `diff --name-only` / `status`）は既定で `"\346\226\207\346\233\270/…"` と 8 進エスケープされるため `-c core.quotepath=off` が必須。絵文字パスも Git 側は通る（制約は ddq の CP932 検査）。**発見**: `core.autocrlf=true` では作業ツリーが CRLF・`git show` が LF になるため、改行の正規化は `--strict` でも必須（§5.3） |

検証で入れた変更:

- `template/postprocess-html.js`: `fileOf` マップと href 補正（+16 行 / −1 行）。**2.2.2 としてリリース済み**（PR #97。既存文書の別章 `@tbl-` 参照も直るため ddq-revision と切り離した）
- `template/lib.typ`: `_xref-page(name)` を `_xref` の直後に追加（+16 行）。**未コミット**。頁列を出す機能なので ddq-revision の実装（段階 4）で入れる

検証に使った試作（`scratchpad/`。リポジトリには入れていない）:

- `tagproto.py`: `ddq tag list/apply` の試作。§4.1・§4.2 の規則をそのまま実装したもので、Rust 実装の期待値（fixture の qmd → 期待 JSON）を作るのに使える
- `v3ext/`: V3 用の最小 VSCode 拡張。`code.exe --extensionDevelopmentPath=<拡張> --extensionTestsPath=<テスト> --user-data-dir=<専用> --disable-workspace-trust <ワークスペース>` で実拡張ホストのまま自動実行できる。ddq-revision の E2E テストも同じ形にできる

---

## 9. 実装ステップ

| 段階 | 内容 | 成果 |
|---|---|---|
| 0 | **V1〜V5 完了（2026-09-22）** | 頁列は採用。`postprocess-html.js` の修正は 2.2.2 として先行リリース済み。タグ付けの試作（Python）は Rust 実装の期待値に使える |
| 1 | **完了**（PR #99）。`cli/src/doc/`（project, units, labels）+ `ddq tag list/apply` | CLI 単体でタグ付けが回る |
| 2 | **完了**（PR #101）。`extensions/` への移動 + `release.rs` 複数 VSIX + CI の matrix | 既存拡張のリリースが変わらないことを確認 |
| 3 | **完了**。`extensions/ddq-revision/`: タグ付け画面（Webview 表 + WorkspaceEdit）。実拡張ホストでの検証も同梱 | 機能 1 完成 |
| 4 | **完了**。`cli/src/doc/`（gitsrc, diff, revfile）+ `ddq rev next/diff/build` + `lib.typ` の `_xref-page` + scaffold（`revisions/history.qmd`・`index.qmd` の include）。版 2.3.0 | CLI 単体で改訂履歴が出る（サンプル文書で PDF の頁列まで確認） |
| 5 | **完了**。Custom Editor（差分表・note・確定）。実拡張ホストで往復を確認 | 機能 2 完成 |
| 6 | **完了**。利用マニュアルに 13 章「改訂履歴を作る」を追加（13〜17 章は 1 つずつ繰り下げ）。サンプル文書（`examples/docs`）に実例の改訂履歴を用意。実装仕様書は `docs/revision-impl/` に起こした（`cli-impl` の更新は別途） | リリース 2.3.0 |

- 段階 1・4 は VSCode 無しで完結するので、拡張と並行できる。
- 自動タグ付け・自動コミット（Q17 で保留）は段階 6 の運用結果を見て追加する。

---

## 10. 未決事項

| # | 内容 | 現時点の扱い |
|---|---|---|
| U1 | `.tbl` でキャプション無し（番号無し）表の改訂 | 包含見出しに帰属（§5.2）。表としての履歴が必要なら `caption` を付ける運用 |
| U2 | 裸の `` ```{mermaid} `` / `![]()` の図 | 自動付与しない（Q4）。警告に留め、手で `::: {#fig-}` に包む |
| U3 | 改訂履歴に「(文書全体)」を載せるか | 既定オフ。`_quarto.yml` の版番号変更などを載せたい場合に人がオンにする |
| U4 | 同一改訂で複数の下書き（複数人並行） | 非対応（draft は 1 つ。2 つ以上で警告）。必要になれば `rev-C-<name>.yml` をマージする案 |
| U5 | 改訂記号の `numeric` 方式の書式（`1`/`1.0`/`第2版`） | `1, 2, 3` のみ。他は自由入力で対応 |
| U6 | 頁列 | 採用（V2 合格） |
