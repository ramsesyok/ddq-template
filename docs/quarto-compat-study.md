# Quarto 標準機能との整合と、機構ファイルの置き場の検討（公開ベータの前）

2026-11-23 作成・**未承認**（§2 の判断待ち）。実装の修正は含まない。
対象はテンプレート 3.1.0（`9beae4f`）。実測は Quarto 1.9.38（同梱の Typst・Pandoc）、Linux で行った。

論点は 2 つある。

1. 執筆フォルダの直下に置くファイルを、発行者・執筆者が変えうるもの（`_quarto.yml` など）だけにし、
   ほかを 1 か所（例 `.ddq/`）にまとめられないか。執筆者が余計なファイルを直してテンプレートを壊すのを避けたい
2. Quarto の標準機能で変えられる設定を、テンプレートが上書きしていないか。執筆者・発行者が Quarto の
   マニュアルどおりに書いて効かず、不具合と思う事態を避けたい

1 と 2 はぶつかる所があるので、本書は 2 を中心に調べ（§3〜§5）、1 は 2 の結論から見た置き場の選び方としてまとめる（§6）。

## 1. 要点

- **壊れるものが 3 つある**（§4.1）。どれも Quarto のマニュアルどおりの書き方で起きる
  - `book: chapters` の部（`- part:`）を使うと、**PDF が組版エラーで止まる**（`unknown variable: part`）。HTML は通る。
    部を扱う e2e の試験は `ddq html` だけで、PDF を作っていない
  - `crossref:` の `fig-title`・`tbl-title` を変えるか、`lang` を `ja` 以外にすると、**HTML の採番の振り直しが止まり、
    通常の表への参照が「?」になる**。PDF は振り直されるので、PDF と HTML で番号の形が食い違う
  - `highlight-style` を変えると、PDF のコードブロックの体裁の補正（文字の大きさ・両端揃えの解除）が外れる
- **黙って無視される設定が多い**（§4.2・§4.3）。PDF では用紙・余白・ページ番号・段組・書体・行間・言語・章番号の有無と深さ・
  `crossref` の接頭辞と区切り・日付・概要・図目次・表目次。HTML では書体と行間。どれも警告が出ない
- **標準の記法があるのに、独自の記法を案内しているもの**がある（§4.4）。列幅の `widths=`（標準の `tbl-colwidths` も実は効く）と、
  mermaid の ```` ```mermaid ````（標準の ```` ```{mermaid} ```` も通るが、PDF ではテンプレートの経路を通らずに PNG になる）
- 様式に要るのに Quarto に無い機能（外枠・資料番号・「図 3.2-1」の採番・IPO 図・表の分割と結合など）は独自で正しい。
  `.landscape`・`{{< pagebreak >}}`・`.unnumbered`・付録などは Quarto と同じ意味で作り直してあり、変えなくてよい（§4.5）
- **方針案**（§5）: Quarto のキーは「既定値は変えてよいが、書かれた値は尊重する」。様式のために尊重できないキーは
  **黙って無視せずに警告**し、マニュアルに「効かない Quarto の設定」の表を載せる。独自のキーは `ddq:` の下にまとめ、
  Quarto のキーと見分けられるようにする
- **論点 1 との関係**（§6）: `.ddq/` も `_extensions/ddq/`（Quarto の拡張機構）も、Quarto 1.9.38 で HTML・PDF とも動いた。
  **推奨は `_extensions/ddq/`**。`_quarto.yml` から機構の配線（`filters:`・`css:`・`post-render:`）が消え、PDF のプロファイル
  （`_quarto-publish.yml`）も要らなくなる。さらに、発行者が Quarto のマニュアルどおりに `css:`・`filters:`・`post-render:` を
  書いても、**テンプレートの指定に足される**（今の置き方では、同じキーを書き換えるとテンプレートの指定が消える）。
  論点 1 と 2 を一緒に満たせる

## 2. 決めてほしいこと

