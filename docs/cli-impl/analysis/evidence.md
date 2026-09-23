# 根拠収集メモ

すべて基準コミットと調査日のワークツリーに対する静的確認。実行観測は別途記録する。

- cli/src/main.rs::Cli/Command/main: トップレベル12コマンド（2.3.0 で tag・rev を追加）、mermaid は help 非表示。正式名 ddq。
- cli/build.rs::main、assets.rs::VERSION: template/VERSION と Cargo.lock をビルド時に読む。
- assets.rs::MECHANISM/PDF_SIDE/SCAFFOLD: 機構5ファイル、PDF4ファイル、repo/content の雛形を埋込む。
- writing_folder.rs::resolve/find_all/ensure_encodable: docs 既定、_quarto.yml 存在検査、5種類の探索除外、Windows ACP 検査。
- quarto.rs::render/run: DDQ_BIN を子プロセスへ渡す。子の失敗は anyhow error、標準入出力を継承。
- mermaid/mod.rs::choose_engine/render_jobs: 自動は browser 発見時に選択、なければ merman。実行失敗時の再選択はない。入力は全件読んだ後、結果ごとに書込む。
- mermaid/merman_engine.rs::render_one: resvg_safe と my-svg を指定。
- zip_archive.rs::create/verify: Deflate、ファイルのみ、ソート、トップフォルダ付き。検証はエントリ数で内容一致ではない。

## 正式な根拠台帳

収集完了後の安定IDは `../appendix-evidence.qmd` のE-0001～E-0023に統合した（E-0020～E-0023は2026-09-23の更新で追加）。
同付録に各主張の確度・情報源種別・パス・シンボル・条件を収録している。
追加の呼出関係と資料差異は architecture-behavior.md、未確認は unknowns.md、
動的観測は validation.md、対象ファイルの識別は source-sha256.json を参照する。

## 2026-09-23 の追加（2.3.0～2.4.0）

- doc/project.rs::chapters_of: `chapters:` の字下げ範囲だけを読む。`appendices:` は範囲外（一時リポジトリで確認）。
- doc/units.rs::scan: 見出し・.tbl・.ipo・パイプ表キャプション・#fig- div。Warning::BareFigure は生成箇所なし。
- doc/labels.rs::candidate: sha1(パス・改行・種別・改行・文言) 先頭6→8→10→40桁。
- doc/diff.rs::units_of: パイプ表ユニットの本文は空。表の行は見出しに帰属（一時リポジトリで確認）。
- commands/rev.rs::diff_cmd: write 判定が json より先。today() は UTC 日付。
- doc/gitsrc.rs: `-c core.quotepath=off`、ls-tree で存在確認後に git show。
- commands/release.rs::build_extensions: extensions/ 直下の package.json を持つフォルダすべて、フォルダ名順。
