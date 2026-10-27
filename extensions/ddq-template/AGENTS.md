# AGENTS.md

## プロジェクト概要

`design-doc-quarto-template`（ddq）の VSCode 拡張 **ddq-template**。旧 `ddq-revision`（見出し・表・図の
ラベル付けと改訂履歴）と旧 `ddq-table-editor`（統一テーブル記法 `::: {.tbl …}` の編集。さらに旧名
`quarto-table-support`）を 1 本に統合したもの。このリポジトリの `extensions/ddq-template/` に置き、
`ddq release` がリリース一式へ VSIX として同梱する（`../../cli/src/commands/release.rs`）。

**設計の正:**

- 拡張全体（統合・ddq の同梱・今後の段階）… [docs/ddq-template-study.md](../../docs/ddq-template-study.md)
- ラベル一覧・改訂履歴 … [docs/revision-study.md](../../docs/revision-study.md)（§4.4・§5.8）
- 表エディタ … [tableEditor-requirements.md](tableEditor-requirements.md)

実装前に該当するものを必ず読むこと。

| パス | 内容 |
|---|---|
| `src/extension.ts` | 入口。機能ごとの `activate*` を呼ぶだけ |
| `src/legacy.ts` | 旧拡張の検出・アンインストールの案内 |
| `src/build.ts` `src/folders.ts` `src/quarto.ts` `src/ddq/{locate,lock,version}.ts` | 発行（pdf / html / diagrams）・版不一致の案内と update・執筆フォルダの決定・ステータスバー |
| `src/activateRevision.ts` `src/ddq/` `src/tagging/` `src/revision/` `src/revisionWebview/` | ラベル一覧・改訂履歴 |
| `src/tableEditor/` | 表エディタ（旧 ddq-table-editor の `src/` をそのまま移したもの） |

ID は `ddqTemplate.*` に統一する（コマンド・設定・viewType）。旧 ID（`ddqRevision.*` /
`ddqTableEditor.*`）は使わない。

## 共通で守ること

- **通信しない。** 完全オフライン・エアギャップ環境で動くこと。`npm run check:offline` が成果物を
  検査する。Webview の CSP は `default-src 'none'`（`connect-src` を書かない）。CDN の JS / CSS /
  フォント / 画像を使わず、フロントエンド資産はすべて拡張に同梱する
- `package.json` の `version` は `../../template/VERSION` と同じ値にする（`ddq release` が照合する）
- 画面の配色は VSCode のテーマ変数（`--vscode-*`）だけを使う
- Webview は画面ごとに別の IIFE（`vite build --mode revision` / `--mode tableEditor`）で、
  出力は `out/webview/<画面>/assets/main.{js,css}`
- `.gitignore` はリポジトリのルートにある（ルートの `/*/.gitignore` 規則でここの `.gitignore` は
  無視される）。`node_modules/` `out/` `*.vsix` はそこで除外済み
- CI は `../../.github/workflows/extension.yml`
- **ddq の終了コード 3 は「執筆フォルダの機構が ddq と一致しない」**（`cli/src/commands/setup.rs` の
  `TEMPLATE_MISMATCH_EXIT_CODE`）。拡張はこれで update の案内を出すので、どちらかを変えるときは両方を直す
- ddq は `src/ddq/locate.ts` の解決順（設定 → 同梱 → PATH）でだけ決める。直接 `'ddq'` を起動しない

## ラベル一覧・改訂履歴

- **qmd を解釈しない。** 文書の走査・ラベルの候補・差分はすべて `ddq` が返す JSON に由来する
  （同じ解釈を CLI と拡張の 2 か所に持たない）。この機能がやるのは画面と `WorkspaceEdit` だけ
- **書き戻しは `WorkspaceEdit`。** ddq にファイルを書かせない（Undo が効き、未保存の
  バッファにも当たるため）

Webview の描画は jsdom のテスト（`src/revisionWebview/*.test.tsx`）で見る。「パネルが真っ白」は
実拡張ホストの検証でもタブが開くだけで気付けないため。

## 表エディタ

`mkdocs-table-editor` をベースにしているが、**出力ルールは全面的に異なる**。

参照すべき外部ドキュメント:

- `../../docs/manual/chapters/` の表の章 … 執筆者から見た `.tbl` の記法
- `../../template/design-doc.lua` … 実際の変換処理。
  `merge_body` と `Div` ハンドラが表エディタの挙動の根拠

### 設計原則

エディタ UI と記法変換ロジックを分離する。React エディタは `.tbl` / パイプ表 /
グリッド表 / `merge-cols` を一切知らない。

```text
React Table Editor
        ↓
TableModel（src/tableEditor/model/）
        ↓
Format Parser / Serializer（src/tableEditor/formats/）
  ├─ pipeTable
  ├─ gridTable
  ├─ mergeCols（判定・展開）
  └─ tbl（div 属性）
```

### 落とし穴（検証済み・変更しないこと）

- **グリッド表の桁は表示幅（東アジア全角=2）で数える。** pandoc は文字数ではなく
  表示幅で列位置を決める。文字数で揃えた表は 1 セルに潰れる。
  シリアライザ・パーサとも `src/tableEditor/formats/displayWidth.ts` を通すこと。
