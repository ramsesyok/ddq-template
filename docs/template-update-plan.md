# ddq-template パブリックベータ修正計画

作成日 2026-11-24。変更前の比較基準はテンプレート 3.1.0、コミット `9beae4f`、Quarto 1.9.38。

執筆者と発行者が編集する設定を Quarto の標準に寄せ、テンプレートの機構を `_extensions/ddq/` に集約する。節単位の図表採番、寸法を厳密に合わせた発行 PDF、前付けの直後に置く目次は DDQ の要件として維持する。

## 1. 作業の始め方

1. 実装先の新しいブランチにこの計画書を含める。ブランチの基点、`git status --short`、`quarto --version` を記録する。基点が比較基準より進んでいる場合は、該当ソースと試験を照合して取り込み済みの変更を把握する。
2. [CLI 設計書](../cli/DESIGN.md)、[VS Code 拡張の作業指示](../extensions/ddq-template/AGENTS.md)、[表エディタの要件](../extensions/ddq-template/tableEditor-requirements.md)、[前回の品質レビュー](refactoring-review.md)の関連節を読む。今回確定した破壊的変更の範囲は本計画に従う。その他の公開コマンド、原稿・改訂データ、成果物、オフライン動作の契約は引き継ぐ。
3. テスト用の設計書は一時領域に作る。リポジトリ内のマニュアル・雛形・サンプルは切り替え段階で更新する。各段階で対象の実レンダーと既存試験を通し、公開候補は CLI・VSIX・テンプレートを同じ版で作る。

## 2. 確定した仕様

| 項目 | 新しい挙動 |
|---|---|
| 配置 | 執筆フォルダの `_extensions/ddq/` に Quarto の拡張記述と DDQ 管理のファイルを置く。直下の `_quarto.yml`、原稿、手置きの図版、改訂資料、レビュー用 PDF は利用者側のファイルとする |
| 設定 | DDQ 専用の利用者向けキーは `_quarto.yml` の `ddq:` 下に置く。章構成、言語、目次、書体、列幅、キャプション位置は Quarto の標準の場所に置く |
| 列幅 | `tbl-colwidths` に一本化する。通常表の Lua による幅計算を外す。`.tbl` のパート間で幅をそろえ、行分割、キャプション再掲、セル結合、ページ通番を維持する |
| HTML | 本文・コードの書体と行間、通常図の配置を標準設定に寄せる。利用者の CSS・フィルタ・後処理を併用できるようにする |
| PDF | A4、外枠、余白、資料番号欄、ページ番号の `n / N` を固定する。縦・横・IPO・スペック様式も維持する。本文書体・文字サイズ・行間は現行値を既定にして標準キーによる明示指定を反映する |
| 採番 | 番号付きの節では種類別の連番を振り直す。番号のない節では最も近い番号付きの親で連番を続ける。番号付きの親がない図表は `図 1`、`図 2` のように文書内で種類別の通し番号を振り、参照可能にする |
| 相互参照 | 公開前に HTML の参照切れと文言の上書きを修正する。明示指定された見出し語、参照ラベル、リンク設定を尊重する。通常図表・コード、`.tbl`、IPO 図の番号は媒体間で一致させる |
| 目次・改ページ | `前付け → 目次 → 本文` を自動で組み立てる。表紙を指定した場合は前付けより前に置く。標準の `toc`、`toc-title`、`toc-depth` と DDQ の既存の目次位置・改ページ指定を維持する |
| 部・付録 | `part:` の題名とファイルの両形式を PDF で扱う。部扉は章番号を消費しない。付録の英字採番には Quarto のブック処理を使い、「付録 A」の表示と DDQ の採番・参照に必要な調整を加える |
| 注意枠・Mermaid | 軽量な Font Awesome のアイコン差し替えを維持する。標準の `{mermaid}` は Quarto で描画し、PNG になることへの警告は追加しない。発行 PDF でベクター SVG が必要な場合は DDQ の `mermaid` を案内する |
| 既存文書 | 配線、専用キー、発行設定、列幅を発行者が手動で移す。`ddq update` は機構を配置・更新する。ベータ利用者への移行説明は個別に扱い、本番マニュアルには新しい使い方を載せる |

### 新規執筆フォルダ

