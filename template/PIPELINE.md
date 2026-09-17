# Quarto 版のしくみ（PDF / 静的HTML の出力と様式調整）

**1つの qmd 一式から PDF と静的 HTML の両方**を出す仕組みと、どこを触れば何が
変わるかをまとめる。テンプレートを改修するとき最初に読む。

- 対象読者は**テンプレートを保守する人**（このファイルは `template/` に置く）。
  執筆者向けの記法と発行者のビルド手順は利用マニュアル（`manual/`）、
  リポジトリの構成・版の上げ方・リリースの作り方は `ADVANCED.md` を参照。
- 用語: 「様式」= 会社の紙のフォーマット（外枠・資料番号欄・社名・ページ番号）。

## 0. フォルダの分担

**`template/` は設計書リポジトリの中に置かない。** 発行者の手元にだけ置き、執筆フォルダの
パスを渡して使う。設計書リポジトリには HTML に要る機構ファイルだけが**コミットされる**ので、
執筆者は Quarto と VSCode 拡張だけで発行版と同じ番号の HTML を出せる。

```
quarto-template-<版>/ ← リリース ZIP を展開したもの（発行者はここで作業する）
└── template/     ← 様式・変換・ビルドの実体（このフォルダ。git 共有しない）
    ├── lib.typ, typst-template.typ, typst-show.typ, quarto-publish.yml,
    ├── design-doc.lua, design-doc.css, postprocess-html.js, mermaid-config.json,
    ├── package.json, VERSION, scaffold/,
    ├── init-doc.*, update-doc.*, setup.*, build-qmd.*, build-html.*, render-diagrams.*,
    └── (setup 実行後: puppeteer.json / node_modules … いずれも .gitignore)

order-design/     ← 設計書リポジトリ（git 共有。執筆者はこれだけ clone する）
└── docs/         ← 執筆フォルダ（名前は自由）
    ├── _quarto.yml, index.qmd, chapters/, diagrams/     … 原稿
    ├── design-doc.lua, design-doc.css, postprocess-html.js,
    │   mermaid-config.json, .template-version           … 機構（コミットする）
    ├── design-doc.pdf                                   … 中間版（コミットする）
    └── (setup 後: lib.typ, typst-*.typ, _quarto-publish.yml, _book/ … .gitignore)
```

保守者のリポジトリでは `template/` がリポジトリ直下にあり、`docs/`（サンプル）と
`manual/`（本書を含むマニュアル原稿）が兄弟として並ぶ。**どちらの配置でも動く**
（下記の通りスクリプトは自分の位置を自力で求める）。

- ビルドは `./template/build-qmd.sh <執筆フォルダのパス>`（省略時 `docs`）。
  スクリプトは `TEMPLATE_ROOT` を渡すので、`template/` がどこにあってもよい。
- **すべてのスクリプトが自分の位置を自力で求める**（`.sh` は
  `$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)`、`.bat` は `%~dp0`）。したがって
  スクリプトを相対・絶対どちらで呼んでも `TEMPLATE_ROOT` は正しく決まり、CWD に
  依存しない。**CWD 基準で解決されるのは引数側だけ**なので、執筆フォルダ／
  設計書リポジトリのパスは絶対パスで渡すのが安全。
  `build-*` / `setup` / `update-doc` は `_quarto.yml` の有無を確認してから動くため
  パス誤りは即エラーになるが、`init-doc` は「無いものを作る」ので検出できない。
- `_quarto.yml` は filter を**同じフォルダ内の相対名**（`design-doc.lua`）で参照する。
  `design-doc.lua` は `quarto.project.directory` から執筆フォルダ（ROOT）を自力で特定し、
  `TEMPLATE_ROOT`（無ければ `ROOT/../template`）を TMPL とする。**TMPL は SVG 化にしか
  使わない**ので、執筆者の環境に `template/` が無くても HTML は出る。
