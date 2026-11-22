# docs — テンプレート自身の文書（設計リポジトリ）

本テンプレートで書いた文書の執筆フォルダを、設計書リポジトリと同じ形で並べたフォルダである。
保守の入口は本書、利用の入口は `manual/`（利用マニュアル）とする。

同じことを二か所に詳しく書かないよう、次のように書き分ける。

- 利用の条件と手順は `manual/` に書く（配布 ZIP と VSIX を受け取った人が読む）
- 理由（なぜその形にしたか）は `design/` と `cli/DESIGN.md` に書く
- ある版のコードの解析は、基準コミットを付けて各 `*-impl/` に書く
- 検討書と記録（点検・対応・計画）は書いた時点のまま残し、後から本文を現行に合わせない。現行の記述は「後継先」で辿る。消した旧パスは、括弧内のコミットの親で読める（例: `git show 7151377^:AUTHORING.md`）

## 執筆フォルダ

| 執筆フォルダ | 用途 | 読む人 | 対象版・基準の時点 | 後継先・旧パス |
|---|---|---|---|---|
| `manual/` | 利用マニュアル（環境構築・執筆・確認・出力・記法・トラブル対処、保守者向けの版とリリース）。`ddq release` が PDF と HTML にして配布 ZIP に入れる | 全員 | 現行（`template/VERSION`）。リリースのたびにビルドする | 利用の手順の正本。旧 `AUTHORING.md`（2026-08-15 の `7151377` で削除）の記法は 6〜10 章 |
| `design/` | テンプレート設計書（様式・変換の仕組み、調整箇所、版の考え方、保守の観点） | 保守者 | 改正 A（3.0.0、2026-11-14） | 理由の正本。旧 `ADVANCED.md`・`template/PIPELINE.md`（2026-09-20 の `4fc52aa` で統合） |
| `cli-impl/` | ddq ソフトウェア実装仕様書（CLI の構造・処理・外部契約・変更影響・解析根拠） | CLI の改修者 | 3.0.0-pre4（`2a9373e`、2026-11-14）。本文の解析基準は 2.4.0（`281d62f`、2026-09-23） | 版付きの解析。基準より後の変更はソースと `cli/DESIGN.md` が正。記載の試験件数・ハッシュは基準の時点の値 |
| `ddq-template-impl/` | DDQ Template ソフトウェア実装仕様書（作成・発行・ddq との契約・ラベル付け・改訂履歴・表編集・変更影響・解析根拠） | `ddq-template` の改修者 | 共通の基盤は 3.0.0-pre4（`2a9373e`、2026-11-14）。ラベル付け・改訂履歴は DDQ Revision 2.3.0（`b5368a1`、2026-09-23）、表エディタは DDQ Table Editor 2.2.1（`b79bfed`、2026-09-21） | 旧 `docs/revision-impl/`・`docs/table-editor-impl/`（2026-10-25 の `e447a47` で統合）。旧 2 冊は `parts/` に旧名・旧パスのまま収める。現行の名前は本文の対応表で読み替える |
| `presentation/` | 紹介スライド（revealjs。LT 版 5 分／ロング版 15〜20 分）。設計書ではないので `ddq` を使わず `quarto render` で出す | 発表者 | 版の数値は載せていない（最終更新 2026-11-17） | — |

## 単一ファイルの文書

執筆フォルダではないので `ddq` は使わない。検討書は実装前に書き、内容が固まったら上の実装仕様書へ昇格させる。
記録はある時点の状態を書き留めたもので、昇格させない。

