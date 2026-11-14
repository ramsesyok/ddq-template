# テストの網羅の点検（CLI ddq と拡張 ddq-template）

- 対象：`main` の `bf1e70e`（3.0.0 の開発中。#131 のマージ後）、2026-11-14
- 方法：
  1. 機能を洗い出し、それぞれを確かめているテストを名前で対応付けた（機能カバレッジ）
  2. 行・関数のカバレッジを計測した（コードカバレッジ）
  3. 足したテストは、コードをわざと壊して検出できることを確かめた（変異検査）
- 実行結果
  - CLI：`cargo test`（計測つき、`DDQ_E2E=1`）。単体 88 件と結合 80 件（12 ファイル）がすべて成功した。手元の環境には Quarto・Java・plantuml.jar・Edge・npm がある
  - 拡張：`vitest run`。38 ファイル 376 件がすべて成功した。実拡張ホスト（`test:host`、本物の ddq）25 項目も合格した
  - main の CI（`bf1e70e`）：CLI・同梱版 VSIX のスモーク・拡張のテスト 4 本・実拡張ホスト・VSIX の生成・脆弱性チェックがすべて成功した

## 0. まとめ

| 対象 | 機能カバレッジ（自動テストあり） | うち CI で毎回走る | 行カバレッジ | 関数カバレッジ |
|---|---|---|---|---|
| CLI（Rust、`cli/src`） | **40／40（100%）** | 40／40 | **90.6%**（4,693／5,181 行） | 86.4% |
| 拡張（TypeScript、`src`） | **44／44（100%）** | 44／44 | **81.5%**（2,251／2,761 行）※ | 82.2% |
| 機構（Lua・JS・Typst） | CLI の #37〜#40 に含む | ✓ | 計測できない（出力の正解比較で代える） | — |

※ 拡張の行カバレッジは **vitest の単体テストだけの値**である。実拡張ホスト（`test:host`、VS Code を実際に起動）が通す次の部分は、この数字に入らない。

- 入口（`extension.ts`）と、表エディタ・改訂履歴のパネルの登録（`activateTableEditor.ts`・`activateRevision.ts`）
- 改訂ファイルの編集画面の提供者（`revision/provider.ts`）
- ddq の子プロセスの起動（`ddq/run.ts`）

機能カバレッジで見ると、これらは host で確かめている。

### 経過

| 時点 | CLI 機能 | 拡張 機能 | CLI 行 | 拡張 行 | 主な変化 |
|---|---|---|---|---|---|
| 最初の点検（`aa5ef5f`） | 92.5% | 84.1% | — | — | |
| `76e2dd6`（#125〜#128） | 97.5% | 95.5% | 87.0% | 58.1% | 機構の出力を正解と比べる（#127）、CI の実拡張ホストを本物の ddq で回す（#126）、表エディタの行・列の操作（#125） |
| #129・#130 | 97.5% | 97.7% | 87.0% | 71.1% | `tag`・`project` のテスト（#129）、表エディタの画面（#130） |
| `bf1e70e`（#131） | **100%** | **100%** | **90.6%** | **81.5%** | 残りの穴（release の実ビルド、ダイアログ、メニューの `when`、keep、`--bind` など） |

### テストで見つけた不具合

- #127：英語版 Windows（CP1252）で、IPO 図の「入力」「出力」の見出しが認識されなかった。pandoc が Lua の文字の扱いを利用者のロケールにするので、`lower()` や `%s` が日本語の UTF-8 を壊していた
- #131：PlantUML サーバの宛先に `localhost` を使うと繋がらなかった（`ddq plantuml serve --bind localhost`、`PLANTUML_SERVER=http://localhost:…`。2.2.0 から）。Windows は `localhost` を `::1` から解決するが、PicoWeb は IPv4 でしか待たず、ddq は先頭のアドレスしか試していなかった

### 変異検査

| PR | 対象 | 壊し方 | 検出 |
|---|---|---|---|
| #127 | 機構（design-doc.lua・postprocess-html.js） | 7 | 7 |
| #129 | `tag`・`project` | 11 | 11 |
| #130 | 表エディタの画面 | 18 | 18 |
| #131 | release・pdf・prerender（6）、フィルタの keep（1）、ダイアログ（13）、メニューの `when`（4） | 24 | 24 |

## 判定の記号