- **3つの配置コマンド**（実体は1か所、責務だけ分けてある）:
  - `update-doc` … 機構ファイル4本＋`.template-version` を執筆フォルダへ置く。
    設計書リポジトリでは**コミット対象**。テンプレート更新の伝播はこれ。
  - `setup` … `update-doc` を呼んだうえで、PDF 用の部品（`lib.typ`／typst partials／
    `_quarto-publish.yml`）を置き、mermaid 用の Chrome/Edge を検出して
    `template/puppeteer.json` に記録する。PDF 用部品は `.gitignore`。
    ビルドスクリプトが先頭で呼ぶ。
  - `init-doc` … 設計書リポジトリの雛形（`scaffold/`）を展開し、`update-doc` を呼び、
    最後に HTML を1回ビルドして検証する。既存ファイルは上書きしない。
- なぜ `lib.typ` と typst partials を執筆フォルダへ置くのか: typst の import は
  「プロジェクト外を読めない」制約に掛かるため（PDF）。`design-doc.css` を置くのは
  Quarto がローカル css として `_book/` に取り込み、単体で配信・zip できるようにするため。
- なぜ typst の設定を `_quarto-publish.yml`（プロファイル）に分けるのか:
  `template-partials` は実在するファイルを要求するので、`_quarto.yml` に書くと
  執筆者の `quarto render`（フォーマット無指定＝全形式）が必ず失敗するため。
  発行側は `quarto render --to typst --profile publish` で読み込ませる。
- **パスは ASCII のみ。** Windows では、執筆フォルダのパスに日本語が含まれると
  Quarto から Lua フィルタへ渡る時点で U+FFFD に置換されて届き（`quarto.project.directory`・
  環境変数のいずれも同じ）、mermaid の SVG 化が成立しない。`init-doc` と
  `design-doc.lua` の両方で検出して止めている。

---

## 1. 全体像

```
                    docs/chapters/**.qmd         ← 執筆者が書くのはここだけ
                    docs/_quarto.yml             ← 章の並び・出力設定
                              │
                              │  quarto render
                              ▼
                    ┌──────────────────┐
                    │  design-doc.lua  │  Pandoc フィルタ（FORMAT で分岐）
                    └────────┬─────────┘
              typst 出力      │      html 出力
        ┌─────────────────────┴────────────────────┐
        ▼                                          ▼
  typst-template.typ                        Quarto の book HTML 生成
  typst-show.typ      ← template-partials          │
        │                                          ▼
        ▼                                   postprocess-html.js
     lib.typ          ← 様式の単一ソース      （図表番号を PDF に合わせる）
        │                                          │
        ▼                                          ▼
  _book/*.pdf → design-doc.pdf              _book/（章ごとの HTML）
     ＝ 発行版・中間版                          ＝ 執筆者の確認用・配布用
```

**同じ qmd から2系統が出る**のが要点。執筆者の記法は完全に共通で、
分岐は `design-doc.lua` の `FORMAT` 判定1か所に閉じている。

### ビルドコマンド

```
# 発行者が実行する（展開フォルダで cd してから。末尾は執筆フォルダのパス＝絶対推奨）
cd ~/tools/quarto-template-1.1.3
./template/init-doc.sh ~/work/order-design        # 設計書リポジトリを新規作成（最初の1回）
./template/update-doc.sh ~/work/order-design/docs # 機構ファイルを最新の template に更新
./template/build-qmd.sh ~/work/order-design/docs  # → <執筆フォルダ>/design-doc.pdf
./template/build-html.sh ~/work/order-design/docs # → <執筆フォルダ>/_book/index.html
```

`build-*` はどちらも執筆フォルダの `_book/` を出力先に使い、**後から走ったほうが前の
出力を消す**。そのため `build-qmd.sh` は PDF を `<執筆フォルダ>/design-doc.pdf` に
取り出す。両方を残したいときは PDF → HTML の順に実行する。

執筆者側は何も要らない。`quarto preview` / `quarto render`（フォーマット無指定）で
HTML が出て、図表番号の振り直しまで `post-render` が自動で行う。

