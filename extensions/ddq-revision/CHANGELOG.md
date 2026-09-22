# 変更履歴

## 2.3.0 — 未リリース

最初の版です。見出し・表・図の **Quarto ラベルを一覧して付ける**画面を提供します
（改訂履歴のキーになります）。

- コマンド **DDQ Revision: 見出し・表・図のラベルを一覧する**（`ddqRevision.tags`）
- 候補ラベルはその場で書き換えられます。書き戻しは `WorkspaceEdit` なので Ctrl+Z で戻せます
- 設定 `ddqRevision.ddqPath`（`ddq.exe` が `PATH` に無いとき）

**改訂履歴の編集画面**（`revisions/rev-<記号>.yml` の Custom Editor）も入りました。

- コマンド **DDQ Revision: 新しい改訂を始める（差分を取る）**（`ddqRevision.newRevision`）
- 見出し・表・図の単位で変更を並べ、1 行ずつ修正内容を書きます
- 差分は VSCode 標準の差分エディタで見ます（旧版は Git から、追加・削除の側は空）
- 「確定する」は表を作り、**次に打つ Git コマンドを案内するだけ**です（コミットとタグは人が打ちます）