```text
<執筆フォルダ>/
├── _quarto.yml
├── index.qmd
├── chapters/
├── diagrams/
├── revisions/
└── _extensions/
    └── ddq/
        ├── _extension.yml
        ├── design-doc.lua
        ├── design-doc.css
        ├── postprocess-html.js
        ├── mermaid-config.json
        ├── plantuml-config.puml
        ├── typst-template.typ
        ├── typst-show.typ
        └── lib.typ
```

機構を分割した場合のファイル名は実装で決める。クローン直後のプレビューと配布サンプルが動くよう、PDF 用の部品を含めて拡張一式をコミット対象にする。発行・レビューで共有する `design-doc.pdf` は引き続き執筆フォルダに出す。

```yaml
project:
  type: ddq
  output-dir: _book

book:
  title: ○○システム 基本設計書
  output-file: XX-2026-001_基本設計書
  chapters:
    - index.qmd
    - chapters/01/index.qmd

lang: ja
toc: true
toc-title: 目 次
toc-depth: 3

format:
  ddq-html: default

ddq:
  doc-number: XX-2026-001
  doc-revision: ""
  spec: false
  company-ja: ○○株式会社
  company-en: ○○ Co., Ltd.
  page-start: 1
  pagebreak-level: 1
  plantuml-server: ""
```

`cover`、`toc-position` も `ddq:` に移す。`toc-placed` は目次の重複防止に使う内部状態とする。CLI と Lua が読む `ddq.plantuml-server` の値と、既存の環境変数の優先順を揃える。

## 3. 実装順

### 3.1 Quarto 拡張の最小ブックで部と付録を通す

機構を一時コピーの `_extensions/ddq/` に配置し、次の三つを比較する。

| 経路 | 確認すること |
|---|---|
| 現行の `quarto render --to typst --profile publish` | 付録の英字と本文からの参照を採取する。`part:` を追加すると `unknown variable: part` になる |
| Quarto 標準の `book` と素の `typst` | 題名の部、ファイルの部、付録を生成する。`keep-typ: true` の中間ファイルと PDF を確認し、後続の章採番と付録の切り替えがどこで入るかを把握する |
| `project: type: ddq` と `quarto render --to ddq-typst` | 既知の試作では組版に成功しても付録が第2章になり、`図 A.1-1` が `図 2.1-1` になった。Quarto 1.9.38 が素の `typst` ブックに適用する内蔵 `orange-book` と、明示的なカスタム形式との差を調べる |