---

## 2. ファイルの役割

場所欄: **doc** = 執筆フォルダ（設計書リポジトリ）/ **tpl** = `template/`。
「配布」欄は、設計書リポジトリにコミットされるか。

| ファイル | 場所 | 配布 | 役割 | 影響する出力 |
|---|---|---|---|---|
| `_quarto.yml` | doc | ○ | 章の並び、html 設定、画面幅（`grid:`）、資料番号、`post-render` | 両方 |
| `index.qmd` | doc | ○ | 前付け（採番なし）。HTML のトップページ | 両方 |
| `chapters/**.qmd` | doc | ○ | 本文 | 両方 |
| `diagrams/` | doc | ○ | qmd が直接貼る静的図 + mermaid 生成 SVG の置き場（`mmd-*` は除外） | 両方 |
| `design-doc.lua` | tpl→doc | ○ | mermaid / `.landscape` / `.ipo` / セル結合 / 表幅 | 両方 |
| `design-doc.css` | tpl→doc | ○ | HTML の見た目 | HTML |
| `postprocess-html.js` | tpl→doc | ○ | 図表番号を「章.節-連番」に振り直す（`post-render` で自動実行） | HTML |
| `mermaid-config.json` | tpl→doc | ○ | mermaid のテーマ・`htmlLabels: false`（SVG 化とプレビューの共通ソース） | 両方 |
| `.template-version` | tpl→doc | ○ | 置いた時点のテンプレート版（`template/VERSION` の写し） | — |
| `design-doc.pdf` | doc | ○ | 中間版（発行者が定期的に作る） | — |
| `typst-show.typ` | tpl→doc | × | フロントマター → `design-doc()` の引数 | PDF |
| `typst-template.typ` | tpl→doc | × | Quarto 既定テンプレートの差し替え口（`lib.typ` を import） | PDF |
| `lib.typ` | tpl→doc | × | **様式の単一ソース**（枠・採番・IPO・横向き） | PDF |
| `quarto-publish.yml` | tpl→doc | × | typst 用プロファイル（`_quarto-publish.yml` として置かれる） | PDF |
| `VERSION` | tpl | — | テンプレートの版。更新したら上げる | — |
| `scaffold/` | tpl | — | 設計書リポジトリの雛形（`init-doc` が展開） | — |
| `init-doc` / `update-doc` | tpl | — | 雛形の作成 / 機構ファイルの更新 | — |
| `setup.sh` / `setup.bat` | tpl | — | ビルド準備（update-doc + PDF 用部品 + Chrome/Edge 検出→`puppeteer.json`） | — |
| `puppeteer.json` | tpl | — | setup 生成の machine ローカル設定（ブラウザパス）。`.gitignore` 済み | mermaid |
| `build-qmd.sh` / `build-html.sh` | tpl | — | ビルド入口（引数 = 執筆フォルダのパス。先頭で setup を呼ぶ） | — |

---

## 3. PDF 側のしくみ

### 3.1 Quarto から lib.typ へ届くまで

Quarto の typst 出力は既定のテンプレートを持っているが、**`template-partials`
で2つの partial を差し替えて**自前の様式に載せ替えている（`_quarto-publish.yml`。
setup が `template/quarto-publish.yml` から置く）。

- `typst-template.typ` … Quarto 既定の本体を空にする役
- `typst-show.typ` … フロントマターの値を `design-doc()` の引数に渡す役

```
title / subtitle / author / doc-number / company / toc …
        ↓ typst-show.typ
#show: design-doc.with(title: …, doc-number: …, toc: true, …)
```

**注意**: Quarto 既定の目次生成は `typst-template.typ` 側にあり、差し替えた
結果として機能しない。目次は `lib.typ` の `#outline()` が出している。

### 3.2 lib.typ の構成

冒頭に **【1】調整パラメータ** の節があり、寸法・色・文字サイズはすべて
そこに名前付き定数として集約してある。様式の微調整は原則そこだけ触ればよい。

