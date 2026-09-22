# 挙動・根拠・残件

## 根拠台帳

正式文書の E-ID は appendix-evidence.qmd に集約する。各根拠は基準コミットのパスとシンボルで再確認できる。全追跡対象の一覧は inventory.txt、解析時実体の SHA-256 は source-sha256.json。

## 変換の確認

- grid: 罫線の + の表示桁の和集合から列を決定。内容行の | の欠落が colspan、区間の空白が rowspan。最初の = 境界までがヘッダ。
- 出力は内容の表示幅とセル物理行数を計算。列の widths 属性はソース罫線の桁数とは別。
- gridCellLines はリスト前の空行補完・継続字下げ・ハード改行処理。読み戻しはソフト改行を空白にし、補完空行を畳む。
- コード span 保護は grid 読込にのみ使用。pipe の cellTextFromMarkdown は保護なし。
- mergeCols: 指定順から prevcol を作るが実評価は列位置左→右。候補列が5列以下の場合は順列も探索。モデルの生文字列比較と Lua の stringify 比較は同値でない。
- parseWidths は符号を含まない正規表現で数値を抽出するため、入力の負号を保持しない。UI からの負幅は Apply 検証で拒否。
- 本文の span は座標が0始まり、merge-cols 属性だけ1始まり。モデル version は文書の競合検出に使わない。

## 状態・入力

- Session は単一グローバル。Apply は await を含み、実行中の多重送信抑止や session ID のメッセージ照合はない。実機競合未確認。
- init で履歴再初期化。選択、編集ドラフトは別状態。getState/setState は型にあるが呼び出しなし。
- 結合時の本文は可視非空セルを走査順に重複排除して LF 連結。解除は元本文を配り直さない。
- 行列挿入は結合内部なら span を伸ばす。アンカー削除は隣の残存セルへ本文/span を継承。最終行/列削除は no-op。
- TSV は全体置換・部分貼付ともセル text を trim。HTML・元の結合は扱わない。
- 分割表で caption/label/unnumbered は readonly、列幅 UI は編集可能だが div を置換しないため widths に反映しない。

## 観測された境界事例

analysis/probe.cjs はローカル TypeScript を CommonJS にメモリ内変換し、純粋処理だけを呼ぶ。probe-results.json に実測を保存。

1. 改行を含む merge-cols 出力を開くと、grid 入力には expandMergeCols が呼ばれず未結合で戻る。
2. pipe 内のコード span `<br>` が LF に変わる。
3. widths="-20,80" の parseWidths は [20,80]。
4. Markdown コード fence 内にある未終了 tbl 開始例でも全文 fence 走査が失敗する。
5. 分割表でも単行・非結合・ヘッダ1行は pipe 出力（常に grid ではない）。

## 依存・ビルド

package-lock.json v3 とインストール実体の直接依存を dependencies.json に記録。React/ReactDOM も devDependencies だが Webview にバンドルされる。npm audit --omit=dev の成否だけで配布 JS 全体の監査済みとはみなせない。EOL、推移依存の全ライセンス、VSIX 実体の収録物は未確認。

## 未確認・次の確認

- U-0001: VSCode の IME、Apply 中の文書切替、多重 Apply、再表示の実機確認。sample/tables.qmd と拡張ホストデバッガを使用。
- U-0002: CRLF 文書への実 WorkspaceEdit と保存後の改行/BOM保持。純粋関数の改行テストと区別。
- U-0003: 装飾 Markdown / 不正 merge-cols 値と Lua の差。双方の同一入力比較が必要。
- U-0004: Unicode 全域、極端な表サイズの UI 応答時間/メモリ。既存 perf テストは限定入力。
- U-0005: VSIX のインストール、最小対応 VSCode、Node20/22 CI 再現、依存監査の実行。
- U-0006: 生成書籍の目視確認。HTMLリンク・図の構造検査とは区別。
