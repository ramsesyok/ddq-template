// ============================================================
//  設計書 共通テンプレート  (Typst / Quarto の typst バックエンド共用)
//  外部パッケージ不要 = 完全オフライン
//  会社様式（再現目標: ルートの portrait.pdf / landscape.pdf / ipo-landscape.pdf）
//   - 全ページ: 本文を囲む外枠
//   - 右上: 枠付き資料番号 + ページ「n / N」
//   - 横向きページ: 様式ごと 90°回転（縦綴じのまま回して読む配置）
//   - ipo() : IPO図（機能名/処理名 + 入力/処理/出力 の定型表）
// ============================================================

// ============================================================
//  【1】調整パラメータ
//
//  様式の微調整は原則ここだけを触れば済むようにしてある。
//  下の関数群は、この節の定数を参照するだけ。
//
//  寸法の考え方（A4 縦 = 210 x 297mm）:
//
//      0                                              210mm
//      ┌──────────────────────────────────────────────┐
//      │        ┌─ FRAME-P-POS.x = 23mm               │
//      │        ↓                                     │
//      │        ┌────────────[資料番号 SD-xxx  1/10]  │ ← 枠の上辺に接地
//      │        │←── FRAME-P-SIZE.width = 178mm ──→│  │   (dy = y - 枠の高さ)
//      │        │                                  │  │
//      │  28mm  │   ← PAGE-P-MARGIN.left = 28mm    │  │  本文は外枠の
//      │ ←───→ │      （枠より 5mm 内側）          │  │  内側 5mm（上下左右）
//      │        │                                  │  │
//      │        │  FRAME-P-SIZE.height = 256mm     │  │
//      │        └──────────────────────────────────┘  │
//      └──────────────────────────────────────────────┘
//
//  縦は上 27mm・下 14mm（297 - 27 - 256）で非対称。
//  左右も非対称で 左 23mm・右 9mm（210 - 23 - 178 = 9）。
//
//  横向き（landscape / ipo）は A4 横 = 297 x 210mm。様式ごと 90°回転する。
//  縦綴じのまま用紙を回して読む配置なので、資料番号を右端に縦置きする。
//
//      0                                                        297mm
//      ┌──────────────────────────────────────────────────────────┐
//      │     ┌─ FRAME-L-POS = (x: 14mm, y: 23mm) 外枠の左上        │
//      │     ↓                                            資  │    │ ← 資料番号は
//      │     ┌──────────────────────────────────────────┐  料  │    │   外枠の右辺に
//      │     │←─ FRAME-L-SIZE.width = 256mm ───────────→│  番  │    │   接して 90°縦置き
//      │     │                                          │  号  │    │   起点 dy =
//      │     │    FRAME-L-SIZE.height = 178mm            │      │    │   上辺 = 外枠上+DOCNUM-L-TOP
//      │     └──────────────────────────────────────────┘      │    │
//      └──────────────────────────────────────────────────────────┘
//
//  資料番号は「外枠の右辺に接する」= FRAME-L-POS.x + FRAME-L-SIZE.width の位置。
//  左右は 14mm・27mm（297 - 14 - 256 = 27）、上下は 23mm・9mm（210 - 23 - 178 = 9）で非対称。
//  これは A4 縦の外枠を時計回りに 90°回転した配置に一致する（左←下, 上←左, 右←上, 下←右）。
//  本文余白は枠より内側で、表向き = PAGE-L-MARGIN、IPO 図 = PAGE-IPO-MARGIN と別値。
// ============================================================

// ---- フォント ----
// 実運用は Noto CJK を前提。無い環境では Yu Gothic → MS PGothic に落ちる
// （字形確認は Noto を入れて行うこと）。
#let JP-SANS = ("Meiryo UI", "Yu Gothic", "MS PGothic")
#let JP-SERIF = ("Meiryo UI", "Yu Mincho", "MS PMincho")
// コードブロック・インラインコード（```…``` と `…`）の書体。
// 満たしたい条件は2つ:
//   (a) 完全な等幅 … 欧文が和文のちょうど半分幅（10pt なら 5pt / 10pt）。
//       グリッド表や図の例をコードとして貼ったときに桁がそろう。
//   (b) `\` がバックスラッシュの字形で出る（円記号 `¥` にならない）。
//
// Windows 標準の書体では、この2つを**1つの書体では両立できない**（実測）:
//   BIZ UDGothic / MS Gothic / MS Mincho / BIZ UDMincho … 等幅だが `\` が `¥`
//   Consolas 0.55em / Cascadia Mono 0.586em / Courier New 0.6em … `\` は出るが
//     和文（1em）の半分でないため、和文混じりで桁がずれる
//   NSimSun / SimSun … 等幅（0.5em）で `\` も出るが、和文が中国語字形になる
//
// **本テンプレートは (a) 等幅を優先し、`\` は `¥` で表示されるのを許容する**
// （Windows 日本語環境の慣習どおりの見え方になる）。ゴシック体で統一され、
// 和文混じりのコードでも桁がそろう。
//
// **注意**: typst には英語のファミリ名で指定する。"BIZ UDゴシック" では一致せず、
// 別の書体にフォールバックして等幅にならない（実測で確認）。
//
// `\` の字形を優先したいときは、次のいずれかに差し替える:
//   ((name: "NSimSun", covers: regex("[ -~]")), "BIZ UDGothic", …)
//       … ASCII だけ NSimSun にする（covers = 書体を使う文字範囲を絞る typst の機能）。
//         等幅と `\` を両立できるが、欧文がタイプライタ体（明朝系）になる。
//   ((name: "Consolas", covers: regex("[ -~]")), "BIZ UDGothic", …)
//       … 欧文は見慣れた Consolas。ただし桁そろえは崩れる。
#let MONO = ("BIZ UDGothic", "MS Gothic", "Consolas", "Courier New")
// spec: true の表題欄（ヘッダ領域）の書体。MS Gothic を既定にし、無い環境では JP-SANS に落ちる。
#let SPEC-HEAD-FONT = ("MS Gothic", "Yu Gothic", "MS PGothic")

// ---- 色 ----
// コードブロックの背景。Quarto の Skylighting() が使う定数と同じ値にしておく
// （この色で block を特定して両端揃えを切るため。§本文の体裁 の show ルール参照）。
#let CODE-BG = rgb("#f1f3f5")

// ---- 線 ----
#let BORDER = rgb("#b0b0b0")   // 本文中の表罫線（様式ではなく中身の罫線）
#let FRAME = 0.8pt + black     // 様式の外枠。太さを変えるならここ
#let RULE = 0.6pt + black      // 資料番号の枠・IPO 図の罫線

// ---- 本文の体裁 ----
#let BODY-SIZE = 10.5pt        // 本文の文字サイズ
#let BODY-LEADING = 0.9em      // 行間
#let PARA-INDENT = 1em         // 段落先頭の字下げ（1文字ぶん）
#let LIST-INDENT = 1em         // 箇条書きの追加字下げ（見出しレベルぶんに加えて1文字ぶん）
#let FURNITURE-SIZE = 9pt      // 様式の文字（ページ番号）

// 番号付きリスト（enum）の採番。規格 表Ｂ.４ の細別番号に合わせ、入れ子の深さで
//   (1) → ① → (ア) → a) → (a)
// と自動で切り替える。full: true と併用し、各段は自分の階層の記号だけを表示する
// （① を (2)① のように親番号と連ねない）。
//   - カタカナは (ア)=アイウエオ順。イロハ順にするなら "(ア)" を "(イ)" に変える。
//   - 6段目以降は (a) を繰り返す（規格は5段まで想定）。
#let ENUM-NUMBERING = (..n) => {
  let nums = n.pos()
  let d = nums.len()
  let k = nums.last()
  if d == 1 { numbering("(1)", k) }
  else if d == 2 { numbering("①", k) }
  else if d == 3 { numbering("(ア)", k) }
  else if d == 4 { numbering("a)", k) }
  else { numbering("(a)", k) }
}

