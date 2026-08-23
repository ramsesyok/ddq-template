# 既存の表紙（Word）から、テンプレートの表紙を作る

Word で作ってある表紙（外枠・資料番号欄・承認欄）を、このテンプレートの PDF の
表紙として使うための手順。**表紙は描き直さない。** 絵は Word から SVG として取り込み、
表題・資料番号などの差し替わる値だけをテンプレート側から差し込む。

> **このフォルダはリリース ZIP に同梱しない。** 移行は一回限りの作業であるため、
> 定常運用の手順は利用マニュアル4章（`cover.typ` の節）にある。本書はそこへ至る
> までの「既存資産の取り込み」を扱う。

## 目次

- [検証環境](#検証環境)
- [全体像](#全体像)
- [手順1 — Word を PDF にする](#手順1--word-を-pdf-にする)
- [手順2 — PDF を SVG にする](#手順2--pdf-を-svg-にする)
- [手順3 — cover.svg を置く](#手順3--coversvg-を置く)
- [手順4 — cover.typ を書く](#手順4--covertyp-を書く)
- [表紙に出すメタ情報](#表紙に出すメタ情報)
- [位置合わせのやり方](#位置合わせのやり方)
- [確認する](#確認する)
- [踏むと痛い罠](#踏むと痛い罠)
- [チェックリスト](#チェックリスト)

---

## 検証環境

本書で「実測」と記した記述は、次の環境で実際に変換・組版して確認した結果である。

| 項目 | 版 |
|---|---|
| Quarto | 1.10.18（同梱 typst 0.15.1）／ 1.9.38（同梱 typst 0.14.2） |
| PyMuPDF（`pdfsvg.py` が使う） | 1.28.2 |
| poppler（`pdftocairo`。Inkscape の PDF 取り込みと同じエンジン） | 24.02.0 |
| テンプレート | 本リポジトリの `template/` |

検証には、Word の表紙を模した PDF（ページ罫線・承認欄の表・日本語ラベル）を使い、
`type: book` の実ビルドまで通している。**Inkscape の GUI 操作そのものは未検証**で、
PDF の取り込みエンジン（poppler）の挙動で確認している。

---

## 全体像

```
Word の表紙
   │  ① 名前を付けて保存 → PDF
   ▼
表紙.pdf
   │  ② pdfsvg.py（Inkscape 不要）または Inkscape
   ▼
cover.svg  ……… 枠・罫線・固定ラベル（絵）
   │  ③ 執筆フォルダに置く（コミットする）
   ▼
cover.typ  ……… 表題・資料番号などを重ねる（値は _quarto.yml から）
   │  ④ build-qmd
   ▼
design-doc.pdf の表紙
```

**役割を分けるのがこつ。** SVG には「変わらないもの」だけを入れ、「文書ごとに
変わるもの」は SVG に入れずテンプレート側から入れる。

| | SVG に入れる | テンプレートから入れる |
|---|---|---|
| 内容 | 外枠・罫線・「承認」「審査」などの固定ラベル・ロゴ | 表題・副題・資料番号・改訂記号・会社名・作成日・部署 |
| 変わるか | 全文書で同じ | 文書ごと・改訂ごとに変わる |
| 書体 | パス化すれば環境に依存しない | 本文と同じ書体（`JP-SANS`）でそろう |

---

## 手順1 — Word を PDF にする

Word で「ファイル」→「名前を付けて保存」→ ファイルの種類に **PDF** を選ぶ。

- **ページ罫線・表・テキストボックスは、ベクターのまま PDF に入る**（ラスタ化されない）。
- 表紙が1ページ目でない場合も、そのまま出してよい。あとでページを指定して取り出す。
- 図形の影・グラデーション・半透明は、この先の SVG 化で再現されないことがある。
  表紙は**罫線・文字・単色塗り**に寄せておくと確実である。

---

## 手順2 — PDF を SVG にする

2通りある。**`pdfsvg.py` は Inkscape が要らず、コマンド1行で済む**ので、
掃除の必要が無ければこちらが早い。

### 方法A — `pdfsvg.py`（推奨）

```bash
# 何ページ目が表紙かを確認する
python migration/pdfsvg.py 表紙.pdf --list

# 1ページ目を、文字を輪郭化して取り出す
python migration/pdfsvg.py 表紙.pdf -o . --pages 1 --outline-text
```

`p001.svg` ができる。`--trim` を付けると余白が落ちるが、**表紙では付けない**
（A4 の紙面と座標をそろえたいため）。

PyMuPDF が要る（`pip install pymupdf`）。閉域環境で Python が使えないときは方法B。

### 方法B — Inkscape

1. Inkscape で PDF を開く。取り込み方式のダイアログが出る。
2. 不要な要素（白い矩形など）を消す。
3. 「名前を付けて保存」で **プレーン SVG** を選ぶ（Inkscape SVG は独自の情報が付く）。

### 文字をどう扱うか

これが分かれ目になる。

| 取り出し方 | 文字 | 書体 | `{{ }}` の差し込み | 大きさ（実測） |
|---|---|---|---|---|
| `pdfsvg.py --outline-text` ／ Inkscape の Poppler/Cairo 取り込み | すべてパスになる | **環境に依存しない** | **使えない** | 37 KB |
| `pdfsvg.py`（既定） ／ Inkscape の内部取り込み | 文字のまま残る | ビルド環境のフォント次第 | 使える | 4 KB |

**推奨は輪郭化（パス）である。** 発行者の環境に Word と同じ書体が無いと字形がずれる
一方、パス化しておけば誰の環境でも同じ紙面になる。差し替わる値は SVG に入れず、
次の手順4で Typst 側から重ねる。

---

## 手順3 — cover.svg を置く

`cover.svg` という名前で**執筆フォルダの直下**に置く（`_quarto.yml` と同じ場所）。

- typst は**プロジェクトの外を読めない**。執筆フォルダの外に置くと読み込めない。
- `cover.typ` と同じく、doc リポジトリでは**コミットする**（機構ファイルではなく、
  その文書の資産である）。
- SVG は A4 実寸で作られていれば、そのまま紙面に一致する。`pdfsvg.py` と
  `pdftocairo` の出力は元の PDF のページサイズを保つので、A4 の Word から作れば
  何もしなくてよい。

---

## 手順4 — cover.typ を書く

執筆フォルダの `cover.typ` を次のようにする。**`lib.typ` は触らない。**

```typst
#import "lib.typ": *

#let front-matter(meta, front) = {
  bare-page(margin: 0mm)[
    // ① 枠・罫線・固定ラベル（Word 由来。パス化ずみ）
    #place(top + left, svg-cover("cover.svg"))

    // ② 差し替わる値は typst で重ねる。書体は本文と同じ、位置は mm 指定
    #place(top + left, dx: 130mm, dy: 30mm,
      text(font: JP-SANS, size: 12pt)[#meta.doc-id])
    #place(top + left, dx: 30mm, dy: 90mm,
      text(font: JP-SANS, size: 22pt, weight: "bold")[#meta.title])
    #place(top + left, dx: 30mm, dy: 250mm,
      text(font: JP-SANS, size: 12pt)[#meta.company-ja])
  ]
}
```

`_quarto.yml` に `cover: true` を書くと出る。

### 差し込み口（`{{ }}`）を使う場合

SVG の文字を残した場合（手順2の方法B・内部取り込みなど）は、SVG の中に
`{{title}}` のように書いておけば、値に置き換えられる。

```typst
#place(top + left, svg-cover("cover.svg",
  ..meta.fields,             // cover-fields の項目をまとめて（{{created}} など）
  title: meta.title,         // → {{title}}
  docid: meta.doc-id))       // → {{docid}}
```

**差し込み口の名前は固定ではない。** `svg-cover` に渡した引数名が、そのまま
`{{名前}}` に対応する。

| | 重ねる方式（手順4の①②） | 差し込み口方式 |
|---|---|---|
| 書体 | 本文と同じ（`JP-SANS`）。環境非依存 | SVG 内の書体指定に従う。環境依存 |
| 位置 | `dx` / `dy` を自分で決める | Word で置いた位置のまま |
| 向いているもの | **既定。** 崩れないことを優先するとき | Word 側のレイアウトを完全に踏襲したいとき |

---

## 表紙に出すメタ情報

`cover.typ` の `meta` から `_quarto.yml` の値を受け取れる。**表紙に資料番号や
会社名を書き写す必要はない。**

| 書き方 | 元の設定 | 内容 |
|---|---|---|
| `meta.title` | `book: title:` | 表題 |
| `meta.subtitle` | `book: subtitle:` | 副題 |
| `meta.author` | `book: author:` | 作成者 |
| `meta.doc-number` | `doc-number:` | 資料番号 |
| `meta.doc-revision` | `doc-revision:` | 改訂記号 |
| `meta.doc-id` | ― | 資料番号＋改訂記号を連結したもの |
| `meta.company-ja` | `company-ja:` | 会社名（日本語） |
| `meta.company-en` | `company-en:` | 会社名（英語） |
| `meta.spec` | `spec:` | スペック様式か |
| `meta.page-start` | `page-start:` | 開始ページ番号 |
| `meta.fields.<キー>` | `cover-fields:` | **任意に増やせる項目**（下記） |

### 足りない項目を増やす（`cover-fields`）

作成日・部署・機密区分・契約番号のように、**表紙にしか出てこない記載**は
`_quarto.yml` に足せる。テンプレート（`lib.typ` / `typst-show.typ`）は直さない。

```yaml
cover-fields:
  created: "2026-04-01"
  dept: 第一開発部
  classification: 社外秘
  contract: C-001
```

```typst
#place(top + left, dx: 30mm, dy: 40mm,
  text(size: 10pt)[#meta.fields.dept　#meta.fields.classification])
```

- 値は素の文字列にする。日付は `"2026-04-01"` と引用符でくくると書式が固定される
  （くくらないと YAML が日付型として解釈し、表示が変わることがある）。
- `"` や `\` を含む値も壊れないようエスケープしてあるが、表紙の記載に使う必要は
  まず無い。
- SVG の差し込み口へまとめて渡すときは `..meta.fields` と書く。

### 前書き（`index.qmd`）を表紙に載せる

「本文書は○○規程に基づき作成したものである」のような前書きを表紙に入れる場合は、
文を `index.qmd` に Markdown で書き、`_quarto.yml` に `front-in-cover: true` を
足したうえで、`cover.typ` の好きな位置に `#front` を置く。詳細は利用マニュアル7章。

---

## 位置合わせのやり方

重ねる方式では `dx` / `dy` を決める必要がある。**Inkscape で座標を読むのが早い。**

1. Inkscape で `cover.svg` を開く。
2. 「ファイル」→「ドキュメントのプロパティ」で単位を **mm** にする。
3. 文字を置きたい場所の要素（罫線の交点など）を選ぶと、ツールバーに X / Y が mm で出る。
4. その値をそのまま `dx` / `dy` に書く。

**Inkscape も typst も左上が原点で、Y は下向き**なので、読んだ値をそのまま使える。

微調整は数値を変えてビルドし直すのが確実である。表紙だけを速く見たいときは、
`cover.typ` の内容を独立した `.typ` に写して `quarto typst compile` で直接組むとよい。

---

## 確認する

```bash
./template/build-qmd.sh <執筆フォルダのパス>
```

うまくいかないときは、中間の `index.typ` を残して中身を見る。`_quarto-publish.yml`
に `keep-typ: true` を足すと、執筆フォルダに `index.typ` が残る。

---

## 踏むと痛い罠

| 症状 | 原因 | 対処 |
|---|---|---|
| SVG が読み込めない | `cover.svg` が執筆フォルダの外にある | typst はプロジェクト外を読めない。執筆フォルダ直下に置く |
| 表紙の文字だけ書体が違う | SVG 内の文字が、ビルド環境に無い書体を指している | `--outline-text` で取り出し直すか、その文字を typst 側で重ねる |
| 表紙が2ページに割れる | 表紙に置いた見出し（h1）が「章は改ページ」に掛かった | 前付けの見出しは `front` 経由で置く。`cover.typ` に直接書くなら `heading(level: 1, numbering: none, outlined: false)` にする |
| 表紙に外枠が二重に出る | SVG に枠があり、様式の外枠も出ている | `bare-page` で組む（様式の外枠・資料番号欄を出さないページ） |
| 影やグラデーションが消える | typst の SVG 描画が対応していない | 罫線・文字・単色塗りに作り替える |
| 表紙の位置が紙面とずれる | `--trim` で余白を落とした SVG を使っている | 表紙では `--trim` を使わない |
| `{{title}}` が置き換わらない | 文字がパス化されている／引数名と綴りが違う | 文字を残して取り出し直すか、重ねる方式にする |

---

## チェックリスト

- [ ] Word の表紙を PDF にした（罫線・表がベクターで入っている）
- [ ] SVG に変換した（原則 `--outline-text` で輪郭化）
- [ ] `cover.svg` を執筆フォルダ直下に置き、コミット対象にした
- [ ] `cover.typ` を書いた（`bare-page` ＋ `svg-cover` ＋ `#place`）
- [ ] 差し替わる値は `meta` から取っている（表紙に直書きしていない）
- [ ] 表紙にしか無い記載は `cover-fields` に足した
- [ ] `_quarto.yml` に `cover: true` を書いた
- [ ] `build-qmd` で PDF を出し、紙面を目視で確認した
- [ ] 別の発行者の環境でもビルドし、書体が変わらないことを確認した