| 記号 | 意味 |
|---|---|
| ◎ | 正常系と異常系の両方を自動テストで確かめている |
| ○ | 正常系だけ、または上位の処理を通して間接的に確かめている |
| △ | 生成は通すが中身を検査しない |
| × | 自動テストが無い |

「CI」列の ✓ は、CI で毎回走ることを表す。

## 1. CLI（ddq）

テスト：単体 88 件（`src/` の `#[test]`）＋結合 80 件（`cli/tests/*.rs` 12 ファイル）。CI は `windows-latest` に Quarto 1.9.38・Java 17・plantuml.jar・Edge を入れ、`DDQ_E2E=1` で「環境が無ければ skip」を失敗に変えて回す。

| # | 分類 | 機能 | 主なテスト | 判定 | CI |
|---|---|---|---|---|---|
| 1 | 作成・更新 | `init`（リポジトリ・執筆フォルダ・`.vscode/extensions.json`） | `commands.rs::init_then_add_then_update_all`、`init_rejects_dot_names` | ◎ | ✓ |
| 2 | 作成・更新 | `add`（2 つ目以降の執筆フォルダ） | 同上（重複・目印の無い場所への add の拒否を含む） | ◎ | ✓ |
| 3 | 作成・更新 | `update`／`update --all`（版上げ時の図キャッシュ削除） | `update_prunes_diagram_cache_only_when_the_version_changes`、`update_rejects_folder_without_quarto_yml` | ◎ | ✓ |
| 4 | 作成・更新 | `setup`（機構の版検査） | `setup_rejects_different_template_version`、`version_matches_template_version` | ◎ | ✓ |
| 5 | 共通 | 終了コード（0／1／版不一致の 3） | `commands.rs`（3 と 1 の両方を固定） | ◎ | ✓ |
| 6 | 共通 | 執筆フォルダのパス検査（コードページ・長さ・ドット） | `writing_folder.rs` 単体、`e2e.rs::rejects_path_outside_code_page`、`too_long_paths_are_refused_with_a_reason` | ◎ | ✓ |
| 7 | 発行 | `pdf`（`design-doc.pdf` の取り出し） | `e2e.rs::pdf_and_html_in_ascii_path` / `_non_ascii_path`、`mechanism.rs`、`pdf.rs::exactly_one_pdf_is_taken_from_the_output_dir`（0 件・複数件・大文字の `.PDF`・読めないフォルダ） | ◎ | ✓ |
| 8 | 発行 | `html`（`_book`、全文検索、図の SVG 参照） | 同上 | ○ | ✓ |
| 9 | 発行 | 出力先 `output-dir` とプロファイルへの追従 | `quarto.rs::output_dir_follows_the_project_and_the_profile` | ○ | ✓ |
| 10 | 発行 | mermaid の一括変換（`prerender`） | `prerender.rs` 単体 3 件、e2e の図の数の照合 | ○ | ✓ |
| 11 | 図 | `diagrams`（`.mmd`／`.puml`→SVG、失敗時に古い SVG を消す、`警告:`） | `diagrams_removes_the_old_svg_of_a_broken_mermaid`、`diagrams_explains_that_fence_caches_are_not_its_target`、`plantuml.rs::diagrams_renders_puml_and_rejects_syntax_error` | ◎ | ✓ |
| 12 | 図 | 図キャッシュの環境照合（`identity`・`DDQ_DIAGRAM_CACHE=keep`） | `identity_reports_what_the_svg_header_will_say`、`prerender.rs::cache_is_fresh_only_for_this_environment_unless_kept`、`e2e.rs::diagram_cache_from_another_environment_is_redrawn_unless_kept`（フィルタ側の keep も） | ◎ | ✓ |
| 13 | 図 | mermaid のブラウザ描画（CDP） | `golden.rs::browser_engine_matches_mermaid_cli_geometry`、`browser.rs` 単体 3 件、`faults.rs`（即終了・無応答・後始末・並列・強制終了） | ◎ | ✓ |
| 14 | 図 | mermaid の代替エンジン（merman） | `golden.rs::merman_engine_renders_all_without_foreign_object` | ○ | ✓ |
| 15 | 図 | PlantUML の HTTP 通信 | `plantuml/mod.rs` 単体 14 件（切れた応答・chunked・タイムアウト・`localhost` の IPv4／IPv6 など） | ◎ | ✓ |
| 16 | 図 | `plantuml`（ローカルサーバ） | `plantuml.rs::serve_answers_serverinfo_and_stops_with_parent`、`serve_with_bind_localhost_starts_and_explains_the_url`、`serve_with_an_unusable_bind_fails_with_a_reason`、`faults.rs` の JVM 3 件 | ◎ | ✓ |
| 17 | 図 | 子プロセスの後始末（Job Object） | `job.rs` 単体 3 件、`faults.rs` | ◎ | ✓ |
| 18 | 図 | フォントの指紋 | `fonts.rs` 単体 2 件 | ○ | ✓ |
| 19 | ラベル | `tag list`（`--json`） | `tag.rs::list_finds_units_and_suggests_labels`、`candidates_are_stable_between_runs`、`list_prints_a_readable_table`（表の形・桁揃え・件数）、`list_unlabeled_shows_only_items_without_a_label` | ◎ | ✓ |
| 20 | ラベル | `tag apply --all`／`--dry-run` | `apply_all_is_idempotent_…`、`dry_run_does_not_touch_files`、`crlf_files_keep_their_line_endings`、`apply_needs_all_or_from` | ◎ | ✓ |
| 21 | ラベル | `tag apply --from`（`original` の照合） | `apply_from_accepts_…`、`apply_from_rejects_a_line_changed_after_list`、`…_without_original`、`…_broken_or_duplicate_labels` | ◎ | ✓ |
| 22 | 文書解析 | 章の順序（`chapters`／`appendices`／include） | `project.rs` 単体 10 件、`rev.rs::appendices_are_part_of_the_document`、`tag.rs::list_warns_when_chapters_cannot_be_read`・`list_warns_about_broken_includes` | ◎ | ✓ |
| 23 | 文書解析 | 見出し・表・図の単位とラベル候補 | `units.rs` 単体 15 件、`labels.rs` 単体 9 件 | ◎ | ✓ |
| 24 | 改訂 | `rev next` | `next_proposes_symbol_and_base`、`next_without_tags_has_no_base`、`rev_ops.rs::shallow_clone_is_refused_…` | ◎ | ✓ |
| 25 | 改訂 | `rev diff`（`--base`・`--strict`・`--json`） | `diff.rs` 単体 13 件、`diff_classifies_every_kind_of_change`、`strict_reports_whitespace_only_changes` | ◎ | ✓ |
| 26 | 改訂 | `rev diff --write`（メモの引き継ぎ） | `write_creates_the_revision_file_and_keeps_notes`、`write_keeps_notes_of_entries_that_left_the_diff` | ◎ | ✓ |
| 27 | 改訂 | `rev build`（`--check`） | `build_makes_the_history_table`、`build_warns_about_empty_notes_and_check_fails`、`build_without_any_revision_file_…` | ◎ | ✓ |
| 28 | 改訂 | 改訂ファイルの読み書き | `revfile.rs` 単体 4 件、`rev_ops.rs::hand_edited_revision_file_keeps_its_notes` | ◎ | ✓ |
| 29 | 改訂 | 環境差（日本語パス＋autocrlf、大きな文書の速さ） | `rev_ops.rs::japanese_path_with_autocrlf`、`large_document_diff_is_fast_enough` | ◎ | ✓ |
| 30 | 配布 | `release` の入力検査（拡張の名前・版、jar、manual） | `release.rs` の `--no-build` の 10 件 | ◎ | ✓ |
| 31 | 配布 | 同梱 ddq.exe のバイト照合 | `bundled_exe_must_be_the_released_ddq`、`real_build_makes_the_manual_and_packages_the_extensions` | ◎ | ✓ |
| 32 | 配布 | zip の作成と中身の一致 | `zip_archive.rs` 単体、`zip_matches_the_stage_byte_for_byte` | ◎ | ✓ |
| 33 | 配布 | 再実行・途中失敗で前の成果物を壊さない | `rerun_replaces_the_previous_stage`、`missing_jar_stops_before_touching_the_stage`、`failing_npm_package_stops_before_touching_the_stage` | ◎ | ✓ |
| 34 | 配布 | THIRD-PARTY-NOTICES の鮮度 | `third_party.rs`、`release.rs` 単体 | ◎ | ✓ |
| 35 | 配布 | 改名の待ち直し（`rename_patiently`） | `release.rs::rename_waits_for_a_file_held_open_inside` | ○ | ✓ |
| 36 | 配布 | 実ビルドの経路（マニュアルの PDF/HTML 作成・`npm ci`・`npm run package:win32-x64`） | `release.rs::real_build_makes_the_manual_and_packages_the_extensions`（`npm ci` は node_modules が無いときだけ。vsce の代わりに `tests/fixtures/release/pack.js`）、`failing_npm_package_stops_before_touching_the_stage` | ◎ | ✓ |
| 37 | 機構 | `.tbl` の PDF 自動分割とキャプション | `table_pagination.rs::table_captions_cover_every_body_page` | ◎ | ✓ |
| 38 | 機構 | 配布 HTML が図を SVG で参照する | `e2e.rs`（`mmd-`・`puml-` の参照と数） | ○ | ✓ |
| 39 | 機構 | `.tbl` の結合（merge-cols・グリッド表）・列幅・分割・採番しない表・IPO 図とその分割・表番号 | `mechanism.rs`（HTML と PDF の描画文字を正解と比べる。フィルタの警告・`did not converge` も失敗） | ◎ | ✓ |
| 40 | 機構 | `postprocess-html.js`（別章・付録の `.tbl`／`.ipo` 参照の補正） | `mechanism.rs`（相互参照の行き先と番号を比べる） | ◎ | ✓ |