// 見出しの文字サイズ（h1〜h5）。採番の書式は【3】の set heading を参照。
// 現在は全レベル 10.5pt（本文と同サイズ）。レベルごとに変えたいときは各値を戻す。
// 規格（1.1.1.1.1）に合わせ 5 段まで対応する。
#let HEAD-SIZES = (10.5pt, 10.5pt, 10.5pt, 10.5pt, 10.5pt)
#let HEAD-INDENT-STEP = 1em    // 見出しの字下げ量。レベル n を (n-1) 段字下げする
                               // （L1=0, L2=1, L3=2, L4=3, L5=4。1em はその見出しの1文字ぶん）

// ---- 表紙・前付け（cover: true のときだけ出る）----
// 中身そのものは cover.typ（文書ごとに差し替えられる）にあり、ここはその
// 既定の表紙 default-cover() が使う寸法。
#let COVER-TITLE-SIZE = 24pt        // 表題
#let COVER-TITLE-GAP = 6pt          // 表題と副題の間隔
#let COVER-SUBTITLE-SIZE = 13pt     // 副題
#let COVER-AUTHOR-GAP = 24pt        // 副題と作成者の間隔
#let COVER-AUTHOR-SIZE = 11pt       // 作成者
#let COVER-SUB-COLOR = rgb("#666")  // 副題・作成者の文字色
// 様式（外枠・資料番号）を出さないページ（bare-page）の余白。表紙を紙面いっぱいに
// 使いたいときに使う。既定の表紙は本文と同じ様式のままなのでこの値を使わない。
#let COVER-MARGIN = 30mm

// ---- 縦ページ（ポートレート）----
// 本文は外枠の内側 5mm に流し込む（外枠と本文の間隔を上下左右で 5mm に統一）。
//   左 = 外枠左23 + 5 = 28 / 右 = 210 - (外枠右201 - 5) = 14
//   上 = 外枠上27 + 5 = 32 / 下 = 297 - (外枠下283 - 5) = 19
#let PAGE-P-MARGIN = (left: 28mm, right: 14mm, top: 32mm, bottom: 19mm)  // 本文の余白
#let FRAME-P-POS = (x: 23mm, y: 27mm)                    // 外枠の左上（上端を 20→27mm に下げた）
#let FRAME-P-SIZE = (width: 178mm, height: 256mm)        // 外枠の大きさ（高さ 268→256mm、12mm 縮小）

// ---- 資料番号の欄（見た目は縦横で共通、寸法・位置は縦横で分離）----
#let DOCNUM-INSET = (x: 3.5mm, y: 1.65mm) // 資料番号枠の内側の余白（y が枠の高さを決める）
#let DOCNUM-SIZE = 14pt                   // 枠内の文字サイズ（これも枠の高さに効く）
// 上記の組合せで枠の高さ（A4横では幅）は約 7mm（Yu Gothic 実測）。
#let DOCNUM-PAGE-GAP = 5mm                // 資料枠とページ番号 n / N の間隔
//   ↑ これを増やすとページ番号欄が外枠の縁側へ寄る（外枠との隙間が縮む）。
#let DOCNUM-PAGE-DIGITS = 4               // ページ番号 n / N の想定最大桁数
#let DOCNUM-PAGE-WIDTH = 20mm             // ページ番号欄の幅（右寄せ・固定幅）。
// 欄を固定幅＋右寄せにし、総数 N を DIGITS 桁ぶんの幅へ右空白詰め（数字幅の空白）する
// ことで "/" の位置と欄の右端を安定させ、"/" と総数の間隔も一定にする。
// 欄の右端は資料枠のすぐ右で外枠内に収まる。
// 資料枠の幅・位置（縦横で独立して調整できるよう分離）:
#let DOCNUM-P-WIDTH = 45mm                // A4縦: 資料枠の幅
#let DOCNUM-P-LEFT  = 105mm               // A4縦: 外枠の左端から資料枠の左端まで
// A4横: 資料枠は 90°回転して置くので、この「幅」は用紙上では高さ方向になる。
// 45mm = A4縦の DOCNUM-P-WIDTH と一致 → 縦を 90°回転した姿にそろう。
#let DOCNUM-L-WIDTH = 45mm                // A4横: 資料枠の幅（回転後は高さ）

// ---- スペック様式（spec: true）の表題欄 ----
// 全ページ上部に表題欄を出す。左＝スペック/SPECIFICATION（固定）、中央＝ラベル（固定）、
// 右＝スペック番号(doc-number) と 改訂符号(doc-revision) を別セルに表示。
// 各行の高さ・右の値列幅は資料番号枠と同じ。左列は残り幅。A4横/IPO では 90°回転する。
#let SPEC-ROW = 7mm             // 表題欄の各行の高さ（資料番号枠と同じ）
#let SPEC-LEADING = 0.55em      // 表題欄「スペック/SPECIFICATION」の行間（2行が重ならない値。14mm 内に収まる）
#let SPEC-LABEL-LEADING = 0.25em // 中央ラベル（スペック番号/SPEC NO.・改訂符号/REV LTR）の2行の行間。7mm 枠に被らないよう詰める
#let SPEC-LABEL-W = 25mm        // 中央ラベル列の幅（スペック番号/改訂符号）
#let SPEC-VALUE-W = 45mm        // 右の値列の幅（資料番号枠と同じ）
#let SPEC-TITLE-SIZE = 22pt     // 「スペック」の文字サイズ
#let SPEC-SUB-SIZE = 18pt       // 「SPECIFICATION」の文字サイズ
#let SPEC-LABEL-SIZE = 9pt      // 中央ラベル（スペック番号/SPEC NO. 等）の文字サイズ
#let SPEC-GAP = 5mm             // 表題欄と本文の間隔（本文の上マージンに加算）
#let SPEC-COMPANY-SIZE = 10.5pt // スペック様式フッターの会社名（日/英とも）の文字サイズ
#let SPEC-COMPANY-GAP = 3mm     // 外枠の下辺から会社名フッターまでの間隔
#let SPEC-COMPANY-LEADING = 0.3em // 日本語会社名と英語会社名の行間（詰めると英語が上に寄る）

// ---- 横ページ（landscape / IPO 共通の様式）----
#let FRAME-L-POS = (x: 14mm, y: 23mm)
#let FRAME-L-SIZE = (width: 256mm, height: 178mm)
#let DOCNUM-L-TOP = 105mm   // 外枠の上端から資料枠の上端まで（回転後の上辺位置）

// ---- 横ページの本文余白 ----
// landscape も外枠の内側 5mm に統一（A4縦の PAGE-P-MARGIN を時計回りに 90°回転した値）。
//   左 = 外枠左14 + 5 = 19 / 右 = 297 - (外枠右270 - 5) = 32
//   上 = 外枠上23 + 5 = 28 / 下 = 210 - (外枠下201 - 5) = 14
//   （left←下, top←左, right←上, bottom←右 の対応で縦と一致）
// IPO は枠いっぱいを使う別値（PAGE-IPO-MARGIN）で、ここは変更しない。
#let PAGE-L-MARGIN = (left: 19mm, right: 32mm, top: 28mm, bottom: 14mm)
// IPO は表示領域を広く取るため左右を外枠の内側 3mm まで詰める（上は 5mm）。
// 左右は fr 列で本文幅に追従するのでここで決まるが、下辺（表の下端）は
// IPO-BODY-ROW（固定高さ）で決まる → 下の隙間は IPO-BOTTOM-GAP 側で調整する。
// bottom は表の下辺をクリップしないための余白（隙間量そのものは決めない）。
//   左 = 外枠左14 + 3 = 17 / 右 = 297 - (外枠右270 - 3) = 30 / 上 = 外枠上23 + 5 = 28
#let PAGE-IPO-MARGIN = (left: 17mm, right: 30mm, top: 28mm, bottom: 9mm)

