# ddq — テンプレート CLI の設計書

`template/*.bat` `*.sh`（7 種 × 2）を Rust 製のシングルバイナリ **`ddq`** に統合し、
mermaid → SVG 変換と PlantUML サーバの起動を内蔵する。保守者向けの最低限の設計書。
利用手順は利用マニュアル（`docs/manual/`）、様式・変換の内部はテンプレート設計書（`docs/design/`）。

- 状態: **実装済み・移行検証済み**（2026-09-19。§12.5 に結果）
- 対象版: テンプレート 2.4.0（PlantUML 対応。§13）

---

## 1. 目的

| 狙い | 内容 |
|---|---|
| 配布物の簡素化 | release フォルダを `ddq.exe` + `manual/` + README だけにする。`node_modules`（閉域向け同梱）・`puppeteer.json`・bat/sh を廃止 |
| 執筆者と発行者のロール統合 | `ddq` があれば誰でも `ddq pdf` / `ddq html` で発行できる。Chrome/npm の準備が要らない |
| bat 特有の制約の解消 | ASCII 限定コメント、`copy` の 0x1A 打ち切り、xcopy の黙殺、.NET zip の MAX_PATH、cmd の括弧ブロック誤解釈（現行 bat のコメント参照） |

### 前提の整理（調査で判明したこと）

- 現行の SVG 化はすでに **Quarto 同梱の Deno**（`quarto run`）で mermaid-cli を動かしており、Node.js 依存は無い。
  残る依存は **`node_modules`（mermaid-cli + puppeteer）と Chrome/Edge** の 2 つ。
- mermaid.js は DOM の `getBBox()` で文字幅を測るため、ブラウザ無しで「本物の mermaid.js」を動かすには
  DOM シム + 文字幅計測の自前実装が要る（Python の mermaidx が実証済みだが R&D コスト大）。
- Windows 10/11 には Edge が標準搭載。**既存 Edge を headless で叩けば mermaid-cli と幾何が完全一致する SVG が得られる**（§9 実測）。

---

## 2. 決定事項

設計インタビュー（2026-09-19）で確定した項目。番号は議論順。

| # | 項目 | 決定 | 理由 |
|---|---|---|---|
| 1 | 配布形態 | exe 1 本に全コマンド統合。`.bat` `.sh` は release から**完全に削除** | ラッパーを残すと bat の制約とメンテ対象が残る |
| 2 | exe の置き場所 | release フォルダ直下。exe は `quarto render` 起動時に自分のパスを `DDQ_BIN` で渡す。ユーザ判断で PATH に置いてもよい | 発行者の運用（展開フォルダをそのまま使う）を変えない |
| 3 | 機構ファイル | **全部 exe に埋め込む**（`include_dir!`）。exe 単体で全コマンドが動く | 「exe = テンプレートの版」になり、`.template-version` と一意に対応 |
| 4 | mermaid エンジン | 自動: **Edge → Chrome → merman**。`DDQ_MERMAID_ENGINE` / `EXECUTABLE_BROWSER` で上書き可 | ブラウザ経路は mermaid-cli と幾何一致。merman は保険 |
| 5 | `quarto preview` | **常にクライアント描画**（現状維持）。SVG を焼くのは `ddq html` / `ddq pdf` だけ | プレビューは Quarto だけで動く速いクイックレビュー。発行物と図が微妙に違い得ることはマニュアルに明記 |
| 6 | フィルタ⇔exe | フィルタが**図ごとに** `ddq mermaid` を呼ぶ（現行 `quarto run mmdc` の置換のみ）。CLI は複数入力対応にしておく | 正しさ優先。一括事前変換はフェンス抽出と hash の二重実装になるので保留 |
| 7 | mermaid.min.js | `template/vendor/mermaid.min.js`（11.16.0）をコミットして埋め込む。npm / node_modules 廃止 | 版の固定と再現性。Quarto 同梱 11.12 は見た目が違う（§9） |
| 8 | リポジトリ構成 | `template/` はソース置き場として残し、`cli/` に Cargo プロジェクトを追加。`build.rs` で `../template` を埋め込む | Lua/CSS/typ の編集体験と利用マニュアルの導線を壊さない |
| 9 | 対象 OS | **Windows x64 のみ**（msvc, `+crt-static`）。CI も Windows 1 本 | まず簡易に。OS 依存部は `cfg(windows)` で分離し、後から他 OS を足せる形にする |
| 10 | 名前 | `ddq`（design-doc-quarto）。環境変数は `DDQ_*` | 短く衝突しにくい |
| 11 | merman の深さ | v1 は決定的計測（既定）+ resvg-safe 相当のみ。実フォント計測（TextMeasurer）は載せない | 例外経路のために本体より重い部品を抱えない |
| 12 | 設計書 | この `cli/DESIGN.md` | コードの隣に置く |
| 13 | 多文書リポジトリ | `ddq add`（執筆フォルダ追加）と `ddq update --all` を新設 | 「init で 1 つ目、add で 2 つ目以降、版上げは update --all」と説明できる |
| 14 | コーディング | **clap + derive** で引数定義。標準関数だけの手書きパースはしない。人間が読みやすいコードを優先 | 保守性 |
| 15 | PlantUML（2.2.0） | **HTTP の描画サーバに一本化**。LAN のサーバがあれば優先、無ければ ddq が jar 内蔵の PicoWeb をローカルに上げる。フィルタは `POST /render` だけ | ブラウザ内で動く実装が無い。サーバなら執筆者に Java も ddq も要らない（§13） |
| 16 | PlantUML の変換器の置き場 | フィルタが図ごとに ddq を呼ぶ（決定 6）のではなく、**ddq はサーバの起動・停止だけ**。変換ロジックは Lua（curl POST）と Rust（`ddq diagrams` 用）の 2 か所だが、どちらも「連結して POST」の数十行 | 執筆者に ddq が無くても LAN サーバで描ける形を優先 |
| 17 | PlantUML のレイアウト | `!pragma layout smetana` を共通設定で固定 | Windows 版 jar は dot.exe を内蔵するが Linux サーバには無い。再現性（同じ原稿→同じ図）を優先 |

---

## 3. 全体構成

### 3.1 リポジトリ

```
quarto-template/
├── cli/                         … 本 CLI（Cargo プロジェクト）
│   ├── DESIGN.md                … このファイル
│   ├── Cargo.toml / Cargo.lock
│   ├── build.rs                 … ../template を埋め込み、VERSION を env に出す
│   ├── src/…                    … §8 参照
│   ├── tests/                   … golden テスト（§10）
│   └── vendor/plantuml.jar      … リリースに同梱する PlantUML（MIT 版。git 管理外。vendor/README.md）
├── template/                    … 機構ファイルのソース（従来どおり編集する）
│   ├── VERSION
│   ├── design-doc.lua / design-doc.css / postprocess-html.js / mermaid-config.json / plantuml-config.puml
│   ├── lib.typ / typst-template.typ / typst-show.typ / quarto-publish.yml
│   ├── release-guide.typ        … 「はじめかた」スライド（Typst。ddq release が PDF にする）
│   ├── scaffold/{repo,content}/
│   └── vendor/mermaid.min.js    … 新規（11.16.0）
├── docs/                        … 設計リポジトリ（執筆フォルダ manual/ = 利用マニュアル、design/ = テンプレート設計書）
├── examples/                    … サンプルの設計書リポジトリ（docs/ = 受注管理システム基本設計書。--with-sample の同梱元）
├── extensions/                  … VSCode 拡張（1 フォルダ = 1 拡張。2.4.0 で extension/ から移した）
│   ├── ddq-revision/            … 見出し・表・図のラベル付け（ddq tag list/apply の画面。2.4.0）
│   └── ddq-table-editor/        … 表の視覚編集（2.1.0 で旧 quarto-table-support を統合。
│                                   ddq release が npm でパッケージして VSIX を同梱する）
├── .github/workflows/ci.yml     … Windows: fmt / clippy / test / build --release
└── .github/workflows/extension.yml … 拡張の CI（Ubuntu / Windows × Node 20 / 22）
```

