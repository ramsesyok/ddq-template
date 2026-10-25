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
    #text(size: 22pt)[リリース #version — はじめかた]
    #v(18pt)
    #text(size: 14pt, fill: MUTED)[
      Quarto と VSCode 拡張 DDQ Template で、Markdown から様式付き PDF 設計書を出すまで
    ]
  ]
  #v(1fr)
  #align(center, text(size: 11pt, fill: MUTED)[
    発行者向け。手順の正は `manual/利用マニュアル.pdf`（2・4・11・12 章）
  ])
]

// ---- 1 ----
#slide[リリースの 2 つの資産][
  #grid(columns: (1.05fr, 1fr), gutter: 18pt,
    [
      #raw(block: true, "ddq-template-win32-x64-" + version + ".vsix
  … VSCode 拡張（ddq.exe 入り）

" + folder + "/  … この配布 zip
├── ddq.exe          … コマンドで使うとき
├── plantuml.jar     … PlantUML 図の描画
├── はじめかた.pdf   … 本書
├── README.md        … 概要と作業の流れ
└── manual/
    ├── 利用マニュアル.pdf
    └── html/index.html")
    ],
    [
      - *VSIX（DDQ Template）* がふだん使う道具。設計書リポジトリの作成・
        PDF / HTML の出力・ラベル付け・改訂履歴・表の視覚編集。
        様式一式を内蔵した *`ddq.exe` が入っている*。*全員に配る*
      - *配布 zip* は利用マニュアル（手順の正。執筆者にも配る）と、
        コマンド・バッチ・CI で使う `ddq.exe` 単体
      - インストールするのは Quarto・VSCode・VSIX だけ。Node.js・npm は要らない
    ],
  )
]

// ---- 1b ----
#slide[設計書リポジトリと執筆フォルダ][
  #grid(columns: (1.15fr, 1fr), gutter: 18pt,
    [
      #show raw: set text(size: 11.5pt)
      ```
      order-design/          … 設計書リポジトリ
      ├── .gitignore, README.md …
      └── docs/              … 執筆フォルダ
          ├── _quarto.yml    … 表題・章立てなど
          ├── index.qmd      … 前付け
          ├── chapters/      … 本文（1 章 1 ファイル）
          ├── diagrams/      … 図
          ├── design-doc.pdf … 中間版 PDF
          └── design-doc.lua … ほか機構ファイル
      ```
    ],
    [
      #set text(size: 14pt)
      - *設計書リポジトリ* = 設計書 1 件を git で共有する単位。
        DDQ Template で作り、執筆者はこれだけを clone する
      - *執筆フォルダ* = 原稿一式（`_quarto.yml` と `.template-version` があるフォルダ）。
        名前は自由（既定 `docs`）。右クリックで PDF などを出せる
      - *機構ファイル*（`design-doc.lua` など 5 点）は `ddq` が置く写し。
        執筆者は触らないが、コミットはする
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
    [発行者], [設計書リポジトリを作る・テンプレートを更新する・*発行版*を出す],
      [DDQ Template ＋ 設計書リポジトリ],
    [執筆者], [原稿（`.qmd`）を書く。プレビューと中間版 PDF で確認する],
      [DDQ Template ＋ 設計書リポジトリ],
    [保守者], [様式・変換の仕組みを直し、リリースを配る], [テンプレートのリポジトリ],
  )
  #v(6pt)
  #note[
    DDQ Template に `ddq.exe` が入っているので*執筆者も PDF を出せる*。
    チーム全員で*同じ版の VSIX* を使う。
  ]
]

// ---- 3 ----
#slide[はじめかた（1/3）— 準備][
  + *Quarto を入れる* … #link("https://quarto.org/docs/get-started/")[quarto.org/docs/get-started]
    （PDF 組版の Typst も同梱。TeX は不要）
  + *VSCode を入れる* … #link("https://code.visualstudio.com/")[code.visualstudio.com]
  + *DDQ Template を入れる* … 拡張機能ビューの「…」→「VSIX からのインストール…」で
    #raw("ddq-template-win32-x64-" + version + ".vsix") を指定する
  + 配布 zip は*設計書リポジトリの外に*展開してマニュアルを読む

  #v(6pt)
  #note(color: rgb("#b45f06"))[
    パスには日本語を使ってよい。絵文字や波ダッシュ `〜` など CP932 に無い文字だけは使えない（2 章）。
  ]
]

