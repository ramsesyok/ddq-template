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
