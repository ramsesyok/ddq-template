# AGENTS.md

## プロジェクト概要

`design-doc-quarto-template`（ddq）の設計書に、見出し・表・図の単位で改訂履歴を付ける
VSCode 拡張（`ddq-revision`）。このリポジトリの `extensions/ddq-revision/` に置き、
`ddq release` がリリース一式へ VSIX として同梱する（`../../cli/src/commands/release.rs`）。

**設計の正は [docs/revision-study.md](../../docs/revision-study.md)（§4.4・§5.8）。実装前に必ず読むこと。**

## 守ること

- **qmd を解釈しない。** 文書の走査・ラベルの候補・差分はすべて `ddq` が返す JSON に由来する
  （同じ解釈を CLI と拡張の 2 か所に持たない）。この拡張がやるのは画面と `WorkspaceEdit` だけ
- **書き戻しは `WorkspaceEdit`。** ddq にファイルを書かせない（Undo が効き、未保存の
  バッファにも当たるため）
- **通信しない。** `npm run check:offline` が成果物を検査する。Webview の CSP は
  `default-src 'none'`（`connect-src` を書かない）
- `package.json` の `version` は `../../template/VERSION` と同じ値にする（`ddq release` が照合する）
- 画面の配色は VSCode のテーマ変数（`--vscode-*`）だけを使う

## 検証

```bash
npm run verify           # 型検査・テスト・ビルド・オフライン検査・脆弱性検査
node tests/host/run.mjs  # 実拡張ホスト（インストール済みの VSCode）で一覧・書き戻し・Undo を見る
```

Webview の描画は jsdom のテスト（`src/webview/*.test.tsx`）で見る。「パネルが真っ白」は
実拡張ホストの検証でもタブが開くだけで気付けないため。
