# 実行確認

日付: 2026-09-21。Windows、Node v24.18.0、npm 11.16.0、Pandoc 3.8.3（Quarto 同梱）。既存 node_modules を使用し、npm ci / npm install は未実施。

| 実行 | 結果 |
|---|---|
| npm run typecheck | 成功、ホスト/Webview の両 tsconfig |
| PANDOC を明示し REQUIRE_PANDOC=1 で npm test | 17 ファイル、168 テスト成功。スキップなし。Vitest 4.1.10、表示時間1.45秒 |
| npm run build | 成功、esbuild/Vite で host と Webview を生成 |
| npm run check:offline | 3生成物と CSP の検査成功 |
| node docs/ext-impl/analysis/probe.cjs | 成功、probe-results.json 参照 |

ビルドの初回は sandbox による親ディレクトリ参照拒否で失敗。権限を付けた同一ビルドの再実行で成功。npm の PATH 上のユーザー側ランチャーに不整合があり、C:/Program Files/nodejs を PATH の先頭に指定して実行した。ソースの修正はしていない。

VSCode Extension Development Host、VSIX 配布、npm audit、CI 4構成は未実行。ユニットテスト成功を GUI 操作の成功とは扱わない。

## 文書の検証

- software-implementation-spec の check_spec.py: 成功。7章と付録の登録、空節、アンカー、内部リンクを検査。
- ddq 2.2.1 update: 機構ファイルを配置。
- DDQ_MERMAID_ENGINE=merman で ddq html: 成功。Quarto 1.9.38、9ページを生成。
- check_output.py: 成功。9 HTMLページ、16表、2 SVG、ローカル参照欠落0件、ソースハッシュ変化0件。output-audit.jsonに保存。
- HTML内の章番号を確認。図はSVGのXML構造とラベル要素を検査。
- git diff --check: 成功。既存ファイルの差分は docs/README.md の一覧1行。
- ブラウザでの目視確認は未実施。以前の同タスクでローカルHTMLへのアクセスがブラウザのURL制約で拒否されたため、代替経路による回避は行っていない。機械検査と視覚検査を区別する。
