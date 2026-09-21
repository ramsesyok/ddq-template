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