拡張側では Quarto のブック構造と部・付録の区切りを利用し、必要な接続だけを補う。部の見出し・原稿・アンカーを保持し、付録の英字とカウンタの切り替えを同じブック処理から受け取る。Quarto の内部表現に依存する場合は、依存箇所と最小再現を記録し、1.9.38 の試験で固定する。部扉が章番号を消費せず、付録の見出し、目次、図表番号、本文からの参照が期待どおりに揃ってから配置を切り替える。 [現行の Typst 設定](https://github.com/ramsesyok/ddq-template/blob/9beae4f/template/quarto-publish.yml) · [部・付録の既存試験](../cli/tests/e2e.rs#L500)

`_extension.yml` はブック派生のプロジェクト型と `ddq-html`・`ddq-typst` を定義する。CSS・フィルタ・パーシャルは拡張から供給し、利用者の CSS・フィルタ・プロジェクトの後処理との併用を確認する。プロジェクト共通で登録した後処理は素の `--to html` でも実行されることを確認済みなので、DDQ 形式で生成したページを識別して採番を適用する。

### 3.2 採番と相互参照を修正する

1. 章直下、番号付きの子節、番号のない子節、次の番号付きの節、前付け、付録を含む原稿で期待値を固定する。図・表・キャプション付きコードは種類ごとに採番する。`.tbl` と IPO 図は通常表と同じ連番に入り、一つの分割表は一番号とする。番号なしと明示した図表は連番を消費しない。
2. 表示する見出し番号と構造上の親から接頭辞を決める。`number-sections: false`、`number-depth`、`.unnumbered` を両形式で確認する。目次の収録を決める `toc-depth` と `.unlisted` は採番と独立に扱う。番号付きの親がない箇所は、文書内で同じ種類のカウンタを共有する。HTML のページが変わっても番号を重複させない。
3. 通常表の参照を先に `?` に置換する経路を解消し、Quarto が解決した通常図表参照を使う。DDQ の `.tbl`、IPO、`@num-` は独自の参照解決を維持する。キャプションの見出し語や区切り文字から図表を判定する処理を、ID と構造に基づく処理へ変更する。
4. HTML と PDF は同じ対象と番号を使い、キャプションと参照の数字に必要な箇所だけを更新する。`lang`、`fig-title`、`tbl-title`、`fig-prefix`、`title-delim`、原稿内の独自ラベル、番号だけの参照、リンクの有無を尊重する。`ref-hyperlink: false` では最終 HTML のリンクから ID を取得できない。必要なら Quarto のフィルタ段階で参照と ID の対応を保持し、表示文字から参照先を推測する処理を避ける。
5. Quarto が解決した章順・有効なプロファイル・出力先を使う。候補は `quarto inspect` のプロジェクト情報と、プロジェクトスクリプトに渡る出力情報。`QUARTO_PROJECT_OUTPUT_FILES` は今回生成したファイルの集合で、増分レンダーでは全ページを表さない。ブック全体の順序と今回の出力を区別し、既存ページの参照も更新対象に含める。古い出力、複数形式、同じ HTML に連続して後処理をかける場合を確認する。

HTML の章順を求めるための部分的な YAML 解釈は整理する。CLI の改訂・ラベル一覧は Git の過去版や未保存原稿を読むため、現行の `Source` ベースの走査はレンダー時の設定解決と区別する。Quarto を実行できる現行の発行経路で得た結果と、改訂・ラベルの章順が対応する範囲も既存の契約試験で照合する。過去版を読むために作業ツリーを書き換えない。 [HTML 後処理](../template/postprocess-html.js) · [Lua の参照処理](../template/design-doc.lua#L1366) · [Typst の採番](../template/lib.typ#L656) · [文書順の契約](../cli/src/doc/project.rs#L249)

#### 採番の期待値

第3章を例に、通常図表と DDQ 専用表を混ぜて確認する。節の深さを変える試験には同じ原稿を使う。

| 原稿を読む順序 | 節番号を表示する場合 | 節番号を表示しない場合 |
|---|---|---|
| 第3章の直下に置いた図 | 図 3-1 | 図 3-1 |
| 第3章の第1節に置いた図 | 図 3.1-1 | 図 3-2 |
| 第1節配下の番号なし見出しに置いた図 | 図 3.1-2 | 図 3-3 |
| 第3章の第2節に置いた図 | 図 3.2-1 | 図 3-4 |
| 第2節の通常表と、その後の二つに分割した `.tbl` | 表 3.2-1、表 3.2-2 | 表 3-1、表 3-2 |

別のケースとして、番号付きの親を持たない前付けの図は `図 1` から振り、同じ条件の次の図はページをまたいでも `図 2` とする。`number-sections: false` の場合も同じ規則を文書全体に適用する。前後の章と付録から参照し、各参照の番号が対象のキャプションと一致することを確かめる。

### 3.3 列幅・HTML の体裁・PDF の標準設定を整理する

- **列幅**。通常表は `tbl-colwidths` の既定値・プロジェクト設定・表属性を Quarto に任せる。現行の `Table()` 中の、セル先頭のリストを保持する Typst 補正は列幅と独立に残す。`.tbl` のブロック属性は `tbl-colwidths="[20,30,50]"` の形で読み書きし、指定があるときは全パートに適用する。文書全体の設定との優先順位、個別指定、無指定、列数の不一致を検証する。旧 `widths` ラッパと属性の対応は廃止する。表エディタでは、他の属性、外側の `.landscape`、編集中のパート以外の内容を保持する。 [Lua の表変換](../template/design-doc.lua#L777) · [表エディタの出力](../extensions/ddq-template/src/tableEditor/formats/tbl/serializeTblBlock.ts)
- **HTML**。標準の `mainfont`、`monofont`、`linestretch`、`fig-align`、`grid` を使う。全体に作用する `body`・`code` の上書き、画像の一律 900 px 上限、図の配置を横取りする指定を整理する。IPO、横幅の広い表、PlantUML の代替表示など DDQ 独自要素に必要なスタイルは対象を限定する。 [現行の HTML スタイル](../template/design-doc.css)
- **PDF の文字とキャプション**。`mainfont`、`fontsize`、`linestretch` などの本文設定をパーシャルから様式へ渡す。コード用の書体も標準の経路で扱い、省略時は現行の既定値を使う。構文強調の配色を変えるとコードの文字サイズが 9.5 pt から 10.5 pt に変わる現行の問題は、補正の対象を背景色に依存させずに解消する。通常図表は `fig-cap-location`・`tbl-cap-location` を使い、`.tbl` のヘッダ再掲と区別する。 [Typst パーシャル](../template/typst-show.typ) · [紙面とコードの書式](../template/lib.typ#L535)
- **PDF の固定条件**。`papersize`、`margin`、`page-numbering` など、固定した紙面と衝突する値を利用者が明示した場合は、キー名と固定する理由を警告する。Quarto から渡る既定値と明示指定を区別する。縦・横・IPO・スペックの寸法、外枠、番号欄は現行出力と比較する。 [固定紙面の設定](../template/lib.typ#L535)
- **目次・注意枠・図**。前付け直後の目次、重複防止、目次より前の見出し、現行の改ページ規則、`ddq.page-start` を維持する。Font Awesome の差し替えは既存の小さな処理を維持し、標準の `callout-icon: false` と共存させる。標準 Mermaid は Quarto に描画を任せる。DDQ Mermaid と PlantUML の SVG およびキャッシュは専用経路を維持する。 [目次の自動挿入](../template/design-doc.lua#L1417) · [注意枠のアイコン](../template/lib.typ#L1084) · [Mermaid の二つの経路](design/chapters/07-filter/02-mermaid.qmd)

### 3.4 CLI と VS Code 拡張を新配置に切り替える

| 入口 | 実装と確認箇所 |
|---|---|
| テンプレートの配置 | [assets.rs](../cli/src/assets.rs)、[add.rs](../cli/src/commands/add.rs)、[init.rs](../cli/src/commands/init.rs) を揃える。Quarto がない環境での `--no-render` と、存在する原稿・設定を保持する作成契約を維持する。HTML の疎通確認は `ddq-html` を指定する |
| 更新・版照合 | [update.rs](../cli/src/commands/update.rs)、[setup.rs](../cli/src/commands/setup.rs) を揃える。`template/VERSION` を原本とし、配布する `_extension.yml` の `version` を版の識別に使う。機構の内容も照合し、不一致時の終了コード 3 と明示的な更新導線を維持する。各ファイルの安全な置き換えを維持し、必要なキャッシュ処理を終えてから版を確定する。途中失敗からの再実行と同じ版の修復を確認する |
| 執筆フォルダの判定 | [writing_folder.rs](../cli/src/writing_folder.rs) と [folders.ts](../extensions/ddq-template/src/folders.ts) で設定ファイル、`project.type`、DDQ の拡張記述の扱いを揃える。`_quarto.yml` と `_quarto.yaml` に対応する。手動で配線を書き換えた後は `ddq update <フォルダ>` で初回配置できるようにする。`--all` は DDQ 文書を識別して更新する。設定名の検出は `add` の既存文書の保護にも使う |
| 発行 | [html.rs](../cli/src/commands/html.rs) は `--to ddq-html`、[pdf.rs](../cli/src/commands/pdf.rs) は `--to ddq-typst` を使う。`ddq pdf` の前処理を機構の照合にし、`_quarto-publish.yml` の生成・上書きを廃止する。公開コマンドの `ddq setup` は版・機構の確認として残す。利用者が管理するプロファイルは Quarto の標準の方法で有効化できるようにする |
| 成果物の取得 | [quarto.rs](../cli/src/quarto.rs) の行単位の出力先判定を見直す。Quarto が採用した `project.output-dir` と `book.output-file` を使い、今回の PDF を既存の `design-doc.pdf` として取り出す。出力先に残った別の PDF をコピーしない。HTML の完了メッセージと拡張が開くパスも実際の出力先に揃える |
| 図と設定 | [mermaid/prerender.rs](../cli/src/mermaid/prerender.rs)、[diagrams.rs](../cli/src/commands/diagrams.rs)、[PlantUML の設定](../cli/src/plantuml/mod.rs#L151) と Lua の参照先を変更する。拡張内の図設定と版を CLI と Lua で揃え、キャッシュのキーと再描画判定を照合する。既存の Java・jar・サーバ指定と環境変数の優先順を維持する |
| VS Code の発見・起動 | [folders.ts](../extensions/ddq-template/src/folders.ts)、[build.ts](../extensions/ddq-template/src/build.ts)、[init.ts](../extensions/ddq-template/src/init.ts)、[add.ts](../extensions/ddq-template/src/add.ts) の発見・監視・版照合と、テストホストの雛形を変更する。メニュー、進捗とキャンセル、発行・更新、ラベルと改訂、表エディタは既存の入口を通す |

新規雛形の `format` は `ddq-html` のみを宣言し、`quarto render` と `quarto preview` は執筆用の HTML を既定にする。発行用には `ddq pdf` が形式を明示する。PDF 専用の設定は発行者が所有するプロファイルで指定できるようにし、同じ有効プロファイルが CLI の設定解決、レンダー、成果物の取得に使われることを確認する。

`ddq update` が書き換える範囲は拡張内の DDQ 管理ファイルと既存の DDQ 図キャッシュ処理に限る。旧配線、旧直下の機構、旧発行プロファイル、旧専用キー、旧列幅を自動変換・削除する処理は作らない。原稿、利用者の YAML、静的図版、兄弟の拡張を保持する。初回配置と照合のエラーには、対象の執筆フォルダと対処を表示する。

### 3.5 雛形・サンプル・配布対象を更新する

1. `template/scaffold/content/` と `template/scaffold/repo/`、[利用マニュアル](manual/)、[テンプレート設計書](design/)、[サンプル](../examples/README.md) の現行手順と原稿を新仕様に更新する。対象は旧列幅、直下の専用キー、機構の参照、発行経路。リポジトリ内で今回のテンプレートを使ってビルドする実装仕様書なども、同じ変更点を点検する。過去時点のレビュー・調査記録は履歴として扱う。
2. 雛形の `.gitignore` と `.gitattributes` を点検する。現行のファイル名だけを使った除外ルールは、拡張内の `lib.typ` とパーシャルまで隠す。新規文書で拡張一式が Git に入り、Windows での改行差が内容照合を妨げないことを確認する。
3. [ddq-release のサンプル除外](../cli/src/bin/ddq-release/release.rs#L46) は、生成物や廃止した直下の部品に適用する。拡張フォルダに置いた同名の Typst ファイルは収録する。配布ガイド、マニュアル、VSIX、サンプル付き ZIP を生成し、展開先に新しい機構がすべて入っていることを検査する。`template/VERSION`、拡張 manifest、VSIX の package version、CLI の版を揃える。
4. マニュアルには標準の列幅、DDQ 専用キー、プレビューと発行、固定する PDF 設定、二つの Mermaid 記法を現行の仕様として載せる。既存のベータ利用者向けの切り替え説明は個別連絡に回す。

## 4. 検証項目と合格条件

既存の Rust 実レンダー試験、契約 fixture、VS Code 実ホスト試験を優先して拡張する。追加ケースには原稿、設定、期待する表示と参照先をそろえる。レンダーの終了コードに加え、HTML のリンク先と表示、PDF の文字・ページ構成、必要な中間ファイルを確認する。

| 項目 | 操作と期待結果 |
|---|---|
| 通常の採番 | 章直下、節、項、次の章に図・表・コードを置く。節または項の切り替えで連番を振り直し、HTML と PDF の採番が一致する |
| 採番の例外 | `number-depth`、`number-sections: false`、`.unnumbered`、番号なしの前付け、複数の番号なしページを含める。親の番号を継ぐか接頭辞なしの通し番号になり、`0-1` や重複が出ない |
| 相互参照 | `fig-title`、`tbl-title`、`fig-prefix`、`title-delim`、`lang: en`、`[custom @fig-x]`、番号だけの参照、`ref-hyperlink: false` を試す。別章の通常表・`.tbl`・IPO・`@num-` は実際の対象を示し、図表の番号はキャプションと一致する |
| 部・付録 | 題名の部とファイルの部を含めて PDF を生成する。部扉が章番号を消費せず、章順と目次が正しい。付録 A/B の見出しと `A.1-1`、`B.1-1` などの図表番号が本文側の参照と一致する |
| 目次・紙面 | 表紙あり・なし、前付けに改訂履歴がある場合、`toc: false`、`toc-depth`、`ddq.page-start`、標準の手動改ページ、縦・横・IPO・スペックを確認する。目次は指定順に一度だけ出る。PDF の寸法・外枠・資料番号欄・ページ番号を現行基準の出力と比較する |
| 設定の優先順位 | 本文書体・文字サイズ・行間と通常図表の上下位置の明示指定が反映される。固定紙面と衝突する指定は理由を警告して紙面を保つ。利用者の CSS・フィルタ・後処理も動く |
| 列幅と表編集 | 通常表の個別指定と文書の既定、`.tbl` の明示幅と分割、縦結合、無指定、列数の不一致を確認する。表エディタで開く、編集、保存、再読込、Undo ができ、新規出力は `tbl-colwidths` を使う |
| レンダー経路 | 新規文書で Quarto 標準のプレビュー、`ddq init`、`ddq add`、`ddq html`、`ddq pdf` を通す。全体レンダーと保存後の増分レンダーを比較する。素の `--to html` の出力では Quarto の採番が保たれる |
| 設定と成果物 | `_quarto.yaml`、フロー形式の `project`、発行者プロファイルの章追加と `output-dir`、ファイル名の空白・日本語、別の PDF が同じ出力先に残る場合を試す。Quarto が実際に生成した成果物を CLI と拡張が案内・取得する |
| 図とキャッシュ | 標準 `{mermaid}`、DDQ `mermaid`、PlantUML を同じ原稿に置く。標準の図が出力され、DDQ の SVG がプレビュー・発行で想定どおりになる。配置・版の変更後はキャッシュを適切に再生成し、手置きの図版を保持する |
| 機構配置 | 新規配置、同版再適用、版変更、途中失敗からの再実行、改変機構の検出、複数執筆フォルダを確認する。利用者の YAML、旧機構、兄弟の拡張、関係のない Quarto プロジェクトに差分が出ない |
| 配布候補 | Windows の日本語と空白を含むパスで、実 VS Code のプレビューと CLI の PDF・HTML を通す。既存 Windows CI の非 ASCII パス試験に加えて、日本語コードページの環境で確認結果を残す。ZIP のサンプルで Quarto が拡張を見つけ、VSIX と ZIP 内の CLI の版・ハッシュが一致する |

### 実行する検証

- `cli/` で `cargo fmt --check`、`cargo clippy --locked --all-targets -- -D warnings`、`cargo test --locked`、`cargo build --release --locked`。Quarto と既存の検証済み PlantUML jar を用意し、実レンダー試験は `DDQ_E2E=1` で実行する。Windows の準備手順は既存 CI に従う。 [CLI の CI](../.github/workflows/ci.yml)
- `extensions/ddq-template/` で `npm run verify`。実物の CLI を `DDQ_BIN` または既定の場所で渡し、`DDQ_HOST_REAL=1` で `npm run test:host` を実行する。Quarto のプレビューも実 VS Code で確認する。手元で実行できないものは、成功した代替試験と区別して未確認として記録する。 [拡張の試験手順](../extensions/ddq-template/DEVELOPMENT.md) · [拡張の CI](../.github/workflows/extension.yml)
- 既存の配布候補ワークフローで、同一コミットから ZIP と VSIX を生成する。サンプル付きの ZIP も展開し、配布ファイル一覧を照合する。展開したサンプルを CLI と Quarto で実際にレンダーする。 [配布候補のワークフロー](../.github/workflows/release-candidate.yml) · [前回の配布候補の検証](refactoring-review.md)

## 5. 実装後に残す情報

- 実装コミット、Quarto・Typst の版、OS、実 VS Code と日本語パスで確認した経路、実施した検証結果を残す。
- 部・付録、採番、プロファイル連携で Quarto の内部表現に依存する箇所が残った場合は、依存の理由と再現原稿を示す。対応する Quarto の版は検証結果に基づいて記載する。
- 配布物の新しい例に旧列幅記法や直下の旧 DDQ キーがないこと、旧発行プロファイルへの自動書き込みが廃止されていること、通常図表の参照先を表示文言から推測する処理が解消されていることを差分で確認する。

## 6. 調査の参照先

この計画は一問一答で確定した方針を反映している。旧列幅の互換維持や標準 Mermaid への警告は調査時に出た案であり、仕様は本計画の §2 に従う。 [Claude Code の調査記録](https://github.com/ramsesyok/ddq-template/blob/claude/dazzling-cori-i5lec4/docs/quarto-compat-study.md)

- [カスタムプロジェクト型](https://quarto.org/docs/extensions/project-types.html) · [カスタム形式](https://quarto.org/docs/extensions/formats.html)
- [ブックの部と付録](https://quarto.org/docs/books/book-structure.html#parts-appendices) · [ブックの相互参照](https://quarto.org/docs/books/book-crossrefs.html) · [相互参照の設定](https://quarto.org/docs/authoring/cross-reference-options.html)
- [プロファイル](https://quarto.org/docs/projects/profiles.html) · [Quarto Inspect](https://quarto.org/docs/advanced/inspect/index.html) · [プロジェクトスクリプト](https://quarto.org/docs/projects/scripts.html)
- [HTML オプション](https://quarto.org/docs/reference/formats/html.html) · [Typst オプション](https://quarto.org/docs/reference/formats/typst.html) · [表の列幅](https://quarto.org/docs/authoring/tables.html#column-widths)

## 7. 実装と検証の記録

作業ブランチは `fix/template-public-beta`、比較基点は `9beae4f`。破壊的変更を含むため、テンプレート、CLI の表示版、VS Code 拡張の版を `4.0.0` に揃えた。実レンダーは Linux、Quarto 1.9.38、Typst 0.14.2 で実施した。

- `_quarto.yaml` と発行者プロファイルで、空白および日本語を含む出力先・PDF 名を設定した。CLI が生成された実ファイルを `design-doc.pdf` に取り出すことと、利用者の CSS・Lua フィルタ・後処理が動くことを確認した。利用者の後処理には Quarto 標準の成果物一覧が届く。
- 英語の見出し語と参照ラベル、リンクなし、`number-depth: 1` を指定して、本文の図 `2-1` と付録の図 `A-1` のキャプションと参照が HTML・PDF で一致することを確認した。CLI の `quarto preview` で同じ本文の HTML を取得した。
- A5 や二段組の明示指定で警告が出て、発行 PDF が A4 のままになることを確認した。縦・横の比較用原稿では、資料番号と開始ページ番号を含む 2 ページを 72 dpi の画像にした結果が比較基点と一致した。
- HTML の章順は Quarto の `config.book.render` を使う。レンダーできる入れ子の `include` を 1.9.38 の `inspect` が解決できない場合には、レンダーの生成順と既存のプレビュー順を使う。Typst の部・付録は `quarto.utils.file_metadata_filter()` が渡す区切りと章番号で処理する。これらの内部形式は Quarto を上げる際に実レンダーで再確認する。
- 標準の `{mermaid}` は HTML で描画できた。ローカル Chromium を使う PDF では、Quarto 1.9.38 が SVG の取得に失敗した。素の `book` と `typst` でも同じエラーを再現した。
- タイトルだけの部・ファイルの部を同じ PDF に出し、部の後の章番号と「付録 A」を確認した。表紙、前付け、目次、本文の順に並び、目次が一度だけ出ることも確かめた。HTML の章単独レンダーでは、別章に残った参照が `2-1` から `2-2` に更新された。素の `--to html` は Quarto の採番で出力した。
- `ddq update --all` で、発行者の YAML、旧直下ファイル、関係のない Quarto プロジェクトがそのまま残ることを確認した。新しい執筆フォルダでは Typst 部品を含む拡張一式が Git の管理対象になる。
- 拡張の `npm run verify` は 531 件が通過。Rust の対象試験では、部・付録、PDF・HTML の出力比較、分割表のページ通番、サンプルの収録を確認した。`cargo fmt --check`、`cargo clippy --all-targets`、Linux の `cargo build --release` も通過した。
- サンプル付き ZIP の収録を Linux 上の検証用パッケージで確認した。クリーンなサンプルに拡張の 12 ファイルが配置され、ZIP 内の内容が原本と一致した。展開先で `ddq setup` と章の HTML レンダーを通した。サンプル全体の HTML では DDQ Mermaid 22 図と PlantUML 1 図の SVG を確認した。
- サンプル全体と利用マニュアルの HTML は生成できた。PDF は、ローカルで Typst の組版がサンプルで 7 分、利用マニュアルで 20 分を超えたため、検証を打ち切った。小規模な文書と分割表の実レンダーでは PDF を確認した。
- PR の CLI CI では、発行者プロファイルの PDF・HTML 出力先と、リンクなしの章間参照が章単独レンダーで更新されることを実レンダーで検証する。配布候補の CI が対象 PR で走る場合は、サンプル付き ZIP と VSIX を組み立てて照合する。
- Windows の日本語コードページでの発行と、実 VS Code でのプレビュー・拡張操作、Windows 用の配布候補一式の検証は公開前に残る。Linux の実拡張ホスト試験では、VS Code の取得先の名前解決で止まった。