削除: `template/*.bat` `template/*.sh` `template/package.json` `template/package-lock.json` `template/node_modules/` `template/puppeteer.json`。

### 3.2 release フォルダ

```
quarto-template-<版>/
├── ddq.exe
├── plantuml.jar         … PlantUML（MIT 版）。ddq が exe の隣から探す（2.2.0 から。§13）
├── README.md            … リポジトリの README
├── AGENT-GUIDE.md       … AI エージェント向けの執筆ガイド（リポジトリ直下の同名ファイル）
├── はじめかた.pdf        … 最初の一歩（template/release-guide.typ を Quarto 同梱の Typst で PDF に。
│                            Marp は npm 依存なので使わない）
├── ddq-table-editor-<版>.vsix … VSCode 拡張（2.1.0 から。extensions/ の各拡張を `npm run package` したもの）
└── manual/
    ├── 利用マニュアル.pdf
    └── html/index.html
```

`template/` は release に**含めない**（すべて exe に埋め込まれている）。

VSIX は `ddq release` が `extension/` で `npm ci`（`node_modules/` が無いときだけ）→
`npm run package` を実行して作る（`--no-build` なら既存の VSIX を使う）。
`extensions/<拡張>/package.json` の `version` が `template/VERSION` と違えば止める（拡張の版＝テンプレートの版）。
発行者・執筆者の端末に Node.js が要らない点は変わらない（要るのは保守者のリリース作成時だけ）。
Windows の `npm` は `npm.cmd` なので `cmd /C npm …` で起動する（`Command::new("npm")` は .cmd を解決しない）。

### 3.3 doc リポジトリへの配置（現状維持）

| コマンド | 書き出すもの | git |
|---|---|---|
| `init` / `add` | scaffold（無いものだけ）+ 機構ファイル 5 本 + `.template-version` | コミット |
| `update` | 機構ファイル 5 本 + `.template-version`（上書き） | コミット |
| `setup`（`pdf` が内部で呼ぶ） | 版・機構ファイルの一致を検査し、`lib.typ` `typst-template.typ` `typst-show.typ` `_quarto-publish.yml` を上書き | `.gitignore` 済み |

`init` 直後の執筆フォルダに `lib.typ` は無い。初めて `ddq pdf` を走らせたときに置かれる。

---

## 4. コマンド仕様

```
ddq init     <repo-path> [writing-folder-name] [--no-render]
ddq add      <writing-folder-path> [--no-render]
ddq update   <writing-folder> | --all <repo-path>
ddq setup    <writing-folder>
ddq html     <writing-folder>
ddq pdf      <writing-folder>
ddq diagrams <writing-folder>
ddq plantuml serve [--port 18080] [--bind 127.0.0.1]
ddq release  [out-dir] [--with-sample] [--no-build]
ddq mermaid  -i <in.mmd>… -o <out.svg>… -c <config.json> [-b <color>]   (hidden)
ddq --version
```

共通: `<writing-folder>` 省略時は `docs`。相対パスはカレント基準（現行 bat と同じ）。
執筆フォルダの妥当性は「`_quarto.yml` があること」で判定する。

### 呼び出し関係

```
init ──► add ──► update
html ──► 版・機構の一致検査 ──► plantuml::ensure ──► quarto render
pdf  ──► setup（版・機構の一致検査 + PDF 側配置）──► plantuml::ensure
          └─(quarto render)─► design-doc.lua ──► ddq mermaid
                                              └─► POST /render（PlantUML サーバ。§13）
diagrams ──────────────────────────────────────► mermaid と同じ変換器 / PlantUML サーバ
plantuml serve ─► ローカルの PicoWeb を上げたままにする（執筆者のプレビュー用）
release ──► update, pdf, html（manual に対して）
tag list ─┐
tag apply ┴► doc::（_quarto.yml → chapters → include 展開 → 見出し・表・図の抽出）
```

### 各コマンド

| コマンド | 処理 | 現行 |
|---|---|---|
| **init** | 1) リポジトリ直下に `.gitignore` `.gitattributes` `.vscode/settings.json` `README.md`（`{{CONTENT_DIR}}` 置換）を「無いものだけ」置く 2) `add <repo>/<name>` | init-doc |
| **add** | 前提: 上のフォルダのどこかにリポジトリの目印（`.gitignore` / `.git` / `.svn` / `.hg`）がある（無ければ「先に `ddq init`」と案内。執筆フォルダは入れ子でもよい）。拒否: 対象に `_quarto.yml` が既にある。処理: scaffold の content 一式を無いものだけ置く → `update` → `quarto render --to html` で疎通確認（`--no-render` で省略） | （新規） |
| **update** | 機構ファイル 5 本と `.template-version` を上書き。`--all <repo>` は配下の `_quarto.yml` を持つフォルダを列挙して全部に適用（`_book/` `.quarto/` `node_modules/` は探索しない） | update-doc |
| **setup** | `.template-version` と機構ファイル 5 本が現在の `ddq` と一致するか検査 → PDF 側4ファイルを上書き。不一致時は `update` せず停止。**ブラウザ検出と puppeteer.json 生成は廃止** | setup |
| **html** | 版・機構の一致検査 → PlantUML サーバの用意（§13.3）→ `quarto render --to html`（env: `MERMAID_SVG=1` `DDQ_BIN` `DDQ_PLANTUML_SERVER`）。出力 `_book/` | build-html |
| **pdf** | `setup` → PlantUML サーバの用意 → `quarto render --to typst --profile publish`（env: `DDQ_BIN` `DDQ_PLANTUML_SERVER`）→ `_book/*.pdf` を `design-doc.pdf` にバイナリコピー | build-qmd |
| **diagrams** | `diagrams/*.mmd *.puml` → 同名 `.svg`（キャッシュ `mmd-*` `puml-*` は対象外）。設定は執筆フォルダ直下の `mermaid-config.json` / `plantuml-config.puml`（無ければ埋め込み） | render-diagrams |
| **plantuml serve** | §13.4。Java と jar を探し、PicoWeb を既定ポートに上げて Ctrl-C まで待つ | （新規） |
| **release** | 1) `--no-build` でなければ `update` `pdf` `html` を `docs/manual/` に実行 2) `release/quarto-template-<版>/` を作り直し、`current_exe()` を `ddq.exe` としてコピー、`cli/vendor/plantuml.jar` を `plantuml.jar` として同梱（無ければ停止）、`README.md`、`AGENT-GUIDE.md`、埋め込みの `release-guide.typ` を `quarto typst compile --input version=<版>` で `はじめかた.pdf` に、`docs/manual/design-doc.pdf` → `manual/利用マニュアル.pdf`、`docs/manual/_book` → `manual/html` 3) `--with-sample` で `examples/docs/` を `docs/` として同梱（`_book` `.quarto` `design-doc.pdf` `lib.typ` 等を除外） 4) zip（§7.3） | make-release |
| **tag list** | 執筆フォルダの見出し・表・図と Quarto ラベルの有無を一覧する。ラベルが無いものには内容由来ハッシュの候補と**編集指示**（ファイル・行・行頭からの文字数・挿入する文字列）を付ける。`--json` は機械可読（VSCode 拡張・CI 向け）、`--unlabeled` はラベルの無いものだけ | （新規。§14） |
| **tag apply** | `--all` で候補をすべて書き戻す。`--from <FILE>` は `tag list --json` の出力（人が候補を直したもの）を読んで書き戻し、当てる前に文書側と突き合わせる。`--dry-run` で書かずに内容だけ出す | （新規。§14） |
| **mermaid** | §5。hidden（`--help` の一覧に出さない） | quarto run mmdc |

