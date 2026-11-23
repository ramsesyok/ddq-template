# リファクタリング実施記録

`docs/refactoring-plan.md`（2026-11-22 承認）の実施記録。計画書の本文は変更せず、ID ごとの
実装状況・変更ファイル・試験と結果・計画からの逸脱・残存リスクをここに残す。

- 基準: `main` `17ed32e`（`template/VERSION` 3.1.0-pre1）。作業開始時の作業ツリーは計画書だけが未追跡
- 記録日: 2026-11-22
- 2026-11-22 の指示で、ブランチ `refactor/refactoring-plan` に ID ごとのコミットとして積み、PR（ramsesyok/ddq-template#142）にした。ID ごとの差分は「差分の取り出し方」参照
- #142 は 2026-11-22 15:45（UTC。日本時間では 11-23 の 0:45）に `f6e27ed` までがマージされた（マージコミット `6bb9232`）。
  その後の 3 件（GEN-02 の既知の差を直す、ddq から release を外す、既存の不具合 2 件を直す）は、2026-11-23 の指示でブランチ
  `refactor/release-split-and-fixes` に移した（main の上に載せ替えた。中身は同じ）。#143 として 2026-11-23 にマージされた（`5e840b3`）
- 2026-11-23 の外部レビュー（`docs/refactoring-review.md`）は総合 FAIL（14 件）。Critical の 3 件と M-05 を、2026-11-23 の指示で
  ブランチ `fix/review-critical` で直した（末尾の「外部レビューの指摘」。`142d742`）。続けて Medium の残り 5 件と Low の 3 件を
  同じブランチで直した（全件の試験が通った後、2026-11-23 の指示でコミットした。`8b9338a`）。High の 2 件も、同じ日の指示で
  同じブランチで直した（全件の試験が通った後、2026-11-23 の指示でコミットし、PR にした）

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

作業中はコミットしなかったので、ID ごとの到達点を一時インデックスで木（tree）にして記録した（作業ツリーには触れない）。
PR にするときに、この木を順にコミットにした（どの木にも計画書を足した。先頭に計画書だけのコミット）。ブランチでは
`git log 17ed32e..refactor/refactoring-plan` の各コミットが下の表の ID に当たる。REL-03・CI-02・REL-01・CI-01・DOC-01 は同じ
ファイルに続けて手を入れたので、木にもコミットにも分けず 1 つ（`1051dbc`）にした。その後の試験の直しは `ed013ed`。

| ID | 木（到達点。前の ID を含む） |
|---|---|
| QA-01 | `ef50041cee7854341ad420c1a1b2da230b84252c` |
| CLI-01 | `4c8a4b4953b2ca49e636ca99b7ed651fcf4cef53` |
| EXT-02 | `acf9cebf1d719b9bfd20edb1b5263f7edb01d8fc` |
| GEN-02 | `d8b6eec10c9118bf355946b52b9ffb389a053761` |
| EXT-03 | `c2ee2db7dc6b39f9618d760fd5747efee971ae45` |
| EXT-04 | `30041c35dd25a6aec5790f0920d279c56e64d366` |
| GEN-01 | `122c2e192abc180bc7e79d8b0cfc94785e5d5b1a` |
| EXT-01 | `25f40fe5879c466f5b63d41e6d276cec402a35dd` |
| DATA-01 | `78e972cb3d4539675ce8b17e506d2df5eba43041` |
| IO-01 | `2c456eafd13e1e925360baeb4c557d1f783da223` |
| IO-02 | `59bd62ee6a9930a885853932b66a10af391447b9` |

GEN-01・EXT-01・DATA-01 は続けて実装したので、木は後から ID ごとのファイルで組み立てた（3 つで変更ファイルは重ならない。
共有の README は GEN-01 の木では DATA-01 で足した行と節を除いた）。本書の更新は DATA-01 の木にまとめて入れた。
IO-01・IO-02 も同じく後から組み立てた。重なるのは `cli/src/replace.rs` だけで、IO-01 の木のものは IO-02 で足した `create_new` と
その試験 2 つを除いた。IO-01 の木だけを展開して clippy（`--all-targets -D warnings`）・`cargo fmt --check`・単体（replace・tag・rev・
revfile の 21）・`tests/tag.rs` 22・`rev.rs` 15・`contract.rs` 5 が通ることを確かめた。本書の IO-01・IO-02 の記録は IO-02 の木に入れた
（表のこの 2 行は木を作った後に足した）。

木は `git gc` で消えうる（どの参照からもたどれない）。消えたら作業ツリーの差分と下の「変更ファイル」が正。

## 項目別の記録

計画 §5 の台帳（実施順）。各 ID の詳細は下の節、全件と配布候補の結果は「統合の検証」。

| ID | 状態 | 対象コミット | 実行した試験（環境は上の「環境」） | 結果／残件 |
|---|---|---|---|---|
| QA-01 | 検証済 | `ec7c412` | 実CLI（`contract.rs`・`e2e.rs`）、TS の契約、実Host、期待値を壊す確認 | 成功 |
| CLI-01 | 検証済 | `94e9691` | 単体（読み取り回数）、新旧の release exe の出力比較 | 成功。新旧同じ出力 |
| EXT-02 | 検証済 | `0a7a70a`・`ed013ed` | 実プロセス・模擬・実Host、変異 | 成功。孫の停止は Windows だけ（計画の範囲。PR の CI で試験の OS 依存を直した） |
| GEN-02 | 検証済 | `0d1cd46`・2026-11-23 の直し | 単体（Rust）、実生成（本物の design-doc.lua）、TS の契約、変異 | 成功。既知の差のうち 3 組を 2026-11-23 に直した（下の「GEN-02 の既知の差を直す」） |
| EXT-03 | 検証済 | `782cb08`・`ed013ed` | 模擬（応答の順番を試験が決める）、変異 8 か所 | 成功（PR の CI で試験の OS 依存を直した） |
| EXT-04 | 検証済 | `f1475d6` | 単体（HTML・CSP・資産の範囲）、模擬、変異 7 か所 | 成功 |
| GEN-01 | 検証済 | `ff21870` | 単体・実スクリプト（JS）・e2e（`quarto inspect`）、変異、過去の `_quarto.yml` 52 版の比較 | 成功。新規作成の雛形が変わる（VERSION は上げていない） |
| EXT-01 | 検証済 | `0c6a37e` | 単体（出荷済みの 30 版）、模擬（ダイアログ）、変異 6 か所 | 成功 |
| DATA-01 | 検証済 | `9f5a926` | 単体（Rust・TS）、golden、変異 8 か所、新旧の ddq と拡張の相互確認 | 成功。移行なし |
| IO-01 | 検証済 | `c53122a` | 単体、実 exe（読み取り専用・開いたままのファイル） | 成功 |
| IO-02 | 検証済 | `c5aedf0` | 単体（同時の作成）、実 exe（失敗後の再実行・同時の `ddq add`） | 成功 |
| REL-03 | 検証済 | `1051dbc` | 単体、実 exe の release 試験、新旧の比較、変異 9 か所 | 成功 |
| CI-02 | 検証済 | `1051dbc` | 模擬の監査結果・実ビルド、実 `npm audit`、変異 7 か所、PR の CI | 成功 |
| REL-01 | 検証済 | `1051dbc` | 実 exe の release 試験、実 vsce、変異 5 か所、配布候補一式 | 成功 |
| CI-01 | 検証済（PR・main への push・手動実行・わざと失敗させた実行を GitHub 上で確かめた。2026-11-23） | `1051dbc`・`810249b` | jar の照合の手元実行、PR の CI、配布候補の流れの手元での再現と GitHub 上の実行、照合の食い違い 5 つ、CI と手元の成果物の比較 | `--locked`・jar の照合・起動対象は PR の CI で成功。配布候補の流れ（`release-candidate.yml`）は PR の CI で全ジョブ成功（38 分）。CI の成果物は手元の出力と見た目が同じ（下の「REL-02・CI-01　配布候補の作成を GitHub Actions に移す」） |
| DOC-01 | 検証済 | `1051dbc` | パスの照合、manual・設計書のビルド | 成功。旧資料は消していない |
| REL-02 | 検証済（PR・main への push・手動実行・わざと失敗させた実行を GitHub 上で確かめた。2026-11-23） | `5d76c67`・`810249b` | 実CLI（`release.rs` の追加 1）、変異 3 か所、Rust 全件、実生成（マニュアル 18 章）、配布候補の流れの手元での再現と GitHub 上の実行 | 手元の `ddq release`（2026-11-23 から保守者用の ddq-release）は、入れ替えの途中で失敗した回の出力を捨てて作り直す（暫定の扱いのまま）。正式に配る一式は CI の配布候補だけ（2026-11-22 承認。計画 §7）。失敗した実行に `release-<版>` が残らないことと、公開済みの資産のハッシュが変わらないことは、2026-11-23 にわざと失敗させた 2 本で確かめた（下の「REL-02・CI-01」の「わざと失敗させた実行」） |

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
- **後で直したこと（PR の CI）**: Linux（ubuntu・Node 20／22）で、取消・`stopAll` の 2 件が孫の停止を待って時間切れになった。
  `killTree` は変更前から Windows でだけ木ごと止め、ほかの OS は子だけを止める（計画も「取消時の Windows 子孫終了」）ので、
  試験が計画より広いことを確かめていた。孫の停止は Windows でだけ確かめ、ほかの OS では子が止まることを確かめて孫を片づける形に
  直した（製品のコードは変えていない）。Windows で子だけを止める変異に戻すと、従来どおり 2 件とも落ちる。下の「統合の検証」

### GEN-02　Rust・Lua・TS で重複している規則の契約試験

- **状況**: 完了
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
  - Rust 全件（計画 §6 の条件。e2e・機構 golden（PDF/HTML の番号と参照）・release を含む）:
    **186 件成功・失敗 0・ignored 0**（基準 168 ＋ QA-01〜GEN-02 で足した 18 件）
- **計画からの逸脱**: なし。既知の差は**直さず**期待値に印を付けて固定した（計画「出力を変えない」「既存のキーや
  図名の変更にはキャッシュ移行判断が必要」）。直すかどうかは判断が要る:
  1. mermaid のタブ: Pandoc は空白 4 つに展開、prerender はタブのまま → 先回りの SVG をフィルタが使わず描き直す（遅くなるだけ）
  2. PlantUML の `@start` の前の空白: Rust は落とし、Lua は残す（Rust はキーを作らないので図名は不変）
  3. CRLF の `mermaid-config.json`: Lua が LF に読むのは Windows だけ。ほかの OS ではキーがずれる（描き直すだけ）
  4. 表エディタの merge-cols 表示: 書式つきのセル・`"ALL"`・`""`・旧 `.merge-rows` で文書の出力と違う
     （docs/ddq-template-impl の表エディタ 7 章に既載の差）

  → 2026-11-23 の指示で 1・3 と、4 のうち `"ALL"`・`""`・旧 `.merge-rows` を直した（下の「GEN-02 の既知の差を直す」）。
  2 と、4 の書式つきのセルは残した
- **残存リスク**: `filter_contract.rs` は Quarto が要る（CI の Windows ジョブは `DDQ_E2E=1` なので skip されない）。
  拡張の CI（Linux）は TS 側だけ。PlantUML の URL の判定で、Rust の `is_local_url` はスキームの無い `localhost:8080` を
  ローカルとみなすが Lua はみなさない差があるが、Lua 側を外から試す手段が無いので期待値には入れていない

### EXT-03　非同期の結果を、始めたときの対象と照合する

- **状況**: 完了
- **変更ファイル**
  - `src/folders.ts`（右クリックメニュー用の執筆フォルダ一覧の作り直しに通し番号。古い走査の結果で上書きしない。拡張の終了後は書かない）
  - `src/tableEditor/activateTableEditor.ts`（パネルと編集セッションを起動ごとの `Host` に持たせる。書き戻しを始めたときの
    パネル・編集セッションを覚え、確認ダイアログの後と反映の後に照合する。返信は受け取ったパネルにだけ送る。閉じたパネルから
    遅れて届いたメッセージは扱わない）
  - `src/revision/provider.ts`（タグの状態の取り直しに通し番号。文書の変更では本文をすぐ送り、`ddq rev next` は入力が止まって
    から 800 ミリ秒後に 1 回だけ。閉じたエディタには送らない。`write` が書けたかを返し、書けなければ知らせて画面を文書に戻す）
  - `src/ddq/launch.ts`・`src/extension.ts`（出力チャネルを起動が持ち、終了で捨てる。`activateLaunch(context)`）、
    `src/activateRevision.ts`（検証用の `internal.writeNote` が書けたかを返す）
  - 試験: `src/folders.watch.test.ts`（3）、`src/tableEditor/activateTableEditor.test.ts`（6）、`src/revision/provider.test.ts`（6）、
    `src/ddq/launch.test.ts`（新しい引数）、`tests/host/index.js`（実Host の確認 2 つ）
- **試験と結果**
  - 模擬（応答の順番をテストが決める）: 応答順の逆転（フォルダの走査・タグの取り直し）、確認ダイアログの間に別の表を開く・
    パネルを閉じる・拡張が終わる、閉じてから開き直す（閉じたパネルからの遅れたメッセージ）、未保存の編集で行がずれた、
    設定（ddqPath）の変更、ルートの追加・削除、閉じた後の結果。15 件すべて成功
  - 変異: 照合 8 か所を 1 つずつ外し、8 か所すべてで落ちることを確認（初めは「閉じた後も送る」を捕まえられなかったので、
    ddq の実行中に閉じる試験を足した）
  - 拡張全体: typecheck 成功、45 ファイル・439 件成功、build・check:offline 成功
  - 実Host: 27/27 合格（`DDQ_BIN` は現在の debug ビルド。表エディタをもう一度開いてもパネルは 1 枚のまま、閉じてから開き直せる、を追加）
- **維持したもの**: 表エディタのパネルは 1 枚を使い回す・開き直しで手前に出す・Undo（`WorkspaceEdit` 1 つ）。改訂履歴エディタの
  外からの書き換え（`rev diff --write`）の反映
- **失敗の UI の変更**（計画「失敗の UI 変更は別に明記」）
  1. 表エディタ: 確認ダイアログの間に別の表を開いたとき、書き戻さずに「確認のあいだに表エディタで別の表を開いたため、
     書き戻しを中止しました。」を出す（従来は、開き直した表の版と比べるため、元の表を書くか誤った理由で止まるかが場合による）
  2. 改訂履歴エディタ: 文書へ書けなかったとき「改訂履歴の文書に書き込めませんでした。ファイルを開き直してください。」を出し、
     画面を文書の内容に戻す（従来は何も出さず、画面だけが直した内容のまま）
- **計画からの逸脱**: なし。「タグ状態取得をメモ入力ごとに繰り返す必要を検討」の結論は、打鍵ごとには不要（タグは Git の操作でしか
  変わらない）。ただし「端末でタグを打った後に入力すれば反映される」は残すため、取り直しをやめずに入力が止まった後へまとめた
- **残存リスク**
  - Webview から届いた Apply が、どの表（編集セッション）に対するものかはメッセージに識別子が無いので確かめていない。Apply が
    届く直前に別の表を開き直すと、新しい表に古い内容を書きうる（EXT-04「Host 受信時の識別子と書込対象を検証する」で扱う。
    → EXT-04 で対処済み）
  - ステータスバー（`build.ts` の `updateStatus`）もアクティブエディタの切り替えごとに非同期で求めるので、同じ型の逆転がありうる。
    計画の対象箇所に無いので手を付けていない（表示が一瞬古いだけで、書き込みは無い）
  - 多ルートの追加・削除と拡張の終了後の振る舞いは模擬だけで確かめた（実Host では、単一フォルダのウィンドウにルートを足すと
    拡張ホストが再起動するため）

### EXT-04　Webview の器を共有し、受け取ったメッセージの対象を照合する

- **状況**: 完了
- **変更ファイル**
  - 追加: `src/webviewHtml.ts`（HTML・CSP・nonce・バンドルの URI の組み立てと `localResourceRoots`。検証用に、画面ごとに
    Webview から `ready` が届いた回数を返す内部コマンド `ddqTemplate.internal.webviewReadies`）、`src/webviewHtml.test.ts`（5）
  - 変更
    - 表エディタ: `src/tableEditor/activateTableEditor.ts`（`buildHtml` を削り共有のものへ。編集セッションに番号を振って init で渡し、
      Apply に付いた番号がいまのセッションと違えば書かない。書き戻した後の取り直しでは同じ番号のまま）、
      `model/WebviewMessages.ts`（init・apply に `session`）、`webview/TableEditor.tsx`（init の番号を覚えて Apply に付ける）
    - ラベル一覧: `src/activateRevision.ts`（資産の範囲を限定。一覧に取り直しの番号を付けて送り、書き戻しの番号が違えば当てない）、
      `src/tagging/messages.ts`（list・apply に `generation`）、`src/revisionWebview/TagTable.tsx`（一覧の番号を覚えて書き戻しに付ける）
    - 改訂履歴: `src/revision/provider.ts`（`html` は共有のものを呼ぶだけにし `nonceOf` を削除。資産の範囲を限定）
    - `src/extension.ts`（`activateWebviewProbe`）
  - 試験: `activateTableEditor.test.ts`（+2）、`activateRevision.test.ts`（+1）、`TableEditor.test.tsx`・`TagTable.test.tsx`（画面が受け取った
    番号を返すこと）、`tests/host/index.js`（3 画面の起動の確認 4 つ）
- **試験と結果**
  - 単体: HTML は共通化前の 2 か所の出力と nonce 以外同じ（表エディタ・ラベル一覧・改訂履歴。タイトル・`data-view` とも）。CSP は
    `default-src 'none'` で `connect-src` が無い。nonce は 4 か所同じで、開くたびに変わる。`localResourceRoots` はそのバンドルの出力先だけ
  - 模擬: 開き直す前の表の Apply が遅れて届いても書かない（確認ダイアログも出さない）。書き戻した後も同じ番号で続けて Apply できる。
    取り直す前の一覧で選んだ書き戻しは当てず、取り直した一覧は新しい番号で届く
  - 変異: 7 か所（表の番号の照合・書き戻し後の番号・一覧の番号の照合・資産の範囲・CSP の `default-src`・画面が番号を返す・
    画面が番号を覚える）を 1 つずつ壊し、すべて落ちることを確認
  - 拡張全体: typecheck 成功、46 ファイル・447 件成功、build・check:offline 成功
  - 実Host: 30/30 合格（確認の行は 27 → 30。開き直した表エディタの確認は既存の行に足した）（追加: ラベル一覧・表エディタ・開き直した表エディタ・改訂履歴の各画面で、Webview から `ready` が届く＝
    バンドルが CSP と資産の範囲の中で読めてスクリプトが動く）。資産の範囲をわざと別のフォルダにすると、タブは開くが `ready` が
    届かずに失敗することを確認（タブが開くかだけを見る従来の確認では気付けない）
  - 旧データ: 照合用の行の無い古い ddq の一覧（実Host・模擬とも従来どおり書き戻さない）、改訂ファイル（実 ddq の
    `rev diff --write` の出力）が開いて書ける
- **維持したもの**: 画面ごとのメッセージの型・TableModel・改訂モデルは各機能に残した。全画面共通の状態ストアは入れていない。
  画面の HTML（タイトル・`data-view`・CSP）は従来と同じ
- **失敗の UI の変更**
  1. 表エディタ: 開いている表が変わった後に、前の表の Apply が届いたとき「表エディタで開いている表が変わったため、書き戻しません
     でした。もう一度反映してください。」（従来は開き直した表に前の表の内容を書きうる）
  2. ラベル一覧: 一覧を取り直した後に、前の一覧の書き戻しが届いたとき「一覧を取り直したため、書き戻しませんでした。」
     （従来は前の一覧の行のまま当てた。行の照合（original）があるので変わった行には書かないが、画面の一覧とずれる）
- **計画からの逸脱**
  1. 計画は「表は `localResourceRoots` を対象資産に限定する」。「資産範囲を共有する」に合わせ、ラベル一覧・改訂履歴（従来は既定＝
     拡張のフォルダ全体とワークスペース）も同じくそのバンドルの出力先に限った。両画面ともワークスペースの画像などを読んでいない
     （`src/revisionWebview/` に資産の参照が無い）ことを確かめ、実Host で起動を確認した
  2. 検証用の内部コマンド `ddqTemplate.internal.webviewReadies` を足した（計画の「2 バンドルの配信・起動・メッセージを実 Host で
     確認」のため。既存の `internal.*` と同じく package.json には出さない）
  3. 改訂履歴エディタの `edit` には識別子を付けていない。Custom Editor は 1 文書に 1 つで書く先を取り違えないこと、`edit` は
     文書全体の書き戻しで、版で照合すると打鍵中の入力を落とすこと（自分の書いた変更で版が進む）から
- **残存リスク**
  - Host から Webview への到達（init・list が画面に描かれたこと）は実Host では見ていない（jsdom の画面試験で見ている）。
    `ready` が示すのは、スクリプトの起動と Webview から Host への到達
  - 番号はパネル（拡張の起動）ごとの通し番号。Webview が作り直されたときは `ready` で新しい init・list を受け取るので照合は保たれる

### GEN-01　章の並びを Quarto と同じに読む（雛形＋両パーサ）

- **状況**: 完了
- **変更ファイル**: `cli/src/doc/project.rs`（`chapters_of`・`as_doc_path`）、`template/postprocess-html.js`（`listOf` を `docPath`・`docsOf` に置換）、
  `template/scaffold/content/_quarto.yml`（行末コメントを前の行へ。上の案内文も合わせた）。追加: `cli/tests/fixtures/contract/chapters.json`・README の節、
  `cli/tests/filter_contract.rs`（2 件。本物の postprocess-html.js を `quarto run`）、`cli/tests/e2e.rs`（`scaffold_chapters_are_all_read`）
- **規則（両実装で同じ）**: 二重引用（`\\`→`\`・`\"`→`"`、ほかの `\x` は残す）、単一引用（`''`→`'`）、引用なしは先頭か空白の後の `#` 以降がコメント、
  途中の空白はパスの一部、引用の後ろはコメントだけ、`:` を含むものは外す。`part:` の入れ子・`appendices`・キーと同じ字下げの列も読む
- **試験と結果**
  - 期待値: `quarto inspect` の `config.book`（PyYAML の読みとも一致）。`items` は YAML の規則から手で決めた
  - 単体（Rust）・実スクリプト（JS）: 共有 6 ケース＋1 件の書き方 23 個がすべて一致。JS は再実行・増分（出力 1 つ）でも同じ結果
  - 新規作成（e2e）: `ddq init` → `quarto inspect` の章が `tag list` の並びに全部入り、`ddq html` でどの章のキャプションにも番号が付く（第 4 章の `tbl-user` が「表 4.1.1-1」）
  - 変異: 14 個のうち 13 個は落ちた。残る「雛形だけ HEAD に戻す」は期待どおり通った（パーサの修正で古い雛形も読める）
  - 差分比較（過去版・既存文書）: 全履歴の `_quarto.yml` 52 版と作業ツリー 9 個で、旧と新の読みが変わるのは 17ed32e の雛形だけ（2 章→8 章）。新しい読みは 61 個すべて YAML と一致
  - Rust 全件: `cargo fmt --check`・`clippy --all-targets -D warnings` 成功、`cargo test`（`DDQ_E2E=1`・PlantUML の jar あり）191 件すべて成功（単体 99・結合 92）
  - 拡張: 拡張のコードは変えていない。EXT-01 の後の全件（46 ファイル・454 件）と実Host（DATA-01 の節）はこの変更を含む
- **分かったこと**: 行末コメントの雛形は PR #141（17ed32e、2026-11-21）で入ったもので、**どのリリースタグにも入っていない**（v3.1.0-pre1 の雛形は 2 章でコメント無し）
- **HTML 側で Quarto の解決済みの並びを使えるか**: サイドバーには part の子と付録を含む全章が順に並ぶ（実測）。それでも採らなかった。HTML の形は Quarto の版で変わりうるうえ、利用者の判断で両パーサの修正に決まっているため。
  post-render の環境変数は確かめていない（確かめるためのスクリプトがシェルの書き方の誤りで動かなかった）
