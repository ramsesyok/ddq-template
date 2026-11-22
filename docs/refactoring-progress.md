# リファクタリング実施記録

`docs/refactoring-plan.md`（2026-11-22 承認）の実施記録。計画書の本文は変更せず、ID ごとの
実装状況・変更ファイル・試験と結果・計画からの逸脱・残存リスクをここに残す。

- 基準: `main` `17ed32e`（`template/VERSION` 3.1.0-pre1）。作業開始時の作業ツリーは計画書だけが未追跡
- 記録日: 2026-11-22
- コミット・プッシュはしていない（指示があるまで行わない）。ID ごとの差分は「差分の取り出し方」参照

## 環境

Windows 11 Home 10.0.26300 / cargo 1.98.1 / Node 24.18.0・npm 11.13.0 / Quarto 1.9.38（同梱 Pandoc・Typst・Deno）/
Java 17.0.2 / VS Code 1.147.0 / Python 3.12.0。PlantUML jar は `cli/vendor/README.md` の SHA-256
（`3629c9cd…dab2f9`）と一致を確認して配置（gitignore 対象）。

Rust の試験は計画 §6 の条件で実行する: Git 管理外の短い一時パス（`TMP`/`TEMP` =
`C:\Users\ramse\AppData\Local\Temp\ddqt`）、`DDQ_E2E=1`、検証済み `DDQ_PLANTUML_JAR`、`npm_config_offline=true`、
`cargo test --locked --offline --no-fail-fast -- --test-threads=1`。拡張は Quarto 同梱 Pandoc を PATH に置き `REQUIRE_PANDOC=1`。

## 試験の種類の区別

| 記号 | 意味 |
|---|---|
| **模擬** | fake VS Code・`vi.mock`・模擬梱包（`tests/fixtures/release/pack.js`）など、実物の代わりを使う試験 |
| **実CLI** | ビルドした `ddq.exe` を子プロセスで起動する試験（Rust の `tests/*.rs`、再現スクリプト） |
| **実Host** | 実 VS Code の Extension Host（`npm run test:host`、`DDQ_HOST_REAL=1`）と VSIX の起動確認（`test:smoke`） |
| **実生成** | 実 Quarto で PDF/HTML を生成して中身を見る試験 |
| **契約** | 実 exe の出力から採った期待値（`cli/tests/fixtures/contract/`）を、Rust と TS の両方が読む試験。TS 側は exe を起動しないが、期待値は模擬ではなく実測 |

## 変更前の基準（計画 §6 の再測定）

| 対象 | 結果 |
|---|---|
| Rust 全件（実CLI・実生成を含む） | **168 件成功・失敗 0・ignored 0、exit 0**（10 分 42 秒）。unit 89 / commands 10 / e2e 4 / faults 8 / golden 2 / mechanism 2 / plantuml 4 / release 11 / rev 13 / rev_ops 4 / table_pagination 2 / tag 18 / third_party 1。計画 §6 の内訳と一致 |
| 拡張 `npm run typecheck` | 2 つの tsconfig とも成功 |
| 拡張 `REQUIRE_PANDOC=1 npm test` | **38 ファイル・376 件成功**（計画 §6 と一致） |
| 拡張 `npm run build` / `npm run check:offline` | 成功 |

## Phase 3a　修正前の現象の再現（実CLI・実ソース）

再現スクリプトは debug ビルドの `ddq.exe`、`template/postprocess-html.js` の `listOf` を
ファイルから切り出したもの、拡張の `version.ts` / `revfile.ts`（Node 24 の型除去で直接実行）を使う。

