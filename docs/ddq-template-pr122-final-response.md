# PR #122 最終品質検証への対応記録

- 対象: 最終品質検証報告書（ddq-review リポジトリ `docs/reviews/ddq-template-pr122-final-validation-20261026.md`、検証 Head 8159f80）
- 対応日: 2026-10-26
- 対応範囲: §12 の 1（R-02 残存 P1）と 2（R-06 の常設回帰試験）。報告書の M-01〜M-12 は未検証のまま（§4）

## 1. 対応内容

### R-02 残存（P1）— 旧 CLI 指定時の変更済み行への書き戻し

原因は報告書 §10 のとおり。`edit.original` の無い一覧では、拡張がラベルの有無だけを見る互換の分岐に入っていた。

| 変更 | 内容 |
|---|---|
| `extensions/ddq-template/src/tagging/labels.ts` | `lineChanged` を `staleReason` に置き換えた。`original` が無ければ、行が同じでも変わっていても**当てない**（理由「この ddq は照合用の行を返しません。ddq を更新してください」）。互換の分岐は削除。`lacksOriginal` を追加 |
| `extensions/ddq-template/src/activateRevision.ts` | 書き戻しは `staleReason` の理由で飛ばす。一覧を表示した時点で、`original` の無い一覧なら「ラベルを書き戻せません」と警告し、`ddqPath` を空にする（同梱版を使う）よう案内 |
| `cli/src/commands/tag.rs` | `tag apply --from` も `original` の無い編集指示を拒否する（設計 docs/revision-study.md §4.4「適用前に対象行が list 時と同一かを検査」に合わせた。旧版は構造の検査だけで、見出しの文言が変わっても当てていた） |

方針: 古い CLI では同じ行かを確かめる手段が無いので、書き戻しを**安全側で拒否**する。一覧の表示はそのまま使える。同梱の最新 ddq なら従来どおり書き戻せる。

### R-06 — 応答順逆転の常設試験

`extensions/ddq-template/src/activateRevision.test.ts` を追加した（vitest、`npm test`／`npm run verify` と CI で毎回実行）。VSCode API と ddq の応答を差し替え、応答の順番をテストが決める。

- A の取得中に B で開き直し、B → 遅れて A の順に応答 → 画面に出るのは B だけ、題名は B、取得中表示は B の完了で消える
- 遅れて返った A が失敗しても、B の画面にエラーを出さない
- A → B の順でも A を捨てて B を出す
- `original` の無い一覧では警告を 1 回出す（R-02）

### 実拡張ホストの検証（tests/host）

- 最新 CLI のときも、一覧から `original` を除いた「古い ddq の一覧」を作って書き戻させ、変わらない行・段落に変えた行のどちらにも当てないことを確かめる（CI の差し替え ddq でも毎回走る）
- 古い ddq を `DDQ_BIN` に渡したときは、書き戻しの拒否を確かめ、書き戻し成功・Undo の確認だけを飛ばして残りを続ける（従来はここで失敗し、後続 16 項目が未実行だった）

## 2. 検証結果

環境: Windows 11（10.0.26300）、Rust/Cargo（`--offline`）、Node・npm・VSCode・Quarto は報告書 §1 と同じ機械。

旧 CLI は 88a0ae2（報告書の「前回 Head」）を `git archive` で取り出し `cargo build --release --offline` で作り直したもの。SHA-256 は 5EFB4E52…AC65 で、報告書の 338BAB96…2D8B とは一致しない（ビルドしたパスが埋め込まれるため）。同じコミットからの生成で、`tag list --json` に `original` が 0 件（`insert` は 111 件）であることを確認してから使った。