| # | 判断 | 選択肢 | 推奨と理由 |
|---|---|---|---|
| 1 | 機構ファイルの置き場 | (A) 今のまま直下　(B) `.ddq/`　(C) `_extensions/ddq/`（Quarto の拡張） | **(C)**。§6.4。`.ddq` という名前はあきらめることになる（§6.5） |
| 2 | 尊重できない Quarto の設定の扱い | (a) 今のまま黙って無視　(b) 警告して無視し、マニュアルに一覧を載せる　(c) すべて尊重する | **(b)**。様式（A4・外枠・資料番号）を崩さずに、効かない理由が分かる。(c) は様式と両立しない設定（`papersize`・`margin` など）がある |
| 3 | 章番号の有無と深さ（`number-sections`・`number-depth`） | (a) PDF は固定のまま　(b) PDF でも尊重する | **`number-depth` は (b)、`number-sections: false` は警告して無視**。HTML は今も両方を尊重しているので、PDF だけ固定だと番号の形が食い違う（付録 A の secnum）。番号の無い設計書は様式の前提から外れる |
| 4 | 書体（PDF の `mainfont`・`codefont`、HTML の `mainfont`・`monofont`） | (a) 固定して警告　(b) 尊重する | **(a)**。PDF・HTML・mermaid・PlantUML の 4 か所で同じ書体にそろえているのがテンプレートの値打ちで、本文だけ差し替わると図の書体と食い違う。(b) にするなら図の設定にも渡す仕組みが要る |
| 5 | 独自のキーの置き方 | (a) 今のまま最上位　(b) `ddq:` の下にまとめる | **(b)**。破壊的変更だが、今が最後の機会。読んでいるのは design-doc.lua・typst-show.typ と、ddq の PlantUML（`plantuml-server`）だけ（§4.6） |
| 6 | 列幅の記法 | (a) `widths=` のまま　(b) Quarto の `tbl-colwidths` を正にし、`widths=` は互換のため読む | **(b)**。素の表は `tbl-colwidths`（Quarto のマニュアルどおり）。`.tbl` は表に属性を付ける場所が無いので、div に同じ名前の `tbl-colwidths` を受け付ける |
| 7 | ```` ```{mermaid} ```` セル | (a) 今のまま（PDF では Quarto が PNG に描く）　(b) 検出して警告し、```` ```mermaid ```` へ誘導する | **(b)**。Quarto のセルは Lua フィルタより前に Quarto が処理するので、テンプレートの経路に載せ替えられない |

判断を待たずに直してよいもの: §4.1 の B-1〜B-3（不具合）。

## 3. 調べ方

- `template/` の機構ファイルと PDF 側のファイルを、`ddq update`・`ddq setup` と同じ配置で試験用の執筆フォルダに置いた。
  `_quarto.yml` に Quarto 標準の設定を 1 つずつ足し、`quarto render --to typst --profile publish -M keep-typ:true` と
  `quarto render --to html` を実行した（ddq は通していない。mermaid の ```` ```mermaid ```` は使っていない）
- PDF は `pdfinfo`（用紙）・`pdffonts`（埋め込まれた書体）・`pdftotext`（番号・参照の文字）と、生成された `.typ` を見た。
  HTML は出力の文字列と、ブラウザ（Playwright の Chromium）で計算された書式を見た
- この環境に Meiryo UI などは無い。書体の結果は「指定した書体が使われたか」だけを見た
- **Quarto 1.9 の book＋Typst は、内蔵の orange-book 形式が既定**になっている（Quarto の `share/extension-subtrees/orange-book/`）。
  テンプレートはそのうち `typst-show.typ` と `typst-template.typ` を差し替えていて、`page.typ`・`numbering.typ`・
  `orange-book.lua`（post-quarto のフィルタ）はそのまま効いている。上書きの多くはここから来る
  - `page.typ` は `papersize`・`margin`・`page-numbering`・`columns` から `#set page(…)` を出すが、
    `design-doc()` の中の `set page(paper: "a4", …)` が後から上書きする
  - Quarto の `typst-show.typ` は `mainfont`・`fontsize`・`lang`・`date`・`abstract`・`section-numbering`・`lof`・`lot`
    などを様式の関数に渡す。テンプレートの `typst-show.typ` が `design-doc()` に渡すのは、表題・著者・独自のキー・`toc` 系だけ
  - `orange-book.lua` は部に `#part[…]` を出すが、`lib.typ` に `part` が無い

## 4. 結果

### 4.1 壊れるもの（優先度 高）

