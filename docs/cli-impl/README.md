# ddq ソフトウェア実装仕様書

`cli/` の現行実装を説明する日本語の Quarto book。本文7章と根拠付録を持つ。
基準コミットは `281d62f1338d1de7f6660c1a47e1e84e72edda1b`、テンプレート2.4.0（以降の修正を2.4.2まで反映）、解析日2026-09-23（初版は `02672eb`・2.2.1・2026-09-21）。

- 本文の入口: `index.qmd`
- HTMLの入口: `_book/index.html`（生成物、Git管理外）
- 解析記録: `analysis/`（調査範囲、根拠、未確認事項、ハッシュ、検証）

リポジトリ共通の様式を使用する。リポジトリルートで、対象版からビルドしたddqを使って生成する。

```powershell
Set-Location cli
cargo build --locked --release
Set-Location ..
cli\target\release\ddq.exe update docs\cli-impl
cli\target\release\ddq.exe html docs\cli-impl
```

機構配置後は `quarto render docs/cli-impl --to html` でもHTMLを生成できる。
`ddq html` はMermaidをSVG化する配布用経路、素のQuartoは通常クライアント描画となる。
この文書ではコードブロックの実行を無効にしている。

ソース更新時は、該当章と `appendix-evidence.qmd` のパス・シンボル・版を照合し、
`analysis/source-sha256.json` と検証記録を更新する。未実行のテストを成功として扱わない。
生成HTMLの検証結果と今回実行したCLIテストは `analysis/validation.md` を参照。
