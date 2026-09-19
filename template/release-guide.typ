// ============================================================
//  リリース一式に同梱する「はじめかた」スライド（→ はじめかた.pdf）。
//  旧 README-release.md の内容を、発行者が最初に開いて 5 分で読める形にしたもの。
//
//  Marp ではなく Quarto 同梱の Typst で組む（保守者のビルドに npm を持ち込まないため）。
//  ビルドは `ddq release` が行う（`quarto typst compile` を呼ぶ）。手で確認するなら:
//    quarto typst compile template/release-guide.typ out.pdf
//  版番号は `ddq release` が `--input version=<版>` で渡す。手で試すときは省略可（VERSION と表示）。
//
//  外部パッケージは使わない（閉域でも保守者の手元でも同じ結果にするため）。
// ============================================================

#let version = sys.inputs.at("version", default: "<版>")
#let folder = "quarto-template-" + version

// ---- 体裁（lib.typ と同じ書体） ----
#let JP-SANS = ("Meiryo UI", "Yu Gothic", "MS PGothic")
#let MONO = ("BIZ UDGothic", "MS Gothic", "Consolas", "Courier New")
#let ACCENT = rgb("#1f4e79")
#let MUTED = rgb("#666666")
#let CODE-BG = rgb("#f1f3f5")

#set page(width: 254mm, height: 143mm, margin: (x: 16mm, top: 14mm, bottom: 12mm))
#set text(font: JP-SANS, size: 16pt, lang: "ja")
#set par(leading: 0.8em)
#show raw: set text(font: MONO, size: 13pt)
#show raw.where(block: true): it => block(
  fill: CODE-BG, inset: 10pt, radius: 3pt, width: 100%, it,
)
#show strong: set text(fill: ACCENT)

#let total = context counter(page).final().first()

// 1 枚 = 見出し + 本文。フッターに ページ / 総ページ。
#let slide(title, body) = page(
  footer: context [
    #set text(size: 9pt, fill: MUTED)
    #grid(columns: (1fr, auto),
      [設計書テンプレート #version — はじめかた],
      [#counter(page).display() / #total])
  ],
)[
  #block(below: 12pt)[
    #text(size: 22pt, weight: "bold", fill: ACCENT)[#title]
    #v(-4pt)
    #line(length: 100%, stroke: 1.5pt + ACCENT)
  ]
  #body
]

// 注意書き（左に色帯）
#let note(body, color: ACCENT) = block(
  width: 100%, inset: (left: 12pt, y: 8pt, right: 8pt),
  stroke: (left: 3pt + color), fill: color.lighten(92%),
  body,
)

// ---- 表紙 ----
#page(footer: none)[
  #v(1fr)
  #align(center)[
    #text(size: 34pt, weight: "bold", fill: ACCENT)[設計書テンプレート]
    #v(6pt)
    #text(size: 22pt)[リリース一式 #version — はじめかた]
    #v(18pt)
    #text(size: 14pt, fill: MUTED)[
      Quarto と `ddq.exe` で、Markdown から様式付き PDF 設計書を出すまで
    ]
  ]
  #v(1fr)
  #align(center, text(size: 11pt, fill: MUTED)[
    発行者向け。手順の正は `manual/利用マニュアル.pdf`（2・4・11・12 章）
  ])
]

// ---- 1 ----
#slide[この展開フォルダに入っているもの][
  #grid(columns: (1.05fr, 1fr), gutter: 18pt,
    [
      #raw(block: true, folder + "/
├── ddq.exe
├── はじめかた.pdf   … 本書
├── README.md        … 概要と作業の流れ
└── manual/
    ├── 利用マニュアル.pdf
    └── html/index.html")
    ],
    [
      - *`ddq.exe`* が様式・変換・ビルドの実体。\
        様式一式を内蔵した単体の実行ファイルで、
        *インストールも追加ランタイムも不要*
      - *利用マニュアル* が手順の正。執筆者にも配る
      - `template/` フォルダは無い（exe に埋め込み済み）
      - Node.js・npm は要らない
    ],
  )
]