| 試験 | 結果 |
|---|---|
| `cargo fmt --check` / `cargo clippy --offline --all-targets -- -D warnings` | 成功 |
| `cargo test --offline` | **145 成功／0 失敗**（tag 12。新規 `apply_from_rejects_edits_without_original`） |
| `cargo build --release --offline` | 成功（最新 CLI。SHA-256 C787D918…9689） |
| `npm run verify` | **263 成功／0 失敗**（31 ファイル）、型検査・build・offline・audit 0 件 |
| `npm run test:host -- <最新 release ddq.exe>` | **23 項目成功**、exit 0（従来の 22 項目＋古い一覧の拒否） |
| `npm run test:host -- <旧 CLI 88a0ae2>` | **18 項目成功**、exit 0。書き戻しは 0 件で本文は変わらない。報告書で失敗した条件（段落に変えた行）も拒否。後続の改訂履歴・発行・init まで実行 |
| `npm run test:host`（差し替え ddq＝CI と同じ経路） | 9 項目成功、exit 0（古い一覧の拒否を含む。改訂履歴は実物の ddq が要るので従来どおり飛ばす） |
| `quarto render --to html`（docs/ddq-template-impl、docs/manual） | 成功 |

### 試験が欠陥を捕まえることの確認（変異試験）

| 変異 | 結果 |
|---|---|
| `staleReason` を旧来の「ラベルの有無だけ」に戻して build、旧 CLI で `test:host` | **exit 1**。「照合できない一覧で当てた: # 概要 {#sec-137d76}」（報告書の再現と同じ破損）を検出 |
| `refresh` の世代の照合（`latest()`）を常に真にして `activateRevision.test.ts` | **4 件中 3 件失敗**（順序逆転の 3 件。警告の 1 件は対象外） |

変異は検証後に元へ戻し、`git diff` で差分の無いことと再 build を確認した。

注意: `npm run test:host` は `out/extension.js` を作り直さない。ソースを変えたら先に `npm run build:ext`（または `verify`）を通すこと。上の最新・旧 CLI の実行は、修正後の `npm run verify` で作った `out/` で行った。

## 3. 確定不具合と未検証の区別

| 区分 | 項目 | 状態 |
|---|---|---|
| 確定不具合（報告書） | R-02 残存（P1） | **解消**。上の旧 CLI の host 試験と変異試験で確認 |
| 報告書の指摘（回帰試験不足） | R-06 の常設試験 | **対応**。`activateRevision.test.ts`（モック境界。実 VSCode＋遅延 exe での再現＝M-09 は未実施） |
| 未検証（不具合ではない） | M-01〜M-12（エアギャップ、GUI Cancel、同時実行の GUI、Workspace Trust、旧 2 拡張との共存、表の全対話操作、外部ビューア、改訂履歴の GUI 操作、R-06 の実ホスト再現、社内セキュリティ製品、Explorer の動的増減、ほぼ空ルートの init） | **今回も未実施**。確定不具合として扱わず、リリース前に実施するか受容判断を記録する |
| 今回の未検証 | 「ラベルを書き戻せません」警告の GUI 表示 | 単体試験（`activateRevision.test.ts`）でのみ確認。実画面では未確認 |
| 今回の未検証 | `DDQ_E2E=1` の厳格な E2E 再実行、正式 release・VSIX 再生成・実インストール | 未実施（今回の変更は描画・配布の経路に触れていない）。報告書 §12 の 4 のとおり、新しい Head で再生成・再検証が要る |

## 4. 再検証（3de86de）の新規指摘への対応

再検証報告書（ddq-review `docs/reviews/ddq-template-pr122-revalidation-20261026.md`）の §10 の N-01（P2）と N-02（P3）に対応した。

### N-01（P2）— 実ブラウザー描画後に一時フォルダが残る

**再現**（修正前、`real_browser_leaves_no_temp_folder` を 8 並列 × 5 回）: **40 回中 14 回**失敗。残ったものは 2 種類あった。

| 残存物 | 原因 |
|---|---|
| `ddq-mermaid-*`（ddq の一時フォルダ） | Job Object を閉じた（`CloseHandle`）だけでは中のプロセスの終了は非同期で、まだ profile を握っているうちに削除の待ち直し（3 秒）が尽きた。さらに、Job の「動いているプロセス数」が 0 になった時点でも、プロセス自体はまだ終わりきっていないことを単体試験で実測した |
| `%TEMP%` 直下の `*.tmp`（GUID 名）と `Importer_0_4` | ブラウザ自身が `%TEMP%` に作る一時ファイル。強制終了で後片付けされずに残る |

**修正**:

