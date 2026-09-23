# 実行観測

調査日 2026-09-21、Windows。CWD はリポジトリ内 cli（特記のない場合）。
Python 3.12.0、Quarto 1.9.38、Cargo 1.98.1 (797e8a9bc 2026-08-05)。

| コマンド | 結果 | 範囲と限界 |
|---|---|---|
| cargo test --locked --offline --bin ddq --test commands | 終了0、unit 10 / integration 4 成功 | 外部描画なし。ACP unit は CP932 以外では return するので成功だけで全ACP検証済みとはしない |
| cargo test --locked --offline --test golden merman_engine_renders_all_without_foreign_object -- --exact | 終了0、1成功、1対象外 | 22 fixtures のSVG生成・foreignObject不在・text存在。幾何一致を要求しない |

Cargo のホーム canonicalize 警告は出たが、上記のビルドとテストは完了した。
ブラウザ golden、PlantUML統合、全E2E、release、regress.py は未実施。

## 最終版での確認

調査中のHEAD更新後、コミット `02672eb3054d0f6b7139e21b31d91aba7dd57595`、
テンプレート2.2.1で上表の2コマンドをcliディレクトリから再実行した。
結果は同じく10+4+1件成功。最終ログは test-commands.log と test-merman.log。
その前にrootから --manifest-path 指定でも14件成功したが、cli/.cargo/config.tomlの適用条件を
揃えるため、正式な確認結果はcliをCWDとする再実行を採用した。

`ddq --version` は2.2.1、`ddq update docs/cli-impl` は終了0。
`DDQ_MERMAID_ENGINE=merman` を設定して `cli/target/debug/ddq.exe html docs/cli-impl` を実行し、
Quarto 1.9.38で9ページを生成した（終了0、render.log）。
初回はサンドボックスからQuartoのユーザーキャッシュを参照できず失敗したが、
承認付きの実行で成功した。既存の機構資産はGit管理外の配置ファイルとして保持する。

skillのcheck_spec.pyは成功。analysis/check_output.pyはHTML9ページ、表13個、
図SVG2点のXML・テキストラベル、全ローカルhref/srcと明示アンカーの解決を確認した。
ソースhashの差分0、リンク切れ0。結果はoutput-audit.json。git diff --checkも成功。
図SVGは初回生成のキャッシュを再利用しており、SVGコメントは生成時の2.2.0を示す。
2.2.1への差分に図変換ロジック／設定の変更はなく、本文の基準版と生成履歴を区別している。

ブラウザでfile:///のHTMLを開く操作はURLポリシーで拒否された。
他の経路で回避せず、目視確認は未実施とする。静的検査は画面上の折返し・余白の保証ではない。

## 2026-09-23 の更新（2.4.0）

基準をコミット `281d62f1338d1de7f6660c1a47e1e84e72edda1b`、テンプレート2.4.0 に更新した。
Cargo 1.98.1、Quarto 1.9.38、Python 3.12.0、git 有り。CWD は cli。

| コマンド | 結果 | 範囲と限界 |
|---|---|---|
| cargo test --locked --offline --bin ddq --test commands --test tag --test rev | 終了0、unit 50 / commands 4 / rev 10 / tag 9 成功 | quarto・ブラウザ・Java は不使用。rev は一時 Git リポジトリで実行（skip なし） |
| cargo test --locked --offline --test golden merman_engine_renders_all_without_foreign_object -- --exact | 終了0、1成功、1対象外 | 初版と同じ範囲 |

一時 Git リポジトリ（`chapters:` と `appendices:`、`rev-A` タグ）で debug ビルドの ddq 2.4.0 を実行し、
付録の変更が `rev diff` に出ないこと、パイプ表の行の変更が見出しの `changed` になること、
`rev diff --write --json` で JSON が出ないことを観測した。`tag list docs/cli-impl --json` の `order` にも
`appendix-evidence.qmd` は含まれなかった。UTC 日付の境界（JST 0:00～8:59）は再現していない。

`ddq update docs/cli-impl`（2.4.0）後、`DDQ_MERMAID_ENGINE=merman` で `ddq html docs/cli-impl` を実行し終了0（render.log）。
check_output.py は HTML 9ページ、表14、図SVG3点（rev の流れ図を追加）、ローカルリンク切れ0、
記録済みソースhash（56ファイル。2.3.0 以降の追加ファイルを含む）の差分0を確認した（output-audit.json）。
ブラウザでの目視は初版と同じく未実施。release・E2E・browser golden・PlantUML 統合・regress.py は今回も未実施。

