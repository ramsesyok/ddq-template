# cli/vendor — リリースに同梱する外部バイナリ（git 管理外）

| ファイル | 何か | 入手先 |
|---|---|---|
| `plantuml.jar` | PlantUML の **MIT 版** jar（`plantuml-mit-<版>.jar` をこの名前に改名したもの）。`ddq release` がリリース直下に `plantuml.jar` として同梱し、`ddq` が exe の隣から探す | <https://github.com/plantuml/plantuml/releases> |

- GPL 版（`plantuml-<版>.jar`）を同梱すると release ZIP に GPL の義務が付くので、MIT 版を使う。
  MIT 版でも UML の全図種と Smetana（Graphviz の Java 移植）は使える（無いのは ditaa / jcckit / sudoku 等）。
- 検証した版は cli/DESIGN.md §13 に記録する。版を変えたらサンプル文書（examples/docs）の PDF で図を確認する。
- 手元の CI・テストでは `DDQ_PLANTUML_JAR` にこのファイルのパスを渡す（`cargo test` は
  `cli/vendor/plantuml.jar` があれば自動で使う。tests/plantuml.rs）。