| # | Quarto の書き方 | 起きること | 原因 |
|---|---|---|---|
| B-1 | `book: chapters:` の `- part: "第1部 概要"`（題名）か `- part: part.qmd`（ファイル） | PDF が `error: unknown variable: part` で止まる。HTML は通る | `orange-book.lua` が `#part[…]` を出すが、テンプレートの `typst-show.typ` は orange-book の `part` を取り込まず、`lib.typ` にも無い。`cli/tests/e2e.rs` の `part_pages_and_appendices_follow_quartos_order` は `ddq html` だけを通している |
| B-2 | `crossref: {fig-title: "Fig.", tbl-title: "Tab."}`、または `lang: en` | HTML: キャプションが Quarto の「Fig. 1.1:」のまま振り直されない。通常の表（`: … {#tbl-x}`）への `@tbl-x` が **「?」**（リンク切れ）になる。PDF は「Fig. 1.1-1」に振り直されるので食い違う。`.tbl` は PDF・HTML とも「表」のままで、同じ文書に「Tab. 1.1-1」と「表 1.1-2」が並ぶ | `postprocess-html.js` の正規表現が見出し語を `図\|表\|コード` に決め打ちしている。`.tbl` と IPO 図の「表」も、`lib.typ`（`_xref`・`_tbl-auto`・`ipo()`）と `postprocess-html.js` に決め打ち |
| B-3 | `highlight-style: zenburn` など | PDF のコードブロックの背景が `#303030` になり、`lib.typ` が「背景が `#f1f3f5` のブロック」を目印にかけている補正（文字の大きさ・両端揃えと字下げの解除）が当たらなくなる。背景色の変化は生成された `.typ` で確認した。紙面への影響は `lib.typ` の記述からの帰結で、紙面は見ていない | `lib.typ` が `show block.where(fill: CODE-BG)` で構文強調のブロックを特定している |

### 4.2 黙って無視されるもの（PDF）

| Quarto の設定 | 実測 | 上書きしている所 |
|---|---|---|
| `papersize`・`margin` | `us-letter`・`1in` を指定しても A4・様式の余白 | `lib.typ` の `design-doc()` の `set page(paper: "a4", margin: …)` |
| `page-numbering` | `"i"` を指定しても「1/3」 | 同じ `set page`（`footer: none`）と背景の様式 |
| `columns` | `2` を指定しても 1 段 | 同上 |
| `mainfont`・`fontsize`・`linestretch`・`codefont`・`mathfont` | `mainfont: "DejaVu Serif"`（この環境に入っている書体）を指定しても埋め込まれない | `typst-show.typ` が渡さない。`lib.typ` が `JP-SANS`・`BODY-SIZE`・`BODY-LEADING`・`MONO` で固定 |
| `lang`・`region` | `lang: en` でも本文の言語は ja。`.tbl` は「表」、節の参照は「1.1 節」のまま | `lib.typ` の `set text(lang: "ja")` と `show ref` |
| `number-sections`・`section-numbering`・`number-depth` | `number-sections: false`・`number-depth: 1` でも「1.」「1.1」と採番する（5 段まで） | `lib.typ` の `set heading(numbering: …)` |
| `crossref:` の `fig-prefix`・`tbl-prefix`・`sec-prefix`・`title-delim`・`labels` | 参照は「Fig. 1.1-1」（`fig-title`＋番号）、節は「1.1 節」、区切りは全角空白 | `lib.typ` の `show ref`・`set figure.caption(separator: [　])`・`set figure(numbering: …)` |
| `date`・`abstract`・`keywords`・`thanks` | 出ない | `typst-show.typ` が渡さない |
| `lof`・`lot` | 図目次・表目次が出ない | 同上（orange-book の `list-of-figure-title` などに替わる仕組みが無い） |
| `linkcolor`・`citecolor`・`filecolor` | 未実測 | `typst-show.typ` が渡さない |
| `brand`（`_brand.yml`）・`logo` | 未実測 | 書体は `lib.typ` が固定。ロゴは orange-book の表紙が出すが、その表紙を使っていない |

### 4.3 黙って無視されるもの・食い違うもの（HTML）