`TEMPLATE_ROOT`（旧フィルタが `mermaid-config.json` のフォールバック探索に使っていた）は廃止した。
フィルタは執筆フォルダ直下の `mermaid-config.json` があれば `-c` で渡し、無ければ渡さない
（ddq が埋め込みの同じ既定を使う）。§6 参照。

### 非 ASCII パス

Windows では Quarto → Lua フィルタへ渡るパスが ANSI コードページ（CP932）のバイト列になる。日本語はそこに入っているので
`design-doc.lua` が `pandoc.text.fromencoding` で UTF-8 に戻す（テンプレート設計書 7 章「パスの文字コード」）。
コードページに無い文字（絵文字・U+301C・é など）は復元できず Quarto 自身も扱えないため、`init` / `add` / `html` / `pdf` は
`quarto` を起動する前に `writing_folder::ensure_encodable`（`WideCharToMultiByte` + `WC_NO_BEST_FIT_CHARS`）で検査し、
該当文字を示して停止する。Windows 以外は検査しない。

### メッセージ

日本語 UTF-8。Rust の `std::io::stdout` はコンソール出力時に `WriteConsoleW` を使うので `chcp` は不要。
エラーは「何が」「どこで」「次に何をすべきか」を 1〜3 行で出す（現行 bat の英語メッセージを日本語に置き換える）。

---

## 5. mermaid 変換（`ddq mermaid`）

### 5.1 エンジン選択

```
DDQ_MERMAID_ENGINE = browser | merman   … 明示指定（省略時 auto）
auto:
  EXECUTABLE_BROWSER（env）
  → レジストリ HKLM/HKCU\...\App Paths\msedge.exe, chrome.exe
  → %ProgramFiles% / %ProgramFiles(x86)% 配下の既知パス（Edge, Chrome）
  → 見つかれば browser、無ければ merman
```

- 探索は変換のたびに行う（十分安価）。`setup` 時の記録（puppeteer.json）は廃止。
- `browser` を明示して見つからなければエラー。`auto` で merman に落ちたときは stderr に 1 行通知する。

### 5.2 browser 経路

結果は **DevTools プロトコル（CDP）** で取り出す。CDP のライブラリは使わず、WebSocket
（`tungstenite`）+ JSON だけで puppeteer と同じ手順を踏む。

> 当初は `--dump-dom` の stdout を読む設計だったが（§9 PoC）、**Edge が常駐している
> （スタートアップ ブースト等で `msedge.exe --win-session-start` がいる）と、起動した
> msedge.exe が即座に別プロセスへ処理を引き渡して終了し、stdout が届かない**（実測。
> `--user-data-dir` を分けても同じ）。実運用の Windows では Edge の常駐が普通なので、
> プロセスの系譜に依存しない CDP に切り替えた。

1. 一時フォルダに HTML を 1 枚書く:
   - `<script>` に埋め込み `mermaid.min.js`
   - `mermaid.initialize(Object.assign({startOnLoad:false}, <mermaid-config.json>))`
   - `window.__ddqRender(source)`: mermaid-cli（src/index.js）と同じ手順で 1 図を SVG 文字列にする
     Promise。**文書に付いたコンテナ**に描かせてから `XMLSerializer` で直列化する
     （切り離した div だと `xmlns:xlink` が付かず、mermaid-cli の出力と差が出る）
2. 起動:
   ```
   <browser> --headless=new --disable-gpu --no-sandbox --no-first-run --disable-extensions
             --user-data-dir=<一時フォルダ>/profile
             --remote-debugging-port=0 about:blank
   ```
   起動した子プロセスの終了は成否に使わない（Edge は即終了する）。
3. `<profile>/DevToolsActivePort`（ポートと `/devtools/browser/<id>`）が書かれるのを待ち（上限 60 秒）、
   WebSocket で接続する。
4. **図ごとに** `Target.createTarget` → `Target.attachToTarget(flatten)` → `Page.enable` →
   `Page.navigate` → `Page.loadEventFired` を待つ → `Runtime.evaluate("window.__ddqRender(<source>)",
   awaitPromise)` → `Target.closeTarget`。読み込み中に評価すると
   「Execution context was destroyed」になるので load を待つ。
   同じページで続けて描かないのは、mermaid が図をまたいで持つ連番（sequenceDiagram の
   actor id など）が前の図に依存し、mermaid-cli（1 図 1 ページ）と出力が変わるため。
5. `Browser.close` で閉じる（閉じないと headless プロセスが残る）。一時フォルダは削除する。

複数入力でも起動は 1 回（1 図 ≒ 1.5 秒、22 図 ≒ 3 秒）。

### 5.3 merman 経路

- `merman` クレート（版固定）の `Renderer::render(RenderRequest::svg(...))`。
- **HTML ラベル（`foreignObject`）を含まない SVG にする**（CLI の `--svg-pipeline resvg-safe` 相当）。
  parity 相当のままだと mindmap / ER / block-beta / requirement で Typst 上の文字が消える（§9）。
- 設定は `mermaid-config.json` をそのまま渡す（theme / themeVariables.fontFamily / htmlLabels）。
- 文字幅は merman 既定の決定的計測。

### 5.4 出力規約

- mermaid-cli と同じく `<svg style="… background-color: transparent;">` を付与（`-b` 既定 `transparent`）。
- 先頭に `<!-- ddq <版> engine=browser|merman mermaid=<mermaid版 or merman版> -->` を 1 行入れる
  （どのエンジンで焼いたかを後から判別するため。フィルタの `svg_size()` は `viewBox` を正規表現で読むので影響なし）。
- 失敗した入力は 0 バイトのファイルを**作らない**（フィルタは「ファイルが存在するか」で成否を見る）。

### 5.5 キャッシュ

現状維持。フィルタが `pandoc.utils.sha1(code):sub(1,8)` で `diagrams/mmd-<hash>.svg` を決め、あれば exe を呼ばない。
exe はハッシュ計算に関与しない。

---

## 6. フィルタ ⇔ exe の契約

`design-doc.lua` の変更は `render_mermaid()` の起動部分だけ。

| 項目 | 内容 |
|---|---|
| exe の発見 | `DDQ_BIN`（exe が `quarto render` 起動時にセット）→ PATH の `ddq` → 無ければエラー（メッセージ: 「`ddq pdf` / `ddq html` から実行するか、ddq を PATH に置いてください」） |
| 呼び出し | `"<ddq>" mermaid -i "<mmd>" -o "<svg>" -c "<config>" -b transparent` |
| 成否 | 出力 SVG の存在で判定（現状維持） |
| `WANT_SVG` | `FORMAT == 'typst' or MERMAID_SVG == '1'`（現状維持）。プレビューは exe を探さない |
| `mermaid-config.json` | 執筆フォルダ直下にあれば `-c` で渡す。無ければ `-c` を付けず、ddq が埋め込みの同じ既定を使う（`TMPL` / `TEMPLATE_ROOT` は廃止） |
| 呼び出し方 | `pandoc.pipe`（シェルを介さない）。`os.execute` だと cmd.exe の引用符解釈（先頭が `"` のコマンド行）に振り回されるため |
| 互換 env | `EXECUTABLE_BROWSER` `MERMAID_SVG` `DOC_ROOT` は読み続ける。`TEMPLATE_ROOT` は読まない |

PATH 上の古い `ddq` と release フォルダの新しい `ddq` が共存しても、`DDQ_BIN` が優先されるので `ddq pdf` の結果は起動した exe の版で決まる。

---

## 7. ビルド・依存・配布

### 7.1 ランタイム依存

配布先に追加インストールは**不要**（Quarto と、mermaid 用の Edge/Chrome、PlantUML をローカルで描くときの Java を除く）。

