# docs — テンプレート自身の文書（設計リポジトリ）

本テンプレートで書いた文書の執筆フォルダを、設計書リポジトリと同じ形で並べたフォルダである。

| 執筆フォルダ | 内容 | 読む人 |
|---|---|---|
| `manual/` | 利用マニュアル（環境構築・執筆・確認・出力・記法・トラブル対処、保守者向けの版とリリース） | 全員 |
| `design/` | テンプレート設計書（様式・変換の仕組み、調整箇所、版の考え方、保守の観点） | 保守者 |
| `cli-impl/` | ddq ソフトウェア実装仕様書（CLI の構造・処理・外部契約・変更影響・解析根拠） | CLI の改修者 |
| `table-editor-impl/` | DDQ Table Editor ソフトウェア実装仕様書（拡張・表編集・記法変換・変更影響・解析根拠） | `ddq-table-editor` の改修者 |
| `revision-impl/` | DDQ Revision ソフトウェア実装仕様書（ddq との契約・ラベル付け・改訂履歴・変更影響・解析根拠） | `ddq-revision` の改修者 |
| `presentation/` | 紹介スライド（revealjs。LT 版 5 分／ロング版 15〜20 分）。設計書ではないので `ddq` を使わず `quarto render` で出す | 発表者 |

このほかに、実装前の検討書を単一の Markdown で置く（執筆フォルダではないので `ddq` は使わない）。
内容が固まったら上の実装仕様書へ昇格させる。`cli/plantuml-study.md` も同じ位置づけである。

| ファイル | 内容 |
|---|---|
| `revision-study.md` | ddq-revision（見出し・表・図のタグ付けと、ラベル単位の改訂履歴）の実現性検討と設計 |

ビルドはリポジトリのルートから（`ddq` は `cli/` で `cargo build --release` したもの）。

```bat
cli\target\release\ddq.exe update docs\manual
cli\target\release\ddq.exe pdf    docs\manual
cli\target\release\ddq.exe html   docs\manual
```

各執筆フォルダ直下の機構ファイル（`design-doc.lua` など）と `design-doc.pdf` は、原本が
このリポジトリの `template/` にあるため git 管理しない（ルートの `.gitignore`）。
