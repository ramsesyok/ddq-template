# examples — サンプルの設計書リポジトリ

本テンプレートの記法の実例と、様式を変更したときの確認に使うサンプル文書を、
設計書リポジトリと同じ形（執筆フォルダを並べる）で置くフォルダである。

| 執筆フォルダ | 内容 |
|---|---|
| `docs/` | 受注管理システム 基本設計書（IPO 図・大きな表・横向きページ・ER 図などの実例） |

サンプルを増やすときは、ここに執筆フォルダを足す（`_quarto.yml` を持つフォルダ）。
`ddq release --with-sample` は `docs/` をリリースに同梱する。

ビルドはリポジトリのルートから。

```bat
cli\target\release\ddq.exe update examples\docs
cli\target\release\ddq.exe pdf    examples\docs
cli\target\release\ddq.exe html   examples\docs
```

各執筆フォルダ直下の機構ファイル（`design-doc.lua` など）と `design-doc.pdf` は、原本が
このリポジトリの `template/` にあるため git 管理しない（ルートの `.gitignore`）。