| ID | 再現結果 |
|---|---|
| GEN-01 | `ddq init --no-render` 直後の雛形で、YAML として読んだ `book.chapters` は 8 件。`tag list --json` の `order` は第 1 章までの 6 ファイル（index・history・第1章の3節）、`listOf('chapters')` は `index.html`・`chapters/01/index.html` の 2 件。`ddq html` は 8 HTML を作るが、第 4 章の `.tbl` キャプション（ユーザ属性一覧 など 3 件）に表番号が付かない。**行末コメントを前行に移すだけ**で Rust は 36 ファイル、JS は 8 章を返す。引用（`"…"`・`'…'`）・空白入りパス・`part:` 入れ子・`appendices` を混ぜた `_quarto.yml` では、YAML 上は章 5 件＋付録 1 件のところ、Rust は二重引用＋コメントの第1章・空白入りの章・付録を警告なしに落とし、JS は `index.html` だけ |
| EXT-01 | `compareVersions` が `preN` を 0 とするため、`3.0.0`↔`3.0.0-pre4`、`pre4`↔`pre1`、`pre10`↔`pre4` がすべて 0 → `modified`（update を案内）。`2.4`↔`2.4.0` も `modified`。版ファイルが空・不正（`abc`）なら `folder-older`、ddq 側が不正なら `folder-newer` |
| DATA-01 | TS: 題名 `path: C:\docs と "引用"` を `toYaml`→`parse` するたびに逆斜線が 2 倍（1 回目で `C:\\docs`、3 回目で 8 本）。`\"` は正しく戻る。Rust: ddq 自身が書く形 `title: "path: C:\\docs と \"引用\""` を `rev build` で読むと、改訂履歴の表に `C:\\docs` と出る |
| IO-01 | BOM＋CRLF の原稿に `tag apply --all`: exit 0、ラベル 2 件を付与、CRLF は保持、**BOM は消える**（先頭 `ef bb bf 23` → `23 20`） |

## 差分の取り出し方

コミットしないので、ID ごとの到達点を一時インデックスで木（tree）にして記録した（作業ツリーには触れない）。
ID の差分は `git diff <前の ID の木> <その ID の木>`、基準からは `git diff HEAD <木>`。

| ID | 木（到達点。前の ID を含む） |
|---|---|
| QA-01 | `ef50041cee7854341ad420c1a1b2da230b84252c` |
| CLI-01 | `4c8a4b4953b2ca49e636ca99b7ed651fcf4cef53` |
| EXT-02 | `acf9cebf1d719b9bfd20edb1b5263f7edb01d8fc` |

木は `git gc` で消えうる（どの参照からもたどれない）。消えたら作業ツリーの差分と下の「変更ファイル」が正。

## 項目別の記録

<!-- 各 ID の節を実施順に追記する -->

### QA-01　ddq と拡張の契約試験

- **状況**: 完了
- **変更ファイル**
  - 追加: `cli/tests/contract.rs`（5 件）、`cli/tests/fixtures/contract/`（`README.md`・`manuscript.json`・`tag-list.json`・
    `tag-list.2.4.4.json`・`tag-applied.json`・`rev-next.json`・`cli.json`）、
    `extensions/ddq-template/src/ddq/contract.ts`（`parseTagList` / `parseRevNext`。形を確かめて `ContractError`）・`contract.test.ts`
  - 変更: `cli/tests/e2e.rs`（結果行の期待を `cli.json` から読む。`{{FOLDER}}` は `std::path::absolute` で解決したパス）、
    `cli/src/doc/revfile.rs`（共有の golden の往復試験）、`extensions/ddq-template/src/ddq/run.ts`（`runJson`）、
    `activateRevision.ts`・`revision/provider.ts`（受け取った JSON を `parseTagList` / `parseRevNext` に通す）
- **試験と結果**
  - 実CLI: `contract.rs` 5/5（`tag list --json`・`tag apply --all`・`rev next --json`・結果行・終了コード 3 →
    `ddq update` → `ddq setup` の順に回復）、`e2e.rs` 4/4（205 秒）、`revfile` 単体 5/5
  - 契約（TS）: `contract.test.ts` 24 件（EXT-02 で 1 件追加後）。期待値を 1 か所ずつ壊すと該当の試験が落ちることを確認
  - 実Host: `DDQ_BIN` に HEAD の release exe を指定して 25/25 合格
  - 期待値の採取: HEAD の release exe。保管版 3.1.0-pre1・3.0.0 の exe も同じ出力、2.4.4 は `edit.original` が無いだけ