| Quarto の設定 | 実測 | 上書きしている所 |
|---|---|---|
| `mainfont`・`monofont` | 指定しても、本文は Meiryo UI 系、コードは BIZ UDGothic 系 | `design-doc.css` の `--bs-body-font-family`・`body`・`code, kbd, samp, pre` |
| `linestretch` | `2.0` を指定しても行の高さは 1.45 倍 | `design-doc.css` の `body { line-height: 1.45 }` |
| `fontsize` | 効く（20px） | — |
| `number-depth` | 効く。ただし PDF では効かないので、`number-depth: 1` で HTML は「図 1-1」、PDF は「図 1.1-1」になる | PDF 側（§4.2） |
| `crossref:` の `sec-prefix`・`ch-prefix` | `lang: ja` では「1.1節」に置き換わる。`lang: en` では「Section 1.1」のまま（PDF は「1.1 節」） | `postprocess-html.js` の節の参照の置き換え |
| `format: html: mermaid: theme:` | ```` ```mermaid ```` には効かない（`mermaid-config.json` で設定し直す）。```` ```{mermaid} ```` だけの文書では効いた | `design-doc.lua` の `inject_mermaid_runtime` |
| 画像の `width` | 900px を超える指定は 900px で止まる（CSS の記述から。未実測） | `design-doc.css` の `#quarto-document-content img { max-width: min(100%, 900px) }` |
| `theme:`（Bootstrap のテーマ） | 表の見出しの背景と罫線の色は固定（CSS の記述から。未実測） | `design-doc.css` の `th`・`td` |
| `css:`・`filters:`・`project: post-render:` を足す | `_quarto.yml` の同じキーにテンプレートの指定が書いてあるので、Quarto のマニュアルどおりに `css: my.css` と書き換えると、テンプレートの CSS が外れる | 置き場の問題（§6） |

### 4.4 標準の記法があるのに、独自の記法を案内しているもの