| 変えたいもの | 触る定数 |
|---|---|
| 外枠の位置・大きさ（縦） | `FRAME-P-POS` / `FRAME-P-SIZE` |
| 本文を書ける領域（縦） | `PAGE-P-MARGIN` |
| 外枠の線の太さ | `FRAME` |
| 資料番号の枠の大きさ | `DOCNUM-INSET` / `DOCNUM-SIZE` |
| 資料番号とページ番号の間隔 | `DOCNUM-PAGE-GAP` |
| 社名の位置（縦ページ） | `FOOTER-DESCENT` |
| 社名の字間 | `COMPANY-TRACKING` |
| 外枠の位置・大きさ（横） | `FRAME-L-POS` / `FRAME-L-SIZE` |
| 資料番号の高さ（横ページ） | `DOCNUM-L-Y` |
| 本文を書ける領域（横） | `PAGE-L-MARGIN` / `PAGE-IPO-MARGIN` |
| IPO の列比・欄の高さ | `IPO-COLS` / `IPO-BODY-ROW` ほか |
| 本文・見出しの文字サイズ | `BODY-SIZE` / `HEAD-SIZES` |

自動で追従する箇所（手で計算し直さなくてよい）:

- **資料番号の枠が外枠の上辺に接地する**位置は `measure()` で実測している。
  文字サイズや inset を変えても接地は保たれる。
- **横ページの資料番号の左右位置**は `FRAME-L-POS.x + FRAME-L-SIZE.width` で
  求めている。外枠を動かせば資料番号も付いてくる。

### 3.3 採番

- 章番号: `set heading(numbering: "1.1.1")` の1か所。付録を `A.1` にする等は
  ここを差し替える。
- 図表番号（**章.節-連番**、例 図 3.2-1）:
  - `set figure(numbering: …)` が `section-prefix(here())-n` を組む
  - h1/h2 の show ルールで `reset-floats()` を呼び、連番を節ごとに 0 に戻す
  - `show ref` が **対象図表の location** でカウンタを読み直す
    （参照した位置で読むと節番号がずれる）
  - リセット対象の kind は4つ: qmd の図表フローが出す `quarto-float-fig` /
    `quarto-float-tbl` と、生の Typst で `#figure`（`image`/`table`）を直接書いた場合の
    保険。qmd 経路（前者2つ）だけを直すと、typst 生ブロックに図表を置いたときに
    連番がずれるので、**4つとも 0 に戻す**。

### 3.4 変更したら

**必ず再ビルドして PDF を目視確認する。** ページ単位で見るなら（ルートから）:

```
./template/setup.sh <執筆フォルダのパス>   # lib.typ・partials を配置
cd <執筆フォルダ>
# _quarto-publish.yml の typst: に keep-typ: true を一時的に足してから
quarto render --to typst --profile publish
typst compile index.typ "chk-{n}.png" --format png --font-path "C:/Windows/Fonts" --ppi 70
```

見た目を変えないリファクタのときは、**変更前後の PNG を `cmp` で比較**すると
確実（本ファイル追加時の lib.typ 整理も全10ページ一致を確認している）。

---

## 4. HTML 側のしくみ

### 4.1 なぜ章ごとに分割するのか

設計書は 400〜1000ページ規模になる。単一 HTML（`embed-resources: true`）だと
図が base64 で内包され、**1000ページ＋図200枚で約27MB** になり閲覧も添付も
破綻する（実測: 本文のみ 288ページで 2.1MB、図1枚あたり約110KB）。

book にすると、閲覧者が1ページで読むのは **約25KB**、共通アセットは
`site_libs/` に1回だけ置かれ、全文検索（`search.json`）も自動で付く。

### 4.2 図表番号を後処理で直す理由