## 2026-09-23 の追補（tag の ID 判定の修正）

`tag` が接頭辞の違う既存 ID を持つ見出しに 2 つ目の ID を足していた不具合を直した（foreign-id／multiple-ids の警告を追加）。
本書 07 章の U-0001～U-0008 に付いていた無効な `#sec-…` を外し、ソースhashを更新した。

## 2026-09-23 の追補（第7章の注意点の修正）

PlantUML の起動待ち・stderr・Job 登録失敗、init の `.`／`..`、ZIP の照合、付録、パイプ表の帰属、
改訂日、`--write --json`、`~~~`、画像の図（bare-figure）を修正した。
`cargo test --locked --offline --bin ddq --test commands --test tag --test rev --test plantuml` は終了0
（unit 61 / commands 5 / plantuml 2 / rev 12 / tag 10）。plantuml は Java 17 とローカル jar で実行した。
merman golden は1件成功。ローカル日付の境界と、無出力 JVM・Job 登録失敗の故障注入は未実施。

## 2026-09-23 の追補（図キャッシュのキー、2.4.1）

design-doc.lua のキャッシュキーを版・設定・エンジン指定を含む16桁にし、ddq update が版上げ時にキャッシュを消すようにした。
cargo test（unit 61 / commands 7 / plantuml 2 / rev 12 / tag 10）と DDQ_E2E=1 の e2e 3件が成功。
docs/cli-impl で、版上げ時の削除（8件）、再ビルドでの再利用、素の quarto render での設定変更・エンジン指定による別キー、
空キャッシュの描き直しを観測した。

## 2026-09-23 の追補（発行時の環境照合、2.4.2）

図キャッシュの先頭コメントにブラウザの実体・フォントの指紋を記録し、発行時に `ddq identity` と照合するようにした。
cargo test（unit 66 / commands 8 / plantuml 2 / rev 12 / tag 10）、DDQ_E2E=1 の e2e 3件、merman golden 1件が成功。
一時的な執筆フォルダで、フォント・ブラウザ・PlantUML の版・ローカルのフォント・描いたサーバの記録を書き換えると
発行時に描き直し、DDQ_DIAGRAM_CACHE=keep とサーバ無しのプレビューでは描き直さないことを観測した。

## 2026-09-23 の追補（U-0004 の一部実行と rev diff の案内）

v2.4.2 の配布で通常の `ddq release` を実行し、50ファイルの ZIP、同梱 ddq.exe の版、日本語名の UTF-8 フラグ、
同梱 ddq.exe での init を確認した（オプション付き・異常系は未実施）。
rev diff の警告を「tag apply で付くもの」と「既存の ID を手で付け替えるもの」に分けた。
cargo test（unit 66 / commands 8 / rev 13 / tag 10）が成功。

## 2026-09-23 の追補（U-0001・U-0002）

U-0001: Windows 11（ACP 932）、Edge 153.0.4234.48、Quarto 1.9.38、Java 17.0.2、PlantUML 1.2026.8、git 2.44 で
`DDQ_E2E=1 cargo test --locked --offline` を実行し全件成功（browser golden の厳格比較、E2E、PlantUML 統合を含む。skip なし）。
U-0002: tests/faults.rs（7件）で故障注入。修正前に、ブラウザの孫プロセスの残存と、成功時も %TEMP% に
ddq-mermaid-* が残ること（1,157件）を観測し、Job Object（src/job.rs）と削除順の修正で解消した。
修正後は全件成功し、browser golden・E2E の実行で ddq-mermaid-* は増えなかった。

## 2026-09-23 の追補（U-0003）

plantuml.jar を公式の plantuml-mit-1.2026.8.jar の digest と、mermaid.min.js を npm の mermaid@11.16.0 と照合し、一致した。
配布物に入る第三者のソフトウェアを棚卸しし（Rust 188 / mermaid 78 / React 3）、THIRD-PARTY-NOTICES.md を作って
ddq release が同梱・照合するようにした。リポジトリ自身のライセンスは未定（保守者の判断待ち）。