| 機能 | テンプレートの案内 | Quarto 標準 | 実測 |
|---|---|---|---|
| 列幅の指定 | `.tbl` の `widths="…"`、素の表は `::: {widths="…"}` で囲む（利用マニュアル 9 章） | 表の属性 `tbl-colwidths="[…]"`、文書全体の `tbl-colwidths:` | 素の表の `tbl-colwidths="[80,20]"` は効く（Quarto がフィルタの後で当てる）。文書全体の `tbl-colwidths: false` は、`.tbl` の `widths="10,90"` も消す |
| 列幅の既定 | 内容量に比例（すべての表。`design-doc.lua` の `Table()`） | 罫線の長さに比例（原稿の行が長いとき。Pandoc の仕様） | 罫線を 82:21 で引いた表が 9:91 になった。利用マニュアル 9 章の「グリッド表の罫線を何桁で描いても出力される列幅には影響しない」は、Pandoc のマニュアルと逆のことを言っている |
| mermaid の図 | ```` ```mermaid ```` を `::: {#fig-x}` で囲む（利用マニュアル 8 章） | ```` ```{mermaid} ```` に `%%| label: fig-x`・`%%| fig-cap:` | ```` ```{mermaid} ```` も HTML・PDF とも通る。ただし PDF では Quarto 自身がブラウザで **PNG** に描き（この環境には Chromium があった）、`mermaid-config.json`・図のキャッシュ・ddq の描画を通らない。HTML では番号も振り直される |
| 目次の位置 | `toc-position: top`（独自） | `toc-location`（HTML だけ） | 名前が似ていて、Quarto のキーと思われやすい |

### 4.5 標準どおりに効くもの・同じ意味で作り直してあるもの（変えなくてよい）

- `toc`・`toc-title`・`toc-depth`（PDF は目次を前付けの後ろに置く。位置のほかは Quarto と同じ意味）
- `fig-cap-location`・`tbl-cap-location`（通常の図表。`.tbl` は常に上）
- `tbl-colwidths`（表の属性に書いたもの）
- `.landscape`（Quarto にも同じクラスがあり、Typst では `#set page(flipped: true)` を出す。テンプレートはフィルタで先に受け、外枠つきの横向き様式にする）
- `{{< pagebreak >}}`・`{.unnumbered}`・`{.unlisted}`
- `book: appendices`（「付録A」に置き換えるが、Quarto が出す `appendices` の呼び出しをそのまま受ける）
- `book: output-file`
- callout（アイコンを差し替えるだけで、`icon=false` などは Quarto に任せている）
- HTML の `toc-location`・`grid`・`number-sections`・`fontsize`

### 4.6 独自の設定キー（Quarto に無いもの）

`doc-number`・`doc-revision`・`spec`・`company-ja`・`company-en`・`cover`・`page-start`・`pagebreak-level`・`toc-position`・
`plantuml-server`（`toc-placed` はフィルタが内部で立てる）。

様式に要るもので、独自で正しい。ただし最上位に Quarto のキーと並んでいて、利用マニュアル 16 章の早見表も `book: title:` と
`doc-number:` を区別せずに並べている。どれが Quarto のキーで、どれがテンプレートのキーかが読み手に分からない。
読んでいるのは `design-doc.lua`・`typst-show.typ` と、ddq の PlantUML（`cli/src/plantuml/mod.rs`・`cli/src/commands/plantuml.rs`）
だけで、VSCode 拡張は読まない。

## 5. 方針案（論点 2）

1. **Quarto のキーは、既定値は変えてよいが、書かれた値は尊重する。** 既定値は「書かれていなければ」入れる
   （Quarto の拡張なら、形式の既定値として持てる。§6.3）
2. **様式のために尊重できないキーは、黙って無視しない。** `design-doc.lua` が `doc.meta` を見て、たとえば
   `[design-doc] 警告: papersize は様式で A4 に固定しているため使いません` と出す（今の `.tbl` の警告と同じ出し方）。
   利用マニュアル 15 章（制限事項）に「効かない Quarto の設定」の表を載せる
3. **独自のキーは `ddq:` の下にまとめる。** Quarto のキーと見分けがつき、綴り違いを警告できる。将来 Quarto に同じ名前の
   キーが入ってもぶつからない
4. **Quarto に記法があるものは、標準の記法を正にする。** 独自の記法は、標準で書けない所（`.tbl` の div など）に限り、
   名前も標準にそろえる
5. **標準の記法をテンプレートが拾えないときは、検出して警告する**（```` ```{mermaid} ````）

キーごとの案:

| Quarto の設定 | 案 | 備考 |
|---|---|---|
| `book: chapters` の `part:` | **直す**（B-1） | `lib.typ` に `part` を用意する（部の扉の見出し。章番号を進めず、目次に載せる） |
| `crossref:` の `fig-title`・`tbl-title`・`lst-title` | **尊重する**（B-2） | Lua で `crossref` の値（無ければ言語の `crossref-*-title`）を読み、`lib.typ`（`.tbl`・IPO 図・参照）と HTML の後処理に渡す。後処理は見出し語ではなく `figcaption` の id（`fig-`・`tbl-`・`lst-`）で拾う |
| `highlight-style` | **尊重する**（B-3） | コードブロックの特定を背景色に頼らない |
| `number-depth` | 尊重する（PDF でも。§2 の 3） | `design-doc()` に渡し、見出しの採番と図表番号の接頭辞の深さに使う |
| `number-sections: false`・`section-numbering` | 固定して警告 | 採番の形は様式 |
| `crossref:` の `*-prefix`・`title-delim`・`labels`・`chapters` | 固定して警告 | 「図 3.2-1」「3.2節」は様式 |
| `lang`（ja 以外） | 固定して警告 | 日本語の様式。見出し語は上の `crossref` で変えられる |
| `papersize`・`margin`・`page-numbering`・`columns` | 固定して警告 | 外枠と資料番号欄が A4 を前提にしている |
| PDF の `mainfont`・`fontsize`・`linestretch`・`codefont`・`mathfont`、HTML の `mainfont`・`monofont`・`linestretch` | 固定して警告（§2 の 4） | HTML の `fontsize` は今どおり効かせてよい（画面の見やすさ） |
| `brand`・`logo` | 固定して警告 | |
| `keywords`・`date` | `keywords` は PDF の文書情報へ渡す。`date` は `cover: true` の表紙に出す | 手間が小さい |
| `abstract`・`thanks`・`lof`・`lot` | 警告する | 図目次・表目次は、要望があれば対応する |
| HTML の `mermaid: theme:` | 警告する | 図の設定は `mermaid-config.json` が正 |
| ```` ```{mermaid} ```` | 警告して ```` ```mermaid ```` へ誘導する（§2 の 7） | 見分け方は未確認（§8） |
| `tbl-colwidths` | 正の記法にする（§2 の 6） | 既定の列幅（内容量に比例）は残し、Pandoc と違う点をマニュアルに書く |
| HTML の画像の上限 900px | 幅を書いた画像には上限をかけない | |

## 6. 論点 1 との関係（機構ファイルの置き場）

### 6.1 今の置き方

執筆フォルダの直下に、原稿（`_quarto.yml`・`index.qmd`・`chapters/`・`diagrams/`・`revisions/`・`design-doc.pdf`）と、
機構ファイル 5 本・`.template-version` が並ぶ。PDF を作るときは、さらに `lib.typ`・`typst-template.typ`・`typst-show.typ`・
`_quarto-publish.yml` が置かれる。

さらに、発行者が書き換える `_quarto.yml` に機構の配線が入っている（`project: post-render: postprocess-html.js`・
`format: html: css: design-doc.css`・`filters: design-doc.lua`）。

### 6.2 案 B: `.ddq/` にまとめる

実測（Quarto 1.9.38）では HTML・PDF とも動いた。ただし、次の点が残る。

- `_quarto.yml` の配線は、パスが変わるだけで残る（`filters: .ddq/design-doc.lua` など）。§4.3 の最後の問題
  （`css:` を書き換えるとテンプレートの CSS が外れる）も残る
- `_quarto-publish.yml` は動かせない（Quarto のプロファイルは執筆フォルダの直下にしか置けない）
- 配布 HTML の CSS が `_book/.ddq/design-doc.css` に出る。ドットで始まるフォルダを配信しない Web サーバの設定がある
  （例: nginx でよく使う `location ~ /\.` の拒否）ので、配る先で確かめが要る
- `lib.typ` の import（`.ddq/lib.typ`）と、`design-doc.lua` が執筆フォルダの直下に探す `mermaid-config.json`・
  `plantuml-config.puml` のパスを直す

### 6.3 案 C: `_extensions/ddq/`（Quarto の拡張）にまとめる

Quarto の拡張機構を使う。プロジェクトの種類（`project: type: ddq`。中身は `type: book` と `post-render`）と出力形式
（`ddq-html`・`ddq-typst`。`filters`・`css`・`template-partials`）を、`_extensions/ddq/_extension.yml` に持たせる。

```yaml
# _extensions/ddq/_extension.yml（試験に使ったもの）
title: DDQ design document
version: 3.1.0
quarto-required: ">=1.9.0"
contributes:
  project:
    project:
      type: book
      post-render:
        - _extensions/ddq/postprocess-html.js
  formats:
    common:
      filters: [design-doc.lua]
    html:
      css: design-doc.css
      number-sections: true
    typst:
      template-partials: [typst-template.typ, typst-show.typ]
```

```yaml
# _quarto.yml（発行者が書くのはこれだけになる）
project:
  type: ddq
  output-dir: _book
book:
  title: ○○システム 基本設計書
  chapters: […]
lang: ja
format:
  ddq-html: default
```

実測（Quarto 1.9.38）:

- HTML（`--to ddq-html`）・PDF（`--to ddq-typst`）とも、今と同じ番号・様式で出た
- `ddq-typst` を `format:` に書かなくても、`--to ddq-typst` で PDF を出せた。**PDF のプロファイル（`_quarto-publish.yml`）が
  要らなくなる**。執筆者の `quarto render`（形式の指定なし）は、今と同じく HTML だけを出す
- 発行者が `format: ddq-html: css: my.css`・`filters: [mine.lua]`・`project: post-render: [mine.js]` を書くと、
  **テンプレートの指定に足された**（テンプレートの CSS・フィルタ・後処理が先、利用者のものが後）
- 配布 HTML の CSS は `_book/site_libs/quarto-contrib/quarto-project/ddq/design-doc.css` に出る（ドットのフォルダにならない）
- `lib.typ` の import は、プロジェクトの根からのパス（`/_extensions/ddq/lib.typ`）で通った
- テンプレートの版は `_extension.yml` の `version` で持てる（`.template-version` の代わりになる）
- `_extensions/ddq/` は**執筆フォルダの直下**（`_quarto.yml` と同じ階層）に置く。リポジトリの直下など執筆フォルダより上に
  置くと、Quarto が見つけられず `ERROR: Unsupported project type ddq` で止まった。執筆フォルダが複数あるリポジトリでは、
  今の機構ファイルと同じく執筆フォルダごとに 1 つずつ置く（`ddq update --all` がそれぞれを更新する）

### 6.4 比較

| 観点 | A 今 | B `.ddq/` | C `_extensions/ddq/` |
|---|---|---|---|
| 直下に並ぶ機構のファイル | 6〜10 本 | 1 フォルダ＋`_quarto-publish.yml` | 1 フォルダ |
| `_quarto.yml` の機構の配線 | ある | ある（パスが変わる） | 無い（`type: ddq` と `format: ddq-html` だけ） |
| Quarto のマニュアルどおりに `css:` などを足したとき | テンプレートの指定が消える | 消える | 足される |
| PDF のプロファイル | 要る | 要る | 要らない |
| Quarto の約束に乗っているか | — | 独自 | 乗っている（拡張は `_extensions/` に置き、バージョン管理に入れ、手で直さない） |
| 配布 HTML の CSS の置き場 | `_book/design-doc.css` | `_book/.ddq/…`（配る先での確認が要る） | `_book/site_libs/quarto-contrib/…` |
| 執筆者が直して壊したとき | 発行時に ddq が内容を照合して止める | 同じ | 同じ（照合の対象をフォルダごとにする） |

推奨は C。論点 1（変えうるファイルだけを直下に置く）と論点 2（Quarto のマニュアルどおりに書いて効く）を一緒に満たす。
なお、どの案でもファイルを書き換えられなくすることはできない。壊されたときに止めるのは、今と同じく発行時の照合
（`setup::ensure_current`）である。

### 6.5 1 と 2 がぶつかる所と、その解き方

- **名前**: 論点 1 の `.ddq` は、Quarto の約束（`_extensions/`）と違う。Quarto のマニュアルを読む人には、`_extensions/` のほうが
  「触らないもの」と伝わる。C を採るなら `.ddq` という名前はあきらめる
- **既定値の置き場**: C では `number-sections: true`・`toc-location`・`grid` などを拡張の既定値に移し、`_quarto.yml` から消せる。
  ただし既定値は利用者が上書きできる。**様式で固定するキー（§5 の「固定して警告」）は拡張の既定値に置かず、Lua・Typst の側で
  固定して警告する**。拡張の既定値に置くのは、変えてよいものだけにする
- **`_quarto.yml` に残る必須の行**: C でも `project: type: ddq` と `format: ddq-html` は `_quarto.yml` に残る。消すとテンプレートが
  外れるので、ddq の照合でこの 2 つも見る
- **`.gitignore`**: 雛形の `.gitignore` は `lib.typ`・`typst-template.typ`・`typst-show.typ` を名前で無視しているので、
  `_extensions/ddq/` の中の同じ名前のファイルも無視される。C では PDF 側のファイルもコミットする（無視をやめる）。
  無視していた理由（部品が無いと執筆者の `quarto render` が失敗する）は、プロファイルが要らなくなるので無くなる

## 7. 破壊的変更の範囲（C を採るとき）

- 既存の設計書リポジトリ: `ddq update` が移す（直下の機構ファイル・`.template-version`・`_quarto-publish.yml` を消し、
  `_extensions/ddq/` を置き、`_quarto.yml` の配線の行を `project: type: ddq`・`format: ddq-html` に替える）。
  `_quarto.yml` は発行者が書き換えるファイルなので、書き換えたら git の差分で確かめてもらう（今の update と同じ考え方）
- ddq: `assets.rs`（置き場）、`setup`（プロファイルが要らない）、`pdf`・`html`（`--to ddq-typst`・`--to ddq-html`）、
  `writing_folder::find_all`（執筆フォルダの見分け方）、`setup::ensure_current`、雛形の `_quarto.yml`・`.gitignore`
- VSCode 拡張: `folders.ts` の執筆フォルダの見分け方（`_quarto.yml` と `.template-version`）と、ddq を呼ぶ所
- 機構ファイル: `design-doc.lua`（設定ファイルを執筆フォルダの直下に探している）、`typst-template.typ`（import のパス）
- 文書: 利用マニュアル 4・7・8・9・11・12・15・16・17・18 章、`AGENT-GUIDE.md`、テンプレート設計書 2・4 章、`cli/DESIGN.md`
- §5 の警告と、§2 の 5（独自のキーを `ddq:` の下にまとめる）も同じ版で入れると、設計書リポジトリの移行が 1 回で済む

## 8. 残る確認

- VSCode の Quarto 拡張のプレビュー（`ddq-html` 形式）と、DDQ Template 拡張の各機能が、C の置き方で動くか（この環境では未確認）
- Windows（日本語のパス・CP932）で、拡張のフィルタのパスの解決と、`design-doc.lua` の ROOT の求め方が今と同じに働くか
- B を採るなら、配る先の Web サーバが `_book/.ddq/` を配信するか
- 実際の書体（Meiryo UI など）がある Windows での、B-3 の紙面への影響
- ```` ```{mermaid} ```` セルを Lua フィルタで見分ける方法
- Quarto の版の下限（拡張の `quarto-required`）。試験したのは 1.9.38 だけ

## 付録 A. 実測に使った設定と結果

基準の原稿は、章 1 つ・節 1 つに、画像の図（`#fig-img`）・通常の表（`#tbl-std`、`tbl-colwidths="[80,20]"`）・`.tbl` の表
（`label="tbl-cls"`）・コードブロックを置き、本文から 4 つを参照したもの。

| 名前 | 足した設定 | PDF | HTML |
|---|---|---|---|
| 基準 | なし | A4。「図 1.1-1」「表 1.1-1」「1.1 節」「表 1.1-2」 | 同じ番号（節は「1.1節」） |
| page | `papersize: us-letter`・`margin: {x: 1in, y: 1in}`・`page-numbering: "i"`・`columns: 2` | A4・1 段・「1/3」のまま | — |
| font | `mainfont: "DejaVu Serif"`・`fontsize: 16pt`・`linestretch: 2`・`codefont: "JetBrains Mono"` | 指定した書体が埋め込まれない | — |
| hfont | `mainfont: "DejaVu Serif"`・`monofont: "JetBrains Mono"`・`fontsize: 20px`・`linestretch: 2.0` | — | 本文は Meiryo UI 系、コードは BIZ UDGothic 系。20px、行の高さ 29px（1.45 倍） |
| secnum | `number-sections: false`・`section-numbering: "1.a"`・`number-depth: 1` | 「1.」「1.1」と採番、「図 1.1-1」 | 節に番号なし、「図 1-1」 |
| crossref | `crossref: {fig-title: "Fig.", tbl-title: "Tab.", fig-prefix: "Figure", tbl-prefix: "Table", sec-prefix: "Section", title-delim: ":"}` | 「Fig. 1.1-1」「Tab. 1.1-1」「1.1 節」「表 1.1-2」 | キャプションは「Fig. 1.1:」（振り直しなし）。参照は「Figure 1.1」「?」「1.1節」「表 1.1-1」 |
| lang | `lang: en` | 「Figure 1.1-1」「Table 1.1-1」「1.1 節」「表 1.1-2」 | キャプションは「Figure 1.1:」。参照は「Figure 1.1」「?」「Section 1.1」「表 1.1-1」 |
| meta | `date`・`abstract`・`lof: true`・`lot: true` | どれも出ない | — |
| caploc | `fig-cap-location: top`・`tbl-cap-location: bottom` | 通常の表のキャプションが下に出る（効く） | — |
| colw | `tbl-colwidths: false` | 通常の表の `[80,20]` は残り、`.tbl` の自動の幅が消える | — |
| colw2 | `tbl-colwidths: false` と `.tbl` の `widths="10,90"` | `widths` も消える | — |
| dash | 罫線を 80:4・82:21 で引いた表 | 列幅は 91:9・9:91（内容量で決まる） | — |
| hl | `highlight-style: zenburn` | コードブロックの背景が `#303030` | — |
| parts | `- part: "第1部 概要"` | `unknown variable: part` で失敗 | 通る |
| partsfile | `- part: chapters/part1.qmd` | 同じく失敗 | — |
| qmermaid | ```` ```{mermaid} ```` に `%%| label: fig-m`・`%%| fig-cap:`。`format: html: mermaid: theme: forest` | Quarto が PNG（`mermaid-figure-1.png`）に描く | 「図 1.1-1」に振り直され、テーマは forest |
| 案 B | 機構を `.ddq/` に移し、`_quarto.yml` と `_quarto-publish.yml` のパスを直す | 基準と同じ | 基準と同じ。CSS は `_book/.ddq/design-doc.css` |
| 案 C | 機構を `_extensions/ddq/` に移し、`project: type: ddq`・`format: ddq-html` にする | 基準と同じ（`--to ddq-typst`） | 基準と同じ。利用者の `css:`・`filters:`・`post-render:` は足される |