**CLI の集計（40 機能）**

| 判定 | 件数 | 割合 |
|---|---|---|
| ◎ | 33 | 82.5% |
| ○ | 7 | 17.5% |
| △ | 0 | 0% |
| × | 0 | 0% |
| 自動テストあり（◎＋○） | 40 | **100%** |

○ の 7 件（#8・#9・#10・#14・#18・#35・#38）は、正常系は通しているが、失敗時の分岐を直接は突いていないもの。

## 2. 拡張（ddq-template）

テストは次の 3 種類。

- vitest：376 件（38 ファイル）
- 実拡張ホスト `test:host`：25 項目
- 同梱 VSIX のスモーク `test:smoke`：3 項目

CI の `test:host` は CLI ジョブと同じ版の**本物の ddq** と Quarto で回る（`DDQ_HOST_REAL=1`。ddq が見つからなければ失敗にする）。

| # | 分類 | 機能 | 主なテスト | 判定 | CI |
|---|---|---|---|---|---|
| 1 | 基盤 | ddq の解決（設定 → 同梱 → `PATH`） | `ddq/locate.test.ts` 4 件、`test:smoke`（同梱に決まる） | ◎ | ✓ |
| 2 | 基盤 | ddq の起動・停止（`running`、`taskkill /T`） | `ddq/run.test.ts` 5 件、`dialogs.test.ts`（キャンセルで止める） | ○ | ✓ |
| 3 | 基盤 | 同じ執筆フォルダの同時実行の拒否 | `ddq/lock.test.ts` 5 件、`dialogs.test.ts`、host（html 中の diagrams が busy） | ◎ | ✓ |
| 4 | 基盤 | 版不一致（終了コード 3）→ update の案内 | `ddq/version.test.ts` 5 件、`dialogs.test.ts`（update して再実行・押さない・手で直した・文書が新しい・update 失敗）、host（mismatch → update → ok） | ◎ | ✓ |
| 5 | 基盤 | Quarto の検出 | `quarto.test.ts` 4 件 | ○ | ✓ |
| 6 | 基盤 | 執筆フォルダの探索と右クリックメニューのコンテキスト | `folders.test.ts` 6 件、`menus.test.ts` 7 件（package.json の `when` を setContext の値で評価） | ◎ | ✓ |
| 7 | 基盤 | 旧拡張（ddq-revision／ddq-table-editor）の検出 | `legacy.test.ts` 4 件、host（旧 ID のコマンドが無い） | ○ | ✓ |
| 8 | コマンド | 設計書リポジトリを作成（`init`） | `init.test.ts` 10 件、`dialogs.test.ts`、host（日本語名または CP1252 の名前、`.vscode/extensions.json`）、`test:smoke` | ◎ | ✓ |
| 9 | コマンド | 執筆フォルダを追加（`add`） | `add.test.ts` 3 件、`dialogs.test.ts`、host（既存の文書に足さない） | ◎ | ✓ |
| 10 | コマンド | PDF を作成 | `build.test.ts`、`dialogs.test.ts`（完了・失敗・キャンセル・実行中・Quarto 不在）、host（章のフォルダから `ddqTemplate.pdf`） | ◎ | ✓ |
| 11 | コマンド | HTML を作成 | `build.test.ts`（`OK: …index.html`）、`dialogs.test.ts`（開く・開けないとき）、host、`test:smoke` | ◎ | ✓ |
| 12 | コマンド | 図を変換（`diagrams`） | `build.test.ts`、`dialogs.test.ts`（警告を見せる）、host | ◎ | ✓ |
| 13 | コマンド | 出力を既定のアプリで開く | `openFile.test.ts` 2 件（コマンドの組み立てだけ） | ○ | ✓ |
| 14 | コマンド | 通知・ダイアログの操作（名前入力・フォルダ選択・ボタン） | `dialogs.test.ts` 31 件（VSCode を `src/testing/fakeVscode.ts` に差し替え、答えを積んで作成・追加・発行のコマンドを最後まで通す） | ◎ | ✓ |
| 15 | ラベル | ラベル一覧のパネル | `TagTable.test.tsx` 7 件、host（パネルが開く） | ◎ | ✓ |
| 16 | ラベル | ラベルの書き戻し（`original` の照合、N-02） | `tagging/labels.test.ts` 19 件、host（WorkspaceEdit、Ctrl+Z、変わった行は飛ばす） | ◎ | ✓ |
| 17 | ラベル | 一覧の取り直しが重なったとき（R-06） | `activateRevision.test.ts` 7 件 | ◎ | ✓ |
| 18 | 改訂 | 新しい改訂を始める（`rev diff --write`） | host | ○ | ✓ |
| 19 | 改訂 | 改訂ファイルの編集画面（Custom Editor の `provider.ts`） | `RevisionEditor.test.tsx` 9 件（画面だけ）、host（メモが文書に入る） | ○ | ✓ |
| 20 | 改訂 | 改訂ファイルの読み書き | `revision/revfile.test.ts` 11 件 | ◎ | ✓ |
| 21 | 改訂 | 差分エディタを開く・箇所へ移る | host | ○ | ✓ |
| 22 | 改訂 | `rev build` との連携 | host | ○ | ✓ |
| 23 | 表エディタ | 編集する表の特定（`.tbl`・パイプ表・グリッド表・素の表） | `findEditTarget.test.ts` 12 件 | ◎ | ✓ |
| 24 | 表エディタ | パイプ表の読み書き | `integration`・`sample`・`pandoc` の各テストを通して間接的に | ○ | ✓ |
| 25 | 表エディタ | グリッド表の読み書き（縦結合を含む） | `gridTable.test.ts` 22 件、`pandoc.test.ts`（本物の pandoc で読み戻す） | ◎ | ✓ |
| 26 | 表エディタ | merge-cols での出力 | `mergeCols.test.ts` 15 件 | ◎ | ✓ |
| 27 | 表エディタ | `.tbl` の属性と出力形式の選択 | `tblAttributes.test.ts` 9 件、`outputFormat.test.ts` 5 件 | ◎ | ✓ |
| 28 | 表エディタ | 分割表のパート編集 | `integration.test.ts`（パートの差し替え、列数の警告） | ○ | ✓ |
| 29 | 表エディタ | 改行コードの保持 | `lineEndings.test.ts` 8 件 | ◎ | ✓ |
| 30 | 表エディタ | 文書への書き戻し | `applyTable.test.ts` 5 件 | ○ | ✓ |
| 31 | 表エディタ | セルの結合 | `mergeCells` を使う 8 ファイルのテスト | ○ | ✓ |
| 32 | 表エディタ | セルの結合解除（`unmergeCell`） | `unmergeCell.test.ts` 6 件 | ◎ | ✓ |
| 33 | 表エディタ | 行・列の挿入と削除（`rowColOps`） | `rowColOps.test.ts` 30 件 | ◎ | ✓ |
| 34 | 表エディタ | セルの選択 | `selection.test.ts` 17 件 | ◎ | ✓ |
| 35 | 表エディタ | TSV のコピー・貼り付け | `tsvClipboard.test.ts` 11 件 | ◎ | ✓ |
| 36 | 表エディタ | 列幅 | `columnWidths.test.ts` 17 件 | ◎ | ✓ |
| 37 | 表エディタ | 元に戻す・やり直す | `history.test.ts` 4 件 | ○ | ✓ |
| 38 | 表エディタ | セルの文字編集・消去 | `cellTextEditing.test.ts` 6 件、`clearCells.test.ts` 3 件 | ○ | ✓ |
| 39 | 表エディタ | Webview の画面（`GridView`・`Toolbar`・`ContextMenu`・`AttributesPanel`・`TableEditor`） | `webview/TableEditor.test.tsx` 22 件（jsdom。選択・編集・ツールバー・属性欄・列幅・メニュー・Undo・クリップボードと Apply の中身） | ◎ | ✓ |
| 40 | 表エディタ | 表エディタのパネルを開く | host（パネルが開く） | ○ | ✓ |
| 41 | 表エディタ | 大きな表の速さ | `perf.test.ts` 8 件 | ○ | ✓ |
| 42 | 配布 | 実行時に通信しない（`check:offline`） | CI のオフライン検査 | ◎ | ✓ |
| 43 | 配布 | 本番依存の脆弱性 0 件（`check:audit`） | CI | ◎ | ✓ |
| 44 | 配布 | 配る VSIX そのものでの動作（同梱 ddq で init → html） | `test:smoke`（CI の vsix-smoke。main への push と手動実行） | ◎ | ✓ |