// ---- IPO 図の枠割り ----
// 上段（機能名/処理名）: ラベル幅・値幅・ラベル幅・残り
#let IPO-TITLE-COLS = (24mm, 72mm, 24mm, 1fr)
#let IPO-TITLE-ROW = 8.5mm
#let IPO-NUM-ROW = 5mm      // 最上部「表番号」の領域高さ（表番号とIPO表の間隔を決める）
#let IPO-NUM-SIZE = 10.5pt  // 表番号の文字サイズ
// 下段（入力/処理/出力）: 列比。処理欄に mermaid を置くので中央を広く取る。
// **この比は design-doc.css の .ipo-frame の grid-template-columns と揃えること**
// （HTML 側の見た目が PDF とずれる）。
#let IPO-COLS = (1fr, 4.9fr, 1.1fr)
#let IPO-HEAD-ROW = 8mm     // 「入力/処理/出力」の見出し行の高さ
#let IPO-INSET = (x: 2.5mm, y: 2mm)
#let IPO-HEAD-TRACKING = 3pt
// 上段と下段の table を隙間なく繋ぐ。罫線1本ぶん（0.6pt）重ねる。
#let IPO-STACK-OVERLAP = -0.6pt
// 本体の高さは固定だが、値を直書きせず「表の下辺が外枠の下辺から IPO-BOTTOM-GAP だけ
// 内側に来る」よう逆算する。表の上辺 = PAGE-IPO-MARGIN.top なので、上マージンを
// 変えても下の隙間は保たれる（PAGE-IPO-MARGIN.bottom は下辺をクリップしないための
// 余白で、隙間量そのものは決めない）。
#let IPO-BOTTOM-GAP = 3mm   // 外枠の下辺と IPO 枠の下辺の隙間
// 最上部の表番号欄（IPO-NUM-ROW）ぶんも差し引いて、表全体が外枠下辺の内側に収まるようにする。
#let IPO-BODY-ROW = (
  FRAME-L-POS.y + FRAME-L-SIZE.height - IPO-BOTTOM-GAP
    - PAGE-IPO-MARGIN.top - IPO-NUM-ROW - IPO-TITLE-ROW - IPO-HEAD-ROW - IPO-STACK-OVERLAP
)

// ============================================================
//  【2】様式パーツ（外枠・資料番号）
// ============================================================

// 様式メタ。design-doc() が設定し、横向きページの様式描画が読む。
// state を使うのは、landscape()/ipo() が design-doc() の引数を直接見られない
// （別の呼び出しなので）ため。
#let _doc-number = state("design-doc-number", "")
// スペック様式かどうか。横ページ/IPO の様式描画（_side-furniture）が読む。
#let _spec = state("design-spec", false)
// 改訂符号（スペック様式で番号と別セルに出す）。横ページ/IPO の様式描画が読む。
#let _doc-revision = state("design-doc-revision", "")
// スペック様式フッターの会社名（日/英）。横ページ/IPO の様式描画が読む。
#let _company-ja = state("design-company-ja", "")
#let _company-en = state("design-company-en", "")
// 直前の見出しレベルに応じた本文の字下げ段数（L1=0, L2=1, L3=2 …）。
// 各 show heading が更新し、show par が本文段落の左字下げに使う。
#let _sec-indent = state("design-sec-indent", 0)
// リスト（list/enum）の入れ子ガード。最上位のリストにだけ見出しレベルの字下げを
// 与え、入れ子は Typst 標準のネスト字下げに任せる（見出しレベルぶんを段ごとに
// 重ね掛けして右へ流れるのを防ぐ）。list/enum 共通の1つの状態で管理する。
#let _in-list = state("design-in-list", false)
// IPO 図の入力/出力欄を描いている間だけ真。真のとき箇条書きの追加字下げ（LIST-INDENT）
// を抜いて左端から詰める（定型枠を広く使う。記号は残す）。_list-pad が読み、ipo() が
// 入出力描画区間で立てる。
#let _ipo-tight = state("design-ipo-tight", false)

// 数字の後ろに数字幅の空白（U+2007）を足して指定桁ぶんの幅に揃える（右空白詰め）。
// ASCII 空白と違い Typst で連続空白が畳まれず、tabular 数字と組めば
// 「/」と総数の間隔が桁数によらず一定になる（総数は「/」直後に左寄せ）。
#let _pad-num(n, digits) = {
  let s = str(n)
  s + "\u{2007}" * calc.max(0, digits - s.len())
}

// ---- 資料番号の欄 + ページ番号 n / N ----
// 縦ページでは右上に水平、横ページでは右端に 90°回転して置かれる。
// 中身は同じなのでこの1つを使い回す。ラベル文字は付けず、番号だけを枠内に置く。
//   [SD-2026-001]  3 / 10
//    └─ 枠あり ─┘  └ 枠なし
// ページ番号欄は固定幅＋右寄せ。総数 N を DOCNUM-PAGE-DIGITS 桁ぶんへ右空白詰めし、
// tabular 数字にすることで "/" の位置・"/" と総数の間隔を安定させる（4桁まで対応）。
// 総ページ数は counter(page).final() で組版後に確定するため context が要る。
// ページ番号 n / N の欄（固定幅・右寄せ）。資料番号欄・スペック様式で共用する。
#let _pagenum-box() = context {
  set text(font: JP-SANS, size: FURNITURE-SIZE)
  let total = counter(page).final().first()
  box(width: DOCNUM-PAGE-WIDTH, inset: (y: DOCNUM-INSET.y),
    align(right, text(number-width: "tabular")[
      #counter(page).display("1") / #_pad-num(total, DOCNUM-PAGE-DIGITS)
    ]))
}