- `x86_64-pc-windows-msvc` + `-C target-feature=+crt-static`（`cli/.cargo/config.toml`）。
  既定の動的 CRT だと `VCRUNTIME140.dll`（VC++ 再頒布）が要るため必ず静的にする。
- `merman`（既定 feature `complete-svg`）は resvg / usvg / rustybuzz / ttf-parser / ICU4X まで純 Rust。C ライブラリの DLL 依存なし。
  ELK レイアウト（EPL-2.0）は feature を有効にしない。
- フォントファイルは同梱しない（merman の決定的計測はフォント不要。SVG 内の `font-family` は Typst/ブラウザが解決）。

### 7.2 依存クレート（想定）

| 用途 | クレート |
|---|---|
| 引数 | `clap`（derive） |
| 埋め込み | `include_dir`（`template/` 一式）, `include_str!`（mermaid.min.js） |
| エラー | `anyhow`（アプリ側）。ライブラリ的モジュールは `thiserror` |
| mermaid | `merman`（版固定）, `serde_json` |
| zip | `zip`（UTF-8 名フラグ付き） |
| Windows | `windows-sys`（コードページ検査・Job Object）、`winreg`（App Paths / JavaSoft 探索） |
| 一時領域 | `tempfile` |
| WebSocket（CDP） | `tungstenite`（`handshake` のみ。TLS なし） |

### 7.3 zip

現行 bat の教訓をそのまま要件にする。

- UTF-8 名フラグ（general purpose bit 11）を立てる（`manual/利用マニュアル.pdf` が化けない）
- 書き終えたら読み直し、エントリの名前とバイト数を staging と照合し、不一致なら zip を削除してエラー。
  書いている途中で失敗したときも書きかけの zip を削除する
- 長いパスは `\\?\` プレフィックスで扱う（.NET の MAX_PATH 問題を持ち込まない）

### 7.4 版

- `ddq --version` = `template/VERSION`（`build.rs` が `cargo:rustc-env` で渡す）。
- `update` は `.template-version` に同じ値を書く。
- 版上げ手順: `template/VERSION` を上げる → `cargo build --release` → `ddq release`。

### 7.5 リリース手順

```
cd cli
cargo build --release
target\release\ddq.exe release            # 既定で ..\release\ に出力
```

保守者の端末には Rust に加えて Node.js（20 以上）と npm が要る（VSIX のパッケージ。§3.2）。
手順の正は利用マニュアル 18 章（`docs/manual/`）。

### 7.6 CI（GitHub Actions, windows-latest）

`cargo fmt --check` → `cargo clippy -- -D warnings` → `cargo test` → `cargo build --release`。
ブラウザ系テストは runner に Edge があるので動く（無い環境では skip）。

`tests/e2e.rs` は runner に Quarto（`quarto-dev/quarto-actions/setup`、版は手元の検証環境に固定）を
入れ、examples/docs を ASCII のパスと非 ASCII のパスの 2 つで `ddq update` → `ddq pdf` → `ddq html`
まで通し、PDF・mermaid の SVG（22 図）・配布 HTML を検査する。`DDQ_E2E=1` で quarto 不在を skip
ではなく失敗にする。非 ASCII のフォルダ名は実行環境の ANSI コードページで表せるものを選ぶ
（CP932 / UTF-8 なら `受注管理/設計書/執筆`、en-US の runner は CP1252 なので `Übung café/docs`）。
runner のロケールは変えられない（`Set-WinSystemLocale` は再起動が要る）ため、日本語パスそのものの
検査は CP932 の手元で `cargo test` を走らせて行う。

---

## 8. コード構成（案）

```
cli/src/
├── main.rs            … clap の Cli / Commands enum と dispatch だけ
├── commands/
│   ├── init.rs  add.rs  update.rs  setup.rs
│   ├── html.rs  pdf.rs  diagrams.rs  release.rs  tag.rs
│   └── mermaid.rs     … hidden サブコマンド（引数→ renderer 呼び出し）
├── assets.rs          … include_dir! の窓口（機構ファイル名の定数、書き出し関数）
├── writing_folder.rs  … 執筆フォルダの検証・列挙（_quarto.yml の有無、コードページで表せない文字の検査）
├── quarto.rs          … quarto の起動（env 付与、終了コード → anyhow::Error）
├── mermaid/
│   ├── mod.rs         … Engine 選択と共通インタフェース（Vec<(input, output)> → Result）
│   ├── browser.rs     … 探索・HTML 生成・起動・CDP（DevToolsActivePort → WebSocket）
│   └── merman.rs      … merman クレート呼び出しと foreignObject 抑止
├── doc/               … 執筆フォルダを「論理文書」として読む層（§14）
│   ├── mod.rs         … 走査の入口（Doc: 文書順のファイル列・見出し等の一覧・警告）
│   ├── project.rs     … _quarto.yml の chapters と {{< include >}} の再帰展開
│   ├── units.rs       … 行頭パターンによる見出し・.tbl・.ipo・#fig-・pipe 表キャプションの抽出
│   └── labels.rs      … 候補ラベル（sha1）と編集指示の生成
├── plantuml/
│   └── mod.rs         … Java / jar / サーバ URL の探索、PicoWeb の起動・停止（Job Object）、手書き HTTP、Session（§13）
└── zip.rs             … release 用
```

方針:

- `main.rs` は薄く。各サブコマンドは `pub fn run(args: XxxArgs) -> anyhow::Result<()>` の形に揃える。
- clap は derive のみ（`#[derive(Parser)]` / `#[derive(Subcommand)]` / `#[derive(Args)]`）。`hide = true` で `mermaid` を隠す。
- OS 依存（レジストリ・既知パス）は `browser.rs` に閉じ込め `#[cfg(windows)]` で囲む。
- 「なぜそうしているか」が非自明な箇所（CDP を使う理由、1 図 1 ページ、文書に付いたコンテナ、UTF-8 フラグ、foreignObject 抑止、`DDQ_BIN` 優先）はコード内コメントに §番号付きで理由を書く。現行 bat / Lua の実測コメントの流儀を引き継ぐ。

---

## 9. 根拠となる実測（2026-09-18）

サンプル（現 `examples/docs/`）と利用マニュアルの `diagrams/*.mmd` 37 本（flowchart 27、stateDiagram-v2 4、sequence 2、mindmap、erDiagram、block-beta 3、xychart 2、requirement 2。すべて日本語ラベル、`htmlLabels:false`）を使用。
基準は現行 mermaid-cli 11.16.0 + Chrome の SVG。

| 経路 | 結果 |
|---|---|
| **Edge 153 headless `--dump-dom` + mermaid.min.js 11.16.0**（PoC。Edge 非常駐時） | 35/37 成功（失敗 2 本は mermaid-cli でも失敗する図）。37 図 1 起動 **1.8〜2.4 秒**、1 図 1.1 秒。基準 SVG と **viewBox・座標が全 35 図で一致**。差は `<rect/>` vs `<rect></rect>` の直列化と `-b transparent` の style 属性のみ |
| 同上、Edge 常駐時 | `--dump-dom` の stdout が空（起動プロセスが即終了し別プロセスに引き渡す）→ CDP 方式に変更（§5.2）。ddq の実装（CDP、1 図 1 ページ）で 22 図の幾何が基準と一致（golden テスト） |
| Edge + Quarto 同梱 mermaid 11.12 | ER のリレーション名が灰色矩形になる、mindmap ラベルに下線、subgraph 内 `direction LR` の解釈が 11.16 と逆 → **同梱版の流用は不可**（決定 7） |
| **merman 0.8.0-alpha.6 `render --svg-pipeline resvg-safe`** | 36/37 成功（xychart の未クォート `/` を parse error）。約 **50ms/図**。Typst 経由の PDF で基準とほぼ見分けがつかない |
| merman `mmdc` サブコマンド（parity） | mindmap / ER / block-beta / requirement が `foreignObject` を含み **Typst で文字が消える** → resvg-safe 必須（決定 11） |
| mmdr 0.3.1 | neutral テーマを無視、ラベル重なり多数 → 不採用 |