**Quarto の図表採番（crossref）は、すべての Lua フィルタより後段で走る。**
`at: pre-quarto` / `post-quarto` のどちらで見ても、その時点で表には id も
キャプションも付いておらず、参照は未解決の `Cite` のまま。したがって
フィルタでは上書きできず、**生成された HTML を直すのが唯一の方法**
（実測で確認済み）。

`postprocess-html.js` は2パス構成:

1. `_quarto.yml` の `chapters:` の順に全章を走査し、見出しの `data-number` から
   章・節を拾って連番を確定する（連番は文書全体で決まるため章をまたぐ）
2. 全ファイルのキャプションと `a.quarto-xref` を、確定した番号に差し替える

書き換えるのは `figcaption` の「図 1.1: 」と相互参照リンクの本文の**2か所だけ**。
章順は `_quarto.yml` から読むので二重管理にならない。

**落とし穴1**: 「図」と番号の間の NBSP は、book 出力では実体参照 `&nbsp;`、
単一 HTML 出力では生の U+00A0 と形が違う。正規表現は `(?:\s|&nbsp;)*` で
両対応にしてある（片方だけにすると 0 件マッチになる）。

**落とし穴2（冪等性）**: この後処理は `_quarto.yml` の `project.post-render` に
登録してあり、`quarto render` だけでなく **`quarto preview` が保存のたびに呼ぶ**。
そのとき再生成されるのは編集した章だけで、他章は前回処理済みの HTML が `_book/` に
残っている。つまり「処理済みの HTML をもう一度読んでも同じ番号を導出し、同じ結果を
書く」必要がある。そのため:

- キャプションの正規表現は未処理（`図 1.1: `）と処理済み（`図 3.2-1　`）の**両方**に一致する
- `.tbl` の分割キャプションは、前回前置したラベルを剥がしてから前置し直す

これを崩すと、プレビューのたびに「表 3-1　表 3-1　…」と番号が積み重なる（実際に
起きていた不具合）。`postprocess-html.js` を触ったら、**同じ `_book/` に2回流して
出力が変わらないこと**を必ず確かめる。

**typst 出力時**: post-render は PDF ビルドでも呼ばれるので、`QUARTO_PROJECT_OUTPUT_FILES`
に `.html` が1つも無ければ何もせず正常終了する。また、解決できない相互参照があっても
異常終了しない（post-render の失敗は `quarto preview` 全体の失敗になり、執筆が止まるため）。

**実行系**: `quarto run` / post-render は **Quarto 同梱の Deno** で走る。使うのは
`node:fs` / `node:path` だけなので、執筆者の環境に node は要らない。

### 4.3 見た目の調整箇所

| 変えたいもの | 触る場所 |
|---|---|
| 画面全体の幅・左ペインの幅 | `_quarto.yml` の `format.html.grid` |
| 章番号を出すか | `_quarto.yml` の `number-sections` |
| 表・IPO・図の見た目 | `design-doc.css` |
| 図表番号の書式 | `postprocess-html.js` |

現在は **1920px 基準**（左ペイン 300px + 本文 1380px + 右余白 240px）。
Quarto 既定は 250 + 800 + 250 ≒ 1280px。左ペインの 300px は
「章番号 + 見出し15文字」が折り返さずに収まる幅。

### 4.4 ブラウザで確認するとき

**`file://` では確認できない**（プレビューが最初に開いたページに固定され、
ナビゲートしても古い内容を返し続ける）。HTTP で配信すること:

```
cd docs/_book && python -m http.server 8899   # → http://localhost:8899/
```

- **再ビルドの前にサーバーを止める。** `_book/` を掴んだままだと
  `Device or resource busy` / `os error 32` でビルドが失敗する。
- CSS だけ直した場合、HTML に `?v=` を付けてもブラウザは
  `design-doc.css` をキャッシュする。`link` の href にクエリを足して読み直させる。

---

## 5. design-doc.lua（両方に効く変換）

執筆者の記法を様式機能に対応付ける Pandoc フィルタ。`FORMAT` で出力先を見て
分岐する。**typst の生ブロック（RawBlock）は HTML 出力時に捨てられる**ため、
HTML では div 構造として組み立て直している。

