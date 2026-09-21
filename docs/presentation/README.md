# presentation — 設計書テンプレートの紹介スライド

社内向けに「設計書を Markdown で書く」テンプレートを紹介するスライド。
Quarto の revealjs 形式（HTML スライド）で、**LT 版（5 分）**と**ロング版（15〜20 分）**の 2 本がある。

| ファイル | 内容 | 枚数 |
|---|---|---|
| `lt.qmd` | LT 版。課題 → できあがるもの → 書き方の要点 → 執筆環境 → 始め方 → まとめ | 表紙 + 8 枚 |
| `long.qmd` | ロング版。LT 版の全スライドを含み、章立て・図・IPO 図・VSCode 拡張・AI 執筆・デモ・運用（役割・置き場所・発行）・仕組み・制約を足したもの | 表紙 + 6 節 + 22 枚 |
| `sections/_*.qmd` | 1〜2 枚単位のスライド断片。両方の `.qmd` から `{{< include >}}` する | |
| `images/` | サンプル設計書（`examples/docs/design-doc.pdf`）と HTML の紙面キャプチャ | |
| `custom.scss` | 見た目（日本語フォント・見出し色・紙面画像の枠など） | |

各スライドには `::: {.notes}` で**発表者ノート**（話す要点と目安時間）が入っている。
発表中に `S` キーで別ウィンドウに表示される。

## ビルドと発表

このフォルダで実行する。Quarto だけあればよい（`ddq` は使わない）。

```bat
quarto render                 :: 両方 → _book\lt.html, _book\long.html
quarto preview lt.qmd         :: 書きながら確認
```

- 発表は `_book\lt.html` をブラウザで開く。`F` で全画面、`S` で発表者ノート、`O` で一覧、`?` でキー一覧
- 1 ファイルで配る：`quarto render lt.qmd -M embed-resources:true`
- PDF にする：URL 末尾に `?print-pdf` を付けて開き、`Ctrl+P` → PDF に保存（用紙は横・余白なし）
- `_book/` はルートの `.gitignore` で無視される（コミットしない）
- render 時の `Unable to resolve crossref @fig-hw` 警告は、記法の説明文（`_04-numbering.qmd` の表）を Quarto が参照と誤認したもので無害。出力には影響しない

## 構成と時間配分

### LT 版（5 分）

| # | スライド（`sections/`） | 目安 |
|---|---|---|
| 1 | 表紙 | 0:10 |
| 2 | `_01-problem` 設計書、こう作っていませんか | 0:30 |
| 3 | `_02-result` こうなります（目次・様式付き紙面） | 0:30 |
| 4 | `_03-markdown-only` 書くのは Markdown だけ（原稿と出力の対比） | 0:30 |
| 5 | `_04-numbering` 番号と参照は全部自動 | 0:30 |
| 6 | `_05-table` 表：セル結合・ページ分割 | 0:30 |
| 7 | `_06-author-env` 執筆者に要るのは Quarto と VSCode だけ | 0:30 |
| 8 | `_07-start` 始め方は 3 ステップ | 0:30 |
| 9 | `_08-summary-lt` まとめ | 0:20 |

### ロング版（15〜20 分）

| 節 | スライド（`sections/`） | 目安 |
|---|---|---|
| 表紙・今日の話 | （`long.qmd` 内） | 0:30 |
| 1. 何が困っているか | `_01-problem` | 1:00 |
| 2. できあがるもの | `_02-result` ／ `_03-markdown-only` ／ `_long-html`（HTML と PDF の使い分け） | 2:30 |
| 3. 書き方 | `_long-structure`（章＝フォルダ）／ `_04-numbering` ／ `_long-figures`（mermaid・PlantUML）／ `_05-table` ／ `_long-table-editor`（VSCode 拡張）／ `_long-ipo`（IPO 図・横向き）／ `_long-agent`（AI 執筆）／ `_06-author-env` | 6:00 |
| 4. デモ | `_long-demo`（手順表 ＋ こけたときの保険） | 3:30 |
| 5. 運用 | `_long-roles`（3 つの役割）／ `_long-repo`（置き場所）／ `_long-publish`（発行の流れ）／ `_07-start` | 3:00 |
| 6. 中身と制約 | `_long-pipeline`（仕組み）／ `_long-limits`（制約・注意）／ `_long-summary` | 2:30 |

