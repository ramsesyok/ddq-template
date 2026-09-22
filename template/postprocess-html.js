// ============================================================
//  book の各章 HTML の図表番号を PDF と同じ「章.節.項…-連番」に振り直す。
//  接頭辞は見出しの深さ（レベル1〜5）に追従する（例 図 3-1 / 図 3.2-1 / 図 3.2.1-1）。
//
//  実行のされ方:
//    執筆フォルダの `_quarto.yml` の `project.post-render` に登録してあり、
//    `quarto render` / `quarto preview` のたびに Quarto が呼ぶ。**Quarto 同梱の
//    Deno で走るので node は要らない**（使うのは node:fs / node:path だけ）。
//    手で走らせるときは `quarto run postprocess-html.js [出力ディレクトリ]`。
//
//  なぜ後処理なのか:
//    Quarto の図表採番（crossref）は **すべての Lua フィルタより後段**で走る。
//    pre-quarto / post-quarto のどちらで見ても、この時点では表に id も
//    キャプションも付いておらず、参照は未解決の Cite のままである。
//    つまりフィルタでは上書きできないため、生成された HTML を直す。
//
//  book 固有の事情が2つある:
//    1) 連番は文書全体で決まるので、まず全章を book の並び順に走査して番号を
//       確定し（パス1）、そのあと全ファイルの参照を解決する（パス2）。
//    2) 相互参照は他の章ファイルを指す（href="../02-architecture.html#fig-x"）。
//       href の #フラグメントだけを見て全体マップを引けばよい。
//
//  Quarto が出す形は規則的で、次の2箇所だけ書き換えれば足りる:
//    キャプション: <figcaption ... id="fig-arch-caption-XXX">図&nbsp;1.1: 説明</figcaption>
//    相互参照    : <a href="...#fig-arch" class="quarto-xref">図&nbsp;<span>1.1</span></a>
//
//  節番号は Quarto が見出しに付ける data-number="3.3.2" から取る。
//  なお NBSP は book 出力では実体参照 &nbsp;、単一 HTML 出力では生の U+00A0 と
//  形が違う。どちらでも拾えるように SP で両対応にしてある。
//
//  **冪等性（重要）**: `quarto preview` は保存のたびに post-render を呼び、その際
//  再生成されるのは編集した章だけで、他章は前回処理済みの HTML が `_book/` に
//  残っている。したがって「処理済みの HTML をもう一度読んでも同じ番号を導出し、
//  同じ結果を書く」ことが必須になる。そのため:
//    - キャプションの正規表現は未処理（図 1.1:）と処理済み（図 3.2-1　）の両方に一致する
//    - 分割表のキャプションは、前回前置したラベルを剥がしてから前置し直す
//  これを崩すと、プレビューのたびに「表 3-1　表 3-1　…」と番号が積み重なる。
// ============================================================
import fs from 'node:fs';
import path from 'node:path';

// PDF(typst) を出すときも post-render は呼ばれる。出力に HTML が1つも無ければ
// 何もせず正常終了する（QUARTO_PROJECT_OUTPUT_FILES は Quarto が渡す。
// preview の初回だけ空になることがあるので「空＝HTML なし」とは見なさない）。
const outFiles = (process.env.QUARTO_PROJECT_OUTPUT_FILES || '')
  .split(/\r?\n/).map((s) => s.trim()).filter(Boolean);
if (outFiles.length > 0 && !outFiles.some((f) => f.endsWith('.html'))) process.exit(0);

const root = process.argv[2] || process.env.QUARTO_PROJECT_OUTPUT_DIR || '_book';

// 章順は _quarto.yml の chapters をそのまま使う（並びを二重管理しない）。
// chapters は「- パス.qmd」が1行1件で並ぶだけなので、簡易パースで足りる。
// 章ファイルは .qmd と .md のどちらでもよい（実行コードセルを使わない章は .md にできる）。
const conf = fs.readFileSync('_quarto.yml', 'utf8');
const block = conf.match(/^\s*chapters:\s*$([\s\S]*?)^\S/m)
  || conf.match(/^\s*chapters:\s*$([\s\S]*)/m);
const order = [...(block?.[1] ?? '').matchAll(/^\s*-\s*(\S+\.(?:qmd|md))\s*$/gm)]
  .map((m) => m[1].replace(/\.(?:qmd|md)$/, '.html'));
if (order.length === 0) {
  console.error('_quarto.yml の chapters を読めませんでした');
  process.exit(1);
}