// ---- 2 ----
#slide[役割 — 環境の違いではなく責任の分担][
  #table(
    columns: (auto, 1fr, 1fr),
    stroke: 0.5pt + MUTED, inset: 8pt,
    fill: (x, y) => if y == 0 { CODE-BG },
    [*役割*], [*やること*], [*手元に置くもの*],
    [発行者], [設計書リポジトリを作る・テンプレートを更新する・*発行版* PDF / HTML を出す],
      [この展開フォルダ ＋ 設計書リポジトリ],
    [執筆者], [原稿（`.qmd`）を書く。VSCode のプレビューで確認する],
      [設計書リポジトリ（＋任意で この展開フォルダ）],
    [保守者], [様式・変換の仕組みを直し、リリースを配る], [テンプレートのリポジトリ],
  )
  #v(6pt)
  #note[
    `ddq.exe` を持っていれば*執筆者も同じ手順で PDF を出せる*。
    役割の境目は「誰が発行版を出す係か」だけ。
  ]
]

// ---- 3 ----
#slide[はじめかた（1/3）— 準備][
  + *Quarto を入れる* … #link("https://quarto.org/docs/get-started/")[quarto.org/docs/get-started]
    （PDF 組版の Typst も同梱。TeX は不要）
  + *ZIP をそのまま展開する*（例 #raw("C:\\tools\\" + folder)）。
    中身を取り出して並べ替えない
  + 展開先も設計書リポジトリも、*パスは ASCII だけ*にする
    （日本語を含むと mermaid 図の変換が失敗する）

  #v(6pt)
  #note(color: rgb("#b45f06"))[
    *この展開フォルダを設計書リポジトリの中に置かない。*
    `git clean` で消える・誤ってコミットする事故のもと。別の場所に置いたまま使う。
  ]
]

// ---- 4 ----
#slide[はじめかた（2/3）— 設計書リポジトリを作る][
  展開したフォルダに `cd` して、作成先を*絶対パス*で渡す。

  #raw(block: true, "cd C:\\tools\\" + folder + "
.\\ddq init C:\\work\\order-design")

  #grid(columns: (1fr, 1fr), gutter: 18pt,
    [
      できるもの:
      - `C:\work\order-design\` … リポジトリ直下（`.gitignore` など）
      - `docs\` … 執筆フォルダ（雛形 ＋ 機構ファイル）
      - 最後に HTML を 1 回作って疎通を確認
    ],
    [
      続けて:
      + `docs\_quarto.yml` の表題・資料番号・会社名・章立てを直す
      + `git init` してコミット、執筆者へ共有
      + *利用マニュアルの PDF も一緒に配る*
    ],
  )
]

// ---- 5 ----
#slide[はじめかた（3/3）— PDF・配布 HTML を出す][
  ```
  .\ddq pdf  C:\work\order-design\docs     … → docs\design-doc.pdf
  .\ddq html C:\work\order-design\docs     … → docs\_book\index.html
  ```

  - 両方出すときは *PDF → HTML の順*（同じ作業フォルダを使うため、後のほうが前を消す。PDF は取り出されるので残る）
  - PDF は*中間版としてリポジトリにコミット*する。執筆者は改ページ・横向き・紙の様式をこれで確認する
  - mermaid 図は Windows 標準の Edge（または Chrome）を裏で使ってベクター化。*準備は不要*。
    どちらも無ければ内蔵レンダラで描く

  #v(4pt)
  #note[
    初回の実行で Windows の SmartScreen が出たら「詳細情報」→「実行」で続行できる。
  ]
]

// ---- 6 ----
#slide[テンプレートが更新されたら][
  新しい版は*別のフォルダ*に展開される（フォルダ名に版が入る）。古い版は残しておける。

  ```
  cd C:\tools\quarto-template-<新しい版>
  .\ddq update C:\work\order-design\docs
  ```

  - 置き換わるのは機構ファイル 5 点（`design-doc.lua` など）だけ。原稿には触れない
  - *差分は git に出るのでコミットする。* 執筆者は pull するだけ
  - 執筆フォルダが複数あるなら `.\ddq update --all C:\work\order-design`
  - 版の確認: `.\ddq --version` と `<執筆フォルダ>\.template-version`
]

// ---- 7 ----
#slide[閉域環境・困ったとき][
  #grid(columns: (1fr, 1fr), gutter: 22pt,
    [
      *閉域（オフライン）環境*

      持ち込むのは 2 つだけ:
      + Quarto のインストーラ
      + この展開フォルダ

      `ddq.exe` はネットワークを使わない。
      Edge は Windows 標準なので mermaid も変換できる。
    ],
    [
      *困ったとき*

      - 利用マニュアル *13 章「困ったときは」*（環境・執筆・出力の症状別）
      - `_quarto.yml がありません` → 執筆フォルダのパスを絶対パスで渡し直す
      - 図が出ない・見た目が違う → 利用マニュアル 11 章
      - コマンド一覧 → 利用マニュアル 15 章
    ],
  )
]