| 記法 | typst 出力 | html 出力 |
|---|---|---|
| ` ```mermaid ` | SVG 化して `image()` | **`MERMAID_SVG=1` 時**は同左（同じ SVG）／**既定**は `<pre class="mermaid mermaid-js">` を出しクライアント描画（§5.1） |
| `::: {.landscape}` | `#landscape[...]` | div のまま（CSS で横スクロール） |
| `::: {.ipo}` | `#ipo(...)` | `.ipo` div 構造 |
| `::: {.tbl}` | `#_tbl-auto[…]` で包んだ表（キャプションをヘッダ行として表に入れ、改ページで割れたら「（i／n）」を自動付与。手動分割は `#pagebreak`、`merge-cols=` で rowspan 結合） | `<div class="split-caption">` ＋表（採番は postprocess、結合は同左） |
| すべての表 | 列幅を本文幅いっぱいに正規化 | CSS の `width: 100%` |

### 5.1 mermaid の2モード（執筆者は node 不要）

執筆者は多数・PDF/配布 HTML を作るのは少数、という運用に合わせ、mermaid の変換先を
**出力とフラグで切り替える**。`WANT_SVG = (FORMAT == 'typst') or (MERMAID_SVG == '1')`。

| 実行者 | 経路 | mermaid の扱い | 追加で要るもの |
|---|---|---|---|
| 執筆者 | `quarto preview`（HTML, env なし） | `<pre class="mermaid mermaid-js">` を出し、Quarto 同梱ランタイムでブラウザ内描画 | **なし** |
| ビルド係 | `build-html.sh`（`MERMAID_SVG=1`） | mermaid-cli でベクター SVG 化して `image()` | `node_modules` ＋ Chrome/Edge |
| ビルド係 | `build-qmd.sh`（typst/PDF） | 同上 | 同左 |

- **mermaid-cli を動かすのは Quarto 同梱の Deno**（`quarto run <mermaid-cli>/src/cli.js`）。
  Node.js の導入は要らず、閉域環境へは `node_modules` を持ち込むだけで足りる
  （node 実行時とバイト単位で同じ SVG が出ることを実測で確認済み。差が出るのは
  classDiagram のように mermaid 自身が乱数描画する図で、これは node 同士でも一致しない）。
  `quarto` が PATH に無い環境のために、失敗したときだけ `node` でも試す
  （`design-doc.lua` の `render_mermaid`／`render-diagrams.*`）。

