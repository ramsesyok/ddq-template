# ddq — テンプレート CLI の設計書

`template/*.bat` `*.sh`（7 種 × 2）を Rust 製のシングルバイナリ **`ddq`** に統合し、
mermaid → SVG 変換を内蔵する。保守者向けの最低限の設計書。
利用手順は利用マニュアル（`manual/`）、様式・変換の内部は [template/PIPELINE.md](../template/PIPELINE.md)。

- 状態: **実装済み・移行検証済み**（2026-09-19。§12.5 に結果）
- 対象版: テンプレート 2.0.0

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
| 8 | リポジトリ構成 | `template/` はソース置き場として残し、`cli/` に Cargo プロジェクトを追加。`build.rs` で `../template` を埋め込む | Lua/CSS/typ の編集体験と `manual/` の導線を壊さない |
| 9 | 対象 OS | **Windows x64 のみ**（msvc, `+crt-static`）。CI も Windows 1 本 | まず簡易に。OS 依存部は `cfg(windows)` で分離し、後から他 OS を足せる形にする |
| 10 | 名前 | `ddq`（design-doc-quarto）。環境変数は `DDQ_*` | 短く衝突しにくい |
| 11 | merman の深さ | v1 は決定的計測（既定）+ resvg-safe 相当のみ。実フォント計測（TextMeasurer）は載せない | 例外経路のために本体より重い部品を抱えない |
| 12 | 設計書 | この `cli/DESIGN.md` | コードの隣に置く |
| 13 | 多文書リポジトリ | `ddq add`（執筆フォルダ追加）と `ddq update --all` を新設 | 「init で 1 つ目、add で 2 つ目以降、版上げは update --all」と説明できる |
| 14 | コーディング | **clap + derive** で引数定義。標準関数だけの手書きパースはしない。人間が読みやすいコードを優先 | 保守性 |

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
│   └── tests/                   … golden テスト（§10）
├── template/                    … 機構ファイルのソース（従来どおり編集する）
│   ├── VERSION / PIPELINE.md
│   ├── design-doc.lua / design-doc.css / postprocess-html.js / mermaid-config.json
│   ├── lib.typ / typst-template.typ / typst-show.typ / quarto-publish.yml
│   ├── release-guide.typ        … 「はじめかた」スライド（Typst。ddq release が PDF にする）
│   ├── scaffold/{repo,content}/
│   └── vendor/mermaid.min.js    … 新規（11.16.0）
├── docs/ manual/                … 従来どおり
└── .github/workflows/ci.yml     … Windows: fmt / clippy / test / build --release
```

削除: `template/*.bat` `template/*.sh` `template/package.json` `template/package-lock.json` `template/node_modules/` `template/puppeteer.json`。

### 3.2 release フォルダ

```
quarto-template-<版>/
├── ddq.exe
├── README.md            … リポジトリの README
├── はじめかた.pdf        … 最初の一歩（template/release-guide.typ を Quarto 同梱の Typst で PDF に。
│                            Marp は npm 依存なので使わない）
└── manual/
    ├── 利用マニュアル.pdf
    └── html/index.html
```

`template/` は release に**含めない**（すべて exe に埋め込まれている）。

### 3.3 doc リポジトリへの配置（現状維持）

| コマンド | 書き出すもの | git |
|---|---|---|
| `init` / `add` | scaffold（無いものだけ）+ 機構ファイル 4 本 + `.template-version` | コミット |
| `update` | 機構ファイル 4 本 + `.template-version`（上書き） | コミット |
| `setup`（`html` / `pdf` が内部で呼ぶ） | `lib.typ` `typst-template.typ` `typst-show.typ` `_quarto-publish.yml`（上書き） | `.gitignore` 済み |

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
ddq release  [out-dir] [--with-sample] [--no-build]
ddq mermaid  -i <in.mmd>… -o <out.svg>… -c <config.json> [-b <color>]   (hidden)
ddq --version
```

共通: `<writing-folder>` 省略時は `docs`。相対パスはカレント基準（現行 bat と同じ）。
執筆フォルダの妥当性は「`_quarto.yml` があること」で判定する。

### 呼び出し関係

```
init ──► add ──► update
html ──► setup ──► update
pdf  ──► setup ──► update
          └─(quarto render)─► design-doc.lua ──► ddq mermaid
diagrams ──────────────────────────────────────► mermaid と同じ変換器
release ──► pdf, html（manual に対して）
```

### 各コマンド

| コマンド | 処理 | 現行 |
|---|---|---|
| **init** | 1) リポジトリ直下に `.gitignore` `.gitattributes` `.vscode/settings.json` `README.md`（`{{CONTENT_DIR}}` 置換）を「無いものだけ」置く 2) `add <repo>/<name>` | init-doc |
| **add** | 前提: 親フォルダに `.gitignore` がある（無ければ「先に `ddq init`」と案内）。拒否: 対象に `_quarto.yml` が既にある。処理: scaffold の content 一式を無いものだけ置く → `update` → `quarto render --to html` で疎通確認（`--no-render` で省略） | （新規） |
| **update** | 機構ファイル 4 本と `.template-version` を上書き。`--all <repo>` は配下の `_quarto.yml` を持つフォルダを列挙して全部に適用（`_book/` `.quarto/` `node_modules/` は探索しない） | update-doc |
| **setup** | `update` → PDF 側 4 ファイルを上書き。**ブラウザ検出と puppeteer.json 生成は廃止** | setup |
| **html** | `setup` → `quarto render --to html`（env: `MERMAID_SVG=1` `DDQ_BIN`）。出力 `_book/` | build-html |
| **pdf** | `setup` → `quarto render --to typst --profile publish`（env: `DDQ_BIN`）→ `_book/*.pdf` を `design-doc.pdf` にバイナリコピー | build-qmd |
| **diagrams** | `diagrams/*.mmd` → 同名 `.svg`。設定は執筆フォルダ直下の `mermaid-config.json`（無ければ埋め込み） | render-diagrams |
| **release** | 1) `--no-build` でなければ `pdf` `html` を `manual/` に実行 2) `release/quarto-template-<版>/` を作り直し、`current_exe()` を `ddq.exe` としてコピー、`README.md`、埋め込みの `release-guide.typ` を `quarto typst compile --input version=<版>` で `はじめかた.pdf` に、`manual/design-doc.pdf` → `利用マニュアル.pdf`、`manual/_book` → `manual/html` 3) `--with-sample` で `docs/` を同梱（`_book` `.quarto` `design-doc.pdf` `lib.typ` 等を除外） 4) zip（§7.3） | make-release |
| **mermaid** | §5。hidden（`--help` の一覧に出さない） | quarto run mmdc |

`TEMPLATE_ROOT`（旧フィルタが `mermaid-config.json` のフォールバック探索に使っていた）は廃止した。
フィルタは執筆フォルダ直下の `mermaid-config.json` があれば `-c` で渡し、無ければ渡さない
（ddq が埋め込みの同じ既定を使う）。§6 参照。

### 非 ASCII パス

Quarto → Lua フィルタへ渡るパスの非 ASCII 文字が U+FFFD に化ける既知問題（`design-doc.lua` 冒頭のコメント）は残る。
`init` / `add` / `html` / `pdf` は `quarto` を起動する前に執筆フォルダの絶対パスを検査し、日本語で理由を説明して停止する。
フィルタ側の検査もそのまま残す。

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

配布先に追加インストールは**不要**（Quarto と、mermaid 用の Edge/Chrome を除く）。

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
| Windows | `windows-sys` または `winreg`（App Paths 探索） |
| 一時領域 | `tempfile` |
| WebSocket（CDP） | `tungstenite`（`handshake` のみ。TLS なし） |

### 7.3 zip

現行 bat の教訓をそのまま要件にする。

- UTF-8 名フラグ（general purpose bit 11）を立てる（`manual/利用マニュアル.pdf` が化けない）
- 書き終えたらエントリ数と staging のファイル数を照合し、不一致なら zip を削除してエラー
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

`ADVANCED.md` §3 をこの手順に書き換える。

### 7.6 CI（GitHub Actions, windows-latest）

`cargo fmt --check` → `cargo clippy -- -D warnings` → `cargo test` → `cargo build --release`。
ブラウザ系テストは runner に Edge があるので動く（無い環境では skip）。

---

## 8. コード構成（案）

```
cli/src/
├── main.rs            … clap の Cli / Commands enum と dispatch だけ
├── commands/
│   ├── init.rs  add.rs  update.rs  setup.rs
│   ├── html.rs  pdf.rs  diagrams.rs  release.rs
│   └── mermaid.rs     … hidden サブコマンド（引数→ renderer 呼び出し）
├── assets.rs          … include_dir! の窓口（機構ファイル名の定数、書き出し関数）
├── writing_folder.rs  … 執筆フォルダの検証・列挙（_quarto.yml の有無、非 ASCII 検査）
├── quarto.rs          … quarto の起動（env 付与、終了コード → anyhow::Error）
├── mermaid/
│   ├── mod.rs         … Engine 選択と共通インタフェース（Vec<(input, output)> → Result）
│   ├── browser.rs     … 探索・HTML 生成・起動・CDP（DevToolsActivePort → WebSocket）
│   └── merman.rs      … merman クレート呼び出しと foreignObject 抑止
└── zip.rs             … release 用
```

方針:

- `main.rs` は薄く。各サブコマンドは `pub fn run(args: XxxArgs) -> anyhow::Result<()>` の形に揃える。
- clap は derive のみ（`#[derive(Parser)]` / `#[derive(Subcommand)]` / `#[derive(Args)]`）。`hide = true` で `mermaid` を隠す。
- OS 依存（レジストリ・既知パス）は `browser.rs` に閉じ込め `#[cfg(windows)]` で囲む。
- 「なぜそうしているか」が非自明な箇所（CDP を使う理由、1 図 1 ページ、文書に付いたコンテナ、UTF-8 フラグ、foreignObject 抑止、`DDQ_BIN` 優先）はコード内コメントに §番号付きで理由を書く。現行 bat / Lua の実測コメントの流儀を引き継ぐ。

---

## 9. 根拠となる実測（2026-09-18）

docs/manual の `diagrams/*.mmd` 37 本（flowchart 27、stateDiagram-v2 4、sequence 2、mindmap、erDiagram、block-beta 3、xychart 2、requirement 2。すべて日本語ラベル、`htmlLabels:false`）を使用。
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

- fixtures: `docs/diagrams/*.mmd` と、mermaid-cli で作った golden SVG（`cli/tests/golden/`）。
- 比較: id（`my-svg` / `svg-<name>`）を正規化したうえで、**数値列（幾何）の一致**を見る。直列化の差（自己閉じタグ）は無視。
- browser 系: Edge/Chrome が見つからなければ `skip`（`#[ignore]` ではなく実行時判定でメッセージを出して return）。
  sequenceDiagram は mermaid 既定の `"Open Sans", sans-serif` で文字幅を測るため、日本語の fallback フォントが
  OS ロケールで変わり、基準環境（ja-JP Windows）以外では配置がずれる（GitHub の en-US ランナーで実測）。
  CI は `DDQ_GOLDEN_LOOSE=1` を設定し、一致しない図は「数値の個数が同じ・viewBox が 15% 以内」の緩い比較にする。
  保守者の手元（ja-JP）では厳密一致のまま。
- merman 系: 常時実行。foreignObject を含まないことをアサート。
- コマンド系: 一時ディレクトリに `init` → `add` → `update --all` を流し、置かれるファイル一覧と `.template-version` を検証。`quarto` の起動はモック（`quarto.rs` を trait 化）か、PATH に `quarto` があるときだけ実施。
- 手動: `ddq release` 後に `manual/` の PDF/HTML を目視。`docs/` の全図を旧 PDF と並べて確認（§9 の比較ページ生成をスクリプト化しておく）。

---

## 11. 保留・将来

| 項目 | 状態 |
|---|---|
| 一括事前変換（`ddq pdf` が render 前に全フェンスを 1 起動で焼く） | 保留。`ddq mermaid` は複数入力対応なので、フェンス抽出と hash 再現の目処が付けば足せる |
| merman の実フォント計測（TextMeasurer + fontdb/rustybuzz） | 保留。フォールバック品質に不満が出たら |
| Linux / macOS バイナリ | 保留。OS 依存は `cfg(windows)` に閉じ込めてある |
| QuickJS + DOM シムで本物の mermaid.js をブラウザ無しで動かす | 見送り（R&D コスト大） |
| `@preview/merman`（Typst パッケージ内で直接描画） | 見送り。Typst 0.15 必須（Quarto 1.9 は 0.14.2）で、配布 HTML もカバーしない |
| 執筆者プレビューでの SVG 焼き込み | 見送り（決定 5）。プレビューは Quarto のみで動くことを優先 |

---

## 12. 移行検証（テンプレート機能の回帰確認）

ddq 自体のテスト（§10）とは別に、**移行前後で作成物（PDF / 配布 HTML）が変わっていないこと**を確認するフェーズを置く。
対象は `docs/`（サンプル）と `manual/`（利用マニュアル）。この 2 つで本テンプレートの記法は網羅されている。

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

- `cli/tools/regress.py`（開発用、配布しない）。サブコマンド `capture <baseline|candidate> <docs|manual>` と `compare`。
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
- [x] 利用マニュアル（`manual/`）2・4・11・12・13 章を `ddq` に書き換え。「`quarto preview` の図は発行物と微妙に違い得る」を明記。多文書リポジトリの手順（`add` / `update --all`）を追加
- [x] scaffold の `.gitignore` から `node_modules/` `puppeteer.json` の項を外す（残しても害はない）
- [x] `.github/workflows/ci.yml` 追加
- [x] `cli/tools/regress.py` を作り、§12.2 の比較で合格条件を満たすことを確認（報告を `regress/report/` に残す）
- [x] `template/VERSION` → 2.0.0