- **利用者に見える差**: 手で書いた `_quarto.yml` に行末コメント・引用・空白入りのパス・part・同じ字下げの列があると、ラベル候補・改訂差分・HTML の番号の対象が増える。
  postprocess-html.js は機構ファイルなので、版を上げないまま新しい exe を使うと、既存の 3.1.0-pre1 の執筆フォルダは「同じ版で中身が違う」（終了コード 3 → update の案内）になる。
  既存の `_quarto.yml` は書き換えない
- **計画からの逸脱**: 列をキーと同じ字下げで書く形も、両実装で読むようにした。旧 JS はこの形を読めていたので、そのまま残すためである
- **残存リスク**: `part` に書いたファイル（部の扉ページ）とフロー形式 `[a, b]` は対象外（扉ページは 2026-11-23 に直した。H-01）。版を上げないため、同じ `3.1.0-pre1` を名乗る exe どうしで機構が違う（次のリリースで版を上げる前提）

### EXT-01　版を preN まで番号順に比べ、分からない版を決めつけない

- **状況**: 完了
- **変更ファイル**: `extensions/ddq-template/src/ddq/version.ts`（`parseVersion`・`compareVersions`・`classifyMismatch`。`Mismatch` に `unknown`）、
  `src/build.ts`（`offerUpdate` の判定を `classifyMismatch` へ。`unknown` の案内を追加）。試験: `src/ddq/version.test.ts`（書き直し）、
  `src/dialogs.test.ts`（+3）
- **規則**: `数字(.数字)*` と任意の `-preN` だけを版とみなす。数字は桁を 0 で埋めて比べる（`2.4` = `2.4.0` は従来どおり）。
  同じ数字なら `pre1 < pre4 < pre10 < 正式版`。どちらかが版の形でなければ比べない（`unknown`）
- **試験と結果**
  - 単体: VERSION の履歴から取った出荷済みの 30 版が古い順に厳密に並ぶ。pre の順（両方向）、版の形でないもの（空・`abc`・`3.0.0-rc1`・
    `3.0.0-pre`・前後の文字）、`classifyMismatch` の 4 分類
  - 模擬（ダイアログ）: `3.0.0-pre4` の文書を `3.0.0` の ddq で発行すると update を勧める。逆（`3.0.0` の文書・pre4 の ddq）は降格になるので
    出さない。版ファイルなし・`abc`・ddq の版が読めない・ddq が起動できないときは、update を出さずに版を確かめるよう案内し、「出力を表示」で
    両方の版を出力に書いたものを見せる
  - 変異: 6 か所（pre を見ない・正式版を pre より前にする・版の形の終わりを見ない・比べられない版を古いとみなす・案内の分岐を外す・
    ddq の版が不明なら古いとみなす）を 1 つずつ壊し、すべて落ちることを確認
  - 拡張全体: typecheck 成功、46 ファイル・454 件成功。実Host は DATA-01 の節（版の案内は模擬だけで見ている。実Host は同じ版の ddq で動かす）
- **利用者に見える差**（修正前の再現は Phase 3a の表）
  1. pre の文書を新しい pre・正式版の ddq で発行すると「テンプレートが古い」の案内になる（従来は「同じ版で中身が違う＝手で直された
     可能性」の案内。どちらも update は勧める）
  2. 文書の方が新しい pre・正式版なら update を出さない（従来は「手で直された可能性」として update を勧め、機構を古い版に戻しえた）
  3. 版ファイルが無い・空・不正、ddq の版が分からないときは update を勧めない（従来は無条件に「古い」として update を勧めた）
- **維持したもの**: `ddq update`（CLI）は変えていない。同じ版で中身が違うときの案内、拡張に同梱した ddq の版の確認（`checkBundled`）も従来どおり
- **計画からの逸脱**: なし
- **残存リスク**: `-rc1` など pre 以外の付記は版とみなさない（これまで出荷していない）。版ファイルを手で消した文書では update を自動で勧めなくなるので、
  案内に従って `ddq update` を手で実行してもらう

### DATA-01　改訂ファイルの引用付きの値を Rust と TS で同じに読む

- **状況**: 完了
- **変更ファイル**: `cli/src/doc/revfile.rs`（`unquote`。試験 +2）、`extensions/ddq-template/src/revision/revfile.ts`（`unquote`）、
  `src/revision/revfile.test.ts`（+26）。追加: `cli/tests/fixtures/contract/rev-values.json`・README の節。書き手（`quote`）は両方とも変えていない
- **規則（両実装で同じ）**: 二重引用は `\\` → `\`、`\"` → `"`、ほかの `\x` はそのまま。単一引用は `''` → `'`（逆斜線は文字）。
  引用なしはそのまま。メモ（`note: |`）は逃がさない（従来どおり）
- **試験と結果**
  - 期待値: 規則から手で決めた（読む 13 個・書く 12 個。どちらの実装の出力も使わない）
  - 単体（Rust・TS とも）: 読む 13 個が一致。書く 12 個は、決めたバイト列で書き、読み戻すと同じ値、二度目の保存も同じバイト列。
    逆斜線・引用・空行を含む複数行のメモと空のメモが往復する。ddq が実際に書いた golden（`rev-B.golden.yml`）は従来どおりバイト単位で往復する
  - 変異: 8 か所（両実装で、`\\` を戻さない・`''` を戻さない・単一引用の中でも `\"` を `"` にする・知らない `\x` の逆斜線を落とす）を
    1 つずつ壊し、すべて落ちることを確認（戻した後のファイルは sha256 で一致）
  - 実物の相互確認（新旧の ddq と拡張の `revfile.ts` を Node で直接読み込む）: 原稿の見出し `## path: C:\\docs と "引用" {#sec-winpath}`・
    `## 手順 #2: C:\\tmp\\a.txt {#sec-hash}`（表示は `C:\docs`・`C:\tmp\a.txt`）を消して `rev diff --write`
    - Rust の書き出しは新旧で同じバイト列（`title: "path: C:\\\\docs と \"引用\""`）
    - Rust → TS: 新しい拡張は原稿どおり `path: C:\\docs と "引用"` と読み、2 回保存してもバイト列が変わらない。旧拡張は逆斜線 4 本と読み、
      保存のたびに倍になる（2 回目の保存でファイル上 16 本）
    - TS → Rust: 新しい拡張でメモ（Windows のパス・引用・空行）を書いて保存したものを、新しい ddq の `rev diff --write` で保存し直しても同じバイト列
    - 改訂履歴の表（`rev build`）: 消えた見出しの名称が `path: C:\docs と "引用"` と見出しと同じに出る（旧 ddq は `C:\\docs`）
    - コメント・未知のキー: 人が書いたコメント行と未知のキー（`reviewer:`）は、拡張・ddq のどちらで保存しても消える（従来どおり。変えていない）
  - 拡張全体: typecheck 成功、46 ファイル・480 件成功（`REQUIRE_PANDOC=1`）。Rust の revfile の単体 7 件成功、`cargo fmt --check` 成功
  - 実Host: 30/30 合格（`DDQ_BIN` は GEN-01・EXT-01・DATA-01 を含む debug ビルド。`rev diff --write` の改訂ファイルを編集画面で開いて
    メモを書き、`rev build` の表に出る、を含む）。確認の行は EXT-04 のときと同じ
- **既存の改訂ファイルの読み方の例**（`rev build` の表で確認）

  | 書かれた形 | 旧（ddq・拡張とも） | 新（ddq・拡張とも） |
  |---|---|---|
  | `"path: C:\\docs と \"引用\""`（ddq が書く形） | `path: C:\\docs と "引用"` | `path: C:\docs と "引用"` |
  | `"C:\\\\docs"`（旧拡張が 1 度保存した形） | `C:\\\\docs` | `C:\\docs`（書かれたとおり。自動では戻さない） |
  | `'it''s "手書き"'`（手書きの単一引用） | `it''s "手書き"` | `it's "手書き"` |
  | `'say \"hi\"'` | `say "hi"` | `say \"hi\"` |
  | `"#タグ: 末尾\\"` | `#タグ: 末尾\\` | `#タグ: 末尾\` |

- **利用者に見える差**
  1. 名称に逆斜線を含む消えた見出し・図表が、改訂履歴の表で原稿と同じに出る（従来は逆斜線が倍）。消えていないものは相互参照なので変わらない
  2. 拡張で改訂ファイルを保存しても逆斜線が増えない。旧拡張で保存して増えた逆斜線は書かれたとおりに読む。直すときは Git の差分を見て手で戻す
     （一括の書き換えはしない）
  3. 手書きの単一引用の `''` が `'` になり、単一引用の中の `\"` は文字のまま
- **戻す方法**: 両方の `unquote` を戻す（書き手は変えていないので、変更後に保存したファイルも旧版で従来どおり読める）
- **計画からの逸脱**: なし
- **残存リスク**: 二重引用の `\n`・`\t` などは YAML と違い文字のまま読む（ddq・拡張は書かない。ほかの道具で書いたファイルだけ違う）。
  旧拡張で 2 回以上保存したファイルは逆斜線が多いまま残る

### IO-01　原稿・改訂ファイルを書く前に照合し、一時ファイルから置き換える

- **状況**: 完了
- **変更ファイル**
  - 追加: `cli/src/replace.rs`（同じフォルダの一時ファイルに書いて付け替える。試験 7。うち Unix 専用 1）
  - 変更: `cli/src/main.rs`（`mod replace`）、`cli/src/commands/tag.rs`（`apply` を「全ファイルの書き戻し後の中身を決める `plan`」と
    「書く直前の照合と置き換えの `write_all`」に分けた。試験 4）、`cli/src/commands/rev.rs`（改訂ファイルは読んだときのバイト列と
    書く直前を比べる `save_revision`、改訂履歴の表も置き換え。試験 1）、`cli/src/doc/revfile.rs`（`load` を `from_bytes` に分けた）
  - 試験: `cli/tests/tag.rs`（+4）、`cli/tests/rev.rs`（+2）。契約の期待値 `cli/tests/fixtures/contract/tag-applied.json`（BOM のある原稿の
    期待値に BOM を足した。README の採取の記録）
- **規則**
  - `tag apply`: どのファイルも書く前に、全ファイルの書き戻し後の中身を決める。書く直前に対象を全部読み直し、走査したときと中身が
    違う（消えた・UTF-8 でなくなったを含む）か読み取り専用のものが 1 つでもあれば、どれも書かずに終了コード 1、stderr にファイルごとの
    理由を挙げる。ファイルはパスの順に 1 つずつ置き換え、途中で失敗したらそこで止めて、書き換えたファイル（件数つき）と書き換えて
    いないファイルを stderr に挙げて終了コード 1。BOM は書く直前のファイルのまま、改行（CRLF / LF）と末尾の改行の有無は元のまま
  - `rev diff --write`: 改訂ファイルを読んだときと書く直前のバイト列が違えば（拡張で保存した・作った・消した）書かずに終了コード 1。
    改訂ファイル・改訂履歴の表（`rev build`）は置き換えで書く（形は従来どおり ddq が決める。BOM は付けない）
  - 置き換え（`replace`）: 同じフォルダに `.ddq-*.tmp` を作って書き、`sync_all` してから付け替える。読み取り専用は先に拒む（Unix では
    付け替えで通ってしまうため）。Windows でほかのプログラムが開いていて付け替えを拒まれたら 1 秒ほど待ち直し、それでもだめなら
    従来どおりその場で書く（**2026-11-22 の利用者判断**。下の「判断を求めたこと」）。一時ファイルはどの場合も残さない
- **試験と結果**
  - 単体（追加 11 を Windows で実行。Unix 専用の 1 は未実行）: BOM＋CRLF＋末尾改行・末尾改行なしが保たれる、パスの順に書く。走査の後に変わった・消えた・読み取り専用の
    ファイルがあると、パスの順で前のファイルも書かない。開いたまま（読みだけ共有）のファイルで止まると、書いたもの・書いていない
    ものを挙げ、閉じてもう一度走査すると残りだけ書き戻す。改訂ファイルは読んだ後に保存・作成・削除されたら書かない。置き換えは
    一時ファイルを残さず、「一時」属性も残さない。置き換える前から開いていた読み手には元のバイト列が見え続ける（その場で書いて
    いない証拠）。すぐ閉じられる排他は待ち直して置き換え、書き込みを共有する開き方ならその場で書く
  - 実 exe（`cli/tests/tag.rs` 22・`rev.rs` 15・`contract.rs` 5）: BOM・改行、読み取り専用（終了コード 1・stdout 空・どのファイルも
    変わらない。`--dry-run` は通る）、開いたままのファイルで止まった後にもう一度実行すると 1 回で書き戻したときと同じ中身、
    書き込みを共有して開いたままでも書き戻せる、改訂ファイルが読み取り専用なら書かない、改訂履歴の表を開いたままなら元のまま
  - 旧 exe（HEAD）との比較（`chapters/02/index.qmd` を CreateFileW で開いたまま）

    | 状況 | 旧 | 新 |
    |---|---|---|
    | 通常（BOM＋CRLF の index.qmd） | 0。BOM が消える | 0。BOM が残る |
    | `--dry-run` | 0。書かない | 同じ（行の並びはパスの順） |
    | 読み取り専用 | 1。**12 回中 7 回はほかのファイルを先に書いた**（どれを書くかは実行ごとに違う）。メッセージは失敗したファイルだけ | 1。どれも書かない。理由つきで挙げる |
    | 読みだけ共有で開いたまま | 1。chapters/01・index.qmd を書いた後に失敗。何を書いたかは出ない | 1。chapters/01 だけ書いて止め、書いた・書いていないを挙げる |
    | 読み・書き共有（削除なし） | 0 | 0（待ち直した後、その場で書く） |
    | 読み・書き・削除を共有 | 0 | 0（置き換え。開いていた側には元のバイト列） |

  - 変異: 14 か所（tag: 書く前の照合・読み取り専用の事前確認・BOM・書いたファイルの列挙・パスの順、rev: 改訂ファイルの照合・
    改訂ファイルと改訂履歴の表をその場で書く、replace: いつもその場で書く・待ち直さない・その場で書く代わりに止める・
    読み取り専用を先に拒まない・「一時」属性つきで作る・失敗した一時ファイルを残す）を 1 つずつ壊し、すべて落ちることを確認
    （戻した後のファイルは sha256 で一致）
  - clippy（`--all-targets -D warnings`）・`cargo fmt --check` 成功。Rust 全件・vitest・実Host は下の「IO-01・IO-02 の全件」
- **利用者に見える差**
  1. `tag apply` が BOM を残す（従来は落とした）
  2. 書けないファイルがあると分かっているときは何も書かずに止め、理由をファイルごとに挙げる。途中で失敗したときは書いたものと書いて
     いないものを挙げる（従来は失敗したファイルだけを挙げ、それまでに書いたファイルは分からなかった）
  3. 書く順がパスの順になり、`--dry-run` の行の並びも毎回同じになる（従来は実行ごとに違った）
  4. `rev diff --write` の途中で改訂ファイルが保存されたら書かずに止める（従来は上書きしてメモを失いえた）
  5. 1 秒以内に閉じられる排他では失敗しない（従来はすぐ失敗）。メッセージの文言が変わった
- **判断を求めたこと**: 一時ファイルからの付け替えは、Windows でほかのプログラムが書き込みを共有して削除を共有せずに開いている
  （CRT の fopen・Python の open・同期ソフトの一部）と拒まれ、従来のその場の書き込みなら通る場面で失敗するようになった（実測）。
  利用者の判断で「1 秒ほど待ち直し、それでもだめなら従来どおりその場で書く」にした（従来通っていた場面はすべて通る）
  → 2026-11-23 の外部レビューの C-01・C-02 を受けて、書き込み用に開いたままなら止める、その場で書くのは付け替えの共有違反だけ、
    に変えた（2026-11-23 の利用者判断。末尾の「外部レビューの指摘」）
- **計画からの逸脱**
  1. 上の判断により、付け替えを拒まれたときはその場で書く。その場合だけ「中断後も途中のバイト列にならない」は保証しない（従来と同じ）
  2. 付け替えは tempfile の `persist`（`MoveFileExW`）ではなく std の `rename` を使った。`MoveFileExW` は削除を共有して開かれている
     ファイルも置き換えられない（実測）のに対し、std は POSIX の意味で置き換える。tempfile の既定の「一時」属性も付けない
  3. 書く順をパスの順にした（途中で止まったときに挙げるファイルの並びを毎回同じにするため）
- **戻す方法**: `tag.rs`・`rev.rs` の書き込みを `fs::write` に戻し、`replace.rs` を消す。契約の期待値の BOM も戻す
- **残存リスク**
  - 書く直前の照合と置き換えの間（数ミリ秒）に保存されたものは検出できない
    → 2026-11-23 に、置き換えるときにファイルごとに照合し直し、Windows では書き込みを拒んで開いておくようにした（末尾）
  - その場で書いたとき（上の判断）は、従来と同じく途中で止まると途中のバイト列になりうる
  - 置き換えると別のファイルになるので、ハードリンクの他方は元の中身のまま、Windows の個別の ACL・代替データストリーム
    （ダウンロード元の印など）・作成日時は引き継がない（新しいファイルはフォルダの ACL を継ぐ）
  - 強制終了が一時ファイルを作ってから付け替えるまでに起きると、`.ddq-*.tmp` が残りうる（原稿は元のまま）
  - Unix 専用の経路（権限の引き継ぎ・シンボリックリンク・フォルダに書けないときのその場の書き込み）は、この環境（Windows）と CI
    （Rust は windows-latest だけ）では実行していない
  - `--from` で途中で止まったときは、書いたファイルの行が変わるので tag list を取り直す必要がある（メッセージで案内する）

### IO-02　作成は上書きしない新規作成にし、update は失敗後の再実行で終わる順にする

- **状況**: 完了
- **変更ファイル**: `cli/src/replace.rs`（`create_new`。同じフォルダの一時ファイルに書いて、上書きしない付け替えで置く。試験 2）、
  `cli/src/assets.rs`（`write_if_absent`・`write_dir_if_absent` は `create_new` で置く。`write_asset` は IO-01 の置き換え。試験 1）、
  `cli/src/commands/update.rs`（機構ファイル → 図のキャッシュ → 版マーカーの順。版マーカーも置き換え）。追加: `cli/tests/rerun.rs`（4。うち Windows 専用 3）
- **規則**
  - init / add の「無いものだけ置く」: 既にあれば従来どおり「既存のため skip」。無ければ一時ファイルに書いて、上書きしない付け替えで
    置く（同時に作られていたら付け替えが失敗し、skip とする）。表示（作成／既存のため skip）は従来どおり
  - `ddq update`: 機構ファイルを置き換え → 版が変わるならキャッシュ（`mmd-*`・`puml-*`）を消す → 最後に版マーカーを置き換える。
    キャッシュを消しきれなければ版マーカーを書かずに終了コード 1（「版マーカーは 0.0.0 のまま」などと案内）。同じ版の修復では
    キャッシュを消さない（従来どおり）。`ensure_current` の内容検査は変えていない
  - 機構ファイル（`update`）・PDF 側ファイル（`setup`）・版マーカーの書き込みは IO-01 の置き換え（待ち直し・その場の書き込みも同じ）
- **試験と結果**
  - 単体: `create_new` は既にあるファイル・フォルダを上書きしない。8 スレッドが同時に作っても作るのは 1 つで中身はその 1 つのもの、
    一時ファイルを残さず「一時」属性も残さない。`write_if_absent` も 8 スレッドで「作成」は 1 つ、人が置いたファイルは上書きしない
  - 実 exe（`rerun.rs`）: 図のキャッシュを掴まれて止まった update は版マーカーが 0.0.0 のまま、閉じてから再実行するとキャッシュを消して
    版マーカーを書き、`setup` が通る。design-doc.lua を掴まれて止まった update は版マーカー・キャッシュとも前のまま、再実行で終わる。
    版マーカーを開いていた読み手には update の後も元の版が見える（切り詰めた空のファイルを読ませない）。同じフォルダへの 2 つの
    `ddq add` を同時に 5 回走らせ、同じファイルを両方が「作成」したことは無い
  - 既存の試験（`commands.rs` 10）: 同じ版の update はキャッシュを残し、版が変わる update はキャッシュだけ消す、手で直した機構は
    update で戻る、など。`contract.rs` 5 も成功
  - 旧 exe（HEAD）との比較

    | 状況 | 旧 | 新 |
    |---|---|---|
    | キャッシュを掴んだまま update → 閉じて再実行 | 1 回目で版マーカーが新しい版になり、再実行は同じ版の修復とみなして**古いキャッシュ 2 件が残る** | 1 回目は版マーカーが 0.0.0 のまま、再実行でキャッシュを消して版マーカーを書く |
    | design-doc.lua を掴んだまま update → 閉じて再実行 | 1 回目は失敗（版マーカーは前のまま）、再実行で終わる | 同じ |
    | 同じフォルダへの `ddq add` を同時に 2 つ（20 回） | 両方が同じファイルを「作成」したことは無かった（競合はこの実行では再現しない） | 同じ |

    同時の add の競合は exe の起動の速さでは窓に入らなかったので、単体（8 スレッドの同時作成）で見ている。旧の書き方
    （無いことを確かめてから書く）に戻す変異はこの単体で落ちる
  - 変異: 6 か所（版マーカーを先に書く・同じ版でもキャッシュを消す・版マーカーをその場で書く・機構ファイルをその場で書く・
    確かめてから書く（旧）・`create_new` が上書きする）を 1 つずつ壊し、すべて落ちることを確認。最初は「版マーカーをその場で書く」が
    素通りしたので、開いていた読み手に元の版が見えることの試験を足した
  - clippy・`cargo fmt --check` 成功。Rust 全件・vitest・実Host は下の「IO-01・IO-02 の全件」
- **利用者に見える差**
  1. キャッシュを消せずに失敗した update を再実行すると最後まで終わる（従来は古い図のキャッシュが残り、次のビルドで古い図を使いえた）
  2. update が途中で失敗したとき、版マーカーが前の版のままであることを案内する。成功時の出力は従来どおり
  3. 機構ファイル・PDF 側ファイル・版マーカーは置き換えで書く（途中で止まっても途中のバイト列にならない。IO-01 と同じ例外あり）
- **計画からの逸脱**: なし（PDF 側ファイルの書き込みも `write_asset` を共有しているので置き換えになった）
- **戻す方法**: `assets.rs` の 3 関数と `update.rs` の順を戻し、`create_new` を消す
- **残存リスク**
  - update が機構ファイルの途中で失敗すると、機構は新旧が混ざり版マーカーは前の版のまま。`ensure_current` が不一致（終了コード 3）
    として update を案内するので、再実行で直る
  - 上書きしない付け替え（tempfile の `persist_noclobber`）を、ハードリンクも RENAME_NOREPLACE も使えないファイルシステム
    （一部のネットワークドライブ・FUSE）では試していない。その場合 init / add が失敗しうる
  - キャッシュの削除は消せないファイルで止まる。それより前に消したキャッシュは消えたまま（次のビルドで描き直す）

### IO-01・IO-02 の全件

両方を入れた作業ツリーで実施した。

- Rust 全件（実CLI・実生成を含む。`DDQ_E2E=1`・PlantUML の jar あり。先に `cargo fmt --check`・`clippy --all-targets -D warnings` 成功）:
  **217 件成功・失敗 0・ignored 0、exit 0**。unit 115 / commands 10 / contract 5 / e2e 5 / faults 8 / filter_contract 7 /
  golden 2（機構の PDF・HTML）/ mechanism 2 / plantuml 4 / release 11 / rerun 4 / rev 15 / rev_ops 4 / table_pagination 2 / tag 22 /
  third_party 1
- vitest 全件（模擬）: 46 ファイル・480 件成功。拡張のコードは変えていないが、拡張の `src/ddq/contract.test.ts` も契約の期待値
  `tag-applied.json`（IO-01 で BOM を足した）を読むので実施した。拡張側は BOM を除いて比べるので結果は変わらない