// ---- 4 ----
#slide[はじめかた（2/3）— 設計書リポジトリを作る][
  VSCode のコマンドパレット（`Ctrl+Shift+P`）で *DDQ: 設計書リポジトリを作成*。
  作る場所・リポジトリ名・執筆フォルダ名（既定 `docs`）を選ぶ。

  #grid(columns: (1fr, 1fr), gutter: 18pt,
    [
      できるもの:
      - リポジトリ直下（`.gitignore`・`.vscode/` など）
      - `docs\` … 執筆フォルダ（雛形 ＋ 機構ファイル）
      - 最後に HTML を 1 回作って疎通を確認し、そのフォルダを開き直す
    ],
    [
      続けて:
      + `docs\_quarto.yml` の表題・資料番号・会社名・章立てを直す
      + 「Git リポジトリとして初期化」→ コミット、執筆者へ共有
      + *利用マニュアルの PDF と VSIX も一緒に配る*
    ],
  )
]

// ---- 5 ----
#slide[はじめかた（3/3）— PDF・配布 HTML を出す][
  執筆フォルダを右クリック → *DDQ: PDF を作成* → *DDQ: HTML を作成*。
  執筆フォルダのファイルを開いていれば、ステータスバーの `PDF: docs` でも出せる。

  - 両方出すときは *PDF → HTML の順*（同じ作業フォルダを使うため、後のほうが前を消す。PDF は取り出されるので残る）
  - PDF はレビュー中は*中間版としてコミット*し、承認済みのコミット／タグから同じ手順で作るものを
    *発行版*と呼ぶ
  - mermaid 図は Windows 標準の Edge（または Chrome）を裏で使ってベクター化。*準備は不要*。
    どちらも無ければ内蔵レンダラで描く
  - PlantUML 図は LAN の PlantUML サーバ（`_quarto.yml` の `plantuml-server:`）か、
    無ければ配布 zip の `plantuml.jar` を端末の *Java* で裏で起動して描く（11 章）
]

// ---- 6 ----
#slide[テンプレートが更新されたら][
  新しい版の VSIX を入れ直し、設計書リポジトリで PDF などを出すと
  *「ddq update を実行」*の通知が出る。押すと機構ファイルが新しい版になり、出力が続く。

  - 置き換わるのは機構ファイル 5 点（`design-doc.lua` など）だけ。原稿には触れない
  - *差分は git に出るのでコミットする。* 執筆者は pull するだけ
  - *執筆者は押さない。* 通知が出たら発行者に伝え、同じ版の VSIX を受け取る
  - 版の確認: 拡張機能ビューの DDQ Template の版と `<執筆フォルダ>\.template-version`
]

// ---- 7 ----
#slide[閉域環境・困ったとき][
  #grid(columns: (1fr, 1fr), gutter: 22pt,
    [
      *閉域（オフライン）環境*

      持ち込むもの:
      + Quarto と VSCode のインストーラ
      + DDQ Template の VSIX（と配布 zip）

      DDQ Template と `ddq.exe` はネットワークを使わない。
      Edge は Windows 標準なので mermaid も変換できる。
      PlantUML 図を使うなら Java も持ち込む（LAN にサーバが無い場合）。
    ],
    [
      *困ったとき*

      - 利用マニュアル *14 章「困ったときは」*（環境・執筆・出力の症状別）
      - 経過とエラーは「出力」パネルの *DDQ* に出る
      - 図が出ない・見た目が違う → 利用マニュアル 11 章
      - 操作の一覧 → 利用マニュアル 16 章、コマンドで使う → 付録 A
    ],
  )
]
