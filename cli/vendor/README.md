# cli/vendor — リリースに同梱する外部バイナリ（git 管理外）

| ファイル | 何か | 入手先 |
|---|---|---|
| `plantuml.jar` | PlantUML の **MIT 版** jar（`plantuml-mit-<版>.jar` をこの名前に改名したもの）。`ddq release` がリリース直下に `plantuml.jar` として同梱し、`ddq` が exe の隣から探す | <https://github.com/plantuml/plantuml/releases> |

- GPL 版（`plantuml-<版>.jar`）を同梱すると release ZIP に GPL の義務が付くので、MIT 版を使う。
  MIT 版でも UML の全図種と Smetana（Graphviz の Java 移植）は使える（無いのは ditaa / jcckit / sudoku 等）。
- 置いた jar が公式の MIT 版と同一であることを、GitHub のリリースが公開する SHA-256 と照合する
  （`gh api repos/plantuml/plantuml/releases/tags/v<版> -q '.assets[]|select(.name=="plantuml-mit-<版>.jar")|.digest'`）。
  1.2026.8 は `sha256:3629c9cd017c7f73e6450396eea0040216c7e1eef8473ce33cc1aad469dab2f9`（17,744,555 バイト。2026-09-23 照合）。
- jar を替えたら `python cli/tools/third_party.py` で THIRD-PARTY-NOTICES.md を作り直す（`ddq release` が照合する）。
- 検証した版は cli/DESIGN.md §13 に記録する。版を変えたらサンプル文書（examples/plantuml。PlantUML の全図種がある）の PDF で図を確認する。
- 手元の CI・テストでは `DDQ_PLANTUML_JAR` にこのファイルのパスを渡す（`cargo test` は
  `cli/vendor/plantuml.jar` があれば自動で使う。tests/plantuml.rs）。