- **計画からの逸脱**: なし
- **残存リスク**: 期待値は採取時の exe の出力。ddq の出力を意図して変えたときは README の手順で採り直し、表に理由を残す必要がある

### CLI-01　原稿の読み取りを 1 回にする

- **状況**: 完了
- **変更ファイル**: `cli/src/doc/project.rs`（`assemble` に章立ての読み取りと本文の読み取りをまとめ、`Sink` で
  行・本文のどちらを集めるかを選ぶ。`order_with_texts` を追加。章立てが読めないときの警告は従来どおり）、
  `cli/src/doc/mod.rs`（`scan_folder` が `order_with_texts` を使い、行の複製をやめる）
- **試験と結果**
  - 単体: 読み取り回数を数える `Source` で、`logical` と `order_with_texts` が 1 ファイル 1 回（変更前は 2 回。
    仮の計数で確認）、文書の順を保つこと、読めないファイルは順序に残して本文から外すこと
  - 実CLI（新旧 exe の差分比較）: HEAD と CLI-01 の release exe に、実フォルダ 9・境界 6（入れ子の相対 include・
    同じファイルの二重 include など）・大きな文書（1081 ファイル・5.1 MB）と Git の `rev diff` 4 場面を通し、
    stdout・stderr・終了コード・書き換え後のファイルを比べて**すべて一致**（panic のスレッド番号と、作り直した
    リポジトリのコミット番号だけを正規化）
  - 計測（大きな文書）: `tag list --json` 0.24 → 0.15 秒、`rev diff --base rev-A --json` 59.4 → 54.9 秒
  - 実Host: CLI-01 の exe で 25/25 合格
- **計画からの逸脱**: なし
- **残存リスク**: なし（出力は同一）。別件として下記「計画外で見つけた既存の不具合」の 1 を記録

### EXT-02　ddq の起動口をまとめ、終了時に孫まで止める

- **状況**: 完了
- **変更ファイル**
  - 追加: `src/ddq/launch.ts`（設定・起動する ddq の決定・出力チャネル・進捗通知・拡張の終了処理）、
    `src/ddq/launch.test.ts`（3 件）、`src/ddq/process.test.ts`（10 件）
  - 変更: `src/ddq/run.ts`（実行中の子を記録して `stopAll` で孫ごと止める。出力を溜めず `keepLine` に合う
    行だけ残す。`StreamResult.output` を `lines` に）、`build.ts`・`init.ts`・`add.ts`・`extension.ts`・
    `activateRevision.ts`・`revision/provider.ts`（`launch.ts` を使う。`build.ts` に `isResultLine`）、
    `dialogs.test.ts`（模擬の `runStreaming` を新しい結果の形に）、`ddq/contract.test.ts`（結果行の判定 1 件）
- **試験と結果**
  - 実プロセス（ddq の代わりに node）: 引数・cwd・PATH の受け渡し、UTF-8 の 1 文字が割れないこと、stdout と
    stderr の行の組み立て、終了コード 3、ENOENT、並行実行、取消と `stopAll` で子と孫が止まること。
    孫は `detached` で起動（libuv のジョブオブジェクトに入ると親と一緒に消え、孫まで止めたことを確かめられない）。
    子だけを止める実装に戻すと 2 件落ちることを確認
  - 模擬: `launch.test.ts`（fake VS Code）で、結果行だけが残ること、取消、終了後は出力・通知を出さないこと。壊すと落ちることを確認
  - 拡張全体: typecheck 成功、41 ファイル・413 件成功、build・check:offline 成功
  - 実Host: 25/25 合格
- **計画からの逸脱**: なし（`ddqCommand` は試験だけが使うが、削除は計画にないので残した）
- **残存リスク**: 孫の停止は `killTree`（Windows は `taskkill /T`）に依存。ddq が孫を別のジョブ・別のセッションで
  起動するようになった場合は試験の前提（`detached` の孫）を見直す

### GEN-02　Rust・Lua・TS で重複している規則の契約試験