既知の差異（いずれも mermaid の版依存。merman は 11.17 準拠）:

- subgraph 内 `direction LR`: 11.12・merman は LR、11.16 は TD。**現行でも執筆者プレビュー（11.12）と PDF（11.16）は食い違っている**。
- merman の文字幅は推定なので、Chrome 表示で ER の `bigint` 末尾が切れる例があった（Typst では発生せず）。

---

## 10. テスト方針

- fixtures: サンプル文書（`examples/docs/`）の mermaid フェンスと、mermaid-cli で作った golden SVG（`cli/tests/golden/`）。
- 比較: id（`my-svg` / `svg-<name>`）を正規化したうえで、**数値列（幾何）の一致**を見る。直列化の差（自己閉じタグ）は無視。
- browser 系: Edge/Chrome が見つからなければ `skip`（`#[ignore]` ではなく実行時判定でメッセージを出して return）。
  sequenceDiagram は mermaid 既定の `"Open Sans", sans-serif` で文字幅を測るため、日本語の fallback フォントが
  OS ロケールで変わり、基準環境（ja-JP Windows）以外では配置がずれる（GitHub の en-US ランナーで実測）。
  CI は `DDQ_GOLDEN_LOOSE=1` を設定し、一致しない図は「数値の個数が同じ・viewBox が 25% 以内」の緩い比較にする
  （実測: 幅は同じで高さが約 17% 低い = fallback フォントの行高の差）。
  保守者の手元（ja-JP）では厳密一致のまま。
- merman 系: 常時実行。foreignObject を含まないことをアサート。
- コマンド系: 一時ディレクトリに `init` → `add` → `update --all` を流し、置かれるファイル一覧と `.template-version` を検証。`quarto` の起動はモック（`quarto.rs` を trait 化）か、PATH に `quarto` があるときだけ実施。
- 手動: `ddq release` 後に `docs/manual/` の PDF/HTML を目視。`examples/docs/` の全図を旧 PDF と並べて確認（§9 の比較ページ生成をスクリプト化しておく）。

---

## 11. 保留・将来

| 項目 | 状態 |
|---|---|
| 一括事前変換（`ddq pdf` が render 前に全フェンスを 1 起動で焼く） | 保留。`ddq mermaid` は複数入力対応なので、フェンス抽出と hash 再現の目処が付けば足せる |
| merman の実フォント計測（TextMeasurer + fontdb/rustybuzz） | 保留。フォールバック品質に不満が出たら |
| Linux / macOS バイナリ | 保留。OS 依存は `cfg(windows)` に閉じ込めてある |
| QuickJS + DOM シムで本物の mermaid.js をブラウザ無しで動かす | 見送り（R&D コスト大） |
| `@preview/merman`（Typst パッケージ内で直接描画） | 見送り。Typst 0.15 必須（Quarto 1.9 は 0.14.2）で、配布 HTML もカバーしない |
| 執筆者プレビューでの SVG 焼き込み | 見送り（決定 5）。プレビューは Quarto のみで動くことを優先（PlantUML はサーバ描画なので例外。§13） |
| `ddq preview`（serve を上げてから `quarto preview` を起動） | 保留。`ddq plantuml serve` で運用は成り立つ。VSCode 拡張の Preview ボタンからは通らない |
| 公式 plantuml-server（Docker）との疎通確認 | 保留（PoC-3）。`POST /render` は PicoWeb で確認済み。公式サーバは `/serverinfo` が無いかもしれないので、到達判定は「HTTP 応答があれば可」にしてある |
| PlantUML の一括変換・`ddq pdf` の JVM 常駐 | 不要になった。サーバ常駐で 1 図数十 ms |

---

## 12. 移行検証（テンプレート機能の回帰確認）

ddq 自体のテスト（§10）とは別に、**移行前後で作成物（PDF / 配布 HTML）が変わっていないこと**を確認するフェーズを置く。
対象は `docs`（サンプル。現 `examples/docs/`）と `manual`（利用マニュアル。現 `docs/manual/`）。この 2 つで本テンプレートの記法は網羅されている。

| 機能 | 実例のある文書 |
|---|---|
| 統一テーブル `.tbl`（採番・`merge-cols`・`widths`・`label=`・自動分割 i／n） | docs / manual |
| 相互参照 `@tbl-` `@fig-`（HTML 側は postprocess-html.js の振り直し） | docs / manual |
| IPO 図 `.ipo`（表番号参照・パート分割） | docs / manual |
| 横向きページ `.landscape` | docs / manual |
| mermaid フェンス・`{#fig-}` 付き図・静的図（`diagrams/*.svg`） | docs / manual |
| 見出しの自動改ページ `pagebreak-level` | docs / manual |
| callout | docs / manual |
| 表紙・目次・ヘッダ/フッタ・資料番号（lib.typ） | 両方 |

### 12.1 手順

```
[移行前・現行 1.3.0 の bat で]  基準を採取 → regress/baseline/
[移行後・ddq で]                候補を採取 → regress/candidate/
                                比較 → regress/report/
```

採取は文書ごとに次を保存する（`regress/` は `.gitignore`。基準は移行前に**必ず先に**採る）。

| 保存物 | 採り方 |
|---|---|
| `index.typ` | `_quarto-publish.yml` の `typst:` に `keep-typ: true` を一時的に足して PDF ビルド |
| `design-doc.pdf` | PDF ビルドの成果物 |
| `_book/`（配布 HTML） | HTML ビルドの成果物（`search.json` 含む） |
| `diagrams/mmd-*.svg` | ビルド前に**キャッシュを消してから**ビルドし、焼き直されたもの |
| `pages/NNN.png` | `index.typ` を `quarto typst compile --format png` で頁画像化（PDF と同じ入力・同じ typst なので同値） |
| 機構ファイル 4 本のハッシュ | `update` 後の執筆フォルダの `design-doc.lua` 等 |

### 12.2 比較の層（感度の高い順）

| 層 | 方法 | 期待 | 差が出たときの読み方 |
|---|---|---|---|
| 0. 機構ファイル | `template/` の原本と執筆フォルダの 4 本をバイト比較 | 一致 | 埋め込み・書き出しのバグ |
| 1. `.typ` | 正規化なしのテキスト diff | **差分なし** | Lua フィルタの挙動が変わった。行番号から記法を特定できる |
| 2. 頁画像 | 頁数一致 + 各頁を PIL で画素差分（差分画素数 > 閾値で NG） | 全頁 0 | 1 で差が無ければ SVG の描画差（フォント・線幅） |
| 3. PDF テキスト | pypdf で頁ごとに抽出して diff | 差分なし | 図表番号・目次・ヘッダの崩れ |
| 4. 配布 HTML | `_book/*.html` を正規化（SVG の自己閉じタグ、`<!-- ddq … -->`、生成時刻）して diff。`search.json` も | 差分なし | postprocess-html.js の採番、mermaid の埋め込み方 |
| 5. SVG | `mmd-*.svg` を id 正規化後に数値列（幾何）比較 | 一致 | mermaid の版・エンジン差（§9 の既知差異に該当するか確認） |

差分が出た頁・ファイルは、基準と候補を左右に並べた HTML（`regress/report/index.html`）に出し、目視で判断できるようにする。

### 12.3 ツール

- `cli/tools/regress.py`（開発用、配布しない）。サブコマンド `capture <baseline|candidate> <docs|manual>` と `compare`（`docs` = `examples/docs/`、`manual` = `docs/manual/`。対応表は `DOCS`）。
  `--builder bat --template <dir>` で旧 template のコピー（`git archive <移行前コミット> template`
  で取り出し、`node_modules` はジャンクション）を使えるので、**移行後でも同じ原稿から基準を採り直せる**。
  Python 3 + Pillow + pypdf（保守者の環境にある前提。CI では走らせない）。ddq 本体は Rust だが、画像・PDF 比較は Python のほうが手数が少ないため（2026-09-19 合意）。
