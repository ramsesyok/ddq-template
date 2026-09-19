# 保守者ガイド（リポジトリの構成とリリース）

このファイルは**テンプレートを保守する人**向けです。リポジトリに何が入っているか、
版をどう上げるか、リリースをどう作って配るかをまとめています。

**執筆・発行の手順はここにはありません。** 行き先は次のとおりです。

| 知りたいこと | 見る場所 |
|---|---|
| 設計書リポジトリの作り方、PDF・HTML の出し方、mermaid の準備、環境変数 | 利用マニュアル（`manual/`）2・4・11・12章 |
| 記法（表・図・IPO図・横向き） | 利用マニュアル 6〜10章、早見表は15章 |
| うまくいかないとき | 利用マニュアル13章 |
| 様式（外枠・採番・IPO図）の調整、変換の内部 | [template/PIPELINE.md](template/PIPELINE.md) |
| `ddq`（CLI）の設計・コマンド・mermaid エンジン・ビルド | [cli/DESIGN.md](cli/DESIGN.md) |

---

## 1. リポジトリの構成

**様式のリポジトリ（このリポジトリ）**と、**設計書リポジトリ**は別物です。
`template/` は設計書リポジトリの中に置きません。

```
quarto-template/                 … このリポジトリ（保守者が持つ）
├── README.md                    … リポジトリの入口（何でできているか・役割・流れ）
├── ADVANCED.md                  … このファイル
├── .gitignore / .gitattributes / .vscode/settings.json
├── .github/workflows/ci.yml     … ddq の CI（fmt / clippy / test / build。windows-latest）
├── docs/                        … サンプル（受注管理システム。記法の実例＋様式の検証用）
├── manual/                      … 利用マニュアルの原稿（執筆フォルダの一つ）
├── cli/                         … ddq（Rust 製 CLI）。template/ を埋め込んだ単一 exe を作る
│   ├── DESIGN.md                … 設計書（決定事項・根拠・検証手順）
│   ├── src/                     … init / add / update / setup / html / pdf / diagrams / release / mermaid
│   ├── tests/                   … golden テスト（mermaid の幾何一致）・コマンドの配置テスト
│   └── tools/regress.py         … 移行前後で PDF / HTML が変わらないことの確認（開発用）
└── template/                    … 様式・変換の実体（ddq に埋め込まれる原本）
    ├── PIPELINE.md              … 出力経路と様式調整箇所の解説（最初に読む）
    ├── VERSION                  … テンプレートの版（= ddq の版。ddq update が .template-version に書く）
    ├── lib.typ                  … 様式の単一ソース（外枠・採番・IPO・横向き）
    ├── typst-template.typ       … Quarto 既定テンプレートの差し替え口（lib.typ を取り込む）
    ├── typst-show.typ           … フロントマター → design-doc() の引数
    ├── quarto-publish.yml       … PDF 用のプロファイル（→ <執筆>/_quarto-publish.yml）
    ├── design-doc.lua           … 執筆記法（mermaid / .landscape / .ipo / .tbl）の変換
    ├── design-doc.css           … HTML の見た目
    ├── postprocess-html.js      … HTML の図表番号を「章.節-連番」に振り直す後処理
    ├── mermaid-config.json      … mermaid 設定（htmlLabels:false 必須）
    ├── vendor/mermaid.min.js    … mermaid 本体（版固定。ddq がブラウザで動かす）
    ├── scaffold/                … 設計書リポジトリの雛形（ddq init / add が配る）
    └── release-guide.typ        … リリース直下に置く「はじめかた」スライドの原稿（ddq release が PDF にする）
```

旧来の `template/*.bat` `*.sh` は 2.0.0 で `ddq` のサブコマンドに置き換わりました。
`template/` を直したら `cli/` で `cargo build --release` して exe に埋め込み直します。

発行者の手元にはこのリポジトリではなく、**リリース ZIP を展開したフォルダ**が
置かれます（`quarto-template-<版>/`）。展開したまま使い、中身を取り出しません。
版がフォルダ名に入るので、新旧のリリースを並べて置けます。

### 何が設計書リポジトリへ配られるか

リリースを作るときに意識すべき区別です。**機構ファイル4本と `.template-version` は
設計書リポジトリにコミットされる**ため、これらを変更したら版を上げる必要があります。

| ファイル | 配布 | 置く人 |
|---|:---:|---|
| `design-doc.lua` / `design-doc.css` / `postprocess-html.js` / `mermaid-config.json` | ○ | `ddq init` / `ddq add` / `ddq update` |
| `.template-version`（`VERSION` の写し） | ○ | 同上 |
| `lib.typ` / `typst-template.typ` / `typst-show.typ` / `_quarto-publish.yml` | × | `setup`（PDF を出すときだけ。`.gitignore`） |

配置コマンド（`ddq init` / `ddq update` / `ddq setup`）の責務分担と、なぜ PDF 用の設定を
プロファイル（`_quarto-publish.yml`）に分けるのかは
[PIPELINE.md](template/PIPELINE.md) の「0. フォルダの分担」にあります。
HTML の図表番号を後処理で直している理由は利用マニュアル17章です。

---

## 2. 版を上げる

`template/VERSION` が版です。**配布される機構ファイル4本を変更したら上げてください。**
設計書リポジトリ側の `.template-version` と突き合わせて、発行者が更新要否を判断します。
`ddq --version` もこの値を表示します（`cli/build.rs` がビルド時に埋め込む）。

版を上げたら、**実行例に埋め込まれた版番号も揃えてください。** 展開フォルダ名に版が
入るため（`cd C:\tools\quarto-template-2.0.0`）、ずれていると発行者が手元のフォルダ名と
合わない手順を読むことになります。全体で30箇所ほどあります。