| 変更 | 内容 |
|---|---|
| `cli/src/job.rs` | `Handle::end_all(grace, limit)` を追加。Job 内のプロセスを `OpenProcess(SYNCHRONIZE)` で開き、ハンドルの終了を待つ。`grace` のあいだは自分で終わるのを待ち、過ぎたら `TerminateJobObject` して終わりきるまで待つ |
| `cli/src/mermaid/browser.rs` | ブラウザの `TMP`/`TEMP` を一時フォルダの中（`<一時フォルダ>/tmp`）に向ける。`Browser.close` を受け付けたら最大 5 秒は自分で終わるのを待ち、その後 `end_all`（全体で最大 15 秒）。削除の待ち直しを 5 秒にし、それでも消せなければ stderr に「一時フォルダを消せませんでした（理由）: パス」を出す（黙って残さない） |
| `cli/tests/faults.rs` | `real_browsers_in_parallel_leave_no_temp_folder` を追加（本物の Edge/Chrome を 8 並列。既存の単独試験と同じく、ブラウザが無ければ skip 表示で戻る） |

**修正後**:

| 試験 | 修正前 | 修正後 |
|---|---|---|
| 単独試験の 8 並列 × 5 回 | 40 回中 14 回失敗 | **40 回中 0 回** |
| 単独試験の 12 並列 × 5 回 | — | **60 回中 0 回**（警告の出力も 0） |
| 新設の 8 並列試験を 5 回 | **5 回中 5 回失敗**（`browser.rs`・`job.rs` を HEAD に戻して実行） | **5 回中 5 回成功** |

試験の assert は弱めていない（一時フォルダが空であることを従来どおり要求し、新設試験は警告が出ないことも要求する）。

### N-02（P3）— 旧 CLI 拒否時の書き戻しボタンが汎用エラーになる

**原因**: すべての行を飛ばしたあとも空の `WorkspaceEdit` で `applyEdit` を呼んでいた。インストールした VSCode ではこれが false を返し、飛ばした理由ではなく「書き戻しに失敗しました。」を出していた（開発ホストでは true を返すため host 試験では出なかった）。

**修正**: `extensions/ddq-template/src/activateRevision.ts` の `applyRows` で、当てる編集が 0 件なら `applyEdit` を呼ばずに「書き戻せるものがありません。N 件を飛ばしました: <行（理由）>」を警告で出して戻る。

**試験**: `src/activateRevision.test.ts` に 3 件追加。`applyEdit` を「空なら false」（インストール版の挙動）に差し替えて、古い ddq の一覧／一覧のあとに変わった行では `applyEdit` を呼ばず、エラー通知を出さず、理由付きの警告を出すこと、当てる行があれば従来どおり `applyEdit` することを見る。修正を外すと 2 件が失敗する（変異試験）。

### 回帰試験（修正後の作業ツリー）

| 試験 | 結果 |
|---|---|
| `cargo fmt --check` / `cargo clippy --offline --all-targets -- -D warnings` | 成功 |
| `DDQ_E2E=1 cargo test --offline --no-fail-fast -- --nocapture` | **147 成功／0 失敗／skip 表示なし**（unit 81〈`end_all` の単体 1 件増〉、faults 8〈8 並列 1 件増〉） |
| `cargo build --release --offline` | 成功 |
| `REQUIRE_PANDOC=1 npm run verify` | **266 成功／0 失敗**（31 ファイル）、audit 0 件 |
| `npm run test:host -- <最新 release ddq.exe>` | **23 項目成功**、exit 0 |
| `npm run test:host -- <旧 CLI 88a0ae2>` | **18 項目成功**、exit 0 |
| `npm run test:host`（差し替え ddq＝CI 経路） | **9 項目成功**、exit 0 |

### 未検証

| 項目 | 状態 |
|---|---|
| N-02 のインストール済み VSIX でのボタン操作 | **未実施**。`applyEdit` が空の編集で false を返す挙動は単体試験の差し替えで再現した。実機での再確認は報告書 §12 の 3 のとおり必要 |
| N-01 の非 Windows | Job Object が無いので、ブラウザの TEMP を一時フォルダに向ける部分だけが効く。Linux の実ブラウザでは未確認 |
| VSIX 再生成・実インストール、M-01〜M-12 | 未実施（§3 と同じ） |