**拡張の集計（44 機能）**

| 判定 | 件数 | 割合 |
|---|---|---|
| ◎ | 28 | 63.6% |
| ○ | 16 | 36.4% |
| △ | 0 | 0% |
| × | 0 | 0% |
| 自動テストあり（◎＋○） | 44 | **100%** |
| うち CI で毎回走る | 44 | 100%（#44 の vsix-smoke は PR では走らず、main への push で走る） |

○ の 16 件の多くは、改訂履歴（#18・#19・#21・#22）とパネルを開く処理（#40）のように、実拡張ホストが正常系だけを通しているもの。

## 3. コードカバレッジ

### 3.1 CLI（Rust）

計測方法

- `RUSTFLAGS="-C instrument-coverage"` を付けて `cargo test` を回した（`DDQ_E2E=1`）
- 結合テストが起動する `ddq.exe` の子プロセスも計測に入れた
- 集計には、手元の `C:\llvm-mingw` の llvm-profdata／llvm-cov（LLVM 22.1.6。rustc の LLVM 22.1.8 と同じ系統）を使った
- stable の rustc では分岐カバレッジは取れないので、行・関数・領域だけを測った

| モジュール | 行 | 関数 | 領域 |
|---|---|---|---|
| `doc/`（文書解析・差分・改訂ファイル・ラベル） | 94.7%（1,688／1,782） | 95.5% | 95.2% |
| `commands/`（各サブコマンド） | 89.9%（1,238／1,377） | 76.7% | 87.2% |
| `mermaid/`（ブラウザ・merman・一括変換） | 89.4%（546／611） | 86.5% | 88.4% |
| `plantuml/`（HTTP・サーバ） | 83.2%（672／808） | 85.0% | 84.1% |
| その他（`job`・`quarto`・`writing_folder`・`fonts`・`zip_archive`・`assets`・`main`） | 90.8%（535／589） | 83.8% | 88.1% |
| **合計** | **90.6%（4,693／5,181）**（`76e2dd6` は 87.0%） | **86.4%**（同 82.5%） | **89.8%**（同 86.4%） |