| ファイル | 種別 | 内容 | 対象版・時点 | 後継先 |
|---|---|---|---|---|
| `revision-study.md` | 検討書（実装済み） | ラベル付けと改訂履歴の実現性検討と設計（実機検証 V1〜V5 つき） | 2.2.2、2026-09-22 | `ddq-template-impl/` のラベル付け・改訂履歴、`cli/DESIGN.md` §14・§15。§5.8 の js-yaml による直列化は採らず、実装は `extensions/ddq-template/src/revision/revfile.ts` の限定した読み書き（形の正本は `cli/src/doc/revfile.rs`）。Q9・Q18 の別拡張とその配置は、統合で変わった |
| `ddq-template-study.md` | 検討書（実装済み） | 2 拡張の統合と `ddq.exe` 同梱の検討と設計 | 2.4.5、2026-10-25 | `ddq-template-impl/` の共通の基盤、利用マニュアル。Q6 の段階計画と旧ディレクトリ図は検討時のもので、現在の機能の一覧は `ddq-template-impl/` |
| `../cli/plantuml-study.md` | 検討書（実装済み） | PlantUML 対応の検討と設計 | 2.1.0、2026-09-20 | `cli/DESIGN.md` §13、利用マニュアル 8 章・11 章。jar を配布 ZIP に同梱する案は 3.0.0 でやめた（利用者が公式リリースから取得する） |
| `test-coverage-review.md` | 点検の記録 | テストの網羅の点検（機能カバレッジ・コードカバレッジ・変異検査） | `bf1e70e`（3.0.0 の開発中）、2026-11-14 | 当時の CI の説明は現在の workflow と違う。現行の CI は `.github/workflows/`、測定の条件は `refactoring-plan.md` §6 |
| `ddq-template-pr122-final-response.md` | 対応の記録 | PR #122 の最終品質検証（検証 Head `8159f80`）の R-02・R-06 への対応 | 2026-10-26 | — |
| `design/analysis/table-pagination.md` | 検証の記録 | 自動分割表のページ通番修正の検証 | 比較元 `dca4f12`、2026-10-22 | — |
| `refactoring-plan.md` | 計画 | アーキテクチャレビューとリファクタリング計画（承認済み） | `17ed32e`（3.1.0-pre1）、2026-11-22 | 実施の記録は `refactoring-progress.md` |
| `refactoring-progress.md` | 実施の記録 | 上の計画の ID ごとの実装状況・試験と結果・計画からの逸脱・残存リスク | `17ed32e` からの作業、2026-11-22 | — |
| `release-ci-proposal.md` | 検討書（承認済み） | 配布候補の作成を GitHub Actions に移す案（REL-02・CI-01 の残り） | 3.1.0-pre1、2026-11-22 | 計画 §7 に追記した。現行の手順は利用マニュアル 18 章と `cli/DESIGN.md` §7.5・§7.6 |

## 検討書で決めて、いまも有効な判断

| 判断 | 理由（検討書の記載） | 採用版 | 検討書 | 現行の参照先 |
|---|---|---|---|---|
| 拡張を DDQ Template 1 本にし、win32-x64 の VSIX に `ddq.exe` を同梱する。単体の `ddq.exe`（配布 ZIP）も残す | init から PDF/HTML の発行まで VSCode で完結させる。CLI はバッチ・CI・VSCode を使わない人向け | 3.0.0（初出 3.0.0-pre1） | `ddq-template-study.md` §1・Q1・Q2 | 利用マニュアル 2 章・18 章、`ddq-template-impl/` |
| 拡張は版を事前に照合しない。ddq が版不一致を終了コード 3 で返したら、通知から利用者が `ddq update` を実行する | ddq 本体への変更を専用の終了コードだけに抑える | 3.0.0 | `ddq-template-study.md` §1・Q4 | `ddq-template-impl/` の共通の基盤、利用マニュアル 11 章・12 章 |
| VSIX はリリースの資産としてローカルに配る（Marketplace・Open VSX には出さない） | 社内利用が主。自動更新は無い前提 | 3.0.0 | `ddq-template-study.md` Q13 | 利用マニュアル 18 章 |
| 改訂は Git の ref（既定は前回の改訂タグ `rev-<記号>`）と作業ツリーを比べる。タグ付け・コミットは人が行う | 自動のタグ付け・コミットは運用の結果を見てから（保留） | 2.3.0 | `revision-study.md` Q6・Q17 | `cli/DESIGN.md` §15、利用マニュアル 13 章 |
| PlantUML は HTTP の描画サーバに一本化する（LAN のサーバを優先し、無ければ ddq が jar をローカルで起動する） | ブラウザ内で動く実装が無い。サーバなら執筆者に Java も ddq も要らない | 2.2.0 | `cli/plantuml-study.md` | `cli/DESIGN.md` §13・決定 15 |

## docs の外にある文書

| ファイル | 用途 | 読む人 |
|---|---|---|
| `cli/DESIGN.md` | ddq の設計（決定事項・コマンド・配布・PlantUML・ラベル・改訂履歴） | CLI の改修者 |
| `extensions/ddq-template/DEVELOPMENT.md`・`AGENTS.md` | 拡張の開発・試験の手順と、改修時の決まり | 拡張の改修者 |
| `extensions/ddq-template/tableEditor-requirements.md` | 表エディタの要件と互換の規則（記法は利用マニュアル 9 章が正） | 拡張の改修者 |
| `examples/README.md` | サンプルの設計書リポジトリ（`docs/`・`plantuml/`）。記法の実例と、様式を変えたときの回帰の入力 | 保守者 |
| `migration/README.md` | 既存の Word / PowerPoint 設計書を移す検討と支援ツール。リリース ZIP に同梱しない。実測は Quarto 1.9.38・pandoc 3.8.3 | 移行する人 |

## ビルド

ビルドはリポジトリのルートから（`ddq` は `cli/` で `cargo build --release` したもの）。

```bat
cli\target\release\ddq.exe update docs\manual
cli\target\release\ddq.exe pdf    docs\manual
cli\target\release\ddq.exe html   docs\manual
```

各執筆フォルダ直下の機構ファイル（`design-doc.lua` など）と `design-doc.pdf` は、原本が
このリポジトリの `template/` にあるため git 管理しない（ルートの `.gitignore`）。
