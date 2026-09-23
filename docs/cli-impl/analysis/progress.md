# 解析進捗

対象: `cli/` の ddq（依頼文の dqq に対応）。解析日: 2026-09-21。
基準コミット: `02672eb3054d0f6b7139e21b31d91aba7dd57595`、ブランチ: `main`。
開始時の追跡ファイル差分なし。未追跡 `.claude/` は対象外。

| Phase | 状態 | 成果・残件 |
|---|---|---|
| 1 Inventory | 完了 | Rust 単一 binary、template 埋込み、Java/ブラウザ/Quarto 境界を特定 |
| 2 Architecture | 完了 | 全自前Rustソース、モジュール、プロセス・所有権を追跡 |
| 3 Behavior | 完了 | 作成・更新・発行・図変換・release/serverの正常／異常経路を追跡 |
| 4 Data/IF | 完了 | 配置ファイル、環境変数、Lua・Quarto・CDP・HTTP・ZIPの両端を照合 |
| 5 Dependency | 制約付き完了 | manifest/lock、直接依存license宣言、jar版を確認。OSS由来等はU-0003 |
| 6 Evidence | 完了 | 本文のE-0001～E-0019にパス・シンボル・確度を登録 |
| 7 Unknown | 完了 | U-0001～U-0007の影響、理由、次の確認方法を登録 |
| 8 Document | 完了 | 本文7章、根拠付録、README、共通様式のQuarto設定を作成 |
| 9 Cross check | 制約付き完了 | 最終版2.2.1で15テスト成功、HTML9ページ・表13・SVG2点・リンク検査成功。ブラウザ目視はURL制限により未実施 |

調査中に別作業でHEADが `0a28da6e4ea95d7b37914b6b6b499978085d4a1c` から
`02672eb3054d0f6b7139e21b31d91aba7dd57595` へ進んだ。
全26ファイルの差分を確認し、版2.2.0→2.2.1と文書例の更新であることを確認。
CLIのRustソース／Cargo.lock／機構原本のロジック変更はなく、正式文書と最終ハッシュを後者へ更新した。

## 2026-09-23 の更新

基準を `281d62f1338d1de7f6660c1a47e1e84e72edda1b`（2.4.0）へ進めた。`02672eb..281d62f` の
`cli/`・`template/`・`.github/`・`extensions/*/package.json` の差分を棚卸しし、次を反映した。

| 対象 | 状態 | 内容 |
|---|---|---|
| ddq tag／rev、doc モジュール | 完了 | 第2～7章、E-0020～E-0022 |
| release の複数拡張対応、拡張CI | 完了 | 第3・5・6章、E-0013・E-0015 |
| 雛形の改訂履歴 include、lib.typ の _xref-page・付録、postprocess | 完了 | 第1・4章、E-0023 |
| 依存 sha1 | 完了 | 第5章、dependencies.json |
| 新たな注意点 | 完了 | 第7章（付録除外、パイプ表の帰属、UTC日付、--write と --json、~~~、BareFigure）、U-0008 |
| 検証 | 制約付き完了 | テスト74件成功、HTML再生成・静的検査成功。目視と外部描画は未実施 |

Mermaid・PlantUML・Lua・Quarto 起動・パス検査・機構照合の Rust ソースと design-doc.lua は、
ハッシュ比較で初版から変更がないことを確認した。