#let _docnum-strip(doc-number, box-width: auto) = stack(dir: ltr, spacing: DOCNUM-PAGE-GAP,
  box(width: box-width, stroke: RULE, inset: DOCNUM-INSET,
    align(center, text(font: JP-SANS, size: DOCNUM-SIZE)[#doc-number])),
  // ページ番号は枠外。固定幅の欄に右寄せ。
  _pagenum-box(),
)

// ---- スペック様式の表題欄（spec: true。全ページ上部・幅いっぱい）----
// 左＝スペック/SPECIFICATION（固定・2行結合）、中央＝固定ラベル、右＝番号・改訂符号。
// width は外枠の幅（縦）／高さ（横・回転後）。左列は残り幅。
#let _spec-title-block(doc-number, doc-revision, width) = {
  // 行ボックスと行間を詰めて 2行を 7mm/14mm に収める。書体は表題欄専用（MS Gothic）。
  set text(font: SPEC-HEAD-FONT, top-edge: "cap-height", bottom-edge: "baseline")
  set par(leading: SPEC-LEADING)
  let left-w = width - SPEC-LABEL-W - SPEC-VALUE-W
  // 中央ラベルの2行は、タイトル(スペック/SPECIFICATION)より行間を詰めて 7mm 枠に収める。
  let lbl(a, b) = { set par(leading: SPEC-LABEL-LEADING); text(size: SPEC-LABEL-SIZE)[#a\ #b] }
  // table ではなく grid で組む。表題欄は「様式の部品」であって表データではないうえ、
  // ここは set page(background:) の中＝レイアウト中に何度も評価される場所なので、
  // 本文用の show table（セル内リストの字下げを外すため state を更新する）を
  // 巻き込むと state が自分自身を更新し続けて "layout did not converge" になる。
  // grid は罫線・inset・整列をすべて明示しているので見た目は table と同じ。
  grid(
    columns: (left-w, SPEC-LABEL-W, SPEC-VALUE-W),
    rows: (SPEC-ROW, SPEC-ROW),
    stroke: RULE, inset: (x: 1.5mm, y: 0pt), align: center + horizon,
    grid.cell(rowspan: 2, {
      text(size: SPEC-TITLE-SIZE)[スペック]
      linebreak()
      text(size: SPEC-SUB-SIZE)[SPECIFICATION]
    }),
    lbl[スペック番号][SPEC NO.], text(size: DOCNUM-SIZE)[#doc-number],
    lbl[改訂符号][REV LTR],     text(size: DOCNUM-SIZE)[#doc-revision],
  )
}

// ---- スペック様式のフッター（会社名。日本語→英語、指定幅の中央に2行）----
// width の中央に配置する。縦は外枠の下（幅=ページ幅）、横/IPO は左余白に 90°回転で置く。
#let _spec-footer(company-ja, company-en, width) = block(width: width, align(center, {
  set text(font: JP-SANS, size: SPEC-COMPANY-SIZE)
  set par(leading: SPEC-COMPANY-LEADING)
  text(company-ja); linebreak(); text(company-en)
}))

// 図表番号の接頭辞「章.節.項…」を、指定位置の見出しカウンタから組み立てる。
// design-doc()（図表の採番・参照）と ipo()（表番号）で共用する。
// 見出しはレベル1〜5まで採番している（heading numbering: 1.1.1.1）ので、
// 図表番号もその深さに追従させる。例: 章直下=「3」、節=「3.2」、項=「3.2.1」…（最大レベル5）。
#let _section-prefix(loc) = {
  let heads = counter(heading).at(loc)
  // 実際に番号が付いている最下位レベル（1〜5）を求める。下位の 0（未使用の
  // レベル）は落とし、「3.2.0」のような 0 を含む番号を出さない。
  let depth = 1
  for i in range(calc.min(heads.len(), 5)) {
    if heads.at(i, default: 0) != 0 { depth = i + 1 }
  }
  // 見出しより前（front matter 等）の図表でカウンタが空でも壊さない。
  let nums = heads.slice(0, calc.min(depth, heads.len()))
  if nums.len() == 0 { nums = (0,) }
  // depth 個の "1" を "." で連結したパターン（例 depth=3 → "1.1.1"）で採番。
  let pat = range(depth).map(_ => "1").join(".")
  numbering(pat, ..nums)
}

// IPO の表番号用の接頭辞。図表番号と同じ「章.節.項…」（最大レベル5）を出す。
#let _ipo-prefix(loc) = _section-prefix(loc)

// 自前採番の表（.tbl・.ipo）を @tbl- で相互参照するためのヘルパ。
// design-doc.lua が @tbl- 参照を #_xref("tbl-x") に置換して呼ぶ（Quarto の crossref は
// フロートにしか効かず、自前採番の表は未解決＝「?」になってしまうため先回りする）。
//   - .tbl / .ipo: 採番位置に置いた <sn-tbl-x> ラベルの location で図表カウンタを読み、
//     キャプションと同じ「表 章.節-連番」を出す（接頭辞は _section-prefix と共有）。
//   - 通常のフロート表（`: cap {#tbl-x}`）: Quarto が付ける <tbl-x> へ ref で委譲し、
//     既存の show ref フック（design-doc 内）が同じ番号を出す。
//   - どちらのラベルも無い（綴り違い等）: build を壊さず赤い「?」を出す。
#let _xref(name) = context {
  let sn = query(label("sn-" + name))
  if sn.len() > 0 {
    let loc = sn.first().location()
    let n = counter(figure.where(kind: "quarto-float-tbl")).at(loc).first()
    link(loc, [表 #_section-prefix(loc)-#n])
  } else if query(label(name)).len() > 0 {
    ref(label(name))
  } else {
    text(fill: red)[?#name]
  }
}

// ---- 横向きページの様式（枠・資料番号を 90°回転で配置） ----
// 縦綴じのまま用紙を回して読む配置なので、資料番号を右端に縦置きする。
// 資料番号の枠は外枠の右辺に接する（原紙準拠）ため、x は
// 「外枠の左端 + 外枠の幅」で求める。外枠を動かせば資料番号も追従する。
#let _side-furniture = context {
  // 外枠はどちらの様式でも描く。
  place(top + left, dx: FRAME-L-POS.x, dy: FRAME-L-POS.y,
    rect(width: FRAME-L-SIZE.width, height: FRAME-L-SIZE.height, stroke: FRAME))
  if _spec.get() {
    // スペック様式: 表題欄を外枠の右側の帯（幅 = 表題欄の高さ 2*SPEC-ROW）に 90°回転で置く。
    // 帯は外枠の高さいっぱい（＝縦様式の幅いっぱいを回転したもの）。
    place(top + left,
      dx: FRAME-L-POS.x + FRAME-L-SIZE.width - 2 * SPEC-ROW,
      dy: FRAME-L-POS.y,
      rotate(90deg, reflow: true,
        _spec-title-block(_doc-number.get(), _doc-revision.get(), FRAME-L-SIZE.height)))
    // ページ番号（回転）。表題欄の右（右余白）・下寄りに置く。
    place(top + left,
      dx: FRAME-L-POS.x + FRAME-L-SIZE.width + 1mm,
      dy: FRAME-L-POS.y + FRAME-L-SIZE.height - DOCNUM-PAGE-WIDTH - 2mm,
      rotate(90deg, reflow: true, _pagenum-box()))
    // フッター（会社名）を左余白に 90°回転で、ページ中央（縦）に置く（縦様式を回転した形）。
    let ft = _spec-footer(_company-ja.get(), _company-en.get(), 210mm)
    let ftw = measure(ft).height   // 回転前の高さ = 回転後の帯の幅
    place(top + left, dx: (FRAME-L-POS.x - ftw) / 2, dy: 0mm,
      rotate(90deg, reflow: true, ft))
  } else {
    // 通常様式: 資料番号（番号+改訂記号）+ ページ番号を外枠の右辺に 90°回転で置く。
    place(top + left, dx: FRAME-L-POS.x + FRAME-L-SIZE.width,
      dy: FRAME-L-POS.y + DOCNUM-L-TOP,
      rotate(90deg, reflow: true,
        _docnum-strip(_doc-number.get() + _doc-revision.get(), box-width: DOCNUM-L-WIDTH)))
  }
}

// ------------------------------------------------------------
//  表紙・前付けの部品（cover.typ から使う）
//
//  表紙と前書きの「中身」は文書ごとに違うので、実装は執筆フォルダの cover.typ に
//  逃がしてある（差し替えても lib.typ は触らない）。ここに置くのは、その cover.typ
//  から呼べる共通部品だけ。
// ------------------------------------------------------------

// 様式（外枠・資料番号・スペック表題欄）を出さない独立ページ。
// 罫線だけの承認欄や、社内様式と違う体裁の表紙・中扉に使う。
//
// page() を「関数として」呼ぶと、その中身だけが指定の設定で別ページに組まれ、
// 後続の本文は元の設定に戻る（set page と違って戻し忘れが起きない）。
#let bare-page(body, margin: COVER-MARGIN) = page(
  margin: margin, background: none, header: none, footer: none, body,
)

// ---- 前付け（index.qmd の中身）を表紙側へ渡す ----
// book では index.qmd が必須で、その中身は既定では目次の後ろ（本文の先頭）に出る。
// 表紙に文章を載せる様式や、表紙の次のページに置く様式もあるので、index.qmd を
// `::: {.front-matter}` で囲むと design-doc.lua がそれを #front-slot[...] に変える。
//
// **本文の中身を前へ動かす方法はこれしかない。** typst は組版順にしか流せないので、
// 「その場では出さずに印だけ置き、表紙から参照して組む」形にしてある。
// body の木を組み替えて切り出す手も一見できそうだが、Quarto は本文の直前に
// show ルール（図表カウンタのリセット）を挿すため body 全体が styled 要素1つに
// 包まれており、children を辿っても中身に手が届かない（実測）。
// 前付けの受け口。その場では何も描かず、内容にラベル付きの metadata で印を付けるだけ。
// どこに出すか（出さないか）は表紙側＝ cover.typ が決める。
//
// **表紙を出さない（cover: false）ときは PDF に出ない。** これは仕様である。
// 「index.qmd に書くことが無い（章はすべて chapters/ に分ける）」文書では、
// 表紙の有無にかかわらず前付けの痕跡（空ページ・目次項目）を消したいため。
// 中身を本文に出したいなら front-in-cover を書かない（＝囲まない）。
#let front-slot(body) = [#metadata(body)<df-front>]

// 前付けの体裁。本文の「章（h1）は常に新しいページから始める」ルールを外す。
// 前付けを表紙のページへ流し込んだとき、見出しで改ページされて表紙が2ページに
// 割れてしまうため（実測）。内側の show ルールが優先されるので、本文側の
// ルールはそのままでよい。見出しの見た目は本文の h1 と同じにそろえる。
// **it ではなく it.body を組むこと。** it を組み直すと「他の」show ルール
// （design-doc 内の h1 ルール）が改めて適用され、その pagebreak が block の中で
// 起きて "pagebreaks are not allowed inside of containers" になる（実測）。
// it.body なら見出し要素そのものを再生成しないので、他のルールは走らない。
#let _front-style(body) = {
  show heading.where(level: 1): it => block(above: 1.4em, below: 0.8em,
    text(size: HEAD-SIZES.at(0), weight: "bold", it.body))
  body
}