- **クライアント描画の仕組み**: Quarto native の ` ```{mermaid} ` は typst では
  フィルタが走る前に PNG へラスタライズされ、自前のベクター SVG に差し替えられない
  （実測）。そこで執筆者記法は ` ```mermaid `（プレーン）に統一し、HTML 既定では
  フィルタが `<pre class="mermaid mermaid-js">` を出す。描画に要る 3 ファイル
  （`mermaid.min.js` / `mermaid-init.js` / `mermaid.css`）は `QUARTO_SHARE_PATH`
  （Quarto が render 時にフィルタへ渡す）配下 `formats/html/mermaid/` から
  `quarto.doc.add_html_dependency` で一度だけ注入する。`mermaid-init.js` は
  `pre.mermaid-js` を拾って SVG 化するので、native ` ```{mermaid} ` と等価に描ける。
- **なぜフラグで分けるか**: 配布 HTML は PDF と図を一致させたい（`htmlLabels:false` の
  同一 SVG）ので `MERMAID_SVG=1` で従来の SVG 経路に載せ、`postprocess-html.js` の
  figure 採番にも乗せる。
- **設定の単一ソース**: `mermaid-config.json` は SVG 化（mmdc の `-c`）と
  クライアント描画の**両方**が読む。後者は `quarto.doc.include_text('after-body', …)` で
  `mermaid.initialize(...)` を流し込む（mermaid-init.js は読み込み時に一度
  `initialize()` を呼んで Quarto 既定テーマを当てるので、**その後ろで**上書きする。
  描画は window の load で走るため間に合う）。これで `theme` / `htmlLabels:false` /
  フォントが揃う。設定ファイルは執筆フォルダ直下のものを優先し、無ければ `template/` を見る。
- **残る差**: 描画エンジンの版が違う（プレビュー = Quarto 同梱 mermaid、発行版 =
  `package.json` の mermaid-cli）。まれに図の形が変わるので、中間版 PDF で確認する。
- **注意**: プレビューでは mermaid をベクター SVG 化しないため `diagrams/mmd-*.svg` は
  増えない（＝執筆者は Chrome/Edge も node_modules も要らない）。

### book 特有の注意（パスまわり）

- **実行時の CWD が「いま処理している章ファイルのディレクトリ」になる**
  （章の階層ごとに変わる）。そのため相対パスは使えず、執筆フォルダの絶対パスが要る。
  フィルタは**環境変数に依存せず**次の順で自己解決する（→ 拡張・素の `quarto render`
  でもそのまま動く）:
  - `ROOT`（執筆フォルダ）= `DOC_ROOT` env → `quarto.project.directory` →
    `QUARTO_PROJECT_DIR` env → `'.'`。図の出力先 `diagrams/` の親。
  - `TMPL`（`template/`）= `TEMPLATE_ROOT` env → `ROOT/../template`。
    mermaid-cli（`node_modules`）・`mermaid-config.json`・`puppeteer.json` の場所。
  env は「明示的に上書きしたいとき」だけ使う。通常は何も export しなくてよい。
- **mermaid のブラウザ**は env に頼らない。`EXECUTABLE_BROWSER` があればそれを、
  無ければ setup が書いた `TMPL/puppeteer.json`（Chrome/Edge のパス）を使う。
  どちらも無ければ `{}`（mmdc 同梱 Chromium を試す）。
- mermaid の SVG は `ROOT/diagrams` に内容ハッシュで置くが、AST に載せる
  パスは章ファイルからの相対でなければ Quarto が解決できない（`diag_rel()`）。
- **画像はプロジェクトルート基準の `/diagrams/...` で書く。** `../` を使うと
  Windows で `chapters\01-overview/../../` とバックスラッシュ結合され、
  typst 0.15 以降が拒否する（Quarto 同梱の 0.14.2 は通るので気付きにくい）。

---

## 6. 変更するときのチェックリスト

- [ ] PDF を再ビルドして目視確認したか（見た目を変えない変更なら PNG 比較）
- [ ] HTML も再ビルドしたか（PDF だけ直すと採番がずれることがある）
- [ ] IPO の列比を変えたなら、`lib.typ` の `IPO-COLS` と
      `design-doc.css` の `.ipo-frame` の**両方**を直したか
- [ ] 採番規則を変えたなら、`lib.typ` と `postprocess-html.js` の**両方**か
- [ ] 機構ファイルを増減したなら、それが `template/` にあるか。執筆フォルダ直下へ
      配置が要るなら、置き先に応じて次を更新したか:
      - 執筆者に要る（HTML）… `update-doc.sh`/`.bat` のコピー対象、`scaffold/repo/gitignore`
        （**無視しない**）、このリポジトリの `.gitignore`（無視する＋`!/template/...`）
      - 発行時だけ要る（PDF）… `setup.sh`/`.bat` のコピー対象、`scaffold/repo/gitignore`
        （無視する）、このリポジトリの `.gitignore`
- [ ] `template/VERSION` を上げたか（設計書リポジトリ側の `.template-version` に反映され、
      発行者が `update-doc` した差分として見える）
- [ ] `postprocess-html.js` を触ったなら、**同じ `_book/` に2回流しても結果が変わらない**
      ことを確かめたか（`quarto preview` は保存のたびに呼ぶ。§4.2）
- [ ] 執筆者に見える記法を増やしていないか（増やすなら利用マニュアル `manual/` も更新する）