- **状況**: 完了（全件の再実行の結果は下に追記）
- **変更ファイル**
  - 追加: `cli/tests/fixtures/contract/diagrams.json`・`merge-cols.json`、`cli/tests/filter_contract.rs`（5 件）、
    `extensions/ddq-template/src/tableEditor/formats/mergeCols/contract.test.ts`（11 件）
  - 変更: `cli/src/mermaid/prerender.rs`（キーの材料をまとめた `mermaid_key` を切り出し、設定の CRLF→LF をその中へ。
    出力は同じ。単体 2 件追加）、`cli/src/plantuml/mod.rs`（単体 2 件追加のみ）、`cli/tests/fixtures/contract/README.md`
- **期待値**: どの実装の出力も使わない。mermaid のコードは Quarto 同梱 Pandoc の `-t json`、キーは Python の
  hashlib、PlantUML のソース・先頭コメントの照合・表の rowspan は design-doc.lua のコメントの規則から手で決めた
  （生成スクリプトはリポジトリに入れていない。決め方は README に記載）
- **試験と結果**
  - 単体（Rust）: ① prerender のコード抽出・キー・`fresh` の判定、② `assemble_source`・`svg_header`・`is_local_url` が期待値どおり
  - 実生成（Lua）: `filter_contract.rs` が本物の design-doc.lua を `quarto pandoc -t json --lua-filter` に通す。
    期待どおりの名前の SVG を置くとフィルタが使う（キーがずれると失敗する）、先頭コメントでの照合（mermaid は
    本物の ddq の `ddq identity` と merman での描き直し、PlantUML は試験が立てる偽のサーバ。ローカル＝127.0.0.1、
    LAN＝[::1]）、描き直した SVG の先頭の 1 行が Rust の `svg_header` と同じ形、表の rowspan。5/5 成功（約 20 秒）
  - 契約（TS）: 表エディタが原稿を開いたときの結合の展開（`findEditTarget`）が期待値どおり。11/11
  - 変異: Lua 7・Rust 5・TS 2 か所の規則を 1 つずつ壊し、14 か所すべてで該当の試験が落ちることを確認。
    （初めに試した「Rust の先頭行の `\r` を落とさない」は、`str::lines` が既に `\r` を落とすため挙動が変わらない等価な変異だった。
    `\n` だけで切る変異に替えて落ちることを確認）
  - 拡張全体: 42 ファイル・424 件成功。Rust: `cargo fmt --check`・`cargo clippy --all-targets -D warnings` 成功
- **計画からの逸脱**: なし。既知の差は**直さず**期待値に印を付けて固定した（計画「出力を変えない」「既存のキーや
  図名の変更にはキャッシュ移行判断が必要」）。直すかどうかは判断が要る:
  1. mermaid のタブ: Pandoc は空白 4 つに展開、prerender はタブのまま → 先回りの SVG をフィルタが使わず描き直す（遅くなるだけ）
  2. PlantUML の `@start` の前の空白: Rust は落とし、Lua は残す（Rust はキーを作らないので図名は不変）
  3. CRLF の `mermaid-config.json`: Lua が LF に読むのは Windows だけ。ほかの OS ではキーがずれる（描き直すだけ）
  4. 表エディタの merge-cols 表示: 書式つきのセル・`"ALL"`・`""`・旧 `.merge-rows` で文書の出力と違う
     （docs/ddq-template-impl の表エディタ 7 章に既載の差）
- **残存リスク**: `filter_contract.rs` は Quarto が要る（CI の Windows ジョブは `DDQ_E2E=1` なので skip されない）。
  拡張の CI（Linux）は TS 側だけ。PlantUML の URL の判定で、Rust の `is_local_url` はスキームの無い `localhost:8080` を
  ローカルとみなすが Lua はみなさない差があるが、Lua 側を外から試す手段が無いので期待値には入れていない

### 計画外で見つけた既存の不具合（直していない）

1. `ddq tag list` が、同じファイルにラベル無しで種類と題名が同じ項目が 5 つ以上あると panic する（exit 101、
   `cli/src/doc/labels.rs:50`「sha1 40 桁が衝突することはない」。候補の長さ `[6, 8, 10, 40]` を使い切る）。
   HEAD でも `migration/docs` で起きる。CLI-01 の差分比較でも新旧同じ panic
