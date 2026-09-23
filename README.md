# 設計書テンプレート — Quarto で「資料番号付きの設計書様式」の PDF を組む

[Quarto](https://quarto.org/) を拡張して、**Markdown の原稿から「外枠・資料番号欄・
社名」付きの PDF 設計書**を組むテンプレートです。組版には Quarto 同梱の
[Typst](https://typst.app/) を使うため、**LaTeX のインストールは要りません**。

システム開発で使われる資料番号付きの設計書の様式 — 全ページの外枠と資料番号欄、
「図 3.2-1」形式の図表採番、IPO 図、横向きページ、大きな表のページ分割 — を
Typst テンプレートと Pandoc の Lua フィルタとして実装してあります。
執筆者が書くのは Markdown だけで、様式は一切書きません。

同じ原稿から、内容確認用の HTML（章ごとのページ・全文検索つき）も出せます。
全文検索は HTTP サーバから開いたときに使えます（`index.html` の直接表示では本文閲覧のみ）。

| 要素 | 使っているもの |
|---|---|
| 原稿 | Markdown（Quarto の `.qmd`） |
| PDF 組版 | Quarto → Typst（Quarto 同梱。LaTeX 不要） |
| 様式の実体 | `template/lib.typ`（Typst テンプレート） |
| 記法の拡張 | `template/design-doc.lua`（Pandoc Lua フィルタ） |
| 図 | mermaid（文字で書いた図をベクター SVG に）、PlantUML（LAN のサーバか、同梱 jar をローカルの Java で） |
| ビルド・配置の道具 | `ddq.exe`（Rust 製の単一実行ファイル。様式一式を内蔵）＋ `plantuml.jar`（PlantUML 用。MIT 版） |
| 表の編集 | VSCode 拡張 `ddq-table-editor`（`.tbl` をセル結合つきで視覚的に編集。リリースに VSIX 同梱） |
| 執筆者に要るもの | Quarto ＋ VSCode の Quarto 拡張だけ（表の編集拡張は任意）。PlantUML 図を使う文書で LAN にサーバが無いときだけ、リリース一式と Java も |

> **このリポジトリは「様式の実体（`template/` と、それを内蔵する `cli/`）と、VSCode 拡張（`extensions/`）、
> それにテンプレート自身の文書（`docs/`）とサンプル（`examples/`）」です。**
> 設計書そのものは、ここから作る**別のリポジトリ（設計書リポジトリ）**に置きます。
> 設計書を書く人がこのリポジトリを持つ必要はありません。

## 3つの役割

| 役割 | 持つもの | やること |
|---|---|---|
| **保守者** | このリポジトリ | 様式・変換の保守。`ddq.exe` と利用マニュアルをリリースとして配る |
| **発行者** | リリース展開フォルダ | 設計書リポジトリを作る・機構を更新する・**中間版／発行版 PDF と配布 HTML を作る** |
| **執筆者** | 設計書リポジトリだけ | 原稿を書く。**Quarto と VSCode 拡張だけ**で HTML を見ながら確認できる |

発行者と執筆者の違いは役割だけで、`ddq.exe` を持っていれば誰でも PDF を出せます。

**リリース一式は設計書リポジトリの中に置きません。** 発行者の手元に別途置き、
設計書リポジトリのパスを渡して使います（`git clean` で消える・誤ってコミットする
といった事故を防ぐため）。

```
C:\tools\
└── quarto-template-2.4.2/   ← リリース ZIP を展開したもの（git 管理外）
    ├── ddq.exe              ← 様式・変換・ビルドの実体（これを実行する。インストール不要）
    ├── plantuml.jar         ← PlantUML 図の描画（ローカルの Java で ddq が起動する）
    ├── はじめかた.pdf        ← 発行者向けの最初の一歩（8 枚のスライド）
    ├── README.md            ← このファイル
    ├── AGENT-GUIDE.md       ← AI エージェント向けの執筆ガイド（設計書リポジトリの AGENTS.md やスキルに取り込む）
    ├── LICENSE              ← このテンプレート（ddq 本体を含む）のライセンス（MIT）
    ├── THIRD-PARTY-NOTICES.md ← 同梱・内蔵している第三者のソフトウェアのライセンス表示
    ├── ddq-table-editor-2.4.2.vsix ← VSCode 拡張（表の視覚編集。執筆者へ配る）
    └── manual/              ← 利用マニュアル（手順の正。執筆者へも配る）

C:\work\
└── order-design/            ← 設計書リポジトリ（git 共有。執筆者はこれだけ clone する）
    ├── .gitignore, .gitattributes, README.md, .vscode/settings.json
    └── docs/                ← 執筆フォルダ（原稿＋機構ファイル）
```

**展開したフォルダはそのまま使います**（中身を取り出して並べ替える必要はありません）。
展開先に `cd` して、設計書リポジトリのパスを渡して `ddq` を実行します。

## できること

- **目次を自動生成** — 章番号・リーダー線・ページ番号つき。本文を直せば自動で追従します
- **章番号を自動採番** — 4階層（1.1.1.1）まで
- **図表番号を自動採番** — 「図 3.2-1」「表 3.1-1」の形式（連番は節ごとにリセット）
- **図表の相互参照** — 本文に `@fig-xxx` と書けば「図 3.2-1」に置き換わります。図を増減して
  番号がずれても、本文側は書き直し不要です
- **表のセル結合** — 大分類・中分類の縦結合を自動化（HTML タグを書く必要はありません）
- **表の視覚編集** — VSCode 拡張 `ddq-table-editor` で、Excel のようにセルを結合しながら表を組み、
  `.tbl` ブロックとして書き出せます（Excel からの貼り付け可。完全オフライン）
- **大きな表のページ分割** — PDF でページをまたぐと、同じ表番号で「（1／3）」と自動で続きます
- **横向きページ・IPO図** — 縦横の混在も定型ページも用意ずみ
- **フローチャート**（mermaid）— 文字で書いた図が、そのまま図版になります
- **UML 図**（PlantUML）— シーケンス・クラス・状態遷移・アクティビティなども文字で書けます。
  LAN の PlantUML サーバがあれば執筆者の環境に何も要りません
- **章ごとのファイル分割** — 何ファイルに分けても、番号は文書全体で通し番号になります
- **執筆者の HTML は発行版と同じ番号** — 章番号・図表番号・相互参照まで一致します
  （PDF 固有の改ページ・横向き・様式だけは中間版の PDF で確認します）

---

## 使い方の全体像

### 1. 保守者がリリースを配る

このリポジトリで `ddq release` を実行すると、`quarto-template-<版>.zip`
（`ddq.exe` ＋利用マニュアルの PDF / HTML ＋ VSCode 拡張の VSIX）ができます。
→ 利用マニュアル 18章（版の上げ方・リリースの作り方と配り方）

### 2. 発行者が設計書リポジトリを作る

ZIP を展開し、**展開したフォルダで**実行します。

```bat
cd C:\tools\quarto-template-2.4.2
.\ddq init C:\work\order-design
```

できた `docs\_quarto.yml` の表題・資料番号・会社名・章立てを整え、`git init` して
執筆者に共有します（利用マニュアルの PDF と `ddq-table-editor-<版>.vsix` も一緒に配ります）。
→ 利用マニュアル 2章・4章

### 3. 執筆者が原稿を書く

設計書リポジトリを clone し、**Quarto と VSCode の Quarto 拡張だけ**で書きます。
`Ctrl+Shift+K` のプレビューに、発行版と同じ章番号・図表番号・相互参照が出ます。
`ddq` も Node.js も要りません。表は同梱の VSCode 拡張（VSIX）を入れると、
セル結合つきで視覚的に編集できます。
PlantUML 図だけは描画にサーバが要ります。LAN にサーバがあれば `_quarto.yml` に URL を
書くだけ、無ければ Java を入れて `ddq plantuml serve` を起動しておきます。
→ 利用マニュアル 3章・5章、記法は 6〜10章

### 4. 発行者が PDF・配布 HTML を出す

```bat
cd C:\tools\quarto-template-2.4.2
.\ddq pdf  C:\work\order-design\docs
.\ddq html C:\work\order-design\docs
```

`pdf` / `html` は、設計書リポジトリの `.template-version` と機構ファイルが
`ddq.exe` と一致することを確認してから出力する。版・内容が違う場合は暗黙に更新せず
停止するため、発行者が `ddq update` を実行し、差分を確認してコミットする。

PDF は**中間版**としてコミットします。改ページ・横向きページ・紙の様式は HTML では
確認できないため、執筆者はこの PDF で紙面を見ます。
mermaid 図は Windows 標準の Edge（または Chrome）を headless で使ってベクター SVG に
焼き込みます。どちらも無い端末では `ddq` 内蔵のレンダラで描きます。
PlantUML 図は LAN のサーバか、無ければ同梱の `plantuml.jar` をローカルの Java で
`ddq` が起動して描きます（起動・停止は `ddq pdf` の内部で行います）。
Node.js も npm も要りません。
→ 利用マニュアル 11章・12章

### テンプレートを更新するとき

新しい版は別のフォルダに展開されるので、そちらから機構ファイルを入れ直します。

```bat
cd C:\tools\quarto-template-2.4.2
.\ddq update C:\work\order-design\docs
```

1 つのリポジトリに文書を増やすときは `ddq add <新しい執筆フォルダ>`、まとめて
更新するときは `ddq update --all <リポジトリ>` です。

差分は git に出るのでコミットします。執筆者は pull するだけです。
→ 利用マニュアル 11章

---

## 次に読むもの

うまくいかないときの対処は、**利用マニュアルの14章「困ったときは」**にまとまって
います（環境・執筆・出力の症状別に引けます）。

| ドキュメント | 内容 | 読む人 |
|---|---|---|
| [docs/manual/](docs/manual/)（リリース展開フォルダでは `manual/`） | **利用マニュアル**（役割別の環境構築・執筆・確認・出力・記法・制限事項・トラブル対処、保守者向けの版の上げ方とリリースの作り方）。本テンプレート自身で書かれており、`ddq pdf docs\manual` で PDF になります | 全員（執筆者にはこの PDF/HTML を配る） |
| [docs/design/](docs/design/) | **テンプレート設計書**（リポジトリの構成、出力経路、様式 `lib.typ` と変換 `design-doc.lua` の仕組み、HTML 採番の後処理、版の考え方、保守の観点）。**このリポジトリのみ**（リリースには同梱しません） | 保守者 |
| [cli/DESIGN.md](cli/DESIGN.md) | `ddq` の設計（コマンド・mermaid エンジン・PlantUML サーバ・ビルド・検証） | 保守者 |
| [extensions/ddq-table-editor/README.md](extensions/ddq-table-editor/README.md) | VSCode 拡張 `ddq-table-editor` の使い方・開発・デバッグ | 執筆者（使い方）・保守者 |
| [extensions/ddq-revision/README.md](extensions/ddq-revision/README.md) | VSCode 拡張 `ddq-revision`（ラベル付け・改訂履歴）の使い方・開発 | 執筆者（使い方）・保守者 |
| [AGENT-GUIDE.md](AGENT-GUIDE.md)（リリース直下にも同梱） | **AI エージェント向けの執筆ガイド**。利用マニュアル全体を読まずに、執筆フォルダの構成・`_quarto.yml`・記法（`.tbl`・IPO 図・相互参照）・コマンドを把握できる要約。設計書リポジトリの `AGENTS.md` / `CLAUDE.md` やスキルにそのまま取り込む | 発行者（AI に原稿を書かせるとき） |

`examples/docs/` はこのリポジトリ同梱の**サンプル**（受注管理システムの基本設計書）です。
記法の実例と、様式を変更したときの確認用に使います。サンプル文書を増やすときは
`examples/` に執筆フォルダを足します。`docs/` はテンプレート自身の文書（利用マニュアル・
設計書）を執筆フォルダとして並べた**設計リポジトリ**です。

## ライセンス

このテンプレート（ddq 本体・機構ファイル・雛形・VSCode 拡張）は [MIT ライセンス](LICENSE) である。
ddq.exe が内蔵する Rust のライブラリと mermaid、同梱の plantuml.jar、VSCode 拡張がバンドルする React は、
それぞれのライセンスに従う（[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md)）。
テンプレートで書いた設計書（原稿・図・PDF・HTML）は、書いた人のものであり、このライセンスの対象ではない。