（モジュールの行数の和が合計と少し違うのは、合計に `build.rs` が生成したコードなどが入るため）

今回テストを足して上がったファイル

| ファイル | `76e2dd6` | `bf1e70e` |
|---|---|---|
| `commands/tag.rs` | 62.8% | 96.7% |
| `doc/project.rs` | 73.0% | 98.8% |
| `commands/pdf.rs` | 84.9% | 98.0% |
| `commands/release.rs` | 80.5% | 87.6% |
| `mermaid/prerender.rs` | 94.3% | 97.5% |

85% 未満のファイル（31 ファイル中 4 つ）

| ファイル | 行 | 通っていない主な処理 | 機能表 |
|---|---|---|---|
| `commands/plantuml.rs` | 21.9% | `ddq plantuml serve` は、テストが ddq を kill で止めて終わらせるので計測結果を書き出せない。**計測の限界で、テスト自体はある**（`serve_*` の 3 件） | #16 |
| `doc/gitsrc.rs` | 79.8% | Git から過去版を読む処理の異常系（git の失敗） | #25 |
| `plantuml/mod.rs` | 83.2% | JVM の起動失敗の経路。`faults.rs` が外から確かめているが、ddq を強制終了させるテストは計測結果を書き出せない | #15・#16 |
| `fonts.rs` | 84.2% | フォントの検出の一部 | #18 |

