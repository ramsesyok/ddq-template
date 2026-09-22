# DDQ Revision 実装仕様書

`extensions/ddq-revision/` の版 2.3.0 を解析した Quarto book。本文 7 章と解析根拠付録を含む。

- 入口: [index.qmd](index.qmd)
- 実行記録: [analysis/execution.md](analysis/execution.md)
- 対象ソース: [analysis/source-sha256.json](analysis/source-sha256.json)（拡張 28 件 + 契約の相手である `ddq` 側 4 件）
- 設計（なぜそうしたか）: [../revision-study.md](../revision-study.md)

リポジトリルートから機構ファイルを配置して HTML を生成する。

```powershell
cli\target\release\ddq.exe update docs\revision-impl
cli\target\release\ddq.exe html  docs\revision-impl
cli\target\release\ddq.exe pdf   docs\revision-impl
```

`ddq` は同じテンプレート版でビルドしたものを使用する。図は Lua フィルタが扱う通常の
`mermaid` コードブロックを使用し、`execute.eval: false` を維持する。

出力は `_book/index.html`。機構ファイル、`_book/`、図キャッシュは
リポジトリの除外規則に従って管理対象外とする。

## 基準版を更新するとき

1. `python`（リポジトリルート）でソースハッシュを採り直す（`analysis/source-sha256.json`）
2. `extensions/ddq-revision/` で `npm run verify` と `npm run test:host` を流し、
   結果を `analysis/execution.md` に書き足す
3. 本文の版・基準コミット・解析日（`index.qmd`・`_quarto.yml`・付録冒頭）を揃える
