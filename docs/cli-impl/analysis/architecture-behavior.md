# 構造・挙動・インタフェースの調査記録

Phase 2～4 の記録を統合する。確度は静的確認済み。名称は Rust のモジュール／シンボル名を正本とする。

## 名称・責務・実行単位

| 名前 | 起動／利用元 | 責務・境界 |
|---|---|---|
| ddq / main::Command | ユーザー、Lua の pandoc.pipe | 単一 binary の同期 dispatch。lib ターゲットなし |
| commands | main | init/add/update/setup/html/pdf/diagrams/plantuml/release/mermaid |
| assets | build.rs、commands、mermaid | template 埋込み、版、雛形と機構の配置 |
| writing_folder | main、init/add/html/pdf/update | パス解決・ACP 検査・探索 |
| quarto | add/html/pdf/release | current_dir と DDQ_BIN、外部コマンド終了の検査 |
| mermaid::browser | mermaid::render_jobs | 一時 profile、1 バッチ1ブラウザ・1図1 target、CDP/WebSocket |
| mermaid::merman_engine | mermaid::render_jobs | 同一プロセスの Rust Renderer、resvg_safe |
| plantuml::Session | html/pdf/diagrams | External/Local/None の所有権と URL 引渡し |
| plantuml::LocalServer | start_local/serve | Java 子、stderr 読取スレッド、Drop kill/wait、Windows Job Object |
| zip_archive | release | 配布フォルダを Deflate ZIP 化、エントリ数検証 |

## コマンドと副作用

- init: repo ファイルは absent-only、add::populate を直接呼び既存原稿を保持する。ただし update 経由で機構は上書き。git init は表示するだけ。
- add: ancestor の .gitignore/.git/.svn/.hg を探索。既存 _quarto.yml を拒否。populate は原稿→機構→任意の HTML 疎通確認。失敗時の巻戻しなし。
- update: 機構5本を書いた後、版 + LF を保存。--all は _quarto.yml のある全フォルダ（除外5名）をソートして順次更新。ddq 専用判定なし。
- setup: trim した版一致と機構の文字列完全一致を確認し、PDF4本を上書き。
- html/pdf: ACP 検査、版・機構検査、PlantUML Session、quarto render。PDF は publish profile、_book 直下の PDF が一つだけの場合 design-doc.pdf へ copy。
- diagrams: diagrams 直下、小文字拡張子、mmd-/puml- 接頭辞を除外。Mermaid→PlantUML、前者失敗で後者未実行。版照合なし。
- mermaid: 入出力同数、全入力読込、engine 全処理、成功分保存、図単位エラー集約。古い出力削除なし。CDP の全体エラーなら結果配列を返す前に中断。
- release: CWD が repo であることを検査。通常 manual update→pdf→html、VSIX作成。stage は再帰削除後再作成、jar/案内PDF/手引等をコピーし ZIP 化。--no-build でも案内 PDF は作成。埋込版と extension version を照合するが、ディスクの template/VERSION の内容は比較しない。

## 入出力・共有状態

- IF-01 ddq→Quarto: render、CWD=執筆フォルダ、DDQ_BIN=self、HTML では MERMAID_SVG=1、Session の URL。
- IF-02 Lua→ddq: pandoc.pipe、mermaid -i/-o/-c/-b、終了成功かつ出力存在を検査。Lua は図ごと起動、CLI の複数入力 API をバッチ利用しない。
- IF-03 ddq→browser: file:/// の UTF-8 パーセント符号化、JSON の source、CDP。60秒定数は各待機等に適用、総時間上限ではない。
- IF-04 ddq/Lua→PlantUML: GET /serverinfo、POST /render {source, options:[-tsvg,-charset,UTF-8]}。Rust は TcpStream、Lua は curl。Rust URL は http の host/port（パスなし）。HTTP200でも X-PlantUML-Diagram-Error を拒否。先頭 < だけの簡易 SVG 確認。
- IF-05 release→ZIP: stage file群、トップフォルダ、順序sort、Deflate。verify はエントリ数のみ、書込み途中の全エラーを削除するわけではない。
- _quarto.yml はパス目印／設定、原稿、埋込5機構、4PDF補助、SVG/ソースcache、配布成果物で構成。業務DB/GUI/RPCは CLI 自前コードに見当たらず。外部実装のDB利用までは判断しない。
- Lua cache: Mermaid は raw code の SHA-1先頭8桁、PlantUML は設定連結後sourceの先頭8桁。版／engine／サーバ版はキーに含まない。存在検査のみ。
- 同じ執筆フォルダの複数 ddq プロセス間ロックなし。E2Eの QUARTO_LOCK はテスト実行内だけ。

## Phase 6 照合と差異

- cli/DESIGN.md §7.5 は cd cli 後に release 実行の例。release::run は repo root 必須。利用マニュアル17章と実装を採用。
- DESIGN §7.2 の thiserror は想定。Cargo.toml の直接依存にはない（推移依存として lock に存在）。
- DESIGN §5.4 と実装の merman header は表記が異なる。実装は merman=<version>。
- DESIGN の「失敗図の出力を作らない」は新規出力の説明。古い SVG の削除／無効化は実装されていない。
- Rust configured_server は最上位設定の1候補だけ、Lua puml_candidates は全候補順次試行。優先順位の見かけが同じでも失敗時の動作は異なる。
- STARTUP_TIMEOUT=30秒は stderr.read_line のブロックを中断しない。stderr drain は Vec に蓄積する。

Phase 1～7 の主要な経路と境界は調査済み。外部実装・長時間・異常終了の観測は unknowns.md に分離。
