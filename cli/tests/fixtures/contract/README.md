# ddq と VSCode 拡張の契約試験の期待値（QA-01）

ddq（Rust）が出し、拡張（TypeScript）が読むものの期待値を、両方の試験が**同じファイル**から読む。
片方だけ直すと、もう片方の試験が落ちる。

| ファイル | 中身 | 読む試験 |
|---|---|---|
| `manuscript.json` | 最小原稿。日本語・サロゲートペア（𠮷）・結合文字（か＋濁点）・BOM・CRLF・include・`.tbl`・`.ipo`・パイプ表・ラベル無しの図 | 下の全部 |
| `tag-list.json` | `ddq tag list <docs> --json`（`folder` は `{{FOLDER}}`） | `cli/tests/contract.rs`、`extensions/ddq-template/src/ddq/contract.test.ts` |
| `tag-list.2.4.4.json` | 2.4.4 の同じ出力。`edit.original` が無い旧版の形（一覧は出せるが書き戻しは拒否する） | `contract.test.ts` |
| `tag-applied.json` | `ddq tag apply <docs> --all` の後の各ファイル。拡張が同じ編集指示を当てた結果とも一致する（BOM の扱いは文字コード層の違いなので除いて比べる） | 両方 |
| `rev-next.json` | Git 管理下で `ddq rev next <docs> --json` | 両方 |
| `cli.json` | 終了コード・結果行（`OK ->` / `OK:` / `警告:`）・エラー行 | 両方（`OK ->` / `OK:` は `cli/tests/e2e.rs`） |

| `diagrams.json` | 図のキャッシュ（GEN-02）。mermaid は原稿 → フィルタが受け取るコード → キー、PlantUML は図＋共通設定 → サーバに送るソース → キー。SVG の先頭コメントでの照合（`fresh` / `pumlEnv`）と、先頭に書く 1 行の形 | `cli/src/mermaid/prerender.rs`・`cli/src/plantuml/mod.rs` の単体試験、`cli/tests/filter_contract.rs`（本物の design-doc.lua） |
| `merge-cols.json` | `.tbl` の merge-cols の結合（GEN-02）。原稿とボディ行ごとの rowspan | `cli/tests/filter_contract.rs`、`extensions/ddq-template/src/tableEditor/formats/mergeCols/contract.test.ts` |
| `chapters.json` | `_quarto.yml` の章の並び（GEN-01）。`cases` は `_quarto.yml` 全体と、そこから読む章・付録の並び（行末コメントで章が落ちた 17ed32e の雛形そのもの。リリースには入っていない）。`items` は `- ` の後ろ 1 件の書き方 | `cli/src/doc/project.rs` の単体試験、`cli/tests/filter_contract.rs`（本物の postprocess-html.js を `quarto run` で） |
| `rev-values.json` | 改訂ファイル（`revisions/rev-*.yml`）の 1 行の値（DATA-01）。`read` は書かれた形 → 読んだ値、`write` は値 → 書き出す形（その形を読むと元の値） | `cli/src/doc/revfile.rs`・`extensions/ddq-template/src/revision/revfile.ts` の単体試験（`parse` / `to_yaml`・`toYaml` を通して） |

`{{FOLDER}}` は、ddq が引数の執筆フォルダを `std::path::absolute` で解決したパス（Windows では
`/` が `\` になり、`.` / `..` が畳まれる）。`{{SEP}}` はその OS のパス区切り。

## 採取の記録

| 日付 | exe | 内容 |
|---|---|---|
| 2026-11-22 | HEAD `17ed32e` の release ビルド（`ddq 3.1.0-pre1`）。保管版 3.1.0-pre1・3.0.0 の `ddq.exe` も同じ出力 | 初回採取 |
| 2026-11-22 | 保管版 2.4.4 の `ddq.exe` | `tag-list.2.4.4.json`。HEAD との差は `edit.original` の有無だけ |
| 2026-11-22 | （exe は使わない。下の「図・表の期待値の決め方」） | `diagrams.json`・`merge-cols.json` の初回作成 |
| 2026-11-22 | （exe は使わない。下の「章の並びの期待値の決め方」） | `chapters.json` の初回作成 |
| 2026-11-22 | （exe は使わない。下の「改訂ファイルの値の期待値の決め方」） | `rev-values.json` の初回作成 |
| 2026-11-22 | （exe は使わない。IO-01 で BOM を保持すると決めた（利用者の承認）ので規則から足した） | `tag-applied.json` の `chapters/02/index.qmd`（原稿に BOM がある）の先頭に BOM を足した。ほかは採取したまま |

期待値を変えたら、上の表に理由と exe の版を足す。**既存の期待値を黙って上書きしない**
（出力が変わったこと自体が、拡張側の対応が要るかどうかを判断する材料になる）。

## 採り直し方

`manuscript.json` の `files` を一時フォルダの `docs/` に書き出し（BOM・CRLF を含めバイトどおり）、
その場で実行した出力の `folder` を `{{FOLDER}}` に置き換えて保存する。`rev next` は `docs/` の親で
`git init` → 全ファイルをコミットしてから実行する。`tag apply --all` は書き換え後のファイルを
`tag-applied.json` の `files` に入れる。

## 図・表の期待値の決め方（GEN-02）

Rust・Lua・TypeScript のどの実装の出力も期待値に使わない。

- mermaid のコード … 原稿を Quarto 同梱の Pandoc（`pandoc -f markdown -t json`）に読ませ、
  `CodeBlock` の text を取ったもの（フィルタが受け取るのはこれ）。```` ```{mermaid} ```` は
  Quarto が直してから渡すので、素の Pandoc では試さない（`quartoOnly`。コードは手で書いた）