- 採取は「キャッシュ削除 → keep-typ 付与 → ビルド → 保存 → keep-typ を戻す」を自動化し、手作業を挟まない。
- 想定される正当な差分（許容リスト）: SVG 先頭の `<!-- ddq … -->` コメント、SVG の直列化差、HTML 内の生成時刻、
  **roughjs の乱数線**（mermaid は ER / requirement / flowchart の stadium・cylinder 等を手描き風の乱数パスで描く。
  同じ mermaid-cli 同士でも一致しない）。SVG の幾何比較は `d` 属性を除いて行い、頁画像の差分は
  該当箇所を目視で確認する。これ以外は差分＝要調査。

### 12.4 合格条件

- 層 0・1・3 が全部一致。層 2 は画素差分 0（roughjs の乱数線による差だけは目視で同一と確認）。
- 層 4・5 は許容リスト以外の差分なし。
- 許容できない差分は、原因が ddq 側なら修正、mermaid の版差（§9 既知差異）なら利用マニュアルの「変更点」に記載して合意のうえで基準を更新する。

### 12.5 結果（2026-09-19、ddq 2.0.0 vs 旧 bat 1.3.0）

| 文書 | 結果 |
|---|---|
| `manual/`（105 頁・4 図。ddq 向けに書き換えた原稿を、旧 bat と ddq の両方で作って比較） | **157 / 157 一致**（画素差分 0） |
| `docs/`（49 頁・22 図） | 132 / 136。NG 4 件はすべて層 2 の頁画像で、ER 図の実体枠・flowchart の stadium/cylinder（roughjs の乱数線）の 14〜2630 px。目視で同一 |

層 1（`index.typ`）は両文書とも差分ゼロ = Lua フィルタの出力は変わっていない。

---

## 13. 移行時にやること（実装チェックリスト）

- [x] **移行前に** §12.1 の基準を `docs/` `manual/` について採取する（現行 bat、1.3.0）

- [x] `cli/` 作成（Cargo、`.cargo/config.toml` に `+crt-static`、`build.rs`）
- [x] `template/vendor/mermaid.min.js` を `node_modules/mermaid/dist/mermaid.min.js`（11.16.0）からコミット
- [x] `design-doc.lua` の `render_mermaid()` を `DDQ_BIN` / PATH 探索に差し替え、エラーメッセージを更新
- [x] `template/*.bat` `*.sh` `package.json` `package-lock.json` `node_modules` `puppeteer.json` を削除、`.gitignore` の `node_modules/` `puppeteer.json` を整理
- [x] `template/release-README.md`（→ 後に `release-guide.typ` のスライドに置換）`README.md` `ADVANCED.md` `template/PIPELINE.md` §1・§5.1 の手順を `ddq` に書き換え
- [x] 利用マニュアル（`manual/`）2・4・11・12・14 章を `ddq` に書き換え。「`quarto preview` の図は発行物と微妙に違い得る」を明記。多文書リポジトリの手順（`add` / `update --all`）を追加
- [x] scaffold の `.gitignore` から `node_modules/` `puppeteer.json` の項を外す（残しても害はない）
- [x] `.github/workflows/ci.yml` 追加
- [x] `cli/tools/regress.py` を作り、§12.2 の比較で合格条件を満たすことを確認（報告を `regress/report/` に残す）
- [x] `template/VERSION` → 2.0.0


---

## 13. PlantUML 対応（2.2.0）

検討の経緯・PoC の実測は `cli/plantuml-study.md`（検討メモ）にある。ここは設計の結論。

### 13.1 方針

1. **LAN 内に PlantUML サーバがあれば優先して使う。** ddq も Java も jar も無い執筆者が、サーバさえあれば図を見られる。
2. LAN サーバが無い執筆者は、リリース一式（ddq + jar）と Java を用意し、執筆中は `ddq plantuml serve` を起動しておく。
3. 発行者は `ddq pdf` / `ddq html` を打つだけ。サーバの起動・停止は ddq が内部で行う。

mermaid と違い PlantUML にはブラウザ内で動く実装が無い。「フィルタが図ごとに ddq を呼ぶ」（決定 6）を
PlantUML にも当てると執筆者に ddq が要る。**HTTP の描画サーバに一本化**すれば、サーバが LAN の常設でも
ローカルの PicoWeb（jar 内蔵）でもフィルタは同じで、執筆者の要件は「サーバに届くこと」だけになる。

### 13.2 構造

```
                 ┌ LAN の PlantUML サーバ（公式 plantuml-server / PicoWeb）
design-doc.lua ──┤                                        … POST /render（curl.exe、HTTP 1 本）
                 └ ローカルの PicoWeb（java -jar plantuml.jar -picoweb）
                        ├ 執筆者が手で起動: ddq plantuml serve（既定 http://127.0.0.1:18080）
                        └ ddq pdf / html / diagrams が内部で空きポートに起動し、終了時に kill
```

