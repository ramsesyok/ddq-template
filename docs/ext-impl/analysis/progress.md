# 解析記録

- 対象: extension/ の DDQ Table Editor 2.2.1。
- 基準: b79bfed4e81f7942bcfc1c5ba19aa1a69df310e5、調査日 2026-09-21。
- 正本: ソースとビルド設定。requirements.md / README.md は意図を照合する資料。
- 調査範囲: 拡張ホスト、文書範囲検出、モデル、記法変換、React Webview、テスト、配布定義、template/design-doc.lua の結合処理。
- 除外: node_modules / out の全コード解析、VSCode 本体、CLI 全体、テンプレートの表以外の機能。

## 棚卸し・構造

- extension.ts: コマンド登録、単一パネルと Session、メッセージ、WorkspaceEdit、CSP。
- markdown-document/: fence 全文走査 → カーソルの tbl パート / 素の表 / 新規を選択 → 置換範囲を作成。
- model/: TableModel、正規化・検証、結合、行列操作、選択、履歴、TSV。
- formats/: pipe/grid/tbl/mergeCols の解析と出力。UI と共用される純粋処理。
- webview/: TableEditor が履歴とモデル、GridView が選択・ドラフト・IME 用 textarea を保持。
- esbuild.mjs / vite.config.mts: ホスト CJS と Webview IIFE を別々にバンドル。
- vitest.config.mts: Node 環境。Webview はテスト対象から除外。

## 確認済みの重要経路

- open: 文書を LF 分割、findEditTarget、document.version を保持、ready → init。
- Apply: 文書版確認 → 正規化 → 幅補完 → 検証 → 警告モーダル → 再確認 → WorkspaceEdit → 対象再検出 → init/applied。
- 単一 tbl は div 全体、複数パートは選択パート本体だけを置換。Apply はファイル保存を呼ばない。
- 通常 pipe 出力は「結合なし・改行なし・ヘッダ1行」の場合。mergeCols 再現可能時は結合展開後の pipe または grid。それ以外は grid。
- scanFences は終了コロン数と開始コロン数の大小を比較しない。文書全体の不整合で検出を中止する。
- Webview は init ごとにモデル履歴を作り直す。セル編集ドラフトは確定時に1回反映。

## 調査完了と残件

- grid の境界判定、cellText の Markdown 往復、mergeCols と Lua の整合を確認し findings.md に記録。
- モデル操作、依存の解決版、CI を確認し、型チェック・168テスト・build・offlineを実行。validation.mdを参照。
- 根拠整理後に本文7章、付録、Quarto設定、READMEを作成。cross-check.mdに章間照合を記録。
- 単なる型定義と実行時検証を区別した。VSCode UI の実機確認などの残件は第7章のU-IDに接続。