// 前付けの内容。cover.typ には第2引数 front としてこれが渡る。
// 囲んでいない文書では query が空になり、何も出ない。
#let front-content = context {
  let q = query(<df-front>)
  if q.len() > 0 { _front-style(q.first().value) }
}

// 前付けがあるときだけ改ページして置く（無ければ空ページも作らない）。
// 既定の cover.typ が使う。表紙ページの中に流し込むときは front をそのまま置けばよい。
#let front-on-new-page(front) = context {
  if query(<df-front>).len() > 0 {
    pagebreak()
    front
  }
}

// ---- SVG の下絵から表紙を作る ----
// Inkscape / Illustrator / PowerPoint などで A4 の表紙を描き、差し込みたい箇所に
// {{title}} のような「差し込み口」を**文字として**書いておく。ここでその文字列を
// _quarto.yml の値に置き換えてから画像として読み込む。
// 罫線の多い表紙（査印欄つきなど）を、座標計算も Typst の記法もほぼ書かずに作れる。
//
//   #bare-page(margin: 0mm)[
//     #place(top + left, svg-cover("cover.svg",
//       title: meta.title, docid: meta.doc-id, company: meta.company-ja))
//   ]
//
// 注意:
//  - SVG は執筆フォルダ（typst のプロジェクトルート）に置く。typst はプロジェクト外を
//    読めないため。doc リポジトリではコミット対象にする（cover.typ と同じ扱い）。
//  - SVG は A4 実寸（width="210mm" height="297mm"）で作ると位置がそのまま合う。
//  - **SVG 内の文字はビルドする環境のフォントで描かれる。** 無い書体は置換されるので、
//    固定文言は描画ツール側で「パスに変換」しておくと確実（Inkscape: パス → オブジェクトをパスへ）。
//    差し込む値（表題・資料番号）は文字のままにしておく必要がある。
//  - 影・グラデーション・ブレンドなど、typst の SVG 描画が再現しない効果がある。
//    罫線・文字・単色塗りに寄せること。
//  - **Word の表紙は「PDF に保存 → Inkscape で開く → プレーン SVG で保存」で取り込める。**
//    Poppler/Cairo で取り込むと文字がすべてパスになるのでフォント非依存になる代わりに、
//    差し込み口（{{...}}）は使えない。その場合は枠・罫線だけを SVG に任せ、
//    差し替わる値は #place(dx:, dy:) で上に重ねる（書体が本文とそろうのでこちらが堅い）。
#let _svg-escape(s) = s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
#let svg-cover(path, width: 210mm, ..subs) = {
  let s = read(path)
  for (k, v) in subs.named() { s = s.replace("{{" + k + "}}", _svg-escape(str(v))) }
  image(bytes(s), format: "svg", width: width)
}

// 既定の表紙。表題・副題・作成者をページ中央に置くだけの最小構成で、
// 様式は本文と同じ（外枠・資料番号が出る）。
// cover.typ から部品として呼べる（既定のまま使う／一部だけ足す、のどちらも可）。
#let default-cover(meta) = {
  align(center + horizon)[
    #text(font: JP-SANS, size: COVER-TITLE-SIZE, weight: "bold", meta.title)
    #v(COVER-TITLE-GAP)
    #if meta.subtitle != "" {
      text(size: COVER-SUBTITLE-SIZE, fill: COVER-SUB-COLOR, meta.subtitle)
    }
    #v(COVER-AUTHOR-GAP)
    #text(size: COVER-AUTHOR-SIZE, fill: COVER-SUB-COLOR, meta.author)
  ]
}