- 実Host（`DDQ_HOST_REAL=1`、`DDQ_BIN` は上の全件でビルドした debug の exe）: 30/30 合格

### REL-03　release の前に版を照合する

- **状況**: 完了（ローカルの release。計画の「CI は同じ checkout のビルドを使い、配布検証の記録にコミットと exe の SHA-256 を
  残す」は CI-01 で扱う。下の「計画からの逸脱」）
- **変更ファイル**: `cli/src/commands/release.rs`（`check_versions`・`extension_dirs`・`read_json` を追加し、`run` の最初で照合。
  `build_extensions` はフォルダの列挙を `extension_dirs` に寄せた。試験 1）、`cli/tests/release.rs`（+3。`fake_repo` の ddq-a に
  package-lock.json を置いた）、`docs/manual/chapters/18-maintainer/03-procedure.qmd`・`06-release.qmd`・`cli/DESIGN.md`（説明）
- **規則**（2026-11-22 の利用者判断「推奨のとおり」）
  - 重いビルド（マニュアルの機構更新・PDF・HTML、拡張の npm）と第三者表示の照合の前に、`template/VERSION`（前後の空白を除く）・
    この exe の版（ビルドしたときの `template/VERSION`）・各拡張の `package.json` の `version`・`package-lock.json` の `version` と
    `packages[""].version` を比べる。1 つでも `template/VERSION` と違えば終了コード 1、何も作らない。`--no-build` でも同じ
  - 違うものはまとめて挙げる。exe の版が違えば `cargo build --release` し直すよう、拡張が違えば `npm version <版> --no-git-tag-version`
    を案内する（従来の拡張の版の案内と同じ文言を含む）
  - 拡張のフォルダが無い・`package-lock.json` が無いときはここでは止めない（従来どおり梱包・`npm ci` で止まる）。lock に
    `packages` が無い（lockfileVersion 1）ときは `packages[""].version` を見ない。読めない JSON は止める
  - 既存の照合（`build_extension` の拡張の版、VSIX の中の exe のバイト照合）は残した
- **試験と結果**
  - 単体: 揃っていれば通る、拡張のフォルダが無い・lock が無い・古い形式の lock・package.json の無いフォルダは止めない、
    lock の版が無いと「（無い）」として挙げる、読めない lock は止める
  - 実 exe（`tests/release.rs` 14 件すべて成功。追加 3）: 古い exe（作業ツリーの VERSION が 99.0.0）、拡張の版が新しすぎる・古すぎる
    （2 つの拡張を 1 回で挙げる）、lock のルートの version だけ・`packages[""].version` だけが古い。それぞれ `--no-build` の有無の
    両方で、終了コード 1・前回の配布フォルダが残る・「リリースを作成」より前に止まる・マニュアルの機構更新も拡張の梱包も始めて
    いないことを見た。同じ版で中身の違う exe の VSIX（既存の `bundled_exe_must_be_the_released_ddq`）も成功
  - 旧 exe（HEAD）との比較（試験と同じ形の小さなリリース用リポジトリを一時フォルダに組み、各 exe を同梱した VSIX を置いて実行）

    | 状況 | 旧 | 新 |
    |---|---|---|
    | 揃っている（`--no-build`） | 0。zip 8 ファイル | 0。同じ（出力の表示も同じ） |
    | 作業ツリーの VERSION が 99.0.0（= 古い exe）、`--no-build` | **0。3.1.0-pre1 の配布一式を作った** | 1。何も作らない。exe・package.json・lock の 3 か所を挙げる |
    | lock のルートの version だけ古い（`--no-build`） | 0 | 1。そのファイルと値を挙げる |
    | lock の `packages[""].version` だけ古い（`--no-build`） | 0 | 1。同上 |
    | package.json の version が古い（ビルドあり） | 1。ただし先にマニュアルの機構ファイルを書き換え、PDF のビルドまで進んだ | 1。何も始めない |

    最初の比較では変異試験が最後に作った（変異した）exe を使ってしまったので、作り直してから取り直した（上の表は取り直したもの）
  - 変異: 9 か所（照合を呼ばない・exe の版を見ない・exe の版を基準にする・lock のルート／`packages[""]` を見ない・古い形式の lock も
    外れにする・lock が無ければ止める・最初の拡張しか見ない・外れを 1 つしか並べない）を 1 つずつ壊し、すべて落ちることを確認
    （戻した後のファイルは sha256 で一致）
  - clippy・`cargo fmt --check` 成功
- **利用者（保守者）に見える差**
  1. 古い exe・揃っていない lock での release が、重いビルドの前に止まる（従来は通るか、マニュアルのビルドの後に止まった）
  2. 拡張の版の不一致のメッセージが変わった（違うものをまとめて挙げる）。成功時の出力・配布物の名前と構成は同じ
- **計画からの逸脱**
  1. 計画の「CI は同じ checkout のビルドを使い、配布検証の記録にコミットと exe の SHA-256 を残す」は CI の配布候補の作り方
     そのもので、`ddq release` の今後（REL-02 の保留）に関わるので CI-01 に回した。→ 同じ checkout のビルドとコミットは `810249b`、
     exe の SHA-256 は完了レビューで `versions.txt` に足した
  2. lock が無いときは止めない（承認された条件は「版が違えば止める」で、lock が無いことは含まないため。従来どおり `npm ci` で止まる）
- **戻す方法**: `run` の `check_versions` の呼び出しを消す（3 関数と試験も消す）
- **残存リスク**
  - exe の版は `template/VERSION` から作るので、`template/` の中身（機構ファイル）を直して版を上げずにビルドし忘れた exe は検出
    できない（同じ版名で雛形が変わった実例は計画にある。中身の照合はしていない）
  - 拡張を複数にしたときの照合は試験の小さなリポジトリ（2 拡張）でだけ見ている

### REL-01　同梱の VSIX に第三者のライセンス表示を入れる

- **状況**: 完了
- **変更ファイル**: `cli/src/commands/release.rs`（同梱の拡張を梱包する前に `THIRD-PARTY-NOTICES.md` を拡張のフォルダ直下へコピー。
  `check_bundled_exe` を `check_bundled` にし、VSIX の中の表示も照合）、`.gitignore`（コピーした表示を git 管理外に）、
  `cli/tests/release.rs`（+2。模擬の VSIX に表示を入れ、ZIP の隣の VSIX から表示を読み直す照合を足した。実ビルドの梱包スクリプトにも
  表示を入れさせた）、`docs/manual/chapters/18-maintainer/06-release.qmd`・`cli/DESIGN.md`（説明）
- **規則**
  - 対象は ddq.exe を同梱する拡張（`package:win32-x64` を持つもの。今は ddq-template だけ）。ビルドするときは exe と同じく
    リポジトリ直下の表示を `<拡張>/THIRD-PARTY-NOTICES.md` にコピーしてから `npm run package:win32-x64`。vsce がそのまま
    `extension/THIRD-PARTY-NOTICES.md` として入れる（`.vscodeignore` に当たらない）
  - ビルドの有無にかかわらず、梱包した VSIX を開いて `extension/THIRD-PARTY-NOTICES.md` がリポジトリ直下のもの（ZIP に入れるもの）と
    バイト一致することを確かめる。無い・違えば終了コード 1、前回の配布物に触れない（`--no-build` で表示の無い・古い VSIX を配らない）
  - 表示の中身は既存の生成物のまま（`cli/tools/third_party.py` の 5 入力のハッシュつき。入力との照合は従来どおり重い処理の前）
  - jar は同梱一覧に加えない（3.0.0 から同梱しない）
- **試験と結果**
  - 実 exe（`tests/release.rs` 16 件すべて成功。追加 2）: 表示の無い VSIX（REL-01 より前の形）で `--no-build` が止まる、表示が違う
    （中身が違う・改行だけ CRLF）VSIX で止まる。ZIP を全部展開した照合の後に、ZIP の隣の VSIX から表示と exe を読み直し、ZIP の
    表示・release している exe と一致。実ビルドの経路（実 Quarto・実 npm、梱包は模擬）で、拡張のフォルダに表示がコピーされ、
    VSIX の表示が ZIP のものと一致
  - 実 vsce（ddq-template で `npm run package:win32-x64`。debug の exe と表示をコピーして）: **14 件**（従来の 13 件＋
    `extension/THIRD-PARTY-NOTICES.md`）。表示はリポジトリ直下とバイト一致（449,346 バイト）、exe も一致、manifest の版
    3.1.0-pre1・TargetPlatform win32-x64。作ったファイルは消した（もともと無かった）。配布候補一式での確認は下の「統合の検証」
  - 変異: 5 か所を 1 つずつ壊し、5 つとも落ちることを確認（表示を拡張にコピーしない［旧］→ 実ビルドの試験、VSIX の表示を
    確かめない［旧］→ 表示の無い VSIX・表示が違う VSIX の試験、有るかだけ見て中身を比べない・改行を揃えて比べる → 表示が違う
    VSIX の試験、ビルドしたときだけ確かめる → `--no-build` の 3 試験）。終了後に `release.rs` の SHA-256 が変異前と一致
  - clippy・`cargo fmt --check` 成功
- **利用者に見える差**: ddq-template の VSIX に `THIRD-PARTY-NOTICES.md` が入る（VSCode の拡張のフォルダに置かれる）。
  拡張の動作は変わらない。保守者は、REL-01 より前に作った VSIX を `--no-build` で使えない（作り直す）
- **計画からの逸脱**: なし（両資産のファイル一覧差をリリースノートに書くのは保守者の公開時の作業。差は 1 ファイルの追加）
- **戻す方法**: `build_extension` のコピーと `check_bundled` の表示の照合を消し、`.gitignore` の行を消す
- **残存リスク**
  - 表示の本文は「この配布物（quarto-template）…（同梱の LICENSE）」と ZIP を前提に書かれている。VSIX では拡張自身の
    `LICENSE.txt` がそれに当たる（生成物の文面は計画どおり変えていない）
  - 開発者が `ddq release` を通さずに `npm run package:win32-x64` すると表示は入らない（exe と同じ。CI の vsix-smoke もこの形で、
    CI の配布候補の作り方は CI-01）。→ `810249b` で、CI の VSIX は `ddq release` が作るもの（表示が入る）になった
  - ddq.exe を同梱しない拡張（通常の VSIX）には入れない（計画は対象指定 VSIX）

### CI-02　配布物に入る依存を監査の必須の対象にする

- **状況**: 完了
- **変更ファイル**: 追加 `extensions/ddq-template/scripts/check-audit.mjs`（VSIX に入らない。`.vscodeignore` の `scripts/**`）・
  `extensions/ddq-template/src/checkAudit.test.ts`（11）。変更 `extensions/ddq-template/package.json`（`check:audit` をこのスクリプトに）、
  `.github/workflows/extension.yml`（必須の監査の名前と説明）、`extensions/ddq-template/AGENTS.md`（「検査を形骸化させないこと」）
- **調べたこと**: 拡張の依存はすべて devDependencies で、lock に dev でないパッケージは 0。従来の `npm audit --omit=dev` は
  **何も見ていなかった**。拡張本体のバンドル（`out/extension.js`）のソースマップに node_modules は無く、Webview を一時フォルダに
  ソースマップつきでビルドすると、入るのは react・react-dom・scheduler の 3 つ（両画面とも）。`cli/tools/third_party.py` の
  `EXTENSION_BUNDLED` と一致
- **規則**
  - 必須の対象 = Webview のバンドルに実際に入るパッケージ（検査のたびに Webview を一時フォルダにビルドしてソースマップから数える）
    ∪ lock で dev でないパッケージ。この中の low 以上の指摘が 1 件でもあれば exit 1（従来の `--audit-level=low` と同じ重さ）
  - 数えたバンドルの中身が `EXTENSION_BUNDLED`（配布物のライセンス表示の一覧）と違えば exit 1（表示が古い）
  - 開発用だけの依存の指摘は「参考」として表示し、exit 0（開発用の依存の扱いは保守者が別に決める。CI の参考の監査
    `npm audit || true` は残した）
  - `npm audit` はオフライン設定（`npm_config_offline`）でも通信させる（オフラインだと何も調べずに 0 件を返すため）。失敗したら exit 1
- **試験と結果**
  - 模擬（`src/checkAudit.test.ts`。`npm audit` は呼ばない）: 指摘なしは通る、react（high）・react-dom（critical）・scheduler（low）は
    失敗、開発用だけの vitest（critical）・@vscode/vsce（high）は参考で通る、配布物でも info は参考、本番依存ができればその指摘も
    失敗、`npm audit` の失敗（通信できない）は通さない、一覧と違うバンドルは失敗
  - 実ビルド（同じファイル。監査結果だけ模擬）: Webview を本当にビルドして 3 つを数える。一覧から 1 つ落とした third_party.py を
    与えると、実際のバンドルとの違いで失敗する
  - 実際の `npm audit`（registry.npmjs.org へ問い合わせ。advisories/bulk が 200）: 指摘 0 件で exit 0
  - vitest 全件 47 ファイル・491 件成功、`npm run typecheck` 成功
  - 変異: 7 か所（一覧と照合しない・バンドルを対象に入れない（旧）・npm audit の失敗を通す・開発用の指摘も失敗にする・info も
    失敗にする・dev の依存を本番依存に数える・バンドルを数えずに一覧を使う）を 1 つずつ壊し、すべて落ちることを確認。最初は
    「バンドルを数えずに一覧を使う」が素通りしたので、一覧を変えて実ビルドと照合する試験を足した
- **利用者（保守者）に見える差**: `npm run check:audit`（`verify` にも入る）が Webview をビルドするようになった（0.5 秒ほど）。
  React 一式に指摘が出たら CI の audit が落ちる（従来は落ちなかった）
- **計画からの逸脱**: なし
- **戻す方法**: `package.json` の `check:audit` を `npm audit --omit=dev --audit-level=low` に戻し、スクリプトと試験を消す
- **残存リスク**
  - 必須の監査は npm の勧告データベースに依る（今回は新しい指摘の有無を判定していない。計画どおり）
  - `EXTENSION_BUNDLED` は third_party.py の行を正規表現で読む。書き方を変えると読めずに失敗する（黙って通りはしない）
  - `docs/ddq-template-impl/` の解析記録（「本番依存の脆弱性検査」）は版付きの記録なので直していない（DOC-01 で索引に注記する）

### CI-01　配布検証の CI（`ddq release` に依らない部分）

- **追記（2026-11-22）**: 配布候補の流れは、REL-02 とあわせて下の「REL-02・CI-01　配布候補の作成を GitHub Actions に移す」で
  実装した。この節は `1051dbc` の時点の記録で、その後に変わった点は下の節に書く（vsix-smoke は消した）
- **状況**: **一部実施**。`ddq release` に依らない部分（Cargo の `--locked`、jar の照合、起動対象のパス）だけを入れた。
  Windows 配布候補の作成者を 1 つにし、main・手動で実リリース候補（ZIP＋対象指定 VSIX）を作って検査する部分は、`ddq release` を
  GitHub Actions などで置き換えるか（REL-02）が決まっていないため作っていない（REL-02 の結論を待つ。下の「判断」）
- **変更ファイル**: `.github/workflows/ci.yml`、`.github/workflows/extension.yml`
- **規則**
  - `cargo clippy`・`cargo test`・`cargo build --release`（ci.yml）と、実Host 用の `cargo build --release`（extension.yml）に `--locked`
  - ci.yml: キャッシュから戻したものも含め、`cli/vendor/plantuml.jar` の SHA-256 が `cli/vendor/README.md` に記録した
    `PLANTUML_VERSION` の値と一致しなければ失敗（版の記録が無くても失敗）
  - ci.yml の起動対象に、`cargo test` が読むリポジトリ直下・拡張のファイルを足した（push・PR とも `THIRD-PARTY-NOTICES.md`、
    PR に拡張の `package-lock.json` と改訂ファイルの golden。push は従来から `extensions/ddq-template/**` を含む）
  - Quarto はすでに全ジョブで 1.9.38 に固定されていた（変更なし）。Node 20/22・Linux の必須 Pandoc・Windows golden の許容差・
    audit・Mermaid の lock からの再生成照合はそのまま
- **試験と結果**（CI はここでは走らせられないので、手元で同じ手順を通した）
  - jar の照合の `run` を ci.yml から取り出し、Actions と同じ `bash --noprofile --norc -eo pipefail` で実行: 検証済みの jar は exit 0、
    記録の無い版（1.2026.9）は「（記録なし）」と出して exit 1、1 バイト足した jar は exit 1
  - 2 つの workflow を YAML として読めること、ジョブと手順の数が変わっていないこと（ci.yml: cli 17（＋1）・mermaid-bundle 6・
    vsix-smoke 10、extension.yml: test 9・host 10・audit 5・package 6）を js-yaml で確認
  - `cargo … --locked` は手元の全件・ビルドで従来から使っている形（計画 §6）
- **計画からの逸脱**: 上の「配布候補の流れ」を作っていない（保留）。起動対象のパスに docs/manual・examples/plantuml は足していない
  （今の CI はどちらも読まない。配布候補の流れを作るときに足す）
- **判断**（2026-11-22 の指示）: 配布候補の流れ（Windows 1 か所で exe→VSIX→ZIP を作り、同じ artifact を検査に渡す）は、
  今の `ddq release` を CI から呼ぶ形では作らず、**REL-02 の結論**（`ddq release` を GitHub Actions などに置き換えるか）を待つ。
  作るときは、通常 VSIX の artifact（extension.yml の package）の利用者の確認も要る（計画）
- **残存リスク**: workflow の変更は PR #142 で初めて GitHub 上で走った（ci の `cli` は成功。結果は「統合の検証」）。vsix-smoke は PR では走らず、まだ一度も走っていない。vsix-smoke の VSIX は `ddq release` を通さないので
  ライセンス表示が入らない（REL-01 の残存リスク）。（訂正）vsix-smoke は main への push では走って成功していた（この PR の CI で
  走らなかっただけ。検討書 §2 の 4 で訂正済み）。`810249b` で release-candidate.yml の verify-vsix に移して消した

### DOC-01　資料の索引と、実装と食い違う記述

- **状況**: 実施（配布用 manual の PDF・HTML の確認は下の「統合の検証」）。旧資料の移動・削除はしていない
- **変更ファイル**
  - `docs/README.md`: 索引に用途・読む人・対象版と基準の時点・後継先と旧パスを足した。単一ファイルの文書（検討書・記録・計画）に
    種別と後継先、検討書で決めていまも有効な判断（理由・採用版・参照先）、docs の外の文書（`cli/DESIGN.md`・拡張の開発文書・
    `examples/README.md`・`migration/README.md`）の表を足した。消した旧パスには消したコミットを付け、`git show <コミット>^:<パス>` で読めるようにした
  - `README.md`: PlantUML の「同梱 jar」を「各自で取得した jar」に（3.0.0 で同梱をやめたことと矛盾していた）
  - `cli/DESIGN.md`: §2 決定 6 と §11 の「一括事前変換は保留」を、§5.5 の現行（2.4.4 から一括）に合わせ、当初の判断は経緯として残した。
    §8 を「案」から現在のソースの構成に（`rev.rs`・`plantuml.rs`・`prerender.rs`・`merman_engine.rs`・`gitsrc.rs`・`diff.rs`・
    `revfile.rs`・`replace.rs`・`zip_archive.rs`）。§13.1 の「リリース一式（ddq + jar）」を、jar は各自で取得する形に。
    §3.2・§7.5・§7.6・§14.5 の更新は REL-03・REL-01・CI-01・IO-01 の記録に含めたもの
  - `docs/design/chapters/03-pipeline/01-overview.qmd`: 「分岐は `FORMAT` 判定 1 か所」を、分岐は `design-doc.lua` の中に閉じているが
    `FORMAT` を見るのは処理ごと（図の幅・mermaid・表・IPO 図・相互参照・目次。5 か所の判定と、それを使う 10 か所あまり）に
  - `docs/manual/chapters/11-publisher-env/05-offline.qmd`: 閉域へ持ち込むものに Java と `plantuml.jar`（LAN のサーバなしで PlantUML を
    発行する人だけ。どちらも配布物に入っていない）
  - `docs/manual/chapters/18-maintainer/06-release.qmd`: 失敗の保証を「集める途中の失敗だけ」に絞り、入れ替え後の zip 作成・VSIX の
    コピーでの失敗を明記。3.0.0 で同梱をやめた jar の失敗例を「はじめかた.pdf を作れない」に（REL-01・REL-03 の追記と同じ節）
  - `extensions/ddq-template/tableEditor-requirements.md`: 現ツリーに無い `AUTHORING.md` の参照 3 か所を利用マニュアル 9 章に
  - `extensions/ddq-template/DEVELOPMENT.md`: `check:audit` の説明を CI-02 の後の対象（配布物に入る依存）に
- **計画 §3 の各行への対応**
  - manual・README・AGENT-GUIDE: 上の 3 つ。`AGENT-GUIDE.md` と拡張の `README.md` は jar・失敗の保証について現行と矛盾が無く、変えていない
  - design・`cli/DESIGN.md`: 上のとおり。§12.5 は見出しに対象（2026-09-19、ddq 2.0.0 vs 旧 bat 1.3.0）が付いているので変えていない
  - `cli-impl/`・`ddq-template-impl/`: 変えていない（本作業で節を書き換えていないので、基準の付け直しも要らない）。索引に、記載の
    件数・ハッシュは基準の時点の値で、基準より後の変更はソースと `cli/DESIGN.md` が正と書いた。
    `ddq-template-impl/05-build-environment.qmd` の「本番依存の脆弱性検査」は 3.0.0-pre4 の時点の説明で、現行は CI-02 の
    `scripts/check-audit.mjs` と `DEVELOPMENT.md`
  - 検討書: 索引の後継先に、`revision-study.md` §5.8 の js-yaml 案は採らず `revision/revfile.ts` の限定した読み書きであること、Q9・Q18 の
    別拡張は統合で変わったこと、`ddq-template-study.md` Q6 の段階計画・旧ディレクトリ図は検討時のものであること、
    `cli/plantuml-study.md` の jar 同梱は 3.0.0 でやめたことを書いた。本文は変えていない
  - 拡張の開発文書: 上の 2 つ
  - 記録（`test-coverage-review.md`・PR #122 の対応・分割表の検証）: 本文は変えず、索引に対象と時点、当時の CI の説明は現行と違うことを書いた
  - presentation・examples・migration: 索引に用途と読む人を載せた（改稿はしていない。計画の対象外）
- **試験と結果**
  - `docs/README.md` に書いたパス 35 個（コマンド例 1 つを除く）を照合。26 個は存在。無い 9 個は、消した旧パス 5 つ（`AUTHORING.md`・
    `ADVANCED.md`・`template/PIPELINE.md`・`docs/revision-impl/`・`docs/table-editor-impl/`）と、親を付けずに書いた下位のもの 4 つ
    （`*-impl/`・`parts/`・`examples/` の `plantuml/`、拡張の `AGENTS.md`。どれも親の下に存在）。旧パスは `git show 7151377^:AUTHORING.md`・`4fc52aa^:ADVANCED.md`・
    `e447a47^:docs/revision-impl/01-overview.qmd` で読めることを確かめた
  - 書き換えた記述は、それぞれソースと突き合わせた（§8 は `cli/src` の全ファイルと各ファイル冒頭の `//!`、`FORMAT` は `design-doc.lua` の
    該当行、採用版は `extensions/ddq-template/CHANGELOG.md`・タグ・各検討書）
  - 配布用マニュアルを release の exe で作り直した（PDF 136 ページ・HTML 20 ページ）。ZIP に入れたものが、本作業で直した 11 章・18 章を含む（保管の 3.1.0-pre1 と比べて中身が違う）ことを確かめた。テンプレート設計書も作り直した（下の「統合の検証」）
- **計画からの逸脱**: なし。旧資料は消していない（削除には個別の承認が要る。今回、削除の候補は挙げていない）
- **残存リスク**: 実装仕様書（`cli-impl/`・`ddq-template-impl/`）の本文は 3.0.0-pre4 の時点のままで、本作業の変更（IO-01 の置き換え、
  REL-01 のライセンス表示、REL-03 の版の照合、CI-02 の監査など）は入っていない。次に基準版を更新するときに反映する

