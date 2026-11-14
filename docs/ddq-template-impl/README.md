# DDQ Template 実装仕様書

`extensions/ddq-template/` を解析した Quarto book。本文 7 章と解析根拠付録を含む。
旧 `docs/revision-impl/`（DDQ Revision 2.3.0）と旧 `docs/table-editor-impl/`（DDQ Table Editor 2.2.1）を
統合したもので、各章は「共通の基盤」「ラベル付け・改訂履歴」「表エディタ」の 3 節からなる。

- 入口: [index.qmd](index.qmd)
- 設計（なぜそうしたか）: [../ddq-template-study.md](../ddq-template-study.md)、[../revision-study.md](../revision-study.md)
- 統合前の本文: `parts/revision/`・`parts/table-editor/`（各章が include する。見出しは 1 段下げ、
  ID には `rev` / `te` の接頭辞を入れてある。中身は統合前の解析のままで、パスと ID は統合前のもの）
- 統合前の解析資料: `analysis/revision/`（実行記録・ソースハッシュ）、`analysis/table-editor/`（調査メモ・
  ソースハッシュ・検証スクリプト）

リポジトリルートから機構ファイルを配置して HTML・PDF を生成する。

```powershell
cli\target\release\ddq.exe update docs\ddq-template-impl
cli\target\release\ddq.exe pdf    docs\ddq-template-impl
cli\target\release\ddq.exe html   docs\ddq-template-impl
```

## 基準版を更新するとき

1. `extensions/ddq-template/` で `npm run verify`・`npm run test:host`・（VSIX を作って）`npm run test:smoke` を流す
2. 共通の基盤の節と付録冒頭の基準コミット・解析日・実行結果を揃える（`index.qmd`・`_quarto.yml`・`appendix-evidence.qmd`）
3. ラベル付け・改訂履歴や表エディタの処理を変えたときは、該当する `parts/` の節を現在のパスで書き直す
   （統合前の解析のまま残す節には、@tbl-path-map の読み替えが効く範囲だけを残す）