// ============================================================
//  【3】文書全体のセットアップ（本文はポートレート）
//
//  Quarto から使うときは typst-show.typ がフロントマターを引数に渡す。
//  素の Typst から使うときは #show: design-doc.with(...) と書く。
// ============================================================
#let design-doc(
  title: "設計書", subtitle: "", author: "",
  doc-number: "",
  doc-revision: "",                    // 改訂記号（A〜Z / NC）。既定は空。資料番号の末尾に結合
  spec: false,                         // スペック様式（全ページ上部に表題欄）にするか。既定 false
  company-ja: "", company-en: "",      // スペック様式のフッターに出す会社名（日本語/英語）
  cover: false,                        // 表紙・前付けを出すか。既定は出さない
  fields: (:),                         // _quarto.yml の cover-fields（任意の項目）。
                                       // 作成日・部署・機密区分など、表紙にだけ要る記載を
                                       // テンプレートを直さずに増やせる。meta.fields で渡る
  front-matter: none,                  // 表紙・前付けを組む関数（cover.typ の front-matter）。
                                       // none なら既定の表紙 default-cover() を使う
  page-start: 1,                       // 開始ページ番号。表紙・前書きを別文書で作り、その
                                       // 続きとして綴じるときに使う。既定 1（＝1から振る）
  toc: false, toc-title: "目 次", toc-depth: 3,
  body,
) = {
  set document(title: title, author: author)
  // 通常様式: 資料番号は「番号 + 改訂記号」を結合して表示（改訂記号が空なら番号のみ）。
  // スペック様式: 番号(doc-number)と改訂符号(doc-revision)を別セルに分けて表示。
  // 横ページは state 経由、縦ページは下の背景で引数を直接使う。
  let doc-id = doc-number + doc-revision
  // state には「生の番号」と「改訂符号」を別々に持たせる（横/IPO で通常様式は結合、
  // スペック様式は別セルに使うため）。縦ページは背景で引数を直接使う。
  _doc-number.update(doc-number)
  _doc-revision.update(doc-revision)
  _company-ja.update(company-ja)
  _company-en.update(company-en)
  _spec.update(spec)
  set page(
    paper: "a4",
    // スペック様式は表題欄（2行）のぶん本文の上マージンを下げる。
    margin: if spec {
      (left: PAGE-P-MARGIN.left, right: PAGE-P-MARGIN.right,
       top: FRAME-P-POS.y + 2 * SPEC-ROW + SPEC-GAP, bottom: PAGE-P-MARGIN.bottom)
    } else { PAGE-P-MARGIN },
    header: none,
    footer: none,
    // 外枠と資料番号（またはスペック表題欄）は「背景」層に描く。本文の流し込みに
    // 影響させないため。通常様式の資料番号は枠の下辺が外枠の上辺に接地するよう
    // measure() で高さを測って引く（文字サイズや inset を変えても接地が保たれる）。
    background: context {
      // 外枠はどちらの様式でも描く。
      place(top + left, dx: FRAME-P-POS.x, dy: FRAME-P-POS.y,
        rect(width: FRAME-P-SIZE.width, height: FRAME-P-SIZE.height,
          stroke: FRAME))
      if spec {
        // 表題欄を外枠の上部（内側）に幅いっぱいで置く。
        place(top + left, dx: FRAME-P-POS.x, dy: FRAME-P-POS.y,
          _spec-title-block(doc-number, doc-revision, FRAME-P-SIZE.width))
        // ページ番号は表題欄の上・右側（横位置は通常様式と同じ、上にスライド）。
        let pg = _pagenum-box()
        let ph = measure(pg).height
        place(top + left,
          dx: FRAME-P-POS.x + DOCNUM-P-LEFT + DOCNUM-P-WIDTH + DOCNUM-PAGE-GAP,
          dy: FRAME-P-POS.y - ph - 1mm, pg)
        // フッター: 会社名を外枠の下・ページ中央に。日本語（上）→ 英語（下）。
        place(top + left, dx: 0mm,
          dy: FRAME-P-POS.y + FRAME-P-SIZE.height + SPEC-COMPANY-GAP,
          _spec-footer(company-ja, company-en, 100%))
      } else {
        // 通常様式: 資料番号枠 + ページ番号を外枠の左端から DOCNUM-P-LEFT、上辺に接地。
        let strip = _docnum-strip(doc-id, box-width: DOCNUM-P-WIDTH)
        let h = measure(strip).height
        place(top + left, dx: FRAME-P-POS.x + DOCNUM-P-LEFT, dy: FRAME-P-POS.y - h, strip)
      }
    },
  )
  set text(font: JP-SANS, size: BODY-SIZE, lang: "ja")
  // 段落の先頭を1文字字下げ。all: true で見出し直後の段落も字下げする（和文の作法）。
  set par(justify: true, leading: BODY-LEADING,
    first-line-indent: (amount: PARA-INDENT, all: true))
  // 本文段落を、直前の見出しレベルと同じ位置から始める（見出し配下の本文をレベルぶん
  // 字下げ）。字下げ段数は _sec-indent（各 show heading が更新）。見出し自身は各
  // show heading の inset で字下げ済み。表・図キャプションなど段落でない要素や
  // セル内の段落には影響しない（top-level の段落だけがこの show に掛かる）。
  // リスト項目の本文（子リストを持つ項目は本文が Para 化する）には掛けない。
  // リスト側で位置決め済みなので、掛けるとマーカーと本文の間に隙間が空く。
  show par: it => context {
    if _in-list.get() { it } else { pad(left: _sec-indent.get() * HEAD-INDENT-STEP, it) }
  }
  // コードブロック（```…```）には本文の体裁を持ち込まない。
  // 本文は justify: true（両端揃え）なので、そのままだと**長い行が折り返されたときに
  // 単語間が引き伸ばされて均等割り付けのように見える**（コマンド行でよく起きる）。
  // 先頭字下げも同様に不要。
  //
  // 対象が2種類あることに注意（実測）:
  //   1) 素の raw ブロック（構文強調が無い場合）
  //   2) Quarto の Skylighting()（構文強調がある場合）… ```lang フェンスはこちら。
  //      raw 要素ではなく「トークンごとの inline raw を並べた**ふつうの段落**」を
  //      block で包んだものなので、raw の show ルールでは捕まえられない。
  //      背景色（CODE-BG = Quarto 側の定数）で block を特定する。
  show raw.where(block: true): it => {
    set par(justify: false, first-line-indent: 0pt)
    it
  }
  show block.where(fill: CODE-BG): set par(justify: false, first-line-indent: 0pt)
  // コード（ブロック・インラインとも）は等幅書体にする。Quarto の構文強調は
  // トークンごとの inline raw なので、raw への set text がそのまま効く。
  show raw: set text(font: MONO)
  // 箇条書き（list/enum）は「最上位のリストだけ」を見出しレベルぶん＋LIST-INDENT
  // 字下げする（段落の先頭字下げと頭を揃える）。入れ子のリストには重ね掛けせず、
  // Typst 標準のネスト字下げに任せる（重ね掛けすると段ごとに右へ流れてしまう）。
  // _in-list は入れ子判定用の共有フラグ。update の順序（true → 中身 → false）で、
  // 中身のリストは _in-list=true を見て追加の pad を掛けない。
  let _list-pad(it) = context {
    if _in-list.get() { it } else if _ipo-tight.get() {
      // IPO の入出力欄では字下げせず左端から詰める（記号は ipo() 側で none）。
      _in-list.update(true)
      it
      _in-list.update(false)
    } else {
      _in-list.update(true)
      pad(left: _sec-indent.get() * HEAD-INDENT-STEP + LIST-INDENT, it)
      _in-list.update(false)
    }
  }
  show list: it => _list-pad(it)
  show enum: it => _list-pad(it)
  // 表のセル内のリストには見出しレベルの字下げを効かせない。セルは本文の字下げ体系と
  // 無関係で、見出しが深いほど右へ流れて狭いセルを圧迫するため（IPO の入出力欄と同じ
  // 考え方）。追加字下げ（LIST-INDENT = 1文字ぶん）だけ残し、HTML 側の
  // 「td/th の中の ul/ol は padding-left: 1.2em」（design-doc.css）と見た目を揃える。
  // 段落（show par）は元から top-level だけに掛かるのでセル内は影響を受けない。
  // ipo() は自前で _sec-indent を 0 にしてから table を組むので、ここは素通りになる。
  // あわせて**セル内は両端揃えにしない**。セルは幅が狭く、ファイル名・コマンドなど
  // 途中で折り返せない語が入ると、その行だけ字間が大きく開いて読みにくくなる
  //（例「<執 筆 フ ォ ル ダ>/chapters/…」）。列の揃え指定（`:---`）は最終行の寄せを
  // 決めるだけで両端揃えは止まらないため、ここで par 側を切る（実測で確認）。
  // 本文の段落は justify: true のまま（和文の作法）。
  show table: it => context {
    let prev = _sec-indent.get()
    _sec-indent.update(0)
    set par(justify: false)
    it
    _sec-indent.update(prev)
  }
  // 番号付きリストの採番を規格 表Ｂ.４ の細別番号（深さ連動）にする。
  set enum(full: true, numbering: ENUM-NUMBERING)

  // 章番号の自動採番。レベル1だけ末尾にドットを付ける（"1." / "1.1" / "1.1.1"）。
  // 付録を A.1 にしたい等はここを差し替える。
  set heading(numbering: (..n) => {
    let nums = n.pos()
    if nums.len() == 1 { numbering("1.", nums.at(0)) }
    else { numbering("1.1.1.1", ..nums) }
  })
  // 各見出し（level 1〜5）の先頭で図表カウンタを 0 に戻す。
  // → 図表番号は「章.節.項…-連番」で、連番はその見出し（節・項…）ごとにリセットされる。
  // （関数にして毎回新しい update を発行する。値を使い回すと1回しか発火しない）
  //
  // ただし番号なし見出し（{.unnumbered} = heading(numbering: none)）ではリセットしない。
  // 番号なし見出しは counter(heading) を進めないので接頭辞（_section-prefix）が親のまま。
  // ここでリセットすると「接頭辞は同じ・連番だけ 1 に戻る」＝直前の番号と重複する
  // （例: 章直下 表 1-1 の後に ## 付録 {.unnumbered} を挟むと次表も 表 1-1）。
  // 各 show ルールでは `if it.numbering != none` で番号あり見出しのときだけリセットする
  // （HTML 側は接頭辞キーで連番を持つため元々継続する。これで PDF と挙動が揃う）。
  // 対象 kind: image/table は素の Typst（jp-figure/jp-table）、
  // quarto-float-* は Quarto qmd フローの図表。両フローで動くよう全て 0 に戻す。
  let reset-floats() = {
    for k in (image, table, "quarto-float-fig", "quarto-float-tbl") {
      counter(figure.where(kind: k)).update(0)
    }
  }
  // 見出しの体裁。サイズは HEAD-SIZES、前後の空きは下の above/below。
  // 左インセットで見出しをレベルごとに字下げ（L1=0, L2=1, L3=2 …段）。
  // 図表カウンタは各レベル（h1〜h5）の先頭でリセットする。図表番号の接頭辞
  // （_section-prefix）がその見出しの深さ「章.節.項…」を出すので、連番も
  // その見出しごとに 1 から振り直す。
  show heading.where(level: 1): it => {
    // 見出しレベル1（章）は常に新しいページから始める。weak: true でページ先頭では
    // 改ページせず、先頭章・目次直後などに空白ページが入らない（PDF のみ）。
    pagebreak(weak: true)
    if it.numbering != none { reset-floats() }
    _sec-indent.update(0)
    set text(size: HEAD-SIZES.at(0), weight: "bold")
    block(above: 1.4em, below: 0.8em, inset: (left: 0 * HEAD-INDENT-STEP), it)
  }
  show heading.where(level: 2): it => {
    if it.numbering != none { reset-floats() }
    _sec-indent.update(1)
    set text(size: HEAD-SIZES.at(1), weight: "bold")
    block(above: 1.1em, below: 0.6em, inset: (left: 1 * HEAD-INDENT-STEP), it)
  }
  show heading.where(level: 3): it => {
    if it.numbering != none { reset-floats() }
    _sec-indent.update(2)
    set text(size: HEAD-SIZES.at(2), weight: "bold")
    block(above: 1em, below: 0.5em, inset: (left: 2 * HEAD-INDENT-STEP), it)
  }
  show heading.where(level: 4): it => {
    if it.numbering != none { reset-floats() }
    _sec-indent.update(3)
    set text(size: HEAD-SIZES.at(3), weight: "bold")
    block(above: 0.9em, below: 0.4em, inset: (left: 3 * HEAD-INDENT-STEP), it)
  }
  show heading.where(level: 5): it => {
    if it.numbering != none { reset-floats() }
    _sec-indent.update(4)
    set text(size: HEAD-SIZES.at(4), weight: "bold")
    block(above: 0.85em, below: 0.35em, inset: (left: 4 * HEAD-INDENT-STEP), it)
  }

  // 図・表の日本語キャプションと採番（「図 3.2-1」＝ 章.節-連番。接頭辞は _section-prefix）
  set figure.caption(separator: [　])
  show figure.where(kind: table): set figure.caption(position: top)
  set figure(numbering: n => context _section-prefix(here()) + "-" + str(n))
  // 参照(@fig-x)も本体と同じ番号にする。必ず対象図表の location で counter を読む
  // （参照位置で評価すると節番号がずれるため）。
  show ref: it => {
    let el = it.element
    if el != none and el.func() == figure {
      let loc = el.location()
      let n = counter(figure.where(kind: el.kind)).at(loc).first()
      link(loc, [#el.supplement #_section-prefix(loc)-#n])
    } else if el != none and el.func() == heading {
      // 見出し参照(@sec-x): レベル1 = 「5章」、レベル2以降 = 「5.3節」。
      // Quarto 既定の「チャプター 5 / セクション 5.3」を、番号＋章/節の後置表記に替える。
      let loc = el.location()
      let nums = counter(heading).at(loc).slice(0, el.level)
      let s = nums.map(n => str(n)).join(".")
      let suffix = if el.level == 1 { "章" } else { "節" }
      link(loc, [#s#suffix])
    } else { it }
  }

  // ---- 開始ページ番号（page-start）----
  // design-doc() が出す最初の「内容」なので、1ページ目からこの番号が振られる。
  // 表紙・前書きを別文書で作り、その続きとして綴じるときに使う（前付けが4ページなら
  // page-start: 5）。フッターの "n / N" の N は counter(page).final()＝最終ページの
  // 番号だから、ずらせば N は綴じ上がり全体の総ページ数と一致する。
  counter(page).update(page-start)

  // ---- 表紙・前付け（cover: true のときだけ出す。既定は出さない）----
  // 中身は cover.typ の front-matter() が決める（文書ごとに差し替えられる）。
  // 渡していないとき（素の Typst で書くとき等）は既定の表紙 default-cover()。
  // 表紙を出さないときは pagebreak も打たない（先頭の空ページを避けるため）。
  //
  // 前書き（まえがき・承認ページ・改訂履歴など）も front-matter() の中に書く。
  // 目次より前に置きたい前付けはすべてここ、目次より後ろでよいものは index.qmd。
  // 見出し（heading）を使うときは numbering: none / outlined: false を付けること。
  // 素の h1 にすると章カウンタを消費し、目次にも項目として載ってしまう。
  if cover {
    let meta = (
      title: title, subtitle: subtitle, author: author,
      doc-number: doc-number, doc-revision: doc-revision, doc-id: doc-id,
      spec: spec, company-ja: company-ja, company-en: company-en,
      page-start: page-start,
      fields: fields,                  // _quarto.yml の cover-fields（任意の項目）
    )
    if front-matter != none {
      front-matter(meta, front-content)
    } else {
      // front-matter を渡していないとき（素の Typst で書くとき等）の既定。
      // 前付けを落とさないよう、あれば表紙の次のページに置く。
      default-cover(meta)
      front-on-new-page(front-content)
    }
    // weak: true にしておくと、front-matter() が bare-page() や pagebreak() で
    // 自分の最後のページを閉じている場合に空ページが増えない。
    pagebreak(weak: true)
  }

  // 目次（toc: true のとき）。章番号・ページ番号・リーダー線は outline が自動生成する。
  if toc {
    show outline.entry.where(level: 1): set block(above: 1.2em)
    outline(
      title: text(size: 16pt, weight: "bold", toc-title),
      depth: toc-depth, indent: 2em,
    )
    pagebreak()
  }

  body
}

// ---- 図ヘルパー（相互参照用ラベルは呼び出し側で <fig-xxx> を付ける） ----
#let jp-figure(content, caption: "") = figure(
  content, caption: caption, kind: image, supplement: [図],
)
#let jp-table(content, caption: "") = figure(
  content, caption: caption, kind: table, supplement: [表],
)

// ============================================================
//  【4】landscape() : 中身だけ横向き様式ページにして、終わったら縦に戻す
//
//  末尾の set page(flipped: false) が「戻す」役。これが無いと以降の本文が
//  全部横になる。header/footer を none にするのは、横ページのページ番号を
//  _side-furniture が回転して描くため（二重に出さない）。
// ============================================================
#let landscape(body) = context {
  // スペック様式では右側に表題欄の帯（幅 2*SPEC-ROW）が入るので本文の右余白を広げる。
  let extra-right = if _spec.get() { 2 * SPEC-ROW } else { 0mm }
  set page(
    flipped: true,
    margin: (..PAGE-L-MARGIN, right: PAGE-L-MARGIN.right + extra-right),
    header: none, footer: none,
    background: _side-furniture,
  )
  body
  set page(flipped: false)
}

// ============================================================
//  【5】IPO図（横向き様式ページ + 機能名/処理名 + 入力/処理/出力）
//
//  構造は table 2つを縦に積んだだけ:
//    ┌──────┬────────────┬──────┬──────────┐  ← IPO-TITLE-COLS
//    │機能名│ 受注管理   │処理名│ 受注処理 │     高さ IPO-TITLE-ROW
//    ├──────┴┬───────────┴─┬────┴──────────┤
//    │ 入力  │    処理      │     出力      │  ← IPO-HEAD-ROW
//    ├───────┼──────────────┼───────────────┤
//    │       │              │               │
//    │       │  (mermaid)   │               │     高さ IPO-BODY-ROW
//    └───────┴──────────────┴───────────────┘  ← 列比 IPO-COLS
//
//  2つの table の境目は罫線が二重になるので IPO-STACK-OVERLAP で1本ぶん重ねる。
//  自由な結線が要る場合は process に SVG/PlantUML 画像を貼る
//  （項目単位の配線は自動組版では再現しない方針。CLAUDE.md「既知の限界」参照）。
//
//  qmd から使うときは design-doc.lua が ::: {.ipo} div をこの呼び出しに変換する。
//  執筆者がこの関数を直接書くのは素の Typst（main.typ）のときだけ。
// ============================================================
//  分割（複数パート）: parts に複数の (input:, process:, output:, cap:) を渡すと、
//  同じ表番号を共有したまま横向きページを M 枚出す（パート間は改ページ）。表番号は
//  先頭パートで1回だけ step し、全パートが同じ番号を表示する（表の分割 .tbl と同流儀）。
//  相互参照: ref="tbl-x" を渡すと採番位置に <sn-tbl-x> ラベルを置き、本文 @tbl-x
//  （design-doc.lua が #_xref に置換）から番号を解決できる。
//  parts 省略時は単一パート（後方互換。素の Typst で書くときの input/process/output）。
#let ipo(
  function-name: "", process-name: "",
  caption: "",                       // 表番号の右に付けるタイトル（機能名/処理名とは別）
  ref: "",                           // 相互参照 id（"tbl-x"）。空なら参照ラベルなし
  input: [], process: [], output: [],
  parts: none,                       // ((input:, process:, output:, cap:), …) 各要素=1パート
) = context {
  // parts 省略時は input/process/output を単一パートとして扱う（cap = caption）。
  let parts = if parts == none {
    ((input: input, process: process, output: output, cap: caption),)
  } else { parts }
  let M = parts.len()
  // IPO 内の入力/処理/出力欄は横向き定型枠であり、本文の見出しレベル字下げ
  // （_sec-indent に基づく show par/list/enum の左パディング）を持ち込まない。
  // IPO 本体を描く間だけ _sec-indent を 0 に落とし、描画後に元の値へ戻す。
  let prev-sec-indent = _sec-indent.get()
  // スペック様式では右側に表題欄の帯（幅 2*SPEC-ROW）が入るので IPO 表の右余白を広げる。
  let extra-right = if _spec.get() { 2 * SPEC-ROW } else { 0mm }
  set page(
    flipped: true,
    margin: (..PAGE-IPO-MARGIN, right: PAGE-IPO-MARGIN.right + extra-right),
    header: none, footer: none,
    background: _side-furniture,
  )
  set text(font: JP-SANS, size: 10pt)
  // 欄見出し（機能名/処理名/入力/処理/出力）の共通スタイル
  let head(t) = align(center + horizon,
    text(weight: "bold", tracking: IPO-HEAD-TRACKING, t))
  // この IPO を「表」として採番する（図ではなく表。ドキュメントの表と同じ連番列に載る）。
  // step は先頭パートの表番号の位置で1回だけ行い、その位置の値を全パートが表示する。
  let tblc = counter(figure.where(kind: "quarto-float-tbl"))
  // 表番号 + タイトル欄。i==0（先頭パート）だけカウンタを step し、ref があれば
  // 採番位置に <sn-ref> ラベルを置く（_xref がこの location で番号を読む）。
  let num-block(i, p) = block(width: 100%, height: IPO-NUM-ROW, {
    if i == 0 {
      tblc.step()
      if ref != "" [#metadata(none)#label("sn-" + ref)]
    }
    align(center + top, context text(size: IPO-NUM-SIZE, top-edge: "cap-height")[
      表 #_ipo-prefix(here())-#tblc.get().first()#if p.cap != "" [　#p.cap]
    ])
  })
  // IPO 本体（2つの table を罫線1本ぶん重ねて接続）
  let body-block(p) = stack(spacing: IPO-STACK-OVERLAP,
    // 上段: 機能名 / 処理名
    table(
      columns: IPO-TITLE-COLS, rows: IPO-TITLE-ROW,
      stroke: RULE, inset: (x: 2mm, y: 0pt),
      head("機能名"), align(horizon, function-name),
      head("処理名"), align(horizon, process-name),
    ),
    // 下段: 入力 / 処理 / 出力（見出しセルだけ inset 0 で中央揃えを効かせる）
    table(
      columns: IPO-COLS, rows: (IPO-HEAD-ROW, IPO-BODY-ROW),
      stroke: RULE, inset: IPO-INSET,
      table.cell(inset: 0pt, head("入力")),
      table.cell(inset: 0pt, head("処理")),
      table.cell(inset: 0pt, head("出力")),
      align(top, p.input), align(top, p.process), align(top, p.output),
    ),
  )
  // ここから IPO 本体。入出力欄のリスト/段落に見出しレベル字下げを効かせない。
  // さらに入出力欄は定型枠を広く使うため、箇条書きの追加字下げ（LIST-INDENT）を抜いて
  // 左端から詰める（_ipo-tight を立てると _list-pad が字下げを抜く）。ただし記号は残す
  // （複数行に折り返したとき項目の区切りが分かるように。折り返し本文は body-indent の
  //  ぶんだけぶら下がる）。
  _sec-indent.update(0)
  _ipo-tight.update(true)
  // 各パートを1ページずつ描画。2枚目以降は改ページで新しい横向きページに載せる。
  for (i, p) in parts.enumerate() {
    if i > 0 { pagebreak(weak: true) }
    stack(dir: ttb, spacing: 0pt, num-block(i, p), body-block(p))
  }
  // IPO 本体を抜けたので、周囲の見出しレベル字下げ・箇条書き設定を元に戻す。
  _ipo-tight.update(false)
  _sec-indent.update(prev-sec-indent)
  set page(flipped: false)
}

// ============================================================
//  【6】callout（.callout-note 等）のアイコンを HTML 版に寄せる
//
//  Quarto が Typst に渡すアイコンは HTML 版と絵柄が違い、とくに
//    note      → fa-info        = 丸の無い裸の "i"
//    important → fa-exclamation = 裸の "!"（実体は ASCII の U+0021）
//  が「ただの文字」に見えて浮く。HTML 側は丸付き（ⓘ / 丸に !）なので、
//  丸付きの字形へ差し替えて PDF と HTML の見た目を揃える。
//  warning / caution / tip は既定でも HTML とほぼ同じ絵柄なので、
//  塗り（solid）だけ HTML に合わせる。
//  ※ caution の HTML アイコンは三角コーンだが Font Awesome Free に無い
//    （Pro 限定）ため、炎のままとする。
//
//  仕組み: Quarto は
//    #callout(..., icon: fa-info(), icon_color: rgb("#0758E5"), ...)
//  の形で呼ぶ。icon から種別は判らないが icon_color が種別ごとに固有
//  （下の CALLOUT-ICONS のキー = quarto の callouts.lua の kColor*）なので、
//  そこから引き当てて icon だけ差し替え、あとは Quarto 既定の callout に
//  委譲する。ブロック構造を変えないので、相互参照付き callout を分解する
//  Quarto 側の show rule もそのまま動く。
//
//  差し替えの適用は typst-template.typ 側で
//    #let callout = with-html-callout-icons(callout)
//  と1行書いて行う。Quarto 既定の callout は definitions.typ で定義され、
//  別モジュールであるこの lib.typ からは見えないため、ここでは「包む関数」
//  だけを提供し、束ね直しは lib.typ を import した先で行う。
//
//  fontawesome の import を関数の中に置いているのは、この lib.typ 単体では
//  外部パッケージに依存しないでおくため（typst の import は遅延評価で、
//  callout を1つも使わない文書ではパッケージを要求しない）。
// ============================================================
#let with-html-callout-icons(base-callout) = (icon: none, icon_color: black, ..args) => {
  // 取り込む名前は1行に並べる（typst の import リストは行を折り返せない）。
  import "@preview/fontawesome:0.5.0": fa-circle-info, fa-circle-exclamation, fa-triangle-exclamation, fa-fire, fa-lightbulb
  let CALLOUT-ICONS = (
    "#0758e5": fa-circle-info(),          // note
    "#cc1914": fa-circle-exclamation(),   // important
    "#eb9113": fa-triangle-exclamation(), // warning
    "#fc5300": fa-fire(solid: true),      // caution
    "#00a047": fa-lightbulb(solid: true), // tip
  )
  base-callout(
    // icon: false 指定のとき Quarto は icon: none を渡す。そのまま無アイコンにする。
    icon: if icon == none { none } else {
      CALLOUT-ICONS.at(lower(icon_color.to-hex()), default: icon)
    },
    icon_color: icon_color,
    ..args,
  )
}