### REL-02　入れ替えの途中で失敗した回の出力は捨てて作り直す、を明示する（暫定）

- **状況**: **一部（暫定）**。計画の提案（実行ごとの作業場所で全資産を準備・検査してから公開する、固定の `.building` を共有しない、
  同じ出力先への同時実行を止める、前回の一式を退避・復元する）は実装していない。保証の範囲を「入れ替えの途中で失敗した回の出力は
  配布せずに消して作り直す。前回の一式は守らない」とし、失敗時の表示と文書で明示した。退避・復元などの実装は、`ddq release` を
  GitHub Actions などで置き換えるかの判断とあわせて決める（保留のまま）。→ 2026-11-22、手元では作らず、正式に配る一式は CI が
  作ると決めた（下の「REL-02・CI-01　配布候補の作成を GitHub Actions に移す」）。手元の扱いはこの節のまま
- **判断**（2026-11-22 の指示）: 計画どおりなら実装まで要る（提案の欄、完了条件の「保証する範囲（失敗後の旧一式復元／読み手から見た切替）を
  決定」と失敗の種類ごとの旧資産のハッシュの検査、Phase 3d の完了条件「失敗時の旧資産保証」）。「捨てて作り直す」は計画の候補に無く、
  範囲を狭める変更なので判断を求めた。`ddq release` を置き換えるなら、手元に作り込む退避・復元は使われなくなる（Actions は毎回新しい
  環境で動き、落ちた回は何も公開しない。読み手から見て一度に切り替えるのも、リリースを下書きで作り、資産を全部載せてから公開すれば済む）。
  壊れるのは保守者の手元の `release/` だけで、失敗すれば exit 1 になる。このため「破棄を明示する」が選ばれた
- **変更ファイル**
  - `cli/src/commands/release.rs`: 前回のフォルダを消してから VSIX をコピーするまでを 1 つにまとめた。どこで失敗しても、残っている出力
    （その版のフォルダ・作業フォルダ・zip・VSIX）を並べた案内を先頭に付けて止まる（`discard_notice`）。前回の zip を消せないときは
    ファイル名を、VSIX のコピーの失敗ではコピー先を出す。何をどの順で消し・作るかは変えていない（VSIX のファイル名を取るのだけ
    入れ替えの前にした。拡張のフォルダから見つけたファイルなので失敗しない）
  - `cli/tests/release.rs`: +1（Windows だけ）
  - `cli/DESIGN.md` §3.2、`docs/manual/chapters/18-maintainer/06-release.qmd`: 保証の範囲と、失敗したときにすること
- **直した記述**: 本作業の `1051dbc`（REL-01・DOC-01）で書いた「入れ替えた後の zip の作成・VSIX のコピーでの失敗では、配布フォルダは
  新しい版、zip は無い状態で止まる」（`cli/DESIGN.md`・マニュアル 18 章）は、2 つの場合に当たっていなかった。前回の zip を消せない
  とき（計画の実測の条件）と、VSIX のコピーで失敗したときで、どちらも前回の zip が同じ名前で残り、正しい一式に見える
- **失敗する場所と残るもの**（変更の前後で同じ。変わったのは表示だけ）

  | 失敗する場所 | 残るもの |
  |---|---|
  | 版・ライセンス表示の照合、マニュアル・拡張のビルド、一式を集める途中 | 前回の一式がそのまま（従来どおり守る。表示は変えていない） |
  | 前回のフォルダを消す・作業フォルダの名前を変える | 前回のフォルダは消えたか一部欠け、今回の分は作業フォルダに残る。zip と VSIX は前回のもの |
  | 前回の zip を消す（他のプログラムが開いている。計画の実測） | フォルダは今回、zip と VSIX は前回のもの |
  | zip を作る | フォルダは今回、zip は無い（書きかけは消す）、VSIX は前回のもの |
  | VSIX をコピーする | フォルダと zip は今回、VSIX は前回のもの |

- **試験と結果**
  - 実 exe（`tests/release.rs` 17 件すべて成功。追加 1）: 1 回目で一式を作り、前回の zip を `FILE_SHARE_READ` だけで開いたまま
    2 回目を実行する。exit 1、「<zip> を消せません」、案内が出て、消すものにフォルダ・zip・VSIX が 1 行ずつ並び、入れ替え済みの
    作業フォルダは並ばない。表示は次のとおり（一時フォルダは `…` に略した）

    ```text
    Error: 前回の一式と入れ替える途中で失敗しました。前回の一式は守っていないので、この版の出力は前回と今回の分が混ざっています。次のものは配布せずに消してから、もう一度 ddq release を実行してください:
      …\out\quarto-template-3.1.0-pre1
      …\out\quarto-template-3.1.0-pre1.zip
      …\out\ddq-a-win32-x64-3.1.0-pre1.vsix
      …\out\ddq-b-3.1.0-pre1.vsix

    Caused by:
        0: …\out\quarto-template-3.1.0-pre1.zip を消せません
        1: プロセスはファイルにアクセスできません。別のプロセスが使用中です。 (os error 32)
    ```

  - 変異: 3 か所を 1 つずつ壊し、3 つとも落ちることを確認（案内を付けない、zip を消せないときにファイル名を付けない、
    残っていないものも並べる）。終了後に `release.rs` を変異前に戻した
  - `cargo fmt --check`・`cargo clippy --locked --offline --all-targets -- -D warnings` 成功。Rust 全件（計画 §6 の条件）: **224 件成功・失敗 0・ignored 0**（8 分 19 秒。release 17、ほかは統合の検証と同じ）
  - 配布用マニュアル（18 章を直した）: 写したマニュアルを debug の exe で `update` → `pdf` → `html`。PDF 136 ページ・HTML 20 ページで、直した段落は PDF の 129 ページと `chapters/18-maintainer/index.html` に出て、旧文は無い
- **利用者に見える差**: なし（`ddq release` は保守者だけが使う）。保守者には、入れ替えの途中で失敗したときの表示が変わる
- **計画からの逸脱**: 提案（実行ごとの作業場所・検査してから公開・`.building` を共有しない・同時実行の制御・退避と復元）と、完了条件の
  失敗の種類ごとの旧資産のハッシュの検査をしていない。保証の範囲は、計画の候補（旧一式の復元／読み手から見た切替）ではなく
  「捨てて作り直す」にした（2026-11-22 の指示）
- **戻す方法**: `run` の入れ替えから VSIX のコピーまでを囲んだクロージャと `discard_notice` を外す。文書の直しは、案内の表示が無くても
  当たる部分（上の表）は残す
- **残存リスク**
  - exit 1 と案内を見落として配ると、前回と今回の混ざった一式を配りうる（同じ名前の前回の zip は正しい物に見える）
  - 同じ出力先で 2 つ同時に動かすと、固定の `.building` を取り合う（計画の指摘のまま）
  - 試験は前回の zip を消せない場合だけ（Windows のロックの条件。CI の `cli` ジョブは Windows）。フォルダの名前の変更・zip の作成・
    VSIX のコピーでの失敗が同じ案内を通ることは、コードで確かめただけ
  - 下の「統合の検証」の release の exe のハッシュと配布候補は、この変更の前のもの。成功したときの出力は変わらないので、配布候補は
    作り直していない（成功の経路は `tests/release.rs` の、ZIP と配布フォルダのバイト一致などの試験が通す）

### REL-02・CI-01　配布候補の作成を GitHub Actions に移す

- **状況**: **検証済（PR・main への push・手動実行・わざと失敗させた実行を GitHub 上で確かめた。2026-11-23）**。実装は `810249b`。手元で workflow の手順を再現して
  全ジョブが通り、その push で PR の CI として GitHub 上でも全ジョブが通った（下の「GitHub 上の最初の実行」）。PR の起動対象は
  PR 全体の差分で判定されるので、この PR が開いている間は push のたびに走る
- **判断**（2026-11-22 の指示「推奨どおり」）: 検討書 `docs/release-ci-proposal.md` §2 をすべて推奨どおりに決めた。計画 §7 の表に
  1 行足した（本文のほかの箇所は変えていない）
  1. 公開は保守者の手作業のまま（CI はリリースにもタグにも書き込まない。計画 §7「自動公開は含めない」のとおり）
  2. 梱包は CI から今の `ddq release` を呼ぶ（版・ライセンス表示の照合、ZIP の UTF-8 名フラグと読み直しは試験済みのものを使う）
  3. 走らせるのは main への push と手動実行（PR で毎回走らせるかは、試走で所要時間を測ってから）
  4. ci.yml の vsix-smoke は新しい workflow に移して消す。extension.yml の通常 VSIX の artifact（`package`）は残す
  5. 正式に配る一式は CI の成果物だけ。手元の `ddq release` は試作・確認用（途中で失敗した回は捨てて作り直す。上の節）
- **REL-02 の保証の範囲**: 公開に使えるのは、すべての検査に通った 1 回の実行の `release-<版>` だけ。失敗した実行には
  `release-<版>` ができない。CI は公開済みの資産に触れない。読み手には、保守者が下書きのリリースを公開した時点で一式が一度に見える
- **変更ファイル**
  - `.github/workflows/release-candidate.yml`（新規）: 起動は main への push（配布物に入るもの: `cli/`・`template/`・
    `extensions/ddq-template/`・`docs/manual/`・`README.md`・`AGENT-GUIDE.md`・`LICENSE`・`THIRD-PARTY-NOTICES.md`、と書体の手順・
    workflow 自身）、手動実行（入力 `with_sample` で `--with-sample`）、workflow 自身・照合の処理・書体の手順を変えた PR。
    `permissions: contents: read`、同じブランチの実行は `concurrency` で 1 つずつ（打ち切らない）。ジョブは 4 つ
    - `build`（Windows）: `cargo build --release --locked` → その exe で `ddq release <RUNNER_TEMP>/out` → `check_release.py` で照合し
      SHA-256 の一覧を作る。使った版の一覧 `versions.txt`（コミット・実行・ランナーのイメージ・ddq・ddq.exe の SHA-256（完了レビューで
      追加）・Quarto・Node／npm・Edge・書体）も
      作る。ZIP・VSIX・2 つの一覧を `candidate`（保存 1 日）として渡す
    - `verify-zip`（Windows。別のランナー）: SHA-256 を照合し、`check_release.py --smoke` で ZIP を照合して、取り出した exe で
      `init --no-render` → `pdf` → `html` を通す
    - `verify-vsix`（Windows。別のランナー）: SHA-256 を照合し、配る VSIX そのもので `npm run test:smoke -- <VSIX>`（旧 vsix-smoke の役目）
    - `publishable`（Linux）: 3 つがすべて通ったら SHA-256 を照合し、同じ 4 つを `release-<版>`（保存は既定の 90 日）として残す
  - `.github/actions/jp-fonts/action.yml`（新規。composite）: `template/lib.typ` の本文・等幅の最初の候補（Meiryo UI・BIZ UDGothic。
    `check_release.py --expected-fonts` が lib.typ から読む）が `quarto typst fonts` に出るかを見て、無ければ Windows の日本語の
    追加フォント（`Language.Fonts.Jpan`）を dism で入れる。入れても見えなければ失敗する。`build` と `verify-zip` が使う
  - `cli/tools/check_release.py`（新規。標準ライブラリだけ）: `ddq release` の出力先を照合する。ZIP の CRC、全項目が
    `quarto-template-<版>/` の下、日本語名に UTF-8 名フラグ、exe は `ddq.exe` だけ、必須のファイル（ddq.exe・はじめかた.pdf・README・
    AGENT-GUIDE・LICENSE・ライセンス表示・manual の PDF と HTML）、ライセンス表示がリポジトリ直下と同じ、jar・VSIX が無い、`docs/` は
    `--with-sample` のときだけ、配布フォルダがあれば ZIP と名前・中身が同じ、`--exe` と ZIP の exe が同じ、2 つの PDF に求める書体が
    埋め込まれている。VSIX は CRC、exe とライセンス表示が ZIP と同じ、manifest の版と TargetPlatform。ほかに `--sums`（SHA-256 の一覧）、
    `--smoke`（取り出した exe で init → pdf → html と、その PDF の書体）、`--expected-fonts`。手元でも同じものを流せる
  - `.github/workflows/ci.yml`: vsix-smoke を消した。push の起動対象を pull_request と同じにした（`extensions/ddq-template/**` は
    vsix-smoke のためだった。`cargo test` が読む拡張のファイルは残る）。`cli` ジョブの `ddq-windows-x64` の artifact はそのまま
  - `cli/DESIGN.md` §3.2・§7.5・§7.6、利用マニュアル 18 章（`06-release.qmd`。「リリースを配る」を CI の成果物から取る手順に
    書き換え、`check_release.py` を足した）、`extensions/ddq-template/AGENTS.md`（vsix-smoke の記述）、`docs/README.md`（索引に検討書）、
    `docs/refactoring-plan.md` §7、`docs/release-ci-proposal.md`（新規。検討書）
  - `cli/src/commands/release.rs`・`cli/tests/release.rs`: コメントだけ（REL-02 の「保留中」を、正式に配る一式は CI が作る、に）
- **試験と結果**（手元。GitHub 上の実行は下の「GitHub 上の最初の実行」）
  - `check_release.py` を、統合の検証の検証用リポジトリの出力（`ddq release --no-build`）に流してすべて OK。作った一覧で
    `sha256sum -c` も OK。わざと食い違わせた 5 つ（`lib.typ` の本文の最初の候補を PDF に無い書体に、リポジトリ直下のライセンス表示を
    ZIP と違うもの（`url` の更新の後の版）に、サンプルの無い出力を `--with-sample` で照合、debug の exe を `--exe` に、配布フォルダの
    README に 1 行足す）は、それぞれ NG を出して exit 1。その後で直した照合の処理（表示の文言と `--expected-fonts`）は、下の再現で流した
  - workflow を js-yaml で読み、ジョブと手順の数を確認: ci.yml は cli 17・mermaid-bundle 6（vsix-smoke が消えた）、
    release-candidate.yml は build 12・verify-zip 6・verify-vsix 8・publishable 3。extension.yml は変えていない
  - **手元での再現**（作業ツリーの写し（追跡中と、無視しない未追跡のファイル）で、各ジョブの手順を順に流した。pwsh の手順
    （書体・使った版）は YAML から取り出し、GitHub と同じく `$ErrorActionPreference = 'stop'` を付けてそのまま実行。exe は
    同じ作業ツリーの release ビルド）: **全ジョブ成功**
    - 書体: Meiryo UI・BIZ UDGothic が Typst から見える（手元には入っているので dism の経路は通らない）
    - 使った版: ddq 3.1.0-pre1、Quarto 1.9.38、Node v24.18.0／npm 11.13.0（CI は Node 22）、Edge 158.0.4370.33
    - `ddq release`（`--no-build` なし。マニュアルの update → pdf → html、拡張の `npm ci` と梱包を含む）: 約 1 分で成功。
      配布用マニュアルは、書き換えた 18 章を含めてこの中で作られた
    - 照合: ZIP 50 件の CRC、日本語名 2 件に UTF-8 名フラグ、配布フォルダと ZIP が名前・中身とも同じ、ZIP の exe が入力の exe と同じ、
      はじめかた.pdf と利用マニュアルの PDF に MeiryoUI と BIZ-UDGothic（太字を含む）が埋め込まれている、VSIX 14 件の CRC・exe と
      ライセンス表示が ZIP と同じ・版 3.1.0-pre1・win32-x64
    - verify-zip: SHA-256 の照合、取り出した exe で init → pdf → html がすべて exit 0。init した文書の PDF にも同じ書体、HTML 8 ページ
    - verify-vsix: 配る VSIX そのもので `test:smoke` 成功（設定・PATH なしで同梱の ddq に決まる、init、html）
    - publishable: SHA-256 の照合 OK
  - **再現で見つけて直したこと**: `quarto typst fonts` は一覧を標準エラーに出すので、最初の版の書体の手順は一覧を受け取れず、
    書体が無いと判断して dism を呼んだ（手元は管理者でないので exit 740 で止まった）。`cmd /c "quarto typst fonts 2>&1"` で受け取る形に
    直し、pwsh の手順はどちらも出力を UTF-8 で読むようにした
  - `cargo fmt --check` 成功（Rust はコメントだけの変更なので、全件は流し直していない）。release の exe を作り直した:
    SHA-256 `da5b016e40518628147fbcdd40581a372a258595a4a817e159ed7edd50686ebc`
- **GitHub 上の最初の実行**（PR #142 への `810249b` の push。実行 45029242034、pull_request）: **全ジョブ成功**。同じ push の
  ci・extension の全ジョブも成功（13 チェックすべて）
  - 所要時間: 全体 38 分。`build` 26 分 18 秒（書体 15 分 21 秒、`cargo build --release` 8 分、`ddq release` 1 分 18 秒）、
    `verify-zip` 11 分 30 秒（書体 10 分 23 秒）、`verify-vsix` 2 分 35 秒（スモーク 1 分 1 秒）、`publishable` 11 秒。同じ push の
    ci の `cli` は 10 分 44 秒、実拡張ホストは 6 分 19 秒。時間の大半は書体を入れる手順
  - 書体: ランナー（win25-vs2026 20261116.315.1）には Meiryo UI・BIZ UDGothic が無く、`build`・`verify-zip` とも dism で
    `Language.Fonts.Jpan~~~und-JPAN~0.0.1.0` を入れる経路を通った（能力名は通り、exit 0）。入れた後は Typst から見えた
  - 照合: `build` と `verify-zip` の `check_release.py` がすべて OK。はじめかた.pdf・利用マニュアル・init した文書の PDF に
    MeiryoUI と BIZ-UDGothic（太字を含む）が埋め込まれている。別のランナーで SHA-256 が一致し、ZIP から取り出した exe で
    init → pdf → html（HTML 8 ページ）、配る VSIX そのもので `test:smoke` が通った
  - 使った版（`versions.txt`）: Quarto 1.9.38、Node v22.23.5／npm 10.9.9、Edge 157.0.4341.60。ddq.exe は SHA-256
    `3cefa937d9c4de19ee75b859bb715f5558b1772351dcab844b5207a0df8e29bc`（30,563,328 バイト）
  - 成果物: `release-3.1.0-pre1`（25,525,741 バイト、2027-02-20 まで）と `candidate`（1 日）。`publishable` は 3 つの検査の後に動いた
  - **`versions.txt` の `commit:` は PR のマージコミット**: pull_request の実行では、`GITHUB_SHA` と checkout が head の `810249b` では
    なく、GitHub が作る `refs/pull/142/merge`（`578c1ea`。main の `17ed32e` に `810249b` を合わせたもの）になる。main がまだ進んで
    いないので中身は `810249b` と同じ。main への push と手動実行では push したコミットなので、タグの手順（マニュアル 18 章）はそのまま
    使える。PR の実行にも同じ名前の `release-<版>` が残るので、公開には main の実行のものを使う（マニュアルの手順は `--branch main` で選ぶ）
  - 注記: `actions/*@v4`（checkout・cache・upload/download-artifact）に Node.js 20 の非推奨の警告が出た（失敗ではない。既存の ci.yml・
    extension.yml にも同じ警告がある）
- **CI の成果物と手元の出力の比較**（`release-3.1.0-pre1` を落として SHA-256 を照合し、上の手元の再現の出力と 1 件ずつ比べた）:
  見た目は同じ
  - ZIP 50 件のうち 42 件はバイト一致（manual の HTML 全ページ、README・LICENSE・AGENT-GUIDE など）。VSIX 14 件のうち 12 件が一致
  - mermaid の SVG 4 枚: 描画の本体（2 行目以降）はバイト一致。違うのは先頭のコメントの Edge の版（157.0.4341.60／158.0.4370.33）と
    フォントの指紋だけ。ランナーの Edge でも同じ図になったので、`DDQ_MERMAID_ENGINE` には切り替えない
  - PDF 2 つ: ページの中身（描画命令）は同じで、配置は変わらない。違うのは作成日時（ランナーは UTC）と、埋め込んだ書体の版。
    ランナーが入れた日本語の追加フォントは Meiryo UI 6.51・BIZ UDGothic 2.01、手元（Windows 11 26300）は 6.52・2.02。
    字幅の表（hmtx）は同じで、字形が違うのは BIZ UDGothic の `_` の 1 字だけ（3 バイト。外形の枠は同じ）
  - ddq.exe: ビルドした環境の違い（想定どおり）
  - `THIRD-PARTY-NOTICES.md`（ZIP・VSIX とも）: 改行だけが違い、LF にそろえると中身は同じ。CI の版は CRLF（Windows の checkout。
    ZIP に入る README・LICENSE・AGENT-GUIDE と同じ形）。手元の作業コピーが LF なのは、`2dc31e2` で `third_party.py` が作り直した
    ため（git の中身は同じ）
- **マージの後の実行**（main の `6bb9232`。#142 のマージ。2026-11-22〜23）
  - main への push（実行 45040063019）: 4 ジョブ成功。全体 39 分 40 秒（`build` 23 分 29 秒、`verify-zip` 15 分 55 秒、
    `verify-vsix` 2 分 13 秒、`publishable` 9 秒）。`release-3.1.0-pre1`（25,525,940 バイト）が残った
  - 手動実行を 2 回続けて起動した（2026-11-22 22:40 UTC。`with_sample` なしの 45079044069 と、ありの 45079051895）。どちらも 4 ジョブ成功
    - なし: ZIP 50 件・VSIX 14 件で、`docs/` は無い。全体 31 分 14 秒（`build` 19 分 51 秒、うち書体 14 分 7 秒）。
      `release-3.1.0-pre1` は 25,526,024 バイト
    - あり: `SAMPLE_ARG: --with-sample` が渡り、ZIP 128 件（`docs/` が有る）・VSIX 14 件。全体 36 分 20 秒（待ちを除く）。
      `release-3.1.0-pre1` は 25,576,533 バイト
    - 両方とも `build` と `verify-zip` の `check_release.py` が結果 OK。成果物の SHA-256 が、作ったジョブと検査したジョブで一致した。
      ZIP から取り出した exe で init → pdf → html が通り、配る VSIX で `test:smoke` が通った
    - `with_sample` の照合は `docs/` が有ることまでで、同梱したサンプルを CI では組まない（手元では完了レビューで組んだ）
  - `concurrency`: 2 回目は 1 回目の実行中ずっと pending で、1 回目の最後のジョブが終わった 2 秒後（23:11:52）に `build` が始まった。
    打ち切られずに順に走った。同じブランチへの push も同じ group なので、同じ動きになる
  - `versions.txt` の `commit:` は `GITHUB_SHA` で、手動実行の head は API で見て `6bb9232`（成果物の中のファイルは開いていない）
- **わざと失敗させた実行**（2026-11-23。main の `6bb9232` から作った検証用の枝 2 本を手動実行した。マージしない。検討書 §4 の
  「版をずらした検証用の枝など」）
  - 作成で失敗（`ci-test/rc-fail-build` の `27030fa`。拡張の `package.json` の version だけを 3.1.0-pre9 にした。実行 45097153978）:
    `build` の「ddq release」の手順が「版が揃っていません（template/VERSION = 3.1.0-pre1）。ビルドの前に止めました（何も作っていません）」
    と `package.json` の 3.1.0-pre9 を挙げて exit 1（`build` は 12 分 52 秒。書体とビルドの後で止まる）。`verify-zip`・`verify-vsix`・
    `publishable` は skipped で、成果物は 1 つも無い
  - 検査で失敗（`ci-test/rc-fail-verify` の `d61078c`。`check_release.py` の `--smoke` の最後に、必ず NG になる 1 行を足した。
    実行 45097156955）: `build` と `verify-vsix` は成功し、`verify-zip` の「ZIP の照合と、取り出した exe での init → pdf → html」だけが
    失敗した（init → pdf → html は最後まで通り、NG はわざと足した 1 行だけ。結果 NG、exit 1）。`publishable` は skipped。成果物は
    `candidate`（1 日で消える）だけで、`release-3.1.0-pre1` は無い
  - 公開済みの v3.1.0-pre1: 失敗させる前（01:54 UTC）と後（02:41 UTC）で、タグ（`2742644`）、リリース（id 447651872、プレリリース、
    2026-11-16 公開）、2 つの資産の大きさ・SHA-256（GitHub の API の `digest`。`quarto-template-3.1.0-pre1.zip` は `99198098…`、
    `ddq-template-win32-x64-3.1.0-pre1.vsix` は `cc38daef…`）・更新時刻がすべて同じ。下書きのリリースも増えていない
  - 計画の失敗の種類（VSIX のコピー・ZIP・rename・並行・再実行）との対応: CI は作るのも検査するのも毎回新しいランナーで、公開済みの
    資産には書き込まない（`contents: read`）。失敗の場所は「作成」と「検査」の 2 つで確かめた。並行は上の `concurrency`。失敗した
    実行の Re-run は試していない（やり直しても、同じ `needs` で、検査がすべて通った回にだけ `release-<版>` ができる作り）
  - 検証用の枝 2 本は GitHub に残している（消すかは保守者が決める）