- **グリッド表のセル内改行は行末バックスラッシュ `\`。** 単に行を分けるだけでは
  pandoc はソフト改行（空白）にする。`<br>` でも改行できるが、バックスラッシュなら
  native な LineBreak になり PDF（typst）でもそのまま効き、後続行に余分な空白が入らない。
- **セル内改行がある表は必ずグリッド表で出す。** パイプ表では改行が `<br>` になり、
  箇条書き・番号付きリストをセルに置けない。`merge-cols` を選んでいるときも、結合を
  展開した**素のグリッド表**にする（`is_plain_grid` ガードを通るのでフィルタ側で結合される。
  pandoc ＋ `design-doc.lua` で確認済み）。
- **リスト項目の前・空行の前後には `\` を付けない。** 付けると項目末尾に `<br>` が入る。
- **文章行の直後にリストが来るときは空行を補う。** pandoc の markdown はリストで段落を
  中断しないため（`lists_without_preceding_blankline` は既定で無効）、空行が無いと
  `- 項目` が文字のまま出る。エディタ上は空行なしで書けるようにし、書き出し時に補って、
  読み戻しでは畳む（`insertBlankLinesAroundLists` と `collapseInsertedBlanks` は対称）。
  ここで空行の代わりに `\` を付けると、リストにならないうえ項目がソフト改行で繋がる。
- **リスト項目の続き（空行なしで続く行）は段落に切り離さない。** `\` を付けて項目本文の
  桁まで字下げする。切り離すとリストが 2 つに割れる。段落にしたいときは書き手が空行を書く。
- **セル内の行頭の空白は落とさない。** 入れ子リストの階層と項目内折り返しの字下げに
  意味がある。読み込み時に外すのはセル共通の字下げだけ（`parseGridTable` の `dedent`）。
- **コード span の中は書き換えない。** `` `<br>` `` を文字として説明するセルがあるため
  （テンプレートの記法例がそれ）、`<br>` → 改行の変換で本文が壊れる。
- **行末のバックスラッシュを重ねない。** `\\` はエスケープされたバックスラッシュになり、
  改行が消えて `\` が文字として出る（pandoc で確認済み）。書き出し時にモデル側の行末
  バックスラッシュを落とし、読み取りでは `\\` も 1 つの改行として扱う（既に壊れた表を
  開いて書き戻すだけで直せるように）。
- **`merge-cols` の連鎖は列を左から 1 パスで評価する。** `design-doc.lua` の `merge_body` が
  `for c = 1, ncol` で回すため、`prevcol[c]` が c より右だとその時点では未評価
  （＝結合していない扱い）になる。並び順を逆にしても階層は作れない。
- **`merge-cols` は div 全体＝すべてのパートに適用される。** ただし `is_plain_grid`
  ガードがあるため、既に結合を持つグリッド表パートは影響を受けない。

### ブロック検出

`.tbl` div 自体がマーカー。旧版の `<!-- table-editor:start -->` コメントは使わない。

置換範囲は最小にすること。分割表ではカーソルのあるパートの行だけを差し替え、
div の fence 行・他パート・外側の `.landscape` には触れない。

開始 fence に対応する終了 fence が無い場合は、ドキュメントを一切変更せずエラーにする。

### Webview セキュリティ

- 制限的な CSP を使う
- リモートのスクリプト・スタイルを許可しない
- nonce ベースでスクリプトを読み込む
- ローカル資産は `webview.asWebviewUri` 経由

## 検証

コミット前は CI と同じ内容を一括で流す:

```bash
npm run verify      # 型検査・テスト・ビルド・オフライン検査・脆弱性検査
npm run test:host   # 実拡張ホストで一覧・書き戻し・Undo・改訂履歴を見る
```

配布する VSIX は ddq.exe を同梱したもの（`npm run package:win32-x64`。`bin/win32-x64/ddq.exe` は
`ddq release` がコピーする。git 管理外）。同梱版は `npm run test:smoke` で、展開した VSIX の中身のまま
init → html を通して確かめる（CI では ci.yml の vsix-smoke。main への push と手動実行のみ）。

`test:host` の VSCode は「インストール済み → 無ければ `@vscode/test-electron` で取得」、
ddq は「ビルド済み → 無ければ golden を返す差し替え」の順に決まる。どちらも開発時だけの
話で、配布物（VSIX）には入らない。golden を採り直すときは README を参照。

UI 以外のロジックにユニットテストを書く。純粋な TypeScript 関数を Webview の外で
テストすることを優先し、手動 UI テストだけに頼らない。

表の記法まわりの検証は `src/tableEditor/formats/pandoc.test.ts` が**本物の pandoc**に対して行う。
pandoc が無い環境では自動スキップし、CI では `REQUIRE_PANDOC=1` でスキップを禁止する
（検査が黙って素通りしないように）。新しい記法を足したらここにケースを追加すること。

テンプレート込みの手動確認が要るときは:

```bash
"/c/Program Files/Quarto/bin/tools/pandoc.exe" -f markdown -t html --lua-filter=../../template/design-doc.lua sample/tables.qmd
```

### 検査を形骸化させないこと

自動検査は「失敗する条件」を必ず確認してから入れる。実績:

- `scripts/check-offline.mjs` … `fetch` / `WebSocket` を含むファイルを渡して exit 1 を確認済み
- `pandoc.test.ts` … `REQUIRE_PANDOC=1` かつ pandoc 不在で exit 1 を確認済み

## 品質方針

- 単純で保守しやすいコードを優先する
- テストしにくい凝ったパースロジックを避ける
- VSCode のドキュメント操作と Webview の UI ロジックを混ぜない
- 無関係なファイルを変更しない
- 大きな依存関係を理由なく導入しない
- 表の新しいフォーマットを足すときは `src/tableEditor/formats/<name>/` に閉じ、React 側のモデルを変えない