`faults.rs` と `serve` のテストのように ddq を強制終了させるものは計測に入らないので、実際より少し低く出ている。

### 3.2 拡張（TypeScript）

計測方法：`@vitest/coverage-v8` 4.1.12 を使った。package.json は変えずに入れている（`--no-save`）。テスト用の差し替え（`src/testing/`）は分母から除いた。

| ディレクトリ | 行 | 分岐 | 関数 | 備考 |
|---|---|---|---|---|
| `tableEditor/model`（結合・行列・選択・履歴） | 97.5% | 92.1% | 96.8% | |
| `tableEditor/formats`（表の読み書き） | 95.9% | 87.9% | 98.7% | |
| `tableEditor/markdown-document` | 95.8% | 77.4% | 94.6% | `generateTableId.ts` だけ 0% |
| `tagging`（ラベルの書き戻し） | 95.2% | 79.3% | 100% | |
| `tableEditor/webview`（表エディタの画面） | 92.2% | 77.2% | 90.7% | `main.tsx`・`vscodeApi.ts` の起動部分だけ残る |
| `revisionWebview`（改訂履歴の画面） | 79.8% | 81.9% | 76.4% | 同上 |
| `src` 直下（コマンド・発行・作成・追加・執筆フォルダ） | 75.5%（`76e2dd6` は 25.2%） | 69.5% | 71.3% | `build.ts` 93.0%、`init.ts` 96.8%、`add.ts` 89.5%、`folders.ts` 91.0%。`extension.ts` 0%、`openFile.ts` 11.8%、`legacy.ts` 16.7% |
| `ddq`（解決・起動・ロック・版） | 54.4% | 64.0% | 59.4% | `run.ts` 24.4%。子プロセスの起動の本体は host で通る |
| `revision`（改訂ファイル・編集画面の提供者） | 47.7% | 49.7% | 30.2% | `provider.ts` 0%。host で通る |
| `tableEditor`（`activateTableEditor.ts`） | 0% | 0% | 0% | host で通る |
| **合計** | **81.5%（2,251／2,761）**（`76e2dd6` は 58.1%） | **76.9%** | **82.2%** | 単体テストだけの値 |