- **まだ確かめていないこと**: GitHub 上の流れでは無い（2026-11-23 に、`concurrency`・main への push・手動実行（`with_sample` を含む）・
  わざと失敗させた実行を確かめた）。失敗した実行の Re-run は試していない
- **計画からの逸脱**
  - 計画の REL-02 の提案（実行ごとの作業場所・`.building` を共有しない・同時実行の制御・退避と復元）は手元の `ddq release` には
    作らず、CI のランナー（毎回新しい）と `concurrency`、検査の後にだけ残す `release-<版>` で満たす形にした（計画 §7 の追記で承認）
  - 完了条件の「失敗の種類ごとの旧資産のハッシュ」は、作成の失敗と検査の失敗の 2 つで確かめた（上の「わざと失敗させた実行」）。
    手元の `ddq release`（いまは ddq-release）の、VSIX のコピー・ZIP・rename の失敗ごとの検査はしていない（正式な配布を CI だけに
    したため）
  - workflow 自身・照合の処理・書体の手順を変えた PR でも走らせる（最初の試走のため。計画 CI-01 の「PR は高速試験」の例外。
    検討書 §6）
- **戻す方法**: `release-candidate.yml`・`.github/actions/jp-fonts/`・`check_release.py` を消し、ci.yml の vsix-smoke と push の
  起動対象を `098aa3a` に戻す。利用マニュアル 18 章の「リリースを配る」を、マージ後の main から手元で作り直してタグを打つ手順に戻す
- **残存リスク**
  - GitHub 上では PR の実行が 1 回成功しただけ（上の「まだ確かめていないこと」）。
    → マージの後に、main への push 1 回と手動実行 2 回も成功し、わざと失敗させた 2 本も思ったとおりに止まった
    （上の「マージの後の実行」「わざと失敗させた実行」）
  - 1 回 38 分のうち 25 分あまりが、2 つのジョブで日本語の追加フォントを入れる時間。PR で毎回走らせるのには向かない
    （検討書 §2 の 3 のとおり main と手動だけにし、PR は workflow などを変えたときだけ）
  - CI の PDF には、ランナーが入れたフォントの版が埋め込まれる（今回は手元より 1 つ古い版で、字形の差は 1 字）。ランナーの
    イメージや Windows の更新で版が変わりうる（`versions.txt` には書体の名前だけで、版は残らない）
  - ci.yml の `cli` ジョブは今も release の exe を `ddq-windows-x64` として残す（読むジョブは無くなった）。公開に使うのは
    `release-<版>` だけだが、取り違えうる。vsix-smoke が残していた `ddq-template-vsix-win32-x64`（ライセンス表示の無い VSIX）は
    無くなった。使っていた人がいれば `release-<版>` の VSIX に移ってもらう
  - 日本語のパスでの確認は手元に残る（ランナーは CP1252）
  - ランナーのイメージが更新されると、同じコミットでも Edge などの版が変わる（`versions.txt` に残す）
  - artifact は 90 日で消える。公開したリリースの資産が正の記録になる
  - 手元の `ddq release` は暫定の扱い（上の節）のまま。ddq.exe から `release` を外すかは、この流れが安定してから別に判断する（検討書 §5）。
    → 2026-11-23 に外した（保守者用の ddq-release。下の「ddq から release を外す」。手元の ddq-release の扱いは同じ）
  - 計画に残るもの: Phase 4 の人の手で行う検証（受入手順 3・4）、Phase 5／6 の完了レビュー（検討書 §8）

### GEN-02 の既知の差を直す（完了レビューの後。2026-11-23）

- **指示**: 2026-11-23「Yes, in this PR」。GEN-02 で印を付けて固定した既知の差のうち、手間の小さいもの（mermaid のタブ、
  CRLF の設定、表エディタの `merge-cols="ALL"`・`merge-cols=""`・旧 `.merge-rows`）を直す（指示の「この PR」は #142 だが、
  #142 はすでにマージされていたので、続きのブランチ `refactor/release-split-and-fixes` に入れた。冒頭参照）。PlantUML の `@start` の前の
  空白（見た目に差が無い）と、書式つきのセルの比較（手間が中くらい）は残す
- **状況**: 実装・検証済み
- **変更ファイル**
  - `cli/src/mermaid/prerender.rs`: `expand_tabs` を足した。`mermaid_blocks` は、各行のタブを Pandoc と同じく次の 4 桁目まで空白に
    広げてから字下げを測り、フェンスの字下げを外す（桁は行頭からの文字数。全角も 1 桁）
  - `template/design-doc.lua`（機構ファイル）: `mermaid_target` が `mermaid-config.json` の CRLF を LF にしてからキーに混ぜる
  - `extensions/ddq-template/src/tableEditor/formats/mergeCols/simulateMergeCols.ts`: `parseMergeCols` を design-doc.lua の `.tbl` の
    Div と同じ読み方にした（無い・空は結合しない、`all` は小文字そのままだけ、数字の無い値は解釈できない）。戻り値は
    `none`・`invalid`・`merge` の 3 通り
  - `extensions/ddq-template/src/tableEditor/markdown-document/findEditTarget.ts`: `.merge-rows` を見る分岐を外した。解釈できない値は
    警告する（「結合しないものとして扱います」。`widths` の警告と同じ形）
  - 期待値と試験: `cli/tests/fixtures/contract/diagrams.json`（タブの例の `rustDiffers` と CRLF の例の `luaWindowsOnly` を外し、
    行の途中のタブと字下げしたフェンスの中のタブの例を 1 つ足した。期待値は Quarto 1.9.38 同梱の Pandoc の `-t json` と hashlib）、
    `cli/tests/filter_contract.rs`（Windows 以外で CRLF の例を飛ばす分岐を外した）、`merge-cols.json`（3 例の `tsDiffers` を外した）、
    `contract.test.ts`（`warns` の例で警告が出ることも見る）、`mergeCols.test.ts`（`parseMergeCols` 4 件）、`findEditTarget.test.ts`
    （3 件。結合せずに開き、書き戻しても `merge-cols` を足さない）、期待値の README の印の表
  - 資料: `docs/ddq-template-impl/parts/table-editor/07-issues-unknowns.qmd` の「merge-cols の不正値」「旧 `.merge-rows`」の 2 行に、
    直したことを注記した（調査時点の記述は残した）
- **試験と結果**
  - 単体（Rust）: prerender 5/5。変異 4 か所（文字でなくバイトで数える、広げない、いつも空白 4 つ、広げる前に字下げを測る）の
    すべてで共有の例の試験が落ちる。初めの例（「受注」の 2 文字）ではバイトで数える変異が落ちなかった（2 文字で 2 バイトずつ多く、
    合わせて 4 桁ずれるのでタブの行き先が同じになる）ので、例を「利用者」の 3 文字に替えた
  - 実生成（Lua）: `filter_contract.rs` の mermaid の試験が、足した例を含めて本物の design-doc.lua で成功。Windows の文字モードは
    元から LF にするので、ほかの OS の読み方を `read_file` を一時的にバイナリモードにして再現した: 直したフィルタは成功し、
    `gsub` を外すと CRLF の例が「置いた SVG を使わなかった」で落ちる
  - TS: 変異 5 か所（大文字小文字を無視する、空を全列とみなす、`.merge-rows` を見る、警告を出さない、結合しないときも警告する）の
    すべてで落ちる。直す前の判定に戻すと、足した書き戻しの試験 3 件が落ち、書き戻した見出しに `merge-cols="1,2"` が付くことを確かめた
  - 実CLI（試験には入れていない）: `ddq init --no-render` で作った執筆フォルダの 1 章に、タブを含む mermaid の図
    （`\tA[利用者]\t--> B`）を足して `ddq html`（`DDQ_MERMAID_ENGINE=merman`）。直す前の release の exe では `diagrams` に SVG が 6 枚
    （prerender が雛形の 4 図とタブの図をまとめて 5 枚描き、フィルタがタブの図をもう 1 枚描いた）、直した debug の exe では 5 枚。
    HTML が参照する図の名前は両方で同じ 5 つ（直す前の余分な 1 枚はどこからも参照されていない）。直した exe が書いた
    `design-doc.lua` はリポジトリのものとハッシュが一致
  - 本物のフィルタでの確認（試験には入れていない）:`"ALL"`・`"all "`・`"none"` は警告して結合しない、`""` は警告せず結合しない。
    `" all"` のように引用の中が空白で始まる値は、Pandoc がその行を div の見出しと読まない（段落になる）ので Lua の判定まで届かない
  - Rust: `cargo fmt --check`・`cargo clippy --locked --offline --all-targets -- -D warnings` 成功。全件（計画 §6 の条件）: **224 件成功・失敗 0・ignored 0**（試験の時間の合計 8 分 11 秒。件数は前と同じ。足した例は既存の試験の中で回る）
  - 拡張: `npm run typecheck` 成功、`REQUIRE_PANDOC=1 npm test` **47 ファイル・496 件成功**（前の 491 件から、`parseMergeCols` 2 件と書き戻しの 3 件が増えた）、`build`・`check:offline` 成功
- **計画からの逸脱**: GEN-02 の「出力を変えない」の外（2026-11-23 に承認）。変わるもの:
  - prerender のキー（タブを含む mermaid の図だけ）。フィルタのキー・図のファイル名・発行物は変わらない。先回りで描いた SVG を
    フィルタが使うようになり、1 図ずつの描き直しが無くなる。前にタブのままのキーで描いた SVG は使われずに残る（版を上げたときの
    `ddq update` が消す）。キャッシュの移行は要らない
  - フィルタのキー: Windows では変わらない（文字モードで元から LF）。ほかの OS で CRLF の設定を使うときだけ、Windows・prerender と
    同じキーになる
  - 表エディタ: `"ALL"`・`""`・旧 `.merge-rows` の表を、文書の出力と同じく結合せずに開く。前は全列の結合を展開して開き、そのまま
    書き戻すと `merge-cols="1,2"` のような指定が付いて、文書でも結合されるようになっていた。いまは書き戻すと、解釈できない値・空の
    `merge-cols` は見出しから消える（`widths` の不正値と同じ扱い。分割表は div の見出しに触れないので残る）。`.merge-rows` の
    クラスは残る
- **残存リスク**
  - design-doc.lua は機構ファイルなので、既存の執筆フォルダは `ddq update` が要る。版を上げる必要は前からある（完了レビューの
    「次のリリースの前に」）
  - prerender は `preserve-tabs`・`tab-stop` の設定を読まない（既定の、4 桁で広げる、を前提にする）。これを設定した文書では、
    タブを含む図だけ先回りの SVG が使われず描き直す（直す前と同じ）
  - 残した差: PlantUML の `@start` の前の空白（Rust はキーを作らないので図・図名に差が無い）、書式つきのセルの比較（表エディタの
    表示だけ）。どちらも期待値の印は残っている
  - 引用の中が空白で始まる属性の値（上）は、Pandoc が div と読まないのに表エディタは `.tbl` として開く。別の既存の差で、直していない

### ddq から release を外す（保守者用の ddq-release。完了レビューの後。2026-11-23）

- **指示**: 2026-11-23「つづいて ddqからreleaseを外しましょう。これに合わせて、実装設計書を更新してください。」。
  置き場所は「保守者用の別 exe（推奨）」（同じ `cli/` パッケージの 2 本目のバイナリ `ddq-release`。配る ddq.exe を `--ddq` で受け取り、
  既定はその隣の ddq.exe。利用マニュアルはその ddq.exe を呼んで作る）、実装仕様書の更新は「release を外した分だけ」（検討書 §5 の判断）
- **状況**: 実装・検証済み
- **変更ファイル**
  - `cli/src/bin/ddq-release/main.rs`（新規）: clap の `Cli`（`out-dir`・`--with-sample`・`--no-build`・`--ddq <PATH>`）と呼び出し。
    `quarto.rs` は ddq と同じファイルを `#[path]` で読む（`render` は使わないので `dead_code` を許す）。Cargo.toml は変えていない
    （`src/bin/<名前>/main.rs` を Cargo が 2 本目のバイナリとして拾う）
  - `cli/src/commands/release.rs` → `cli/src/bin/ddq-release/release.rs`（`git mv` してから直した）: `default_ddq`（自分と同じフォルダの
    `ddq.exe`）、`run` が配る ddq.exe の有無を確かめ、`ddq_version`（`--version` を読む）を版の照合に足す。マニュアルの update → pdf → html は
    配る ddq.exe を子プロセスで呼ぶ。配布フォルダと同梱の VSIX に入れる exe、`check_bundled` の比べる相手を、自分自身から配る ddq.exe に替えた。
    「はじめかた」の原稿と名前（`RELEASE_GUIDE_TYP`・`RELEASE_GUIDE_PDF`）を `assets.rs` からここへ移した。表示は「ddq-release」に
  - `cli/src/zip_archive.rs` → `cli/src/bin/ddq-release/zip_archive.rs`（中身は変えていない）
  - `cli/src/main.rs`・`cli/src/commands/mod.rs`・`cli/src/assets.rs`: `Release` のサブコマンド・`ReleaseArgs`・`mod zip_archive`・
    `pub mod release`・上の 2 つの定数を外した
  - `cli/tests/release.rs`: `CARGO_BIN_EXE_ddq-release` を起動する。足した試験 3 件（`a_missing_ddq_stops`、
    `the_ddq_to_ship_must_report_the_same_version`（ddq でない exe を `--ddq` に渡すと版の照合で止まる）、`ddq_no_longer_has_release`
    （ddq の `release` は終了コード 2、`--help` に出ない））。`zip_matches_the_stage_byte_for_byte` は配布フォルダの `ddq.exe` が配る ddq.exe と
    同じことも見る。期待する表示を ddq-release の言葉に替えた
  - `.github/workflows/release-candidate.yml`: build のビルドの手順名と、一式を作る手順
    （`cli/target/release/ddq-release.exe "$RUNNER_TEMP/out" --ddq cli/target/release/ddq.exe $SAMPLE_ARG`）。手順の数は変わらない
    （build 12・verify-zip 6・verify-vsix 8・publishable 3）
  - コメントと説明: `extension.yml`、`cli/tools/check_release.py`、`cli/tools/third_party.py`、`cli/tests/third_party.rs`、
    `template/release-guide.typ`、`.gitignore`
  - 資料（現行のもの）: `cli/DESIGN.md`（決定 1 の注記、§3.1・§3.2、§4 のコマンドの一覧・呼び出し関係、表から release の行を外して
    「保守者用の ddq-release」の節、§7.4〜§7.6、§8 のコード構成、§10）、利用マニュアル 17 章・18 章（`tbl-release-opts` に `--ddq` の行）、
    `README.md`、`docs/README.md`、`examples/README.md`、`migration/README.md`、拡張の `AGENTS.md`・`DEVELOPMENT.md`、テンプレート設計書
    （`docs/design/` の「拡張の作法」）、検討書 §5 に決定の注記
  - 実装仕様書（release を外した分だけ）: `docs/cli-impl/` の 1〜7 章の配布の箇所と根拠 E-0013 の注記・E-0037（新規）、
    改版の表に 1 行（**本書はこの分離だけを反映した**こと、3.0.0 から後の他の変更と plantuml.jar を同梱しなくなったことは反映していないこと、
    `analysis/source-sha256.json` は基準のままであることを書いた）。`docs/ddq-template-impl/` の同梱の箇所（1・5・6 章、改訂履歴の 6 章、
    根拠の追補）と「統合後に変えた処理」の表に 1 行（拡張の処理は変えていない）
  - 変えていないもの: 過去の記録（検討書の本文、`docs/studies/`・各 study、変更履歴、実装仕様書の過去の観測 E-0027・E-0031 など、
    `analysis/validation.md`）。当時の `ddq release` のまま残した
- **試験と結果**
  - 実CLI（`cli/tests/release.rs`。模擬の梱包 `pack.js` と、Windows では実ビルドの経路を含む）: 20 件成功（足した 3 件を含む）。実ビルドの経路で、
    マニュアルの update → pdf → html を配る ddq.exe（ddq-release の隣の ddq）を子プロセスで呼んで作ることを確かめた
  - 変異 5 か所（試験には入れていない）: 配る ddq.exe の版の照合を外す → 2 件落ちる、配布フォルダに配る ddq.exe でなく ddq-release 自身を入れる →
    `zip_matches_the_stage_byte_for_byte` が落ちる、配る ddq.exe の有無を見ない → `a_missing_ddq_stops` が落ちる、`check_bundled` の比べる相手を
    ddq-release 自身にする → `zip_matches…` が落ちる、`--version` が「ddq」で始まれば何でも通す → `the_ddq_to_ship…` が落ちる
  - Rust: `cargo fmt --check`・`cargo clippy --locked --offline --all-targets -- -D warnings` 成功。全件（計画 §6 の条件）: **228 件成功・失敗 0・
    ignored 0**（試験の時間の合計 8 分 11 秒）。前の 224 件から、release の試験 3 件と、ddq-release でも走る `quarto.rs` の単体 1 件
    （`output_dir_follows_the_project_and_the_profile`。`#[path]` で共有するので 2 つのバイナリで走る）が増えた
  - 実生成・実CLI（試験には入れていない）: 作業ツリーの写し（`%TEMP%\ddqint\relsplit-repo`。ここの未コミットの変更に触れないため）で
    `cargo build --release --locked --offline`（4 分 31 秒。`ddq.exe` 29,651,968 バイト、`ddq-release.exe` 1,760,768 バイト）→ ルートで
    `cli\target\release\ddq-release.exe <出力先>`（`--ddq` なし）が終了 0、55 秒。`check_release.py <出力先> --exe cli/target/release/ddq.exe --smoke <作業フォルダ>`
    がすべて OK（ZIP と VSIX の `ddq.exe` が配る ddq.exe と同じ、取り出した exe で init → pdf → html）。ZIP 50 件・VSIX 14 件の項目の名前は、
    分ける前の CI の配布候補（PR の実行の成果物）と同じ。同梱のマニュアルの 18 章は新しい記述（`ddq-release`・`--ddq` の行）
  - 資料のビルド（実生成）: 写しで `docs/cli-impl`・`docs/ddq-template-impl`・`docs/design` を `ddq update` → `ddq html`。どれも終了 0 で
    フィルタの警告なし。`cli-impl` の構成図（mermaid）に ddq-release の節が出て、E-0037 と `@sec-update-scope` へのリンクが解決する
  - 拡張: コードは変えていない（`AGENTS.md`・`DEVELOPMENT.md` の文だけ）ので、試験は流し直していない（上の GEN-02 の直しの 496 件が最後）
  - GitHub 上: まだ（コミット・push していない）
- **計画からの逸脱**: 計画に無い変更（検討書 §5 の「別の項目として決める」を 2026-11-23 に保守者が決めた）。**互換性が変わる**のは
  保守者の操作だけ:
  - `ddq release` は無くなった（未知のサブコマンドで終了コード 2）。保守者は `cli\target\release\ddq-release.exe` を使う。引数とオプションは
    同じで、`--ddq` が増えた
  - 発行者・執筆者・VSCode 拡張が使うコマンド（init / add / update / setup / html / pdf / diagrams / plantuml / tag / rev / mermaid）と
    その出力・終了コードは変わらない。配布の一式（ZIP・VSIX の中身と名前）も変わらない
  - 配る ddq.exe が、release を実行した exe そのものから、ddq-release が受け取った ddq.exe に替わった。CI は明示して渡す
- **残存リスク**
  - 保守者の手元に古い手順（`ddq.exe release`）が残っていると「unrecognized subcommand」で止まる。18 章と DESIGN §7.5 は直した
  - `zip` クレートは ddq-release だけが使うが、同じパッケージの依存なので ddq.exe のビルドでもコンパイルされ、`THIRD-PARTY-NOTICES.md` にも
    載ったまま（ddq.exe には入らない。表示が 1 つ多いだけで、足りないわけではない）
  - `quarto.rs` を 2 つのバイナリが `#[path]` で共有する。ddq-release が使わない関数は `dead_code` を許しているので、ddq 側で使わなくなった
    関数があっても警告が出ない（ddq 側は従来どおり警告が出る）
  - 実装仕様書は分離の分だけを直したので、配布工程の他の記述（jar の同梱、版の照合の詳細、入れ替え途中の失敗の扱い）は 3.0.0-pre4 の
    基準のまま。改版の表にそう書いた
  - GitHub 上の配布候補の流れは、この変更ではまだ走らせていない（PR に push すると、`release-candidate.yml` を変えているので走る）

### 既存の不具合 2 件を直す（tag list の panic・改訂履歴の表の逆斜線。完了レビューの後。2026-11-23）

- **指示**: 2026-11-23「作業中に見つけた既存の不具合 2 件（ddq tag list の panic、改訂履歴の表で逆斜線が TeX の命令になる）を
  修正してください。」（下の「計画外で見つけた既存の不具合」の 1・2）
