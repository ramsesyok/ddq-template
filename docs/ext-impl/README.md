# DDQ Table Editor 実装仕様書

`extensions/ddq-table-editor/`（解析時は `extension/`）の版 2.2.1 を解析した Quarto book。本文7章と解析根拠付録を含む。

- 入口: [index.qmd](index.qmd)
- 調査メモ: [analysis/progress.md](analysis/progress.md)、[analysis/findings.md](analysis/findings.md)
- 実行確認: [analysis/validation.md](analysis/validation.md)
- 対象ソース: [analysis/source-sha256.json](analysis/source-sha256.json)

リポジトリルートから機構ファイルを配置して HTML を生成する。

```powershell
cli\target\debug\ddq.exe update docs\ext-impl
$env:DDQ_MERMAID_ENGINE = 'merman'
cli\target\debug\ddq.exe html docs\ext-impl
python docs\ext-impl\analysis\check_output.py
```

`ddq` は同じテンプレート版でビルドしたものを使用する。release ビルドを使う場合はパスを読み替える。図は Lua フィルタが扱う通常の `mermaid` コードブロックを使用し、`execute.eval: false` を維持する。

出力は `_book/index.html`。機構ファイル、`_book/`、図キャッシュはリポジトリの除外規則に従って管理対象外とする。本文の基準版を更新する場合は、解析メモと根拠、依存情報、ソースハッシュを合わせて見直す。