ロジック（表の読み書き・結合・ラベル）と画面（Webview）は 9 割前後。低いのは、本物の VS Code やプロセスを動かすだけのコード（パネル・Custom Editor の登録、子プロセスの起動、既定のアプリで開く）で、実拡張ホストと同梱版のスモークが代わりに確かめている。実拡張ホストの計測は vitest の数字に入らない。

## 4. 足りないところ

自動テストの無い機能は無くなった。残りは、数字を上げる・見える化するためのもの。

| 優先 | 対象 | 内容 | 手間 |
|---|---|---|---|
| 1 | 計測 | コードカバレッジを CI に載せる（`cargo-llvm-cov` と `vitest --coverage`）。今は手元で測るだけ | 小（ツールのダウンロードが要る） |
| 2 | 計測 | 実拡張ホスト（`test:host`）も計測する。パネル・Custom Editor の登録や子プロセスの起動が数字に入る | 中（c8 などが要る） |
| 3 | 計測 | ddq を強制終了させるテスト（`faults.rs`・`serve`）の計測。止める前に計測結果を書き出す仕組みが要る | 中 |
| 4 | テスト | ○ の項目の異常系（CLI の `html`・`output-dir`・merman・`rename_patiently`、拡張の改訂履歴の host 項目など） | 項目ごとに小 |

## 5. この点検の記録

| PR | 内容 |
|---|---|
| #125 | 表エディタの行・列の挿入／削除と結合の解除の単体テスト |
| #126 | CI の実拡張ホストを本物の ddq で回す |
| #127 | 機構の出力を正解と比べる（英語版 Windows の IPO 図の不具合を修正） |
| #128 | マニュアルの分割 IPO 図の例の修正 |
| #129 | `tag list` の表形式・`--unlabeled`・`apply --from` の拒否・章の並べ方・include の警告。時間待ちのテストが時々落ちる問題の修正 |
| #130 | 表エディタの画面の jsdom テスト |
| #131 | release の実ビルド、`pdf` の取り出し、keep、`--bind`、ダイアログ、メニューの `when`、host の pdf（`localhost` の不具合を修正） |