const SP = String.raw`(?:\s|&nbsp;)*`;
// 章番号は本文の章タイトル <h1 class="title"><span class="chapter-number">3</span>…
// から取る。data-number は節見出し（h2 以降）にしか付かないため、章直下（節なし）の
// 図表では章番号が拾えず 0 になってしまう。サイドバー／パンくずにも chapter-number は
// あるが、それらは <h1 class="title"> に包まれないので誤って拾うことはない。
// 番号なし章（{.unnumbered}）は chapter-number を持たず、章番号 0（例 図 0-1）になる。
// 章見出しに id を付けた（`# タイトル {#sec-x}`）ときは、Quarto が中身を
// <span class="quarto-section-identifier"> で包むので chapter-number が
// <h1 class="title"> の直後に来ない。包みの span を読み飛ばせるようにしておく
// （読み飛ばす側に chapter-number 自体を食わせないよう否定先読みで除く）。
// これを怠ると章直下（節なし）の図表が「表 0-1」になる。
const CHAP = String.raw`<h1 class="title">(?:<span(?![^>]*\bclass="chapter-number")[^>]*>)*(?:<span class="chapter-number">(\d+)</span>)?`;
const HEAD = String.raw`<h[2-6][^>]*\bdata-number="([\d.]+)"`;
// 「図 1.1: 」（Quarto 既定＝未処理）と「図 3.2-1　」（前回この後処理が書いた形）の
// どちらにも一致させる（冪等性のため。ヘッダのコメント参照）。
const NUM = String.raw`[\d.]+(?:-\d+)?(?::${SP}|　)`;
const CAP = String.raw`<figcaption[^>]*\bid="((?:fig|tbl)-[^"]*?)-caption-[^"]*"[^>]*>${SP}(図|表)${SP}${NUM}`;
const CAP_TAIL = new RegExp(String.raw`(図|表)${SP}${NUM}$`);
// 自前採番の表(.tbl/.ipo)のキャプション div（design-doc.lua が各パートの上に置く）。
// 先頭パート(data-split-first)で1つの表番号を確定し、続くパートも同じ番号を共有する。
// 番号は本文の表と同じ連番列に載せる（走査順に採番）。
// class は "split-caption" の後ろに Quarto が id 付き要素へ付与する " anchored" 等が
// 続くことがあるので、追加クラスを許容して**そのまま書き戻す**。
const SPLIT = String.raw`<div class="(split-caption[^"]*)"([^>]*)>([\s\S]*?)</div>`;
// 前回この後処理が前置したラベル（「表 3-1　」）。剥がしてから付け直す。
const SPLIT_LABEL = /^(?:図|表)\s*[\d.]+-\d+　/;

const numberOf = new Map();   // floatId -> "図 3.3-1"
const fileOf = new Map();     // floatId -> 章 HTML の相対パス（他章からの参照の href を直すため）
const staged = new Map();     // file -> { html, repl }

// --- パス1: 章順に走査して番号を確定する ---
for (const rel of order) {
  const file = path.join(root, rel);
  // preview 中に章を増やした直後など、まだ出力の無い章は飛ばす。
  if (!fs.existsSync(file)) continue;
  const html = fs.readFileSync(file, 'utf8');
  const scan = new RegExp(`${CHAP}|${HEAD}|${CAP}|${SPLIT}`, 'g');
  let chap = 0;
  // いま処理中の見出しの番号（data-number）。章直下（節なし）は空文字。
  // 図表番号は PDF と同じく見出しの深さ「章.節.項…」（最大レベル5）に追従する。
  let secKey = '';
  const seq = new Map();      // "図|3.3.2" -> 連番
  const repl = [];            // [start, end, 置換文字列]
  let splitLabel = '';        // いま処理中の分割表グループの共有ラベル
  // 図表番号の接頭辞: 節なし（章直下 = h1）は「3.0」ではなく章のみ「3」。
  const prefixOf = () => (secKey === '' ? `${chap}` : secKey);

  for (let m; (m = scan.exec(html)) !== null; ) {
    if (m[0].startsWith('<h1')) {               // 章タイトル（章番号を確定・節を戻す）
      chap = Number(m[1]) || 0;
      secKey = '';
      continue;
    }
    if (m[2]) {                                 // 節以下の見出し
      // data-number="3.3.2" をそのまま接頭辞に使う（レベル5まで、以降は切る）。
      const parts = m[2].split('.').slice(0, 5);
      chap = Number(parts[0]) || 0;
      secKey = parts.join('.');
      continue;
    }
    if (m[5] !== undefined) {                   // 分割表のキャプション div
      const cls = m[5];
      const attrs = m[6];
      // 前回の実行で前置したラベルは剥がす（再実行で二重に積まないため）。
      const inner = m[7].replace(SPLIT_LABEL, '');
      // .unnumbered な .tbl は番号を付けない。前置も連番の消費もせず素通し。
      if (/data-unnumbered="true"/.test(attrs)) continue;
      if (/data-split-first="true"/.test(attrs)) {
        // 先頭パート = ここで表番号を1つ確定（本文の表と同じ「表」列の連番）
        const prefix = prefixOf();
        const key = `表|${prefix}`;
        const n = (seq.get(key) || 0) + 1;
        seq.set(key, n);
        splitLabel = `表 ${prefix}-${n}`;
        // data-ref があれば id→番号を登録し、@tbl-x の参照（design-doc.lua が
        // 出す <a class="quarto-xref">）を pass2 で本文の表と同じ経路で解決させる。
        const refm = attrs.match(/data-ref="([^"]+)"/);
        if (refm) { numberOf.set(refm[1], splitLabel); fileOf.set(refm[1], rel); }
      }
      // 続くパートも先頭と同じ番号。キャプション先頭に前置する（本文「（i／M）…」は残す）。
      const label = splitLabel || '表 ?';
      repl.push([m.index, m.index + m[0].length,
        `<div class="${cls}"${attrs}>${label}　${inner}</div>`]);
      continue;
    }
    const [floatId, kind] = [m[3], m[4]];
    const prefix = prefixOf();
    const key = `${kind}|${prefix}`;
    const n = (seq.get(key) || 0) + 1;
    seq.set(key, n);
    const label = `${kind} ${prefix}-${n}`;
    numberOf.set(floatId, label);
    fileOf.set(floatId, rel);
    // マッチした「図&nbsp;1.1: 」の部分だけを差し替える（キャプション本文は残す）
    repl.push([m.index, m.index + m[0].length, m[0].replace(CAP_TAIL, `${label}　`)]);
  }
  staged.set(file, { html, repl });
}