- キー … `hashlib.sha1("\n\0\n".join([版, "mermaid", エンジン, 設定(LF), コード]).encode()).hexdigest()[:16]`
  （PlantUML は `[版, "plantuml", ソース]`）。版は `version`（試験では `.template-version` に書く）
- PlantUML のソース・先頭コメントの照合・表の rowspan … design-doc.lua のコメントに書かれた規則から手で決めた

**既知の差**は期待値の側に印を付けて固定している。直したら印を外す（試験が差の消えたことを知らせる）。

| 印 | 中身 |
|---|---|
| `rustDiffers`（mermaid・タブ） | Pandoc はコードのタブを空白 4 つに展開するが、prerender の `mermaid_blocks` はタブのまま。キーがずれ、先回りで描いた SVG をフィルタが使わずに描き直す（図・図の名前は同じ） |
| `rustDiffers`（PlantUML・`@start` の前の空白） | Rust の `assemble_source` は落とし、Lua は残す。Rust はこれでキーを作らない（`ddq diagrams` が `.puml` を送るときだけ）ので図の名前は変わらない |
| `luaWindowsOnly`（mermaid・CRLF の設定） | Lua の文字モードで CRLF が LF になるのは Windows だけ。ほかの OS ではフィルタのキーに CR が残る（描き直すだけ） |
| `tsDiffers`（merge-cols） | 表エディタの表示だけが文書の出力と違う。書式つきのセル（原文で比べる）、`merge-cols="ALL"`・`""`（全列とみなす）、旧 `.merge-rows`（結合を展開する） |

## 章の並びの期待値の決め方（GEN-01）

ddq（Rust）・postprocess-html.js のどちらの出力も期待値に使わない。

- `cases` … 各 `_quarto.yml` を一時フォルダに置き、並びに出てくるファイルを見出しだけの中身で作って
  `quarto inspect` を通し、`config.book.chapters`（`part` はその `chapters` を展開）と
  `config.book.appendices` を書かれた順に並べたもの。PyYAML で読んだ並びとも一致することを確かめた。
  逆斜線は `/` に揃える（両実装ともパスの区切りを `/` にする）
- `items` … YAML の規則から手で決めた。二重引用の中の `\x` だけは改訂ファイルの読み方（DATA-01）と同じく
  「`\\` は `\`、`\"` は `"`、ほかはそのまま」で、YAML（`\t` はタブ）とは違う
- `part` にファイルを書いたとき（`- part: part1.qmd`。Quarto は部の扉ページとして出す）は対象外。
  両実装ともその扉ページを文書の並びに入れない

## 改訂ファイルの値の期待値の決め方（DATA-01）

ddq（Rust）・拡張（TS）のどちらの出力も期待値に使わない。規則から手で決め、
生成スクリプトの中で「write の形を read の規則で読むと元の値に戻る」ことを確かめた。

- 読み方（`read`）… 二重引用は `\\` を `\`、`\"` を `"` にし、ほかの `\x`（`\n` も）は文字のまま。
  単一引用は `''` を `'` にし、逆斜線は文字のまま。引用なしはそのまま。YAML とは違う（`\n` を改行にしない）が、
  書き手が書く形を読み戻すにはこれで足りる
- 書き方（`write`）… 書き手は変えていない。ddq の `quote` と同じ判定で引用し、`\` と `"` を逃がす
- 旧拡張が保存して逆斜線を 2 倍にした値（`"C:\\\\docs"`）は、書き手の意図した本数をファイルだけでは
  決められないので、書かれたとおり（逆斜線 2 本）に読む。一括の書き換えはしない
- メモ（`note: |`）は逃がさないので対象外。名称（`title`）は見出し・キャプションの Markdown 原稿そのもの
