# 対象台帳

版・調査条件は progress.md を参照。`template/VERSION` は 2.4.0（初版 2.2.1）、Cargo package version は 0.0.0。
実行表示版は build.rs → DDQ_VERSION → assets::VERSION → clap の経路で決まる。

| 範囲 | 深度・扱い |
|---|---|
| cli/src/**/*.rs、cli/build.rs、Cargo.toml/lock、.cargo/config.toml | 全体構造と主要経路を追跡 |
| cli/tests、cli/tools/regress.py | テストの期待・実行条件を調査 |
| cli/DESIGN.md、plantuml-study.md | 現行コードと照合する補助資料 |
| template/ | 埋込みと Lua・Quarto・Typst の I/O 境界を調査。組版内部の詳細は対象外 |
| .github/workflows、extensions/*/package.json、docs/manual | ビルドと配布の境界を調査 |
| cli/src/doc/**、commands/tag.rs、commands/rev.rs | 2.3.0 追加。論理文書・ラベル・差分・改訂ファイル・Git 呼出しを追跡 |
| cli/target | 生成物。ソース根拠として利用しない |
| cli/vendor/plantuml.jar、template/vendor/mermaid.min.js、Cargo 推移依存 | 版・配布・API 境界のみ。OSS 内部全量の解析は対象外 |
| extensions/*/src、migration、examples の全本文 | CLI 本体ではない。呼出しやテスト入力の範囲のみ |

ルートと対象配下に適用される AGENTS.md は検出されなかった（追跡一覧および cli/docs 配下）。
rg.exe は起動不可のため Git 追跡一覧と PowerShell の探索を使用。
Git はプロセス限定の safe.directory 指定を使い、永続設定は変更していない。