- **状況**: 実装・検証済み
- **変更ファイル**
  - `cli/src/doc/labels.rs`: `candidate` は、候補の長さ（6・8・10・40 桁）を使い切ったら、6 桁の後ろに `-2`・`-3`… と使われていない
    最初の番号を付ける（前は `unreachable!` で panic して exit 101）。4 つ目までの候補は変わらない。番号つきの候補も `valid` を満たす
  - `cli/src/commands/rev.rs`: `note_cell` が修正内容を `escape_tex_commands` に通す。Pandoc が TeX の命令と読む逆斜線（後ろが文字。
    英字だけでなく漢字も命令になる）を `\\` にする。コード（同じ数の ` で閉じるもの）と数式（Pandoc の tex_math_dollars の規則。迷う形は
    数式とみなす側に倒す）の中と、エスケープの 2 字（`\\`・`\*` など）は変えない。後ろが数字・空白・行末の逆斜線は、もとから書いたとおりに
    出るので変えない
  - 試験: `labels.rs` の単体 2 件（同じ見出し 6 つ、書き戻した番号を避ける）、`rev.rs` の単体 2 件（16 例、セルの組み立て）、
    `cli/tests/tag.rs` の `many_identical_headings_get_distinct_labels`（一覧 → 書き戻し → 一覧で、すべてにラベルが付き重複の警告が無い）、
    `cli/tests/rev.rs` の `notes_with_backslashes_appear_as_written`（rev build の表を本物の Pandoc で読み、TeX の命令が無く、パス・コード・
    数式が書いたとおり。quarto が無ければその部分を飛ばし、`DDQ_E2E=1` なら失敗）
  - 資料: `cli/DESIGN.md` §14.4（番号を付けること）・§15.4（修正内容の扱い）
- **試験と結果**
  - 再現（直す前の exe）: 同じ見出し 6 つの文書と `migration/docs` で、`tag list` が exit 101（`labels.rs:50`）
  - 単体: `labels` 11 件、`commands::rev` 6 件が成功
  - 実CLI: `tests/tag.rs` 23 件、`tests/rev.rs` 16 件が成功（`DDQ_E2E=1`。Pandoc で読む部分も通った）
  - Pandoc の読み方（試験には入れていない）: Quarto 1.9.38 同梱の Pandoc 3.8.3 で、単体試験の 16 例の直す前と直した後を表のセルに入れて
    読んだ。直した後は 16 例とも `RawInline tex` が無く、コードと数式は直す前と同じ。直す前に TeX の命令になっていたのは 13 例
  - 変異: 9 か所（直す前の形 2 つ＝修正内容を直さない・伸ばしきったら panic、ASCII の英字だけを直す、エスケープの 2 字を 1 字ずつ読む、
    コードの中も直す、数式の中も直す、閉じる `$` の直後の数字を見ない、開く `$` の直後の空白を見ない、番号を 1 から付ける）がすべて落ちた。
    初めは「開く `$` の直後の空白を見ない」が落ちなかった（例の `$ \docs $` は閉じる側の空白でも数式にならない）ので、`$ \docs$` の例を
    足した
  - 実物（試験には入れていない。直す前の release の exe と、直した debug の exe）
    - `tag list migration/docs`: 直す前は exit 101、直した後は exit 0 で、番号つきの候補が 80 件
    - `ddq init --no-render` の文書で見出しを 1 つ改名し、`rev diff --write` の修正内容に
      ``C:\docs\a.qmd を直した。`C:\x` と $\alpha$ と C:\\keep と \文書`` を書いて `rev build` → `pdf` → `html`。直す前は HTML・PDF とも
      `C:.qmd を直した。…C:\keep と` で、`\docs\a` と `\文書` が消えた。直した後は HTML のセルが
      `C:\docs\a.qmd を直した。<code>C:\x</code> と（数式）と C:\keep と \文書`、PDF（PyMuPDF で読んだ文字）も同じ並び。コードと数式は
      直す前と同じに出る
  - Rust: `cargo fmt --check`・`cargo clippy --locked --offline --all-targets -- -D warnings` 成功。全件 **234 件成功**
    （前の 228 件から、上の 6 件が増えた。17 本、全体で 532 秒）
  - 拡張は変えていないので流していない
- **計画からの逸脱**: 計画に無い直し（完了レビューで「計画の外・別の判断」に挙げたもの）。番号つきの候補の形（`-2`・`-3`…）は新しく
  決めた（前は panic して、その文書の候補が 1 つも出なかった）
- **互換性**
  - `tag list`・`tag apply`・`rev diff`: 変わるのは、前に panic した文書（同じファイルに種類と文言が同じものが 5 つ以上）だけ。
    4 つ目までの候補と、ほかの出力は同じ
  - `rev build`: `history.qmd` は、修正内容の逆斜線の後ろが文字のところだけが変わる。表に出る文字は、前は消えていたところが出るように
    なるだけ（TeX の命令は HTML・PDF のどちらにも出なかった）。前の回避策（`C:\\docs`・`` `C:\docs` ``）は同じに出る。`history.qmd` は
    生成物なので、`ddq rev build` をやり直すまでは前の表のまま
- **残存リスク**
  - 数式の見分けは Pandoc の規則をなぞったもので、迷う形は数式とみなす。数式とみなした中の命令は前と同じく消える（前より悪くはならない）
  - 修正内容に TeX の命令をわざと書いていた場合は、文字として出るようになる（前も HTML・PDF には出なかった）
  - 消えた見出し・図表の名称（「箇所」の列）は原稿の Markdown のままで、直していない（原稿の見出しでも同じに出るため）
  - 番号つきの候補は、2〜4 つ目の桁と同じく、同じものどうしの文書の順で決まる。間に同じ見出しを足すと、後ろの書き戻していない候補が
    ずれる（前からの性質。書き戻したラベルは変わらない）

## 統合の検証（Phase 4）

### PR の CI で見つけた試験の OS 依存（直した）

ブランチを PR にしたところ、拡張の CI（ubuntu・Node 20／22）で 3 件が落ちた（Windows の手元ではすべて成功していた）。
どれも本作業で足した試験で、製品のコードの不具合ではなかった。

| 試験 | 原因 | 直し方 |
|---|---|---|
| `process.test.ts` 取消（kill）・`stopAll`（EXT-02） | `killTree` は Windows でだけ `taskkill /T` で木ごと止め、ほかの OS は子だけ（変更前と同じ）。試験は全 OS で孫の停止を待っていた | 孫の停止は Windows でだけ確かめる。ほかの OS は子が止まること・起動中の数が 0 に戻ることを確かめ、孫は試験の後で片づける |
| `provider.test.ts` 取り直しの応答が逆順（EXT-03） | 模擬の文書のパスが Windows の形（`C:\docs\revisions\rev-B.yml`）で、Linux の `path.basename` では改訂記号が取れず、確定済みの判定が常に偽 | 模擬の文書のパスをその OS の形（`path.resolve`）にする |

### 全件と配布候補（最終の作業ツリー。ブランチ `refactor/refactoring-plan`）

| 対象 | 種類 | 結果 |
|---|---|---|
| `cargo fmt --check`・`cargo clippy --locked --offline --all-targets -- -D warnings` | — | 成功 |
| Rust 全件（計画 §6 の条件） | 実CLI・実生成・契約・模擬 | **223 件成功・失敗 0・ignored 0**（8 分 45 秒）。unit 116 / commands 10 / contract 5 / e2e 5 / faults 8 / filter_contract 7 / golden 2 / mechanism 2 / plantuml 4 / release 16 / rerun 4 / rev 15 / rev_ops 4 / table_pagination 2 / tag 22 / third_party 1。基準の 168 件から 55 件増（足した試験） |
| `cargo build --release --locked --offline` | — | 成功。`ddq 3.1.0-pre1`、SHA-256 `94042ffbec9dbc8c9d954a80b6f88b82273c9afb3e98f90c7ff8826cf1bae4ec`（30,385,664 バイト） |
| 拡張 `typecheck`・`REQUIRE_PANDOC=1 npm test` | 模擬・実 Pandoc・実プロセス | 2 つの tsconfig とも成功。**47 ファイル・491 件成功**（基準の 38・376 から増）。上の OS 依存の直しの後も同じ |
| 拡張 `build`・`check:offline`・`check:audit` | — | 成功。監査の対象は react・react-dom・scheduler（Webview のバンドル 3・本番依存 0）、配布物に入る依存の指摘 0 件 |
| 実 Extension Host（`DDQ_HOST_REAL=1`、debug の ddq） | 実Host | **30/30 合格**（基準 25 から増） |
| 同梱版の VSIX（release の exe と表示を拡張にコピーして `package:win32-x64` → `test:smoke`） | 実Host | VSIX は 14 件・10.86 MB。展開した中身で 3/3 合格（設定・PATH なしで同梱の ddq に決まる、init、html）。コピーした exe・表示と VSIX は後で拡張のフォルダから消した（もともと無かった） |
| 配布用マニュアル（検証用リポジトリに写し、release の exe で `update` → `pdf` → `html`） | 実生成 | 成功。PDF 136 ページ、HTML 20 ページ、`.template-version` 3.1.0-pre1 |
| `ddq release <out> --no-build`（検証用リポジトリ。上のマニュアルと VSIX、同じ release の exe） | 実CLI | 成功。読み直して照合: ZIP 50 件の CRC、ZIP と配布フォルダの名前と中身がすべてバイト一致、日本語名 2 件に UTF-8 名フラグ、ZIP の exe と VSIX の exe がどちらも入力の exe と一致、ライセンス表示はリポジトリ直下・ZIP・VSIX の 3 つが一致（449,346 バイト）、ZIP に jar・VSIX が無い、ガイド・README・AGENT-GUIDE・LICENSE・manual がある。VSIX は 14 件の CRC、manifest の版 3.1.0-pre1・TargetPlatform win32-x64 |
| 保管の 3.1.0-pre1 との一覧の比較 | — | ZIP は 50 件で増減なし。中身が違う 12 件は、exe、本作業（README・manual 11 章・18 章とその PDF・search.json）、#141（manual 4 章）、はじめかた.pdf（作り直し）、mermaid の SVG 4 枚。SVG 4 枚は先頭のコメントの Edge の版（157 → 158）とフォントの指紋だけが違い、2 行目以降はバイト一致。VSIX は 13 → 14 件で、増えたのは `extension/THIRD-PARTY-NOTICES.md` だけ（REL-01）。中身が違うのは exe・拡張の JS 3 本・package.json（`check:audit` の行）|
| ZIP から取り出した exe（入力の exe と同じハッシュ）で、日本語のパス（`…\日本語 の検証\設計書`、執筆フォルダ `基本設計`）に `init --no-render` → `pdf` → `html` | 実CLI・実生成 | すべて exit 0。PDF 30 ページ、HTML 8 ページ（index と雛形の 7 章。GEN-01 の後の雛形の章がすべて出る） |
| サンプルの版間比較（`examples/docs` を写したリポジトリで、HEAD `17ed32e` の exe と新しい release の exe で順に `update` → `regress.py capture`、`compare docs`。両側とも内蔵 merman、`TYPST_PACKAGE_PATH` は Quarto 同梱の packages） | 実生成 | **141 比較一致・差分 0**（機構 5、中間 Typst 1、ページ画像 51 とページ数、PDF 本文 1、HTML 59、SVG 23）。機構は `postprocess-html.js` だけが新旧で違う（GEN-01）が、作った HTML 59 件は新旧でバイト一致（サンプルの章の並びの読み方は変わらない）。計画 §6 の基準と同じ件数 |
| PlantUML のサンプル（`examples/plantuml` の写し。新しい exe で `update` → `diagrams` → `pdf` → `html`、検証済みの jar でローカルの Java サーバ） | 実生成 | すべて exit 0。PDF 42 ページ、PlantUML のキャッシュ 25 図（計画 §6 の基準と同じ） |
| テンプレート設計書（`docs/design` の写し。新しい exe で `update` → `pdf` → `html`。DOC-01 で 03-pipeline を直した） | 実生成 | すべて exit 0。PDF 38 ページ |

### PR の CI（GitHub Actions）

本作業の workflow の変更は、この PR（ramsesyok/ddq-template#142）で初めて GitHub 上で走った。

| push | ジョブ | 結果 |
|---|---|---|
| 1 回目（`1051dbc`） | ci: `cli`（Windows。cargo audit・fmt・`--locked` の clippy・test・release ビルド、plantuml.jar の SHA-256 照合）、`mermaid-bundle` | 成功（CI-01 の jar の照合と `--locked` が GitHub 上で通った） |
| 1 回目 | extension: テスト（ubuntu・Node 20／22） | **失敗 3 件**（上の「PR の CI で見つけた試験の OS 依存」）。実拡張ホストはテストの後なので走らなかった |
| 1 回目 | extension: テスト（windows・Node 20／22）・脆弱性チェック（CI-02） | 成功 |
| 2 回目（`ed013ed`。試験の直しだけ） | extension: テスト 4 本（ubuntu・windows × Node 20／22）・脆弱性チェック・実拡張ホスト（CI で ddq をビルドし、`DDQ_HOST_REAL=1`）・VSIX の生成、ci: mermaid-bundle | 成功 |
| 2 回目 | ci: `cli` | 成功（記録の後に確認） |
| 3・4 回目（`115409d`・`6f37060`。記録だけ） | ci・extension の全ジョブ（vsix-smoke は PR では走らない） | 成功（extension の `115409d` の分は次の push で取り消され、`6f37060` で成功） |
| 5 回目（`5d76c67`。REL-02 の暫定） | extension 全ジョブ（実拡張ホストを含む）、ci: mermaid-bundle | 成功 |
| 5 回目 | ci: `cli` | **失敗**。`cargo audit` が `url` 2.5.8 の RUSTSEC-2026-0592 で止まった（下の「PR の CI で見つけた依存の脆弱性」） |
| 6 回目（`2dc31e2`。`url` の更新） | ci・extension の全ジョブ（vsix-smoke は PR では走らない） | 成功（ci の `cli` 19 分 23 秒、実拡張ホスト 11 分 28 秒）。`cargo audit` も通った |
| 7 回目（`098aa3a`。記録だけ） | ci・extension の全ジョブ | 成功 |
| 8 回目（`810249b`。配布候補の流れ） | ci・extension の全ジョブ、release-candidate の 4 ジョブ（workflow 自身を変えた PR として初めて走った） | 成功（13 チェック）。release-candidate は 38 分（上の「REL-02・CI-01」の「GitHub 上の最初の実行」） |

vsix-smoke は PR では走らなかった（main への push と手動実行だけ。`810249b` で release-candidate.yml の verify-vsix に移して消した）。

### PR の CI で見つけた依存の脆弱性（直した。計画外）

- **現象**: 5 回目の push（`5d76c67`）で、ci の `cli` が `cargo audit` で落ちた。`url` 2.5.8 の RUSTSEC-2026-0592
  （`Url::origin()` が入れ子の `blob:` URL でスタックをあふれさせる。2026-11-20 公表、2.5.9 で修正）。`url` は内蔵の mermaid 描画
  （merman-core 0.8.0-alpha.6）の依存で、ddq が直接は使っていない。本作業は `Cargo.lock` を変えておらず、4 回目までの `cli` は
  成功していたので、公表が脆弱性のデータベースに載ったのがその間だった。main も同じ `Cargo.lock`
- **直したこと**（2026-11-22 の指示で、この PR で直した）: `cargo update -p url`（`url` 2.5.8 → 2.5.9、`form_urlencoded` 1.2.2 → 1.2.3。
  ほかは変わらない）。`python cli/tools/third_party.py` で `THIRD-PARTY-NOTICES.md` を作り直した（版の表記 6 か所と入力のハッシュだけが
  変わり、本文は同じ。Rust 190・mermaid 66・拡張 3、本文 182 種類）
- **試験と結果**: 手元の `cargo audit`（advisory-db `248aea097`。RUSTSEC-2026-0592 を含む）で脆弱性 0 件、許可済みの警告 1 件
  （`ttf-parser` の unmaintained。従来から）。`cargo fmt --check`・clippy 成功。Rust 全件（計画 §6 の条件）: **224 件成功・失敗 0・ignored 0**（8 分 37 秒。1 回目はビルド中に一時フォルダを Windows がつかんでいて消せず（os error 32）コンパイルで止まったので、流し直した）。PR の CI（6 回目の push）も `cargo audit` を含めて全ジョブ成功
- **計画からの逸脱**: 計画の項目ではない依存の更新（版の更新だけで、コードは変えていない）
- **残存リスク**: 上の「全件と配布候補」の release の exe のハッシュと配布候補は、この更新の前のもの（exe は変わる）。配布候補は
  作り直していない。merman を通る図の描画は、全件のうち e2e・golden・release などの試験が通す

### 統合の検証でしていないこと

- 通常の `ddq release`（`--no-build` なし）をこのリポジトリで流すこと。`npm ci` で拡張の依存を入れ直すため、計画 §6 と同じく
  検証用リポジトリで `--no-build` からの梱包を確かめた。通常の経路は `tests/release.rs` の実ビルドの試験（実 Quarto・実 npm、梱包は模擬）が通す。
  → その後、作業ツリーの写し（手元の再現）と CI の新しい checkout で通常の `ddq release` を流した（上の「REL-02・CI-01」）
- `--with-sample` の ZIP、途中失敗からの再実行の手での確認（`tests/release.rs` が見る）。→ `--with-sample` の ZIP は完了レビューで
  手元で確かめた（下の「完了レビュー」）
- OS で通信を遮断した閉域端末、実 LAN の PlantUML サーバ、公開後の資産の再取得、実画面の IME・旧二拡張が同居したときの手操作（計画 §6 の未実施範囲と同じ）
- 配布候補の流れ（`release-candidate.yml`）の、わざと失敗させた実行と main・手動での実行（PR の実行は成功した。上の「REL-02・CI-01　
  配布候補の作成を GitHub Actions に移す」）。旧 ci.yml の vsix-smoke は PR では走らなかったので、この PR の CI では一度も走らせていない（消した）
  → 2026-11-23 に、main・手動での実行と、わざと失敗させた実行を確かめた（上の「REL-02・CI-01」）

## 完了レビュー（Phase 5／6）

2026-11-22、HEAD `f08dfa7`（基準 `17ed32e` からの 20 コミット）で行った。計画 §4 の各 ID の完了条件を、この記録の試験と差分に
突き合わせた。§2 の「固定する外部契約」は基準とのソースの差分と試験で照合し、§3・§5 の Phase 5 の判定（変更したものと索引・配布用
マニュアルの整合、利用者が変更を追跡できること）を確かめた。計画書の本文は §7 の 1 行のほかは変えていない（台帳はこの記録に置く）。

### ID ごとの照合

| ID | 完了条件（計画 §4 の要点） | 照合の結果 | 残り |
|---|---|---|---|
| QA-01 | 現行／旧版の JSON・stdout/stderr・exit・明示 update・Undo を固定し、fixture の更新理由と exe の版を記録 | 満たす（`cli/tests/fixtures/contract/`、旧版は `tag-list.2.4.4.json`、README に採取の手順と exe の版。exit 3 → update → setup の回復は実CLI、Undo は既存の実 Host の項目） | — |
| CLI-01 | 読込回数・大文書の実行時間、警告・相対パス・ラベル・diff の順序が一致 | 満たす（新旧の release exe の出力が同じ） | — |
| EXT-02 | 引数・cwd・PATH、UTF-8 のチャンク境界、exit 3、ENOENT、別フォルダの並行、取消時の子孫の終了、deactivate 後の出力 | 満たす（孫の停止は Windows だけ。計画の範囲） | — |
| GEN-02 | 出力を変えず、3 組の重複を同じ入力で検証 | 満たす（既知の差は期待値に印を付けた。そのうち 3 組は、レビューの後の 2026-11-23 の指示で直した。キャッシュの移行は要らなかった） | 残した 2 つ（PlantUML の `@start` の前の空白、書式つきのセルの比較） |
| EXT-03 | 応答順の逆転・close→reopen・未保存・設定変更・多ルート・deactivate 後を模擬と実 Host で | 満たす（多ルートと deactivate 後は模擬だけ。理由は節に記録） | ステータスバーの逆転（計画の対象外。表示だけ）。2026-11-23 の外部レビューは部分完了と判定（H-02・M-01・M-02）→ 同日に M-01・M-02 を直し、続けて H-02 も直した（末尾） |
| EXT-04 | 2 バンドルの配信・起動・メッセージ・CSP・旧データを実 Host と `check:offline` で | 満たす（逸脱 3 件を記録） | — |
| GEN-01 | 新規／既存／過去の Git 版、増分 HTML と再実行で章・付録が一致し、従来の fixture の出力が一致 | 満たす（過去の `_quarto.yml` 52 版、サンプルの版間比較 141 一致） | 版を上げていない（下の「次のリリースの前に」）。2026-11-23 の外部レビューは部分完了と判定（H-01）→ 同日に直した（末尾） |
| EXT-01 | 正式版／pre1／pre4／pre10／旧 `2.4`、同版の内容差、欠落・不正値を両方向で | 満たす（出荷済みの 30 版。ダイアログは模擬） | 版の違う ddq での案内を実 Host では見ていない |
| DATA-01 | Rust→TS、TS→Rust、二度目の保存で比較。自動一括移行は別承認 | 満たす（移行はしていない） | — |
| IO-01 | 同時変更・読み取り専用・ロック・再試行・改行・末尾改行・`--dry-run`、BOM の保持は承認 | 満たす（付け替えを拒まれたときはその場で書く例外を記録） | Unix の経路は未実行（CI の Rust は Windows だけ）。2026-11-23 の外部レビューは部分完了と判定（C-01・C-02）→ 同日に直した（末尾） |
| IO-02 | 二つの init/add の競合、開かれたファイルの update、同版の修復、版の変わる update | 満たす | 2026-11-23 の外部レビューは部分完了と判定（M-03）→ 同日に直した（末尾） |
| REL-03 | 重い処理の前に 4 つの版を照合。CI は同じ checkout のビルドを使い、配布検証の記録にコミットと exe の SHA-256 を残す | 満たす（手元）。照合と同じ checkout のビルドは `810249b`。exe の SHA-256 は `check_release.py` のログにだけ出て成果物に入っていなかったので、このレビューで `versions.txt` に足した（`ddq.exe sha256:` の行。ZIP・VSIX の exe がこれと同じことは次の手順の `check_release.py --exe` が確かめる） | GitHub 上は次の実行で確かめる |
| CI-02 | 模擬の監査結果で、配布対象が失敗し開発用だけの指摘は参考になる | 満たす | — |
| REL-01 | 両アーカイブの表示・ライセンス本文・同梱 exe・版・target を照合 | 満たす（手元と CI の両方） | 次のリリースノートに両資産のファイル一覧差 |
| CI-01 | 配布候補の作成者を 1 つにし artifact で渡す。PR は高速試験、main・手動は実候補の検査。既存の検査を保つ | 満たす（PR の実行で 4 ジョブ成功。ci・extension の既存ジョブは vsix-smoke 以外そのまま） | main・手動・`concurrency` はマージの後に確かめた（2026-11-23） |
| DOC-01 | リンク・旧 ID の注記・配布用マニュアルの PDF/HTML を確認。旧資料の削除は個別承認 | 満たす（削除はしていない）。拡張の開発文書の追随はこのレビューで直した（下の「資料の整合」） | 2026-11-23 の外部レビューは部分完了と判定（M-05・M-06）→ 同日にどちらも直した（末尾） |
| REL-02 | 保証する範囲を決め、失敗の種類ごとに旧資産のハッシュを検査 | **一部** → 2026-11-23 に満たす。範囲は決めた（計画 §7 に追記）。わざと失敗させた 2 本（作成・検査）で `release-<版>` が残らず、公開済みの資産のハッシュが変わらないことを確かめた | なし（失敗した実行の Re-run は試していない）。2026-11-23 の外部レビューは、手での公開の手順（C-03）で部分完了と判定 → 同日に直した（末尾） |

### 外部契約の照合（計画 §2）

- **プロセス**: `cli/src/main.rs` の clap の定義は基準から変わっていない（差分は `mod replace;` の 1 行）。終了コードを決める
  `main` も同じ（0・1・3、clap の 2）。人向けの目印の出現数は `"OK: "` 2・`"OK -> "` 2・`警告:` 9 で基準と同じ。JSON モードの stdout は
  QA-01 の契約試験が固定する
- **データ**: `tag list --json` などの形は QA-01 の fixture（旧版 2.4.4 を含む）と、CLI-01 の新旧 exe の比較で同じ。変わったのは承認した
  BOM の保持（IO-01）と、引用付きの値の読み方（DATA-01）
- **設定・環境**: 製品のソース（`cli/src`・拡張の `src`・機構の Lua と JS）が読む環境変数の名前は、基準と同じ 20 個
  （`DDQ_BIN`・`MERMAID_SVG`・`EXECUTABLE_BROWSER`・`DDQ_MERMAID_ENGINE`・`DDQ_DIAGRAM_CACHE`・`DDQ_BROWSER_TIMEOUT`・
  `DDQ_PLANTUML_SERVER`・`PLANTUML_SERVER`・`DDQ_PLANTUML_JAR`・`PLANTUML_JAR`・`DDQ_JAVA`・`JAVA_HOME`・
  `DDQ_PLANTUML_STARTUP_TIMEOUT`・`DDQ_VERSION` ほか）
- **VS Code**: `package.json` の差は `scripts.check:audit`（開発用）だけ。publisher/name・コマンド 8 件・設定 2 件・Custom Editor・
  `when` は変わらない。足した内部コマンド `ddqTemplate.internal.webviewReadies` は既存の `internal.*` と同じく manifest に出さない
- **出力と更新**: サンプルの版間比較 141 一致（統合の検証）。新規作成の雛形は GEN-01 で変わる（承認済み）。ZIP の UTF-8 名と
  VSIX の対象 OS/CPU は手元と CI の `check_release.py` が確かめる

### 資料の整合（Phase 5 の判定）

- 整合している: `docs/README.md` の索引（計画・この記録・検討書）、`cli/DESIGN.md`（§3.2・§7.5・§7.6、§8 の `replace.rs`、
  `cli/tools/check_release.py`）、利用マニュアル 11 章・18 章（配布用マニュアルの PDF・HTML は CI の `ddq release` の中でも作られた）
- **ずれ（このレビューで直した）**: 拡張の開発文書のソースの表（`AGENTS.md` の冒頭・`DEVELOPMENT.md`）に、EXT-02 の `src/ddq/launch.ts`
  （ddq の起動口。機能モジュールはここを通す）、QA-01 の `src/ddq/contract.ts`（ddq の JSON の検査）、EXT-04 の `src/webviewHtml.ts`
  （Webview の器と資産の範囲）が載っていない。`AGENTS.md` の「共通で守ること」「Webview セキュリティ」にも、この 3 つを使う規則と、
  Webview から受け取ったメッセージの対象を照合する規則が無い（計画 §3・DOC-01「AGENTS 追加は実装で必要なルールが生じた場合のみ」に当たる）。
  → 3 つを両方の表に足し、`AGENTS.md` の「共通で守ること」に起動口と JSON の検査、「Webview セキュリティ」に器と書き戻しの照合の
  規則を足した（どれも実装済みの規則を書いただけで、コードは変えていない）
- **ずれ（直した）**: この記録の CI-01 の節の「vsix-smoke は…まだ一度も走っていない」（main では走って成功していた）と、REL-01 の節の
  「CI の配布候補の作り方は CI-01」（`810249b` で解消）に注記を足した。「統合の検証でしていないこと」のうち、その後に済んだものに注記を足した
- **利用者が変更を追跡できるか**: いまはこの記録と PR だけ。`extensions/ddq-template/CHANGELOG.md` はリリースのコミットで版ごとに
  書く形で、この PR では書いていない（下の「次のリリースの前に」）

### 完了レビューで足した確認（実CLI・実生成）

- **`--with-sample` の ZIP**（Phase 4 の受入手順 3。未実施だったもの）: 作業ツリーの写しで `ddq release <出力先> --with-sample`
  （52 秒）→ `check_release.py --with-sample --smoke`: すべて OK（ZIP 128 件の CRC、`docs/` がある、配布フォルダと ZIP が同じ、
  2 つの PDF の書体、VSIX、取り出した exe で init → pdf → html）。ZIP から取り出した exe で同梱のサンプルを `update` → `pdf` → `html`:
  すべて exit 0（PDF 52 ページ、HTML 15 ページ、図表 69 件・参照 96 件）。同じ exe でリポジトリの `examples/docs` の写しを発行しても
  52 ページ（統合の検証の版間比較の 51 ページは、両側を内蔵 merman で描き直した条件。ここは同梱の SVG を使う）
- 同梱のサンプルは生成物を入れないので、発行の前に `ddq update` が要る（無いと「`.template-version` がありません」で exit 3 と
  案内が出る。従来どおり）。PlantUML を使う章があるので、jar（`DDQ_PLANTUML_JAR`）か LAN のサーバも要る（3.0.0 から同梱しない）

### 残りと次に決めること

1. **マージの後に確かめること**（CI-01・REL-02）: 2026-11-23 にすべて確かめた（REL-02・CI-01 の「マージの後の実行」「わざと失敗させた
   実行」）。main への push・手動実行（`with_sample` を含む）・`concurrency` は成功し、わざと失敗させた 2 本は `release-<版>` を残さず、
   公開済みの v3.1.0-pre1 の資産は変わらなかった。検証用の枝 2 本（`ci-test/rc-fail-build`・`ci-test/rc-fail-verify`）は GitHub に残っている
2. **人の手で行う検証**（Phase 4 の受入手順 3 の後半と 4）: 公開した ZIP・VSIX を利用環境に持ち出してのスモーク、外部通信を OS で
   遮断した端末（ローカルの Java＋jar・LAN のサーバ・PlantUML なし）、実画面での IME・セル結合・未保存の編集・パネルの切り替え・
   旧二拡張からの移行、版の違う ddq での案内（EXT-01 は模擬だけ）
3. **次のリリースの前に**
   - **版を上げる**: 機構ファイル（`postprocess-html.js`）と雛形（`_quarto.yml`）が変わったのに `template/VERSION` は 3.1.0-pre1 のまま
     （GEN-01 で上げないことにした）→ 2026-11-23 に 3.1.0 に上げ、変更履歴を書いた（末尾の「2026-11-23 の残作業」）。同じ版名で機構が違うので、3.1.0-pre1 で作った文書を新しい ddq で発行すると「同じ版で中身が違う」
     （exit 3）になる。拡張の `package.json`・`package-lock.json` も揃える（REL-03 が照合する）。マージの後の main の release-candidate も
     `release-3.1.0-pre1` を作るが、公開済みの v3.1.0-pre1 とは中身が違うので、版を上げるまで公開に使わない
   - **変更履歴とリリースノート**に書く利用者に見える差: 章の並びの読み方（GEN-01。行末コメント・引用・空白入りのパス・part・同じ字下げの列）、
     新規作成の雛形の章立て、版の比較と update の案内（EXT-01）、改訂ファイルの引用付きの値（DATA-01）、BOM の保持と書く前の照合（IO-01）、
     init/add が既存のファイルを上書きしない・update の順（IO-02）、VSIX に `THIRD-PARTY-NOTICES.md` が入る（REL-01。両資産のファイル一覧差）、
     表エディタの merge-cols の読み方（`"ALL"`・`""`・旧 `.merge-rows` を結合せずに開く。解釈できない値の警告）、タブを含む mermaid の図を
     先回りの SVG で描く（GEN-02 の直し）、機構ファイル（`postprocess-html.js`・`design-doc.lua`）が変わるので `ddq update` が要ること、
     `tag list` が同じ見出しの多い文書で落ちない（5 つ目からの候補は `-2`・`-3`…）、改訂履歴の表で修正内容の逆斜線が書いたとおりに出る
     （既存の不具合 2 件の直し。表は `ddq rev build` をやり直すと新しくなる）、`tag apply`・`rev diff --write` がほかのプログラムが書き込み用に
     開いたままのファイルを書かずに止める・一時ファイルを書けないときに元のファイルに触れない（外部レビューの C-01・C-02 の直し）、
     確定済みの改訂を開いた直後に編集できない・保存できないときに取り直し・確定しない・図のキャッシュを消しきれないと update が
     止まる（M-01〜M-03）、部の扉ページの本文が tag・rev の対象になり HTML で番号が付く・付録は書いた順にかかわらず章の後ろ（H-01）、
     改訂履歴の画面がほかで直したメモを古い値で戻さない（H-02）
   - PR で release-candidate を毎回走らせるか（計画 §7「PR でのフル配布検査の所要時間を確認してジョブ配置を決める」）: 所要時間は 38 分
     （うち書体を入れる時間が 25 分あまり）。検討書 §2 の 3 の形（main と手動。PR は workflow などを変えたときだけ）のままを勧める
4. **計画の中の小さな直し**（2026-11-22 の指示で、このレビューで直した）
   - REL-03: CI の成果物に exe の SHA-256 を残す（`versions.txt` に 1 行）。workflow から手順を取り出して手元で流し、書いた値が
     exe と ZIP の中の exe の SHA-256 と一致することを確かめた。workflow は YAML として読め、手順の数は変わらない（build 12）
   - 拡張の開発文書の追随（上の「資料の整合」のずれ）
5. **計画の外・別の判断**: GEN-02 の既知の差のうち残した 2 つ（3 組は 2026-11-23 に直した）、ddq.exe から `release` を外すか（検討書 §5。2026-11-23 に外した。上の「ddq から release を外す」）、既存の不具合 2 件（下。2026-11-23 に直した）、
   実装仕様書（`cli-impl/`・`ddq-template-impl/`）の本文の更新（release の分離の分だけは 2026-11-23 に直した）、書体を入れる時間の短縮、`actions/*@v4` の Node.js 20 の警告、
   `versions.txt` に書体の版を残すか、EXT-03 のステータスバー
   → 2026-11-23 に、GEN-02 の 2 つは残すと決め、実装仕様書を 3.1.0 基準にし、書体はビルドと重ね、Action を Node 24 の版にし、
   ステータスバーを直した（末尾の「2026-11-23 の残作業」。書体の版は L-02 で `versions.txt` に書くようにした）

## 計画外で見つけた既存の不具合（2026-11-23 に直した）

1. `ddq tag list` が、同じファイルにラベル無しで種類と題名が同じ項目が 5 つ以上あると panic する（exit 101、
   `cli/src/doc/labels.rs:50`「sha1 40 桁が衝突することはない」。候補の長さ `[6, 8, 10, 40]` を使い切る）。
   HEAD でも `migration/docs` で起きる。CLI-01 の差分比較でも新旧同じ panic
   → 2026-11-23 に直した（6 桁の後ろに `-2`・`-3`… を付ける。上の「既存の不具合 2 件を直す」）
2. 改訂履歴の表（`ddq rev build` が書く `history.qmd`）は修正内容（note）を Markdown としてそのまま載せる。メモに `C:\docs\a.qmd` と
   書くと `\docs`・`\a` が TeX の命令とみなされ、表には `C:.qmd` と出る（新旧同じ。DATA-01 の相互確認で見つけた）。
   パスは `` `C:\docs\a.qmd` `` か `C:\\docs` と書けば出る
   → 2026-11-23 に直した（TeX の命令になる逆斜線を `\\` にする。上の「既存の不具合 2 件を直す」）

## 外部レビューの指摘（2026-11-23。`docs/refactoring-review.md`）

2026-11-23 に、`acbec86`（#143 のマージの前の `refactor/release-split-and-fixes`。main の `5e840b3` と同じ木）を対象にした
最終品質レビュー `docs/refactoring-review.md` が置かれた（未追跡）。総合判定は FAIL、指摘は Critical 3・High 2・Medium 6・Low 3。
上の台帳では計画の 17 項目をすべて検証済にしたが、レビューは EXT-03・GEN-01・IO-01・IO-02・DOC-01・REL-02 の 6 項目を部分完了と
判定した（下の表の指摘による）。指摘のうち 12 件は、コードと手順書を読んで指摘どおりであることを確かめた（L-02・M-04 は細部を見ていない）。

| ID | 指摘 | 状況（2026-11-23） |
|---|---|---|
| C-01 | 書き戻しの待ちの間にほかで保存された追記を消し、成功を返す（IO-01） | 直した（下。ブランチ `fix/review-critical`） |
| C-02 | 一時ファイルの書き込み・同期の失敗でも、元のファイルを切り詰めて書き直す（IO-01） | 直した（下） |
| C-03 | 公開の手順で下書きの作成と公開が別のコマンドで、片方の資産だけの下書きを公開しうる（REL-02） | 直した（下） |
| H-01 | `part: part.qmd` の部の本文が走査から抜け、`appendices` を先に書くと付録が先に並ぶ（GEN-01） | 直した（下の「High 2 件」） |
| H-02 | 改訂メモを 1 件直すと、文書全体を古い画面の状態で置き換え、別の行の変更が戻る（EXT-03） | 直した（下の「High 2 件」） |
| M-01 | 確定済みの改訂が、開いた直後は編集できる（EXT-03。比較元からの回帰） | 直した（下の「Medium 5 件と Low 3 件」） |
| M-02 | 保存に失敗しても、取り直し・確定に進む（EXT-03） | 直した（下） |
| M-03 | `diagrams/` を列挙できなくても update が成功し、版マーカーを進める（IO-02） | 直した（下） |
| M-04 | 通常 CI の失敗が、公開に使う候補の選択を止めない | 直した（下。公開の手順で照合する） |
| M-05 | 公開の手順で、既にあるタグの指すコミットを照合しない。例に `--prerelease` が無い（DOC-01） | 直した（C-03 と一緒に。下） |
| M-06 | 開発資料の手での梱包だと、VSIX に `THIRD-PARTY-NOTICES.md` が入らない（DOC-01） | 直した（下。手で作る VSIX は渡さないと書いた） |
| L-01 | 2 本目のバイナリを足した後、`cargo run` の既定のバイナリが無い | 直した（下） |
| L-02 | 再ビルドの条件を配布の記録から作り直せない（rustc・書体・Action の SHA） | 直した（下） |
| L-03 | 同じ ref への 3 件目の要求で、待っている候補が取り消される（コメントが不正確） | 直した（下。PR の試走で要確認） |

レビューに無いずれも 1 つ見つけた: `docs/cli-impl/05-build-environment.qmd:56` の「`npm ci` は node_modules が無いときだけ」は、
いまの実装（毎回 `npm ci`）と違う（M-06 と一緒に直した）。

### Critical 3 件と M-05 を直す（2026-11-23。ブランチ `fix/review-critical`）

- **状況**: **実装済**（単体・実 exe・変異・偽の gh で手順を確かめた）。main の `5e840b3` から切ったブランチ
  `fix/review-critical` に、2026-11-23 の指示でコミットした（`142d742`。プッシュはしていない）
- **利用者の判断（2026-11-23）**
  1. C-01: ほかのプログラムが原稿・改訂ファイルを**書き込み用に**開いたまま、1 秒ほど待っても閉じないときは、書かずに止める
     （IO-01 の 2026-11-22 の判断「1 秒ほど待ち、それでもだめならその場で書く」を変えた）。読むだけで開いているものは、照合し直して
     から従来どおりその場で書く。Unix はロックが無いので、付け替えの直前に照合し直すだけ
  2. C-02: その場で書く代わりの動作は、最後の付け替えが共有違反で拒まれたときだけ。一時ファイルの作成・書き込み・同期の失敗では
     元のファイルに触れない（レビューの推奨）
  3. C-03: 公開の手順を、`--draft` を付けない 1 つの `gh release create` に戻す（分ける前の形。新しいツールは作らない）
  4. M-05: C-03 と一緒に直す
- **変更**
  - `cli/src/replace.rs`: 一時ファイルの作成・書き込み・同期（`prepare`）と付け替え（`swap`）を分けた。`replace` は共有違反の
    付け替えだけその場で書く。人が書くファイル向けの `replace_unchanged`（読んだバイト列との照合つき）と、止めた理由の
    `Refused`（`Changed`・`Busy`）を足した。試験のビルドだけの障害注入（`FAULT`）
  - `cli/src/commands/tag.rs`: 書く前の照合で読んだバイト列を残し、ファイルごとに `replace_unchanged` に渡す
  - `cli/src/commands/rev.rs`: `save_revision` を `replace_unchanged` にした（読み直しは置き換えの中。文言は同じ）
  - 試験: `cli/src/replace.rs` の単体 14（追加 6・照合つきも流すように書き換え 4）、`cli/tests/tag.rs`（+2）、`cli/tests/rev.rs`（+1）。
    `cli/src/commands/tag.rs` の単体 1 件は、止まった理由の文言の期待値を新しいものにした（下の「利用者に見える差」の 3）
  - `docs/manual/chapters/18-maintainer/06-release.qmd`（公開の手順）、`cli/DESIGN.md` §14.5・§15、`docs/release-ci-proposal.md`（追記）
- **規則**: `cli/DESIGN.md` §14.5「置き換え（`replace.rs`）の規則」。公開の手順は、`set -euo pipefail` で失敗したところで止まり、
  `SHA256SUMS.txt` を照合し、既にあるタグ（注釈つきは剥がしたもの）が `versions.txt` の `commit:` と違えば作る前に止まる。
  版に `-` が付けば `--prerelease`。作った後に、資産の名前と GitHub の `digest` を `SHA256SUMS.txt` と照合する
- **試験と結果**
  - 単体（Windows）: 障害注入で、作成・書き込み・同期・付け替え（共有違反以外）のどこで失敗しても、`replace` も `replace_unchanged`
    も「元のファイルは変えていません」で止まり、元のバイト列のまま、一時ファイルも残らない。共有違反の付け替えだけはその場で書く。
    書き込み用に開いた手（読み・書きを共有／読み・書き・削除を共有）が 200 ミリ秒後に行を足して閉じると、待ち直した後の照合で
    見つけて `Changed`、足した行は残る。開いたままなら 1 秒ほど待って `Busy`、元のまま。読みと削除を共有する読み手がいても
    付け替える（その読み手には元のバイト列）。読むだけで削除を共有しない読み手なら、照合し直してその場で書く（従来どおり）
  - 実 exe: `tag apply --all` の途中（パスの順で前のファイルを書いた後）に、CRT の fopen の追記と同じ開き方で原稿に 1 行足して閉じると、
    終了コード 1・「読み込んだあとに変わっています」・書いた／書いていないファイルを挙げ、足した行は残る（従来は足した行が消えて 0）。
    もう一度実行すると、足した行を残して書き戻す。書き込み用に開いたままなら終了コード 1 で元のまま、閉じて再実行すると 1 回で
    書き戻したときと同じ中身。`rev diff --write` の改訂ファイルを書き込み用に開いたままなら終了コード 1 で元のまま
  - 変異 13 か所（照合しない・書き込みを拒まない・開けるまで待ち直さない・書き込み用に開いていてもその場で書く・読み手がいると
    止める・消えていたら Busy・無かったはずのものを上書き・準備の失敗でその場で書く（2 か所）・付け替えのどの失敗でもその場で書く・
    tag と rev で照合しない・tag の BOM）を 1 つずつ入れ、すべてで試験が落ちた。「書き込みを拒まない」だけは、実 exe の
    「途中で足した行」の試験が時間の前後で通る（単体と実 exe の残り 2 つが落ちる）。その場で書く前の照合し直しは、落とせる試験が無い
  - 公開の手順: 手順の bash を、偽の gh・git で 8 場面流した（プレリリースで最後まで、正式版に `--prerelease` を付けない、同じ
    コミットの軽量タグ・注釈つきタグなら通る、別のコミットのタグなら作る前に止まる、資産を上げられなければそこで止まり次を呼ばない、
    公開した資産の digest が違えば知らせて止まる、SHA-256 の一覧が合わなければ作らない。`versions.txt` の CRLF も除く）。jq の式は
    公開済みの v3.1.0-pre1 を読むだけで試し、2 資産の SHA-256 が `sha256sum` と同じ形で出た。gh 2.92.0 のソース
    （`pkg/cmd/release/create/create.go` の 496–567 行）で、`--draft` を付けずに資産を渡すと、下書きで作って上げ終えてから公開し、
    失敗したら下書きを消すことを読んだ。章は Quarto 同梱の Pandoc で読め、手順の塊は原文と同じ。実際のリリースは作っていない
  - Rust 全件: **243 件成功・失敗 0・ignored 0**（`DDQ_E2E=1`・`--test-threads=1`、505 秒）。unit 121 / ddq-release 6 / commands 10 / contract 5 / e2e 5 / faults 8 / filter_contract 7 / golden 2 / mechanism 2 / plantuml 4 / release 20 / rerun 4 / rev 17 / rev_ops 4 / table_pagination 2 / tag 25 / third_party 1。1 回目は `commands::tag` の単体 1 件が、止まった理由の文言の古い期待値で落ちた（期待値を直して、通しで流し直した）
  - `cargo fmt --check`・`cargo clippy --locked --offline --all-targets -- -D warnings` 成功。拡張のコードは変えていない
- **利用者に見える差**
  1. `tag apply`・`rev diff --write`: ほかのプログラムが書き込み用に開いたままの原稿・改訂ファイルは、1 秒ほど待っても閉じなければ
     書かずに終了コード 1（従来はその場で書いた）。待っている間に保存されたものは消さずに終了コード 1（従来は消えて 0）
  2. 一時ファイルを書けない（容量不足など）ときは、元のファイルに触れずに止まり「元のファイルは変えていません」と出る（従来は
     その場で書き直した）
  3. 原稿・改訂ファイルをほかのプログラムが開いていて書けないときの理由の文言が、「<ファイル> を置き換えられません（ほかの
     プログラムが開いていないか確かめてください）」から「ほかのプログラムが開いているため、書き換えていません（閉じてから、もう一度
     実行してください）」になった（`tag apply` は前の行の「<ファイル> を書き換えられないため、途中で止めました」、`rev diff --write` は
     「<ファイル> を書けません」が付く）。終了コード・書いた／書いていないファイルの一覧は同じ。拡張と契約の期待値はこの文言に依らない
  4. 公開の手順（保守者だけ）: 1 つの `gh release create` に戻り、タグの照合・`--prerelease`・公開後の digest の照合が付いた
- **計画からの逸脱**: なし（レビューの後の直し）。IO-01 の 2026-11-22 の判断を、上の判断 1 で変えた
- **戻す方法**: `cli/src/replace.rs`・`cli/src/commands/tag.rs`・`cli/src/commands/rev.rs` と足した試験を `5e840b3` に戻す。手順書・
  設計書・検討書の追記も戻す
- **残存リスク**
  - Windows でも、ほかのプログラムが「安全な保存」（一時ファイルからの付け替え）で原稿を置き換えるのは拒めない（自分の付け替えの
    ために削除を共有するため）。照合と付け替えの間（ミリ秒より短い）に起きると検出できない
  - Unix は付け替えの直前に照合し直すだけ（配る exe は Windows だけ。CI の Rust も Windows だけ）
  - その場で書く経路（削除を共有しない読み手）は、従来どおり途中で止まると途中のバイト列になりうる
  - 公開の手順は文書で、CI の試験は無い（偽の gh で流したのは手元だけ）。公開した後に digest が違ったら、公開は取り消せない
    （下書きに戻して調べる）

### Medium 5 件と Low 3 件を直す（2026-11-23。ブランチ `fix/review-critical`）

- **状況**: **実装済**（単体・実 exe・変異・偽の gh・手元での手順の実行で確かめた）。Critical の直し（`142d742`）の上に、
  全件の試験が通った後、2026-11-23 の指示（「テストが通ったらコミットしてください」）でコミットした（プッシュはしていない）。
  High の 2 件（H-01・H-02）は、同じ日の指示（「終わったら High も直してください」）で続けて直す
- **利用者の判断（2026-11-23）**: 「コミットして Medium と Low にも対応してください」。決め方を尋ねた 4 件はすべて推奨を選んだ
  1. M-04: 公開の手順で、選んだ実行が main のそのコミットの通った配布候補であることと、同じコミットで `ci.yml`・`extension.yml` が
     通ったことを照合し、無ければ止める。CI の設定は変えない（文書だけの変更では通常の CI が path の条件で走らないので、
     `gh workflow run` で走らせるよう案内する）
  2. M-06: 手で作る VSIX は確かめる用で人に渡さず、配るのは CI の配布候補だけと書く。手で作るときのライセンス表示のコピーも書く。
     梱包のスクリプトに検査は足さない。`docs/cli-impl/05-build-environment.qmd` の `npm ci` の記述も直す
  3. L-02: `versions.txt` に Rust（rustc・cargo）、書体の実際の版、スモークに使った VSCode の版を足す。`release-candidate.yml` の
     外部の Action をコミットの SHA に固定し（版はコメント）、上げる手順を書く。Rust の版は固定しない
  4. L-03: `release-candidate.yml` の同時実行に `queue: max` を足し、要求をすべて待たせる
  - M-01・M-02・M-03・L-01 は、レビューの推奨どおりに直した（尋ねていない）
- **変更**
  - M-01（`extensions/ddq-template/src/revision/messages.ts`・`provider.ts`・`src/revisionWebview/RevisionEditor.tsx`）: 画面に渡す
    状態に `checked`（タグの状態を確かめ終えたか）を足した。拡張は最初の `ddq rev next` が返るまで `checked: false` で本文を送り、
    画面は確定済みのときと同じく編集欄・取り直し・確定・stale の削除を使えなくし、「タグの状態を確かめています…」と出す。
    拡張も、確かめ終えていない・確定済みの改訂への書き戻し・取り直し・確定を受け付けず、画面を文書に戻して知らせる。照会に失敗した
    （Git が無いなど）ときは、従来どおり確かめ終えた・未確定とする。確かめた後の入力は、従来どおり入力が止まってから取り直す
  - M-02（`provider.ts` の `saveBeforeDdq`）: 取り直し・確定の前の保存で、`save()` が偽を返した・例外になった・保存の後にまた
    変わったときは、ddq を起動せず（表も作らず、確定のコマンドも案内せず）に知らせる
  - M-03（`cli/src/commands/update.rs`）: `diagrams/` が無いときだけ「消すものが無い」。フォルダでない・読めない・途中の項目を
    読めないときは失敗にし、版マーカーを古いまま残す（「図のキャッシュを消しきれません（版マーカーは … のまま…」）
  - M-04（`docs/manual/chapters/18-maintainer/06-release.qmd`）: 公開の手順に、`gh run view` で実行の workflow・枝・コミット・
    契機・結論の照合と、`gh run list --workflow ci.yml|extension.yml --branch main --commit <コミット>` で通常の CI の照合を足した
  - M-06（`extensions/ddq-template/DEVELOPMENT.md`・`docs/cli-impl/05-build-environment.qmd`・`cli/tests/release.rs` の注記）
  - L-01（`cli/Cargo.toml`）: `default-run = "ddq"`
  - L-02（`.github/workflows/release-candidate.yml`・`extensions/ddq-template/tests/host/smoke.mjs`・`smoke-index.js`）: 外部の
    Action 8 つ（checkout v5.1.0・rust-toolchain の stable の枝・rust-cache v2.9.3・quarto-actions/setup v2.2.0・setup-node v5.0.0・
    upload-artifact v4.6.2・download-artifact v4.3.0・cache v4.3.0）を、2026-11-23 にそのタグ・枝が指していたコミットに固定した。
    `versions.txt` に `rust:` 行を足し、`fonts:` 行を書体の名前から「名前・版・ファイル」にした（WPF の GlyphTypeface で name 表を
    読む）。スモークは使った VSCode の版を書き出し（環境変数 `SMOKE_VERSIONS`）、`publishable` が `versions.txt` に足す
  - L-03（`release-candidate.yml`）: `concurrency` に `queue: max`
  - 設計書（`cli/DESIGN.md` §3 の別の exe・図のキャッシュ・§7.6 の配布候補）、検討書（追記）。§7.5 の「下書きのリリースに載せてから
    公開」という C-03 の前の書き方も直した
- **試験と結果**
  - M-03（`cli/tests/rerun.rs` +3）: `diagrams` がファイルなら終了コード 1・版マーカーは古いまま、フォルダに戻して再実行すると
    キャッシュ 2 件を消して版を上げる。`diagrams` をほかのプログラムが共有せずに開いていても同じ（Windows）。無いときは従来どおり
    成功する。直す前の `update.rs` では、前の 2 つが落ちる（無いときの試験は前後とも通る）
  - M-01・M-02（`provider.test.ts` +8、`RevisionEditor.test.tsx` +1）: 照会が返る前は `checked: false` で、届いた書き戻しを当てず、
    返った後は当てる。確定済みなら書き戻し・取り直し・確定を受け付けず、表は作れる。照会の失敗では編集できる。保存が偽・例外なら
    取り直しも確定もしない（ddq を起動せず、案内も出さない）。保存の後にまた変わっていたら取り直さない。保存できたら取り直す。
    画面は、中身が届く前と確かめ中は編集欄・ボタンが使えず、確かめたら使える。変異 7 か所（拡張が照合しない・最初から確かめ
    終えたことにする・画面が確かめ中でも編集させる・`save()` の偽を無視・例外を握りつぶす・保存後の再変更を見ない・確定で保存を
    確かめない）を 1 つずつ入れ、すべてで試験が落ちた
  - 拡張: 型検査・Vitest **47 ファイル 506 件成功**・build・`check:offline`・`check:audit` 成功
  - L-02: 「使った版を書く」の手順を手元の pwsh で流し、`rust:`・`fonts: BIZ UDGothic Version 2.02 (BIZ-UDGothicR.ttc), Meiryo UI
    Version 6.52 (meiryo.ttc)` が出て、CRLF・BOM なしのまま（`publish.sh` の読み方は変わらない）。M-06 の手順どおり
    （ライセンス表示をコピーしてから `npm run package:win32-x64`）に作った VSIX は 14 項目で、`SMOKE_VERSIONS` を付けた
    `npm run test:smoke` が合格し、`vscode (smoke): 1.147.0 (…Code.exe)` を CRLF で書いた（作ったものは消した）。workflow の
    YAML は読め、外部の Action に版のタグが残っていない
  - M-04: 手順の bash を偽の gh・git で 20 場面流した（Critical のときの 8 場面に、手動実行の候補なら通る、`ci.yml`・`extension.yml`
    を main のそのコミットで探す、PR・別のコミット・落ちた実行・別の workflow の実行なら作る前に止まる、`ci.yml` が落ちた・走って
    いない、`extension.yml` が途中・打ち切られたなら作る前に止まる、を足した）。照合の `gh` の式は、実際の main の配布候補
    （実行 45106998667。`5e840b3`）と、無いコミットで読むだけで試し、`release-candidate main 5e840b3… push success`・
    `completed success`（2 つ）・`実行が無い` が出た。章は Quarto 同梱の Pandoc で読め、手順の塊は原文と同じ
  - Rust 全件: **246 件成功・失敗 0・ignored 0**（`DDQ_E2E=1`・`--test-threads=1`、517 秒）。unit 121 / ddq-release 6 / commands 10 / contract 5 / e2e 5 / faults 8 / filter_contract 7 / golden 2 / mechanism 2 / plantuml 4 / release 20 / rerun 7 / rev 17 / rev_ops 4 / table_pagination 2 / tag 25 / third_party 1。1 回目の全件は rerun の新しい 2 件が落ちた（変異の確認の後に `update.rs` を `Copy-Item` で戻したので更新日時が古いままになり、cargo が作り直さず直す前のコードの exe で走った。日時を更新して作り直し、通しで流し直した）。実拡張ホスト（`DDQ_HOST_REAL=1`、新しくビルドした release の ddq）の 30 確認に合格
  - `cargo fmt --check`・`cargo clippy --locked --offline --all-targets -- -D warnings` 成功。`cargo run -- --version` が ddq を起動する
- **利用者に見える差**
  1. 改訂履歴の画面: 開いた直後、タグの状態を確かめ終わるまで（`ddq rev next` が返るまで）は編集できず、「タグの状態を確かめています…」
     と出る。保存できなかったときは、取り直し・確定が「改訂履歴の文書を保存できなかったため、…をやめました。」で止まる
  2. `ddq update`: `diagrams` がフォルダでない・読めないときは、終了コード 1 で版マーカーを古いまま残す（従来は成功し、版だけ上げた）
  3. 開発者・保守者だけ: `cargo run` の既定が ddq。公開の手順は通常の CI の成功も照合する。配布候補の `versions.txt` の
     `fonts:` 行の形が変わり、`rust:`・`vscode (smoke):` 行が増えた（`commit:` 行は同じ）
- **計画からの逸脱**: なし（レビューの後の直し）
- **戻す方法**: 上の変更のファイルを `142d742` に戻す
- **残存リスク**
  - L-03 の `queue: max` は GitHub の文書にあるが、プレビューかどうかの記載が無い。受け付けられるかは、PR を出したときの
    release-candidate の試走で確かめる（受け付けられなければ workflow が読めずに失敗する）
  - L-02 の書体の版は WPF で name 表を読む。`C:/Windows/Fonts` と利用者のフォントフォルダに無い書体は「版を読めない」と書く。
    固定した Action は、上げる手順を人が踏まない限り古いまま（Dependabot などは入れていない）
  - M-04 の照合は手順書の中だけで、CI の試験は無い（偽の gh で流したのは手元だけ）。`extension.yml` は同じ枝への次の push で
    前の実行を打ち切るので、続けて push すると、前のコミットの配布候補は公開に使えなくなる
  - M-01 は、確かめた後にほかで打ったタグを、入力が止まって取り直すまで（0.8 秒）知らない（従来どおり。拡張も同じ状態で照合する）
  - H-01・H-02 は未着手（→ 下の「High 2 件を直す」）

### High 2 件を直す（2026-11-23。ブランチ `fix/review-critical`）

- **状況**: **実装済**（単体・実 Quarto・実 exe・実拡張ホスト・変異で確かめた）。2026-11-23 の指示でコミットし、ブランチをプッシュして PR にした。Medium・Low の直し
  （`8b9338a`）の上の作業ツリー
- **利用者の判断（2026-11-23）**: 「終わったら High も直してください」。決め方は尋ねていない（レビューの推奨どおり）。部の扉ページの
  図表の番号は、既にある番号なしの章の扱い（「図 0-1」。雛形の `index.qmd` にも書いてある）に揃えた
- **確かめた Quarto の振る舞い（1.9.38）**: 付録を先に書き、部の扉ページ（ファイル）・部の題名・付録の中の部・ファイルの無い値
  （`総論.qmd`）を混ぜた book を実際に描いた。入力の並び（`quarto inspect` の `files.input`）は
  `index, part, a, b, parts/p3, c, d, app, app-part, app2`、扉ページは番号なし（「図 1:」）、付録の中の扉は記号を使わない（`app2` が
  付録 B）、ファイルの無い値は部の題名。同梱の `quarto.js` の `bookRenderItems`（章 → references → 付録）・`normalizeSidebarItem`
  （値がファイルなら扉ページ）とも合う
- **変更**
  - H-01（`cli/src/doc/project.rs`・`gitsrc.rs`、`template/postprocess-html.js`）: `- part: <値>` の値が執筆フォルダにあるファイル
    なら部の扉ページとして子の章の前に入れ、無ければ部の題名として入れない（作業ツリーはファイルか、旧版は `git ls-tree` の一覧に
    あるか。`Source::exists` を足した）。付録は書いた順にかかわらず章の後ろ。後処理も扉ページの番号を振り直し（番号なしの章と同じく
    0）、付録の中の扉ページには付録の記号を振らない
  - H-02（`extensions/ddq-template/src/revision/messages.ts`・`provider.ts`・`revfile.ts`、`src/revisionWebview/RevisionEditor.tsx`、
    `src/activateRevision.ts`）: 画面は改訂全体ではなく、通し番号と直した 1 か所（メモ・改訂日・stale の削除と、直す前の値・直した
    後の値）を送る。拡張は届いた順に 1 つずつ、文書のいまの内容に当てる（`applyChange`）。直す前の値が文書と違えば当てず（同じ値に
    直してあれば通す）、`reset` を付けた内容で画面を文書に戻して知らせる。差分に戻った行は消さない。値は書いて読み戻した形で比べる
    （メモの末尾の空行と全行に共通の字下げは文書に残らないため。比べないと、末尾で改行を打つと消える）。画面は、拡張が処理し終えて
    いない自分の変更を、届いた文書の内容に重ねて出す（打っている値を巻き戻さず、ほかの行の変更は出す）。処理済みは値ではなく、
    拡張が返す番号（`applied`）で決める（A → B → A と打ったときに途中の版で手放さない）。取り直し・確定も同じ順番に並べる。検証用の
    内部コマンド `writeNote` も同じ経路にした
  - 試験の正解: `cli/tests/fixtures/contract/chapters.json` に H-01 のケース（上の book）と `part_files` を足し、全 7 ケースの正解を
    `quarto inspect` の `files.input` に照らし直した（すべて一致）。`cli/tests/e2e.rs` の `quarto_chapters` も `files.input` にした
    （以前は `config.book` の `chapters` だけをたどり、扉ページを正解からも落としていた）
  - 資料: `cli/DESIGN.md` §14.3・§14.6、`docs/design/chapters/06-html/01-postprocess.qmd`、`docs/cli-impl/03-behavior.qmd`、
    `cli/tests/fixtures/contract/README.md`、`docs/revision-study.md` §5.8、`extensions/ddq-template/AGENTS.md`、
    `docs/ddq-template-impl/parts/revision/` の 02〜04 章
- **試験と結果**
  - H-01: `project.rs` の単体 +2（扉ページ・付録の位置・`part:x`・値の無い `part:`、作業ツリーでフォルダの名前の部は題名・扉ページの
    include も展開）、共有ケース +1、`tests/rev.rs` +1（旧版（git）でも扉ページを読み、扉と付録の本文の変更が「変更」になる）、
    `tests/filter_contract.rs`（扉ページの原稿を置き、扉・付録の HTML を Quarto の形にした）、`tests/e2e.rs` +1（`ddq init` の雛形を
    部立てにし、`tag list` の文書順が Quarto の入力の並びと同じ、扉ページの表がラベルの走査に入る、`ddq html` で扉ページと付録の中の
    扉の表が「表 0-1」、付録が A・B、別の章からの参照が「表 0-1」）。変異 8 か所（扉ページを拾わない・付録を後ろに回さない・値が
    ファイルかを見ない・フォルダもあるとする・旧版で扉が無いとする、後処理が扉を拾わない・付録の扉に記号を振る・付録の扉を並びから
    落とす）を 1 つずつ入れ、すべてで試験が落ちた
  - H-02: `messages.test.ts`（新規 11 件）、`provider.test.ts` +5（レビューの手順: ほかで 2 行目を `editor B` にした後に、古い画面から
    1 行目の変更が届いても 2 行目は残る。同じ行の食い違いでは当てずに reset。同じ値なら書かずに通す。続けて届いた書き戻しを順に当てる。
    stale の削除）、`RevisionEditor.test.tsx` +5（1 か所と直す前の値を送る・改訂日・打っている最中の版で巻き戻さずにほかの行は出す・
    A → B → A・末尾の改行・reset）。変異 10 か所（拡張が直す前の値を照らさない・順に並べない・reset を送らない、画面が届いた内容を
    使わない・値で処理済みを決める・reset を無視、差分に戻った行も消す、読み戻した形で比べない（画面・拡張）、直す前の値を文書から
    取る）を 1 つずつ入れ、すべてで試験が落ちた
  - 拡張: 型検査・Vitest **48 ファイル 527 件成功**・build・`check:offline`・`check:audit` 成功
  - Rust 全件: **250 件成功・失敗 0・ignored 0**（`DDQ_E2E=1`・`--test-threads=1`、540 秒）。unit 123 / ddq-release 6 / commands 10 / contract 5 / e2e 6 / faults 8 / filter_contract 7 / golden 2 / mechanism 2 / plantuml 4 / release 20 / rerun 7 / rev 18 / rev_ops 4 / table_pagination 2 / tag 25 / third_party 1
  - 実拡張ホスト: `DDQ_HOST_REAL=1`、新しくビルドした release の ddq で 30 確認に合格（画面で書いたメモの確認は、新しい経路 `writeNote` → `applyChangeTo` を通る）
  - `cargo fmt --check`・`cargo clippy --locked --offline --all-targets -- -D warnings` 成功
- **利用者に見える差**
  1. `part:` にファイルを書いた部の扉ページの本文が、`ddq tag`・`ddq rev` の対象になる（従来は抜けていた）。付録を章より先に書いても、
     文書順・改訂の並びは章 → 付録
  2. HTML で扉ページの図表に番号が付く（番号なしの章と同じく「図 0-1」。従来は Quarto の既定の「図 1:」のまま）
  3. 改訂履歴の画面: ほかの所で直したメモを、古い画面の値で戻さない。打った行がほかで変わっていたら当てずに「…ほかの所から変わって
     います。いまの内容を出し直したので、確かめてからもう一度直してください。」と出し、画面を文書の内容に戻す
- **計画からの逸脱**: なし（レビューの後の直し）。テンプレートの版は上げていない（GEN-01 の判断どおり。機構の `postprocess-html.js` が
  また変わったので、次のリリースで版を上げる。上の「次のリリースの前に」）
- **戻す方法**: 上の変更のファイルを `8b9338a` に戻し、`extensions/ddq-template/src/revision/messages.test.ts` を消す
- **残存リスク**
  - H-01: `part:` を `chapters:` の後ろに書いた項目、フロー形式 `[a, b]`、`book.references`（Quarto は章と付録のあいだに入れる）、
    `index.qmd` を先頭以外に書いたとき（Quarto は先頭に回す）は、従来どおり Quarto と並びが違う（レビューの指摘の外）。PDF での
    扉ページは確かめていない（部は利用マニュアルに書いていない）
  - H-02: 打った値が文書に当たる前にほかで同じ行が直されると、当てずに画面を戻す（打った値は画面から消える。知らせは出る）。
    改訂全体を YAML に作り直して置き換える点は従来どおり（手で崩した書式は整え直される）。実画面での IME の変換中の入力は見ていない
    （jsdom では見られない。「人の手で行う検証」）

## 2026-11-23 の残作業（版 3.1.0・章の書き方・CI・実装仕様書。ブランチ `fix/review-critical`）

- **状況**: **実装済**（単体・実 Quarto・実 exe・実拡張ホストで確かめた）。コミットしていない（High の直し `f2c6e0e`・PR の上の作業ツリー）。
  GitHub Actions では走らせていない
- **利用者の指示（2026-11-23）**: 「外部レビューおよび人の手で行う検証以外で実行できるものは実施してください。判断が必要なものは
  確認を実施してください」。尋ねた判断と答え:
  - GitHub: 「コミットしてPRを出す (推奨)」（High の直しのコミットと PR。検証用の枝 `ci-test/*` は消さず、失敗した実行も走らせ直さない）
  - 版: 「3.1.0 に上げる」（変更履歴も書く。タグとリリースは作らない）
  - 章の並びの残りの書き方: 「Quarto に合わせて直す (推奨)」
  - GEN-02 の残りの差 2 つ: 「残す (推奨)」
  - 書体を入れる時間: 「ビルドと重ねる (推奨)」
  - 実装仕様書: 「3.1.0 を基準に反映する」
  - 試験の中で見つけた、部（`- part:`）のある book が PDF にできない前からの制約: 「記録だけにする (推奨)」（部の扉の体裁は
    別に設計してから直す）
- **変更**
  - 章の並び（`cli/src/doc/project.rs`、`template/postprocess-html.js`）: H-01 の残存リスクに挙げた書き方を Quarto と同じ並びにした。
    フロー形式 `[a, b]`（複数行も、部の中の `chapters:` も）、`book.references`（章と付録のあいだ）、`index.` で始まるページを先頭へ、
    対応付けの項目で `chapters:` を `part:` より先に書いたとき（扉ページは子の章の前）。読まないのはフロー形式の対応付け
    `{part: …}` と `href:`・`file:` で書いた項目（`cli/DESIGN.md` §14.3 に書いた）。共有の期待値 `chapters.json` を 9 ケースにし、
    正解は `quarto inspect` の `files.input`（Quarto 1.9.38）
  - EXT-03 の残り（`extensions/ddq-template/src/build.ts`）: ステータスバーの問い合わせに通し番号を付け、先に始めた問い合わせの答えで
    後の表示（発行中の表示を含む）を上書きしない。試験 `src/build.status.test.ts`（新規 3 件）
  - CI: `actions/*` を Node 24 で動く版にした（`ci.yml`・`extension.yml` は版の枝、`release-candidate.yml` はコミットの SHA）。
    `.github/actions/jp-fonts` に `phase`（`all`・`start`・`wait`）を足し、`release-candidate.yml` の build では書体を裏で入れ始めて
    （`start`。Quarto の導入を前に移した）ビルドと重ね、使う前に待つ（`wait`。裏の処理が終了コードを残さずに消えたら待たずに止める。
    長くても 45 分）。verify-zip は従来どおり（`all`）
  - 版 3.1.0: `template/VERSION`、拡張の `package.json`・`package-lock.json`、`extensions/ddq-template/CHANGELOG.md`（3.1.0 の節。
    日付 2026-11-23）、`cli/DESIGN.md` の対象版、README・AGENT-GUIDE・利用マニュアル・設計書・発表資料の展開フォルダ名などの版
  - 実装仕様書: `docs/cli-impl/`・`docs/ddq-template-impl/` の基準を 3.1.0 にし、IO-01・IO-02・REL-01・REL-03・CI-01・CI-02・
    GEN-01・GEN-02・QA-01・DATA-01・EXT-01〜04・H-01・H-02 と既存の不具合 2 件の直しを本文・根拠（`cli-impl` の E-0038）に書いた。
    古くなっていた記述（`mermaid.min.js` が npm の dist と一致する、zip 8.6.0、配布工程の jar、`npm ci` は node_modules が無いときだけ、
    CI の vsix-smoke）も直した。`analysis/source-sha256.json` を取り直した（91 ファイル）
  - `cargo fmt` と clippy（`collapsible_if`）の指摘を直した（`project.rs`。fmt は `e2e.rs`・`filter_contract.rs` も整えた）
- **試験と結果**
  - Rust 全件（`DDQ_E2E=1`・`DDQ_PLANTUML_JAR`・`--test-threads=1`）: **253 件成功・失敗 0・ignored 0**（562 秒）。
    unit 125 / ddq-release 6 / commands 10 / contract 5 / e2e 7 / faults 8 / filter_contract 7 / golden 2 / mechanism 2 / plantuml 4 /
    release 20 / rerun 7 / rev 18 / rev_ops 4 / table_pagination 2 / tag 25 / third_party 1。`cargo fmt --check`・clippy・release
    ビルドも成功
  - 章の並び: `project.rs` の単体 +2、共有ケース +2、`tests/e2e.rs` +1（フロー形式・references・後ろに書いた index の book を
    `quarto inspect` の入力の並びと `tag list` の文書順で照らし、`ddq html` の番号も見る）
  - 拡張: 型検査・Vitest **49 ファイル 530 件成功**（`REQUIRE_PANDOC=1`）・build・`check:offline`・`check:audit`（配布物に入る依存の指摘 0 件。開発用の依存の参考も 0 件）
  - 実拡張ホスト: `DDQ_HOST_REAL=1`、3.1.0 のリリースビルドの ddq で 30 項目に合格
  - 書体の 2 段: workflow を YAML として読めることと、`phase` の手順を手元の PowerShell で流した結果（workflow 3 本と jp-fonts を YAML として読めた。偽の dism と「書体が無い」状態を差し込んで 7 通りを流し、すべて期待どおり（start → wait、2 回目の wait は待たない、書体があれば入れ始めない、dism の失敗で止まる、裏の処理が消えたら待たずに止まる、既定の all、知らない phase は止まる）。実際の dism は流していない（システムを変えるため））
  - 実装仕様書: 新しい ddq で `ddq update` → `ddq html`（cli-impl: 警告なし。`analysis/check_output.py` で 9 ページ・図 3 つ・壊れたリンク 0・ハッシュの不一致 0、ddq-template-impl: update → pdf（55 ページ）→ html、警告・未解決の参照なし）
  - PDF の部の扉ページ: E2E と同じ部立ての book（扉ページのファイル・題名だけの部・付録の中の部）を `ddq init` で作って `ddq pdf` すると、Typst の `unknown variable: part` で失敗した。Quarto 1.9.38 は部を `#part[…]` と出すが、`template/` の Typst 側は `part` を定義していない（前からの制約。今日の変更は Typst 側に触れていない）。利用者の判断で記録だけにした（下の残存リスク）
- **利用者に見える差**: `extensions/ddq-template/CHANGELOG.md` の 3.1.0 の節。今日の分は、フロー形式・`book.references`・後ろに書いた
  index・`chapters:` を先に書いた部の章が tag・rev・HTML の番号の対象になること、ステータスバーが前のフォルダに戻らないこと
- **計画からの逸脱**: なし（利用者の判断に沿った残りの片付け）
- **戻す方法**: 上の変更のファイルを `f2c6e0e` に戻し、`extensions/ddq-template/src/build.status.test.ts` を消す
- **残存リスク**
  - CI の変更（Action の版・書体の 2 段）は GitHub で走らせていない。PR に push した後の実行で確かめる
  - 変更履歴の日付は 2026-11-23。公開する日が違えば直す。タグ・リリースは作っていない
  - 章立てでフロー形式の対応付け `{part: …}` と `href:`・`file:` の項目は読まない（Quarto とは並びが違う）
  - GEN-02 の既知の差 2 つは残した（利用者の判断）
  - 部（`- part:`）を書いた book は、扉ページのファイルでも題名だけでも `ddq pdf` が Typst の `unknown variable: part` で失敗する
    （前からの制約。Quarto 1.9.38 が出す `#part[…]` を `template/lib.typ`・`typst-template.typ` が定義しない）。HTML・`tag`・`rev` は
    扉ページを含めて読む。部は利用マニュアルに書いていない。利用者の判断で記録だけにした（扉の体裁を決めてから直す）