// --- パス2: キャプションを差し替え、全ファイルの参照を確定した番号に揃える ---
let refCount = 0;
let missing = 0;
for (const [file, { html, repl }] of staged) {
  let out = html;
  // 後ろから差し替えて位置ずれを避ける
  for (let i = repl.length - 1; i >= 0; i -= 1) {
    const [s, e, text] = repl[i];
    out = out.slice(0, s) + text + out.slice(e);
  }
  // 自前採番の表(.tbl/.ipo)への参照は design-doc.lua が href="#tbl-x"（同一ページ内）で
  // 出す。表が別の章ファイルにあるとリンク切れになるので、パス1で控えた所在へ相対パスを
  // 足す（Quarto 標準の相互参照は最初から "../x.html#fig-x" の形なので触らない）。
  const relFile = path.relative(root, file).split(path.sep).join('/');
  out = out.replace(
    /(<a href=")(#(?:fig|tbl)-[^"]+)("[^>]*class="[^"]*quarto-xref[^"]*"[^>]*>)/g,
    (whole, open, frag, close) => {
      const target = fileOf.get(frag.slice(1));
      if (!target || target === relFile) return whole;
      const href = path.relative(path.dirname(relFile), target).split(path.sep).join('/');
      return `${open}${href}${frag}${close}`;
    });
  out = out.replace(
    /(<a href="[^"]*#((?:fig|tbl)-[^"]+)"[^>]*class="[^"]*quarto-xref[^"]*"[^>]*>)([\s\S]*?)(<\/a>)/g,
    (whole, open, id, _text, close) => {
      const label = numberOf.get(id);
      if (!label) { missing += 1; return whole; }
      refCount += 1;
      return `${open}${label}${close}`;
    });
  // 見出し参照(@sec-x): Quarto 既定の「チャプター 5 / セクション 5.3」を、番号＋章/節の
  // 後置表記へ。番号にドットが無い（＝章）→「5章」、ドットあり（＝節以下）→「5.3節」。
  // 言語に依存しないよう、番号の前の語（チャプター/セクション等）は捨てて番号だけ使う。
  //
  // href に #sec- は要求できない。章ファイルの h1 に付けた id（`# 章題 {#sec-ipo}`）を
  // 参照すると、Quarto はフラグメントを省いて章ページ自体へリンクするためである:
  //   節を指す → <a href="../05-review/index.html#sec-preview-diff" class="quarto-xref">…
  //   章を指す → <a href="../10-ipo/index.html" class="quarto-xref">…（#sec- が無い）
  // そこで href は問わず「中身が <span>非数字の語＋番号</span> だけ」で拾い、フラグメントが
  // 付いている場合だけ sec- を要求する（＝図表など他種の参照は素通しする）。
  // 冪等性: 変換後は <span>10章</span> となり、番号の直後が </span> でなくなるので
  // 二度目の実行では一致しない（「10章章」にはならない）。
  out = out.replace(
    /(<a href="([^"]*)"[^>]*class="[^"]*quarto-xref[^"]*"[^>]*>)<span>[^0-9<]*([\d.]+)<\/span>(<\/a>)/g,
    (whole, open, href, num, close) => {
      const frag = href.split('#')[1];
      if (frag && !frag.startsWith('sec-')) return whole;   // 図表など見出し以外の参照
      const suffix = num.includes('.') ? '節' : '章';
      refCount += 1;
      return `${open}<span>${num}${suffix}</span>${close}`;
    });
  fs.writeFileSync(file, out);
}

console.log(`OK -> ${root} (図表 ${numberOf.size} 件 / 参照 ${refCount} 件を章.節.項…-連番に変換)`);
// 解決できない参照があっても**異常終了しない**。post-render の失敗は
// `quarto render` / `quarto preview` 全体の失敗になり、執筆が止まってしまう。
// 綴り間違いは警告として出し、出力はそのまま残す。
if (missing > 0) {
  console.error(`警告: 解決できない相互参照が ${missing} 件あります（id の綴りを確認）`);
}