合計 19 分。15 分に収めるなら `_long-agent`・`_long-pipeline` を飛ばし、デモを 2 分に縮める
（`long.qmd` の該当 `include` 行をコメントアウトするか、当日 `→` で飛ばす）。

### 断片の使い分け

- `_0N-*.qmd` … LT 版の本体。ロング版にもそのまま入る（**LT 版はロング版の部分集合**）
- `_long-*.qmd` … ロング版だけで使う
- 節見出し（`# 1. 何が困っているか {.unlisted}`）は `long.qmd` に直接書いてある

LT 版の言い回しを変えるときは `_0N-*.qmd` を直せばロング版にも反映される。
ロング版だけ詳しくしたい場合は `_long-*.qmd` を足し、`_0N-*.qmd` は簡潔なままにする。

## 画像の差し替え

`images/` の紙面キャプチャは `examples/docs/design-doc.pdf`（受注管理システム 基本設計書のサンプル）から
切り出したもの。サンプルや様式を変えたら作り直す。

| ファイル | 元 | 使う断片 |
|---|---|---|
| `pdf-toc-crop.png` | サンプル PDF 1 ページ目（目次）上半分 | `_02-result` |
| `pdf-table-merge-crop.png` | 7 ページ目（3. 設計条件、セル結合の表）上半分 | `_02-result` |
| `pdf-figure-crop.png` | 8 ページ目（4.1.1 mermaid 図）上半分 | `_03-markdown-only` ／ `_long-demo` |
| `pdf-split.png` | 35 ページ目下部 ＋ 36 ページ目上部（表 11.3.3-1（1／2）（2／2））を縦に連結 | `_05-table` |
| `pdf-ipo.png` | 18 ページ目（受注登録処理の IPO 図、横向き） | `_long-ipo` |
| `html-preview.png` | `examples/docs/_book/chapters/04-system/index.html` を 1440×900 でキャプチャ | `_06-author-env` ／ `_long-demo` |
| `table-editor.png` | **未作成**。VSCode 拡張 `ddq-table-editor` の画面。用意したら `_long-table-editor.qmd` の注記を画像に置き換える | `_long-table-editor` |

PDF からの切り出しは pymupdf（`pip install pymupdf pillow`）で行った。例：

```python
import fitz
d = fitz.open("../../examples/docs/design-doc.pdf")
p = d[6]; r = p.rect                       # 7 ページ目
clip = fitz.Rect(r.x0, r.y0 + r.height*0.06, r.x1, r.y0 + r.height*0.42)
p.get_pixmap(dpi=150, clip=clip).save("images/pdf-table-merge-crop.png")
```

HTML のキャプチャは Edge の headless で行った（`examples/docs/_book` を HTTP で配信して開く。`file://` では CSS が効かない）。

```bat
python -m http.server 8765 --directory ..\..\examples\docs\_book
"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe" --headless=new --disable-gpu --hide-scrollbars --window-size=1440,900 --screenshot=images\html-preview.png http://localhost:8765/chapters/04-system/index.html
```

## 発表前の確認

- [ ] 表紙の `author:`（発表者名・所属）と `date:` を直す（`lt.qmd` / `long.qmd` の YAML）
- [ ] スライド内の版番号（`quarto-template-2.2.1`）が配るリリースと一致している
- [ ] デモ環境：リリース ZIP 展開済み、Quarto ＋ VSCode 拡張導入済み、`ddq pdf` を 1 回通して Edge の初回起動を済ませておく
- [ ] デモ手順 3 で貼る原稿（見出し・`.tbl`・mermaid）をクリップボードかファイルに用意