```bat
:: 旧版が残っていないか確認する（VERSION と一致すること）
findstr /S /C:"quarto-template-1." README.md ADVANCED.md template\PIPELINE.md manual\chapters\*.qmd
```

「新しい版を受け取ったとき」の説明で使う版番号（現行＋1）も併せてずらします。

---

## 3. リリースを作る

発行者へ配る一式は `ddq release` で作ります。マニュアルのビルドから zip 化までを
1コマンドで行うので、**ビルド順（PDF → HTML）の間違い**や入れ忘れが起きません。
`ddq` 自身がリリースに入るので、先に release ビルドしておきます。

```bat
:: このリポジトリのルートで実行する
cd cli
cargo build --release
cd ..
cli\target\release\ddq.exe release
```

ビルドには Rust（stable）と MSVC のリンカ（Visual Studio の「C++ によるデスクトップ開発」
ワークロード）が要ります。配布先には何も要りません（CRT は静的リンク。`cli/DESIGN.md` §7.1）。

| 引数・オプション | 効果 |
|---|---|
| 第1引数 | 出力先フォルダ（既定は `release/`） |
| `--with-sample` | サンプル文書（`docs/`）も同梱する（原稿と図だけ。生成物は除く）。発行者へ記法の実例を渡したいときに使う |
| `--no-build` | マニュアルを再ビルドせず、既にある成果物を使う |

出力は `release/quarto-template-<版>/`（展開済み）と同名の `.zip` です。

```
quarto-template-<版>/
├── ddq.exe             … 様式・変換・ビルドの実体（template/ 一式を内蔵）
├── はじめかた.pdf       … 発行者向けの最初の一歩（展開したらまずこれ。template/release-guide.typ から生成）
├── README.md           … テンプレートの概要と作業の流れ
└── manual/利用マニュアル.pdf, manual/html/   … 発行者が読む・執筆者へ配る
```

### 閉域（オフライン）環境向けに作る

特別な版は要りません。`ddq.exe` は単体で動き、mermaid の SVG 化には端末の Edge / Chrome
（無ければ内蔵レンダラ）を使うため、発行者は **Quarto だけ**で PDF・配布 HTML を作れます。

- typst の `@preview` パッケージ（`lib.typ` が callout アイコンに使う `fontawesome`）は
  **Quarto 本体が同梱**しており、render のたびに `<執筆フォルダ>/.quarto/typst/packages`
  へ展開して `--package-cache-path` で渡す。したがって閉域でも追加作業は要らない。
  ただし **Quarto が同梱していない `@preview` パッケージを文書側で import すると、
  閉域ではダウンロードに失敗して PDF が作れない**（同梱物は
  `<Quarto>/share/formats/typst/packages/preview/` で確認できる）。

**このファイル（`ADVANCED.md`）は同梱されません。** 保守者専用であり、発行者は
`はじめかた.pdf`・`README.md`・利用マニュアルで足ります。

### 作るときの注意

- zip は `ddq` が Rust の `zip` クレートで作り、**UTF-8 名フラグ（general purpose bit 11）
  を立てます**（`manual/利用マニュアル.pdf` が日本語名のため。旧 bat 時代に bsdtar で
  文字化けした教訓）。作成後に**ファイル数が展開フォルダと一致するか検査**し、
  欠けていればエラーにして zip を消します。
- `--no-build` を使うときは、`manual/design-doc.pdf` と `manual/_book/` の**両方**が
  最新であることを確かめてください（片方だけ古いまま同梱される事故を防ぐため）。
- `template/` を直したあと `cargo build` を忘れると、古い機構ファイルを内蔵した exe が
  リリースに入ります。`cargo build` は `template/` の変更を検知して再ビルドしますが、
  実行するのは保守者です。

---

## 4. リリースを配る

`main` にマージし、**マージ後の `main` からリリースを作り直してから**タグを打ちます
（タグの指すコミットと配布物を一致させるため）。

```bash
gh release create v2.0.0 "release/quarto-template-2.0.0.zip" \
  --target main \
  --title "v2.0.0 — （変更の要約）" \
  --notes-file <リリースノートのファイル>
```

- タグは `v<版>`（`template/VERSION` と揃える）。
- **ZIP を資産として添付します。** 発行者はこれをダウンロードして展開します。
- リリースノートには次を書きます。
  - 発行者のはじめかた（展開してそのまま使う・`cd` して絶対パスで `ddq init`）
  - 主な変更
  - 既存利用者向けの `ddq update` 手順
  - 互換性（配布される機構ファイル4本に変更があるか）

機構ファイル4本に変更が無い版では、既存の設計書リポジトリで `ddq update` を実行しても
実質 `.template-version` の更新だけになります。その旨をノートに書くと、発行者が
更新の要否を判断できます。

---

## 5. 変更したときの検証

様式・変換を直したら、次の順で確認します。手順の詳細は利用マニュアル17章にあります。

1. `cli/` で `cargo build --release` して exe に埋め込み直し、`ddq update` を再実行して
   執筆フォルダへ反映する（忘れると版・内容の不一致としてビルドが停止する）
2. サンプル文書（`docs/`）で PDF・HTML の両方を出し、体裁を確認する
3. 本書（`manual/`）でも出力を確認する（記法の網羅度が高く、退行を見つけやすい）
4. `postprocess-html.js` を直したときは、同じ `_book/` に2回続けて流しても結果が
   変わらないこと（冪等性）を必ず確認する
5. `cli/` を直したときは `cargo test`（golden テストが mermaid の幾何一致を見る）。
   出力の同一性を厳密に確かめたいときは `cli/tools/regress.py`（`cli/DESIGN.md` §12）

様式のどこを触れば何が変わるかは [PIPELINE.md](template/PIPELINE.md) の
「3. PDF 側のしくみ」に集約してあります。
