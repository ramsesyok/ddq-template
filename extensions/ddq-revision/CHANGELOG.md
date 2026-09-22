# 変更履歴

## 2.3.0 — 未リリース

最初の版です。見出し・表・図の **Quarto ラベルを一覧して付ける**画面を提供します
（改訂履歴のキーになります）。

- コマンド **DDQ Revision: 見出し・表・図のラベルを一覧する**（`ddqRevision.tags`）
- 候補ラベルはその場で書き換えられます。書き戻しは `WorkspaceEdit` なので Ctrl+Z で戻せます
- 設定 `ddqRevision.ddqPath`（`ddq.exe` が `PATH` に無いとき）

改訂履歴そのもの（`revisions/rev-<記号>.yml` の編集画面）は次の版で追加します。
CLI 側は先に揃っているので、それまでは `ddq rev diff --write` と `ddq rev build` で作れます。