| 要素 | 内容 |
|---|---|
| 記法 | ```` ```plantuml ```` フェンス。`@startuml` / `@enduml` は省略可（`@start…` で始まらなければフィルタが補う）。採番・参照・大きさは mermaid と同じ（`::: {#fig-x}`、`fig_width`） |
| サーバの決め方（フィルタ・ddq 共通） | `DDQ_PLANTUML_SERVER`（ddq が内部起動したもの）→ `PLANTUML_SERVER`（端末）→ `_quarto.yml` の `plantuml-server:`（設計書リポジトリで共有）→ `http://127.0.0.1:18080`（`ddq plantuml serve` の既定）。render 中に 1 回だけ `/serverinfo` で到達を調べる |
| 届かないとき | プレビュー（HTML・`MERMAID_SVG` 無し）: ソースを枠付き（`.plantuml-fallback`）で表示して render を止めない。発行（typst / `MERMAID_SVG=1`）: エラー停止 |
| HTTP の手段（Lua） | Windows 同梱の `curl.exe` を `pandoc.pipe` で呼ぶ。`pandoc.mediabag.fetch` は GET のみ・タイムアウト不可（落ちたサーバに章ごとに 21 秒。実測）で不採用。`--connect-timeout 2`、`-H Expect:`（100-continue の 1 秒待ちを避ける）、`-D -`（ヘッダを読む） |
| 構文エラー | サーバは 200 で「エラー内容を描いた SVG」を返す。`X-PlantUML-Diagram-Error` / `-Line` ヘッダで判定し、その図は書かない。行番号は連結した設定の行数を差し引いて原稿の行に戻す |
| 共通設定 | `plantuml-config.puml`（機構ファイル 5 本目）。`-config` はサーバに渡せないので、中身を `@start…` の直後に**連結して送る**。連結後のソースをハッシュするので、設定を変えるとキャッシュが自動で無効になる（mermaid には無い利点）。既定: `!pragma layout smetana`、`defaultFontName "Yu Gothic"`、`backgroundColor transparent` |
| 改行 | CRLF を LF に揃えてから連結する（CRLF だと `@startuml\n` に当たらず設定が連結されない。実測） |
| キャッシュ | `diagrams/puml-<sha1 8 桁>.svg`（+ 送ったソース `.puml`）。git 管理外（`**/diagrams/puml-*`）。SVG 先頭に `<!-- ddq <版> engine=plantuml plantuml=<版> server=<url> -->` |
| フォント | SVG の `<text>` は `textLength` で幅が固定され、Typst（resvg）はそれを尊重する（実測）。サーバ側と Typst 側でフォントが違っても箱からはみ出さず、違いは行の高さだけ。`Yu Gothic` は Windows のローカルと発行者の Typst が同じ実体を引くための既定 |
| レイアウト | `!pragma layout smetana` 固定（決定 17）。Windows 版 jar は `%TEMP%\_graphviz\dot.exe`（2.44.1）を内蔵展開するので既定は本物の dot だが、Linux サーバには無い |

### 13.3 ddq 側（`src/plantuml/mod.rs`）

- 探索: java = `DDQ_JAVA` → `JAVA_HOME/bin` → PATH → レジストリ `JavaSoft\{JDK,JRE,…}` → 既知パス（Java / Adoptium / Microsoft / Zulu / Corretto / BellSoft）。
  jar = `DDQ_PLANTUML_JAR` → exe の隣の `plantuml.jar` → `PLANTUML_JAR`。
- `ensure(dir)`（pdf / html / diagrams）: 設定済みサーバに届けばそれ → 既定ポートの `serve` が上がっていればそれ →
  原稿に ```` ```plantuml ```` が無ければ何もしない（**PlantUML を使わない文書に Java を要求しない**）→ ローカルを空きポートで起動。
  URL は `DDQ_PLANTUML_SERVER` で quarto に渡す。
- PicoWeb の起動（PoC-4 の実測に基づく）: `-picoweb:0:127.0.0.1` で起動すると実ポートが `webPort=<n>` として **stderr** に出る
  （stdout は空。stdout を待つと永久に止まる）。stderr は専用スレッドで読み、起動待ちの間だけ行をチャネルで渡す。
  待つ側は `recv_timeout` で 30 秒を上限にする（JVM が何も出さない・改行を出さないときも打ち切る）。
  その後は `io::sink` に捨て続ける（パイプ詰まり防止。溜め込まない）。
  `/serverinfo` が 200 になるまで待つ（実測 0.2 秒、上限 30 秒）。
- 停止: `/stopserver` は JVM を終了しない（実測）ので `kill`。Drop で必ず kill する。
  親（ddq）が異常終了しても JVM を残さないよう、**Job Object（`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`）**に入れる
  （`windows-sys` の `Win32_System_JobObjects` `Win32_System_Threading` `Win32_Security`）。
  Job Object に入れられなかったときは、その場で JVM を kill してからエラーを返す（まだ Drop の持ち主がいない）。
- HTTP は手書き（`TcpStream`、HTTP/1.1、`Connection: close`、chunked 対応）。新しいクレートは足さない。

### 13.4 コマンド

| コマンド | 内容 |
|---|---|
| `ddq plantuml serve [--port 18080] [--bind 127.0.0.1]` | 執筆者向け。Java と jar を探して PicoWeb を上げ、Ctrl-C まで待つ。既定ポートならフィルタが設定なしで見つける |
| `ddq pdf` / `ddq html` | `ensure` でサーバを用意し、render 後に止める。打つのは 1 コマンドだけ |
| `ddq diagrams` | `diagrams/*.puml`（キャッシュ `puml-*` を除く）も同名 `.svg` に。同じサーバの決め方 |
| `ddq release` | `cli/vendor/plantuml.jar`（MIT 版）を `plantuml.jar` として同梱。無ければ停止（`cli/vendor/README.md`） |

### 13.5 テスト

- `tests/plantuml.rs`: `ddq diagrams` の `.puml`（LF / CRLF、設定の連結、構文エラーの図を書かない）、`ddq plantuml serve --port 0`
  の起動と、親を kill したとき JVM も消えること。Java か jar が無ければ skip（CI は `DDQ_E2E=1` で失敗）。
- `tests/e2e.rs`: `examples/docs` に PlantUML の状態遷移図を 1 枚置き、PDF・配布 HTML で `puml-*.svg` が 1 つできること。
- CI: `actions/setup-java`（Temurin 17）と、`cli/vendor/plantuml.jar` をリリースから取得してキャッシュ（`PLANTUML_VERSION`）。
- 検証した版: PlantUML 1.2026.8（MIT）、Java 17.0.2、Quarto 1.9.38。

### 13.6 大方針との関係

「執筆者は Quarto と VSCode 拡張だけ」は、PlantUML を使わない文書と LAN サーバのある組織では**そのまま**。
変わるのは「PlantUML を使い、かつ LAN サーバが無い執筆者」だけで、その持ち物は発行者と同じ（リリース一式 + Java）。
利用マニュアルの「役割の違いは持ち物だけ」の延長として、例外の範囲を明示する（3 章・8 章・11 章）。

---

## 14. 見出し・表・図のラベル（`ddq tag`）

検討と設計の全体は `docs/revision-study.md`（ddq-revision）にある。ここは CLI 側の実装に絞る。

### 14.1 なぜ要るのか

改訂履歴を**ページ単位ではなく「見出し・表・図」単位**で作るために、その 3 者を一意に指す
キーが要る。キーは Quarto のラベル（`{#sec-x}` / `label="tbl-x"` / `{#fig-x}`）をそのまま使う。
見出し文言やキャプションは改訂で変わるがラベルは変わらないので、版をまたいだ対応付けが安定し、
改訂履歴表からは `@sec-x` でそのまま参照リンクになる（二重管理にならない）。

### 14.2 何を拾うか（`doc::units`）

行頭のパターンだけで拾う。Pandoc の完全なパーサは要らない（ラベルを足す位置が分かればよい）。

| 種別 | 記法 | ラベル |
|---|---|---|
| 見出し | `# 見出し {属性}` | 属性の `#sec-…` |
| 統一テーブル | `::: {.tbl caption="…" label="…"}` | 属性の `label=` |
| IPO 図 | `::: {.ipo … label="…"}` | 同上（IPO は表番号を持つ） |
| パイプ表 | 表の前後（空行 1 つまで可）の `: キャプション {#tbl-…}` | 属性の `#tbl-…` |
| 図 | `::: {#fig-x}` | この記法は id 必須なので常に有り |
| 図（画像） | 行頭の `![キャプション](パス){#fig-x}` | 属性の `#fig-…`。キャプションだけで ID の無い画像は `bare-figure` の警告（ID を足すと図番号がずれるので足さない）。キャプションも ID も無い画像は拾わない |

走査から外すもの:

- YAML front matter とコードフェンスの内側
- **`.tbl` / `.ipo` / `#fig-` ブロックの内側の見出し**。IPO 図は `## <機能名>` `### 入力`
  `### 処理` `### 出力` という見出しで中身を書く記法で、これは文書の節ではない
- キャプションの無い `.tbl`（採番されない表。ラベルを付けても参照できないので `no-caption` の警告だけ）
- 種別の接頭辞で始まらない ID を既に持つ見出し・パイプ表（`{#u-0001}` など）は `foreign-id`、
  ID を 2 つ以上持つものは `multiple-ids` の警告だけ。Pandoc の ID は 1 つで、2 つ目を足すと
  後ろの 1 つしか使われず、足したラベルがリンク先にならない（実際に `{#sec-x #u-0001}` を作ってしまった）。
  既存の ID を付け替えるとリンクが変わるので、人が決める

### 14.3 文書順（`doc::project`）

`_quarto.yml` の `book.chapters`（`part:` の入れ子も含む）と `book.appendices`（付録。2.4.0 から）を書かれた順に起点として `{{< include >}}` を再帰展開する。
**include のパスの基準は「`chapters:` に並べた章ファイルのあるディレクトリ」で、入れ子の include でも
変わらない**（include 元ファイルの位置ではない）。執筆フォルダの `_quarto.yml` にも同じ注意書きがある。
ここを取り違えると章の大半を取りこぼす（実際に踏んだ）。

`_quarto.yml` が無いフォルダでは配下の qmd/md をパス順に並べる（テンプレート外でも一応動く保険）。

### 14.4 候補ラベル（`doc::labels`）

`sha1(相対パス + 種別 + 正規化した文言)` の先頭 6 桁に接頭辞を付ける（衝突したら 8 → 10 → 40 桁）。

- **決定的**にするのは、「一覧 → 一部だけ書き戻し → 再実行」で未書き戻し分の候補が揺れないため。
  CLI と VSCode 拡張で同じ結果が出るのでテストもしやすい。
- 章番号の連番にしないのは、章構成を組み替えると番号と場所がずれて却って分かりにくいため。
- 一度書き戻したラベルは、内容が変わっても**不変のキー**として扱う（候補生成に内容を使うのは初回だけ）。

### 14.5 書き戻し

`tag list` は編集指示（`file` / `line` / `col` / `insert`）を返し、`tag apply` がそれを当てる。
**`col` は行頭からの文字数**（バイト数ではない。日本語の見出しで意味が変わる）。挿入はすべて
1 行の中で完結し、行の増減を伴わない。VSCode 拡張はこの編集指示を `WorkspaceEdit` として自分で
当てる（Undo が効き、未保存のバッファにもそのまま当たる）。

- 同じファイルの中は**行番号の大きい順**に当てて位置ずれを防ぐ
- 改行コードは保つ（CRLF の行に挿入しても CRLF のまま）
- `--from` は当てる前に文書側と突き合わせる: 行が動いた・既にラベルがある・ラベルの形が違う・
  他と重複する、のいずれかなら**何も書かずに止める**

### 14.6 テスト

- `src/doc/*` の単体テスト: `chapters:` の読み取り（`part:` 入れ子・字下げの終わり）、入れ子 include の
  パス解決、行頭パターン（見出し・`.tbl`・`.ipo`・`#fig-`・pipe 表）、ユニット内側の見出しを外すこと、
  候補ラベルの決定性と衝突回避、挿入位置（既存属性あり／なし、CRLF）。
- `tests/tag.rs`: 2 段の入れ子 include を持つ執筆フォルダを組み立て、`tag list --json` の
  `order` と `items`、`tag apply --all` の冪等性、`--dry-run` が書かないこと、CRLF の保存、
  `--from` で人が直したラベルを当てられること、形の違うラベル・重複ラベルを**何も書かずに**弾くこと。
- 実文書での確認（手動、`docs/revision-study.md` §8 V4）: `examples/docs` の見出し 112 件ほかに
  一括付与し、付与前後で PDF は全ページのテキストが一致、HTML も本文・図表番号・内部リンクが不変。

---

## 15. 改訂履歴（`ddq rev`）

検討と設計の全体は `docs/revision-study.md`。ここは CLI 側の実装に絞る。§14（`ddq tag`）が前提で、
対象の見出し・表・図すべてにラベルが付いていることを要求する。

```text
rev next  … 次の改訂記号と比較基準の候補（前回の rev-* タグ）を出す
rev diff  … 基準の版と作業ツリーを比べ、ラベル単位の変更を出す（--write で yml に落とす）
rev build … revisions/*.yml から改訂履歴の表（revisions/history.qmd）を作る
```

### 15.1 比較の両側

**基準**は任意の Git ref（タグ・ブランチ・コミット ID）で、既定は前回の `rev-<記号>` タグ。
**現在**は作業ツリー（未コミットを含む）である。編集しながらメモを書き、書き終えてから
コミットしてタグを打つ、という順で使うため。タグ付けとコミットは人が行う（自動化は
運用が回ってから）。

旧版側も**旧版の `_quarto.yml`** から同じ手順で論理文書を組み立てる（章の追加・削除も
検出できる）。`doc::project::Source` を作業ツリー（`Folder`）と Git（`GitFolder`）で
差し替えるだけで、組み立てのコードは 1 つである。

### 15.2 Git の扱い（`doc::gitsrc`）

libgit2 は使わず `git` の子プロセスで済ませる。踏んだ落とし穴が 2 つある:

- **パスを出力する git コマンドは既定で非 ASCII を 8 進エスケープする**
  （`"\346\226\207\346\233\270/…"`）。`-c core.quotepath=off` を必ず付ける。
  `git show <ref>:<path>` の**中身**は影響を受けず常に UTF-8 で出る。
- 旧版に無いファイルの `git show` は `fatal: path … does not exist` で失敗する。
  先に `ls-tree` で一覧を取り、その中にあるものだけ読む（エラー文字列の解析に頼らない）。

### 15.3 ユニットと帰属（`doc::diff`）

論理文書を「ラベルを持つ見出し・表・図」＝ユニットに切り、**ユニットごとに本文を比べる**。
行単位の差分は取らない（差分の中身は VSCode の差分エディタが見せる。ここが答えるのは
「どのラベルが変わったか」だけ）。

変更は**最も深いユニットに一意に帰属**する。地の文は直近上位の見出し、表・図の中身は
その表・図自身に付き、親の見出しには伝播しない（伝播させると改訂履歴に同じ変更が
親子で二重に載る）。ラベルの無い表・図は、包んでいる見出しの本文に含める。
パイプ表は、ラベルのあるキャプションに接する表の行（空行 1 つまで挟んだ直前、無ければ直後）を
その表のユニットにする（キャプションは表の前にも後にも書けるので、行を読む前に対応を作る）。
画像の図（`![…](…){#fig-x}`）はその 1 行がユニットである。

判定は `changed` / `added` / `removed` / `renamed`（名称だけ変わった）。並びは新版の
文書順で、`removed` は旧版で直前にあった項目の後ろに挿す。

**改行コードの正規化だけは `--strict` でも必ず行う。** `core.autocrlf=true` の環境では
作業ツリーが CRLF・`git show` の出力が LF になり、そのまま比べると全ユニットが
「変わった」になってしまう（実測）。`--strict` が戻すのは行内の空白と空行の扱いだけである。

### 15.4 改訂ファイルと生成物

改訂 1 回 = `revisions/rev-<記号>.yml` 1 本（`doc::revfile`）。`base`（人が選んだ ref）と
`base_commit`（解決した SHA）の両方を持つ。タグは付け替えられるので名前だけでは再取得の
結果が変わり、SHA だけではそのコミットが到達不能になったときに復元できないためである。
`--write` は既存のメモを label をキーに引き継ぎ、差分から消えたのにメモが残っているものは
`stale: true` にして残す（人が消すまで捨てない）。各行の `line` は拡張が場所・差分を開くときの
移り先で、removed は旧版での行を書く（`rev diff --json` の `line_old` から）。

YAML は決まった形だけを読み書きする（依存を増やさないための割り切り。受け付ける形は
`doc/revfile.rs` の冒頭にある）。

`rev build` の生成物は `revisions/history.qmd` で、`index.qmd` の前付けから include して使う
（独立した章は作らない。テンプレートは前付けに改正履歴を置く規約で、`lib.typ` も
「目次より前のページ」を想定している）。表は書式ごとに出し分け、PDF だけ「頁」列を持つ
（`lib.typ` の `_xref-page` が参照先のページ番号を出す）。**caption は付けない**
（採番されない前付けに置くため。付けると「表 0-1」になる）。**`merge-cols` も使わない**
（自動分割で次ページに続いたとき、結合セルが空欄になって改訂日と記号が読めなくなる）。

### 15.5 テスト

- `src/doc/diff.rs` の単体テスト: 帰属（地の文は見出し・表の中身は表）、空白のみ・CRLF のみの
  差を無視すること、`added` / `removed` / `renamed`、IPO 内側の見出しをユニットにしないこと、
  削除が文書順の位置に入ること。`src/doc/revfile.rs`: 往復（書いて読む）、人が手で書いた形、
  記号の採番（Z → AA）。
- `tests/rev.rs`: 実際に `git init` したリポジトリで `rev-A` を打ち、代表的な編集
  （変更・改名・追加・削除・空白だけ）を加えて `next` / `diff` / `diff --write` / `build` を通す。
  メモの引き継ぎ、`stale`、空メモの警告と `--check` の異常終了も見る。git が無ければ skip。
