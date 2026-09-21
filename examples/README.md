# examples — サンプルの設計書リポジトリ

本テンプレートの記法の実例と、様式を変更したときの確認に使うサンプル文書を、
設計書リポジトリと同じ形（執筆フォルダを並べる）で置くフォルダである。

| 執筆フォルダ | 内容 |
|---|---|
| `docs/` | 受注管理システム 基本設計書（IPO 図・大きな表・横向きページ・ER 図などの実例。図は主に mermaid） |
| `plantuml/` | 受注管理システム プログラム概要設計書（`docs/` の基本設計書を上流とする内部仕様。図はすべて PlantUML: ユースケース・コンポーネント・配置・シーケンス・アクティビティ・状態遷移・クラス・オブジェクト・ER・タイミング・マインドマップ・WBS。12 章は記法例の付録） |

サンプルを増やすときは、ここに執筆フォルダを足す（`_quarto.yml` を持つフォルダ）。
`ddq release --with-sample` は `docs/` をリリースに同梱する。

ビルドはリポジトリのルートから。

```bat
cli\target\release\ddq.exe update examples\docs
cli\target\release\ddq.exe pdf    examples\docs
cli\target\release\ddq.exe html   examples\docs
```

`plantuml/` は PlantUML サーバが要る。LAN のサーバが無ければ、`cli/vendor/plantuml.jar`
（git 管理外。取得手順は `cli/vendor/README.md`）を環境変数で指すと、`ddq pdf` / `ddq html` /
`ddq diagrams` が内部でサーバを上げる。`diagrams/context.puml` は静的な図の例で、
`ddq diagrams` で `context.svg` に変換したものをコミットしている。

```bat
set DDQ_PLANTUML_JAR=%CD%\cli\vendor\plantuml.jar
cli\target\release\ddq.exe update   examples\plantuml
cli\target\release\ddq.exe diagrams examples\plantuml
cli\target\release\ddq.exe pdf      examples\plantuml
cli\target\release\ddq.exe html     examples\plantuml
```

各執筆フォルダ直下の機構ファイル（`design-doc.lua` など）と `design-doc.pdf` は、原本が
このリポジトリの `template/` にあるため git 管理しない（ルートの `.gitignore`）。
