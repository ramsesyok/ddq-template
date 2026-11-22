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
| GEN-02 | `d8b6eec10c9118bf355946b52b9ffb389a053761` |
| EXT-03 | `c2ee2db7dc6b39f9618d760fd5747efee971ae45` |
| EXT-04 | `30041c35dd25a6aec5790f0920d279c56e64d366` |
| GEN-01 | `122c2e192abc180bc7e79d8b0cfc94785e5d5b1a` |
| EXT-01 | `25f40fe5879c466f5b63d41e6d276cec402a35dd` |
| DATA-01 | `78e972cb3d4539675ce8b17e506d2df5eba43041` |

GEN-01・EXT-01・DATA-01 は続けて実装したので、木は後から ID ごとのファイルで組み立てた（3 つで変更ファイルは重ならない。
共有の README は GEN-01 の木では DATA-01 で足した行と節を除いた）。本書の更新は DATA-01 の木にまとめて入れた。

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
- **残存リスク**: `part` に書いたファイル（部の扉ページ）とフロー形式 `[a, b]` は対象外。版を上げないため、同じ `3.1.0-pre1` を名乗る exe どうしで機構が違う（次のリリースで版を上げる前提）

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
- **計画からの逸脱**
  1. 上の判断により、付け替えを拒まれたときはその場で書く。その場合だけ「中断後も途中のバイト列にならない」は保証しない（従来と同じ）
  2. 付け替えは tempfile の `persist`（`MoveFileExW`）ではなく std の `rename` を使った。`MoveFileExW` は削除を共有して開かれている
     ファイルも置き換えられない（実測）のに対し、std は POSIX の意味で置き換える。tempfile の既定の「一時」属性も付けない
  3. 書く順をパスの順にした（途中で止まったときに挙げるファイルの並びを毎回同じにするため）
- **戻す方法**: `tag.rs`・`rev.rs` の書き込みを `fs::write` に戻し、`replace.rs` を消す。契約の期待値の BOM も戻す
- **残存リスク**
  - 書く直前の照合と置き換えの間（数ミリ秒）に保存されたものは検出できない
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

### 計画外で見つけた既存の不具合（直していない）

1. `ddq tag list` が、同じファイルにラベル無しで種類と題名が同じ項目が 5 つ以上あると panic する（exit 101、
   `cli/src/doc/labels.rs:50`「sha1 40 桁が衝突することはない」。候補の長さ `[6, 8, 10, 40]` を使い切る）。
   HEAD でも `migration/docs` で起きる。CLI-01 の差分比較でも新旧同じ panic
2. 改訂履歴の表（`ddq rev build` が書く `history.qmd`）は修正内容（note）を Markdown としてそのまま載せる。メモに `C:\docs\a.qmd` と
   書くと `\docs`・`\a` が TeX の命令とみなされ、表には `C:.qmd` と出る（新旧同じ。DATA-01 の相互確認で見つけた）。
   パスは `` `C:\docs\a.qmd` `` か `C:\\docs` と書けば出る
