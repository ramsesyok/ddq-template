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
// 章ファイルは .qmd と .md のどちらでもよい（実行コードセルを使わない章は .md にできる）。
const conf = fs.readFileSync('_quarto.yml', 'utf8');

/**
 * `- ` の後ろが文書ファイルのパスならそれを返す（`part:` などは undefined）。ddq（Rust）の
 * `cli/src/doc/project.rs` の `as_doc_path` と同じ規則（GEN-01。両方を
 * cli/tests/fixtures/contract/chapters.json に照らす）:
 *   - 二重引用 "…": `\\` は `\`、`\"` は `"`。ほかの `\x` はそのまま
 *   - 単一引用 '…': `''` は `'`
 *   - 引用なし: 先頭か空白の後の `#` から後ろはコメント。前後の空白は落とし、途中の空白はパスの一部
 * 引用の後ろには空白とコメントしか置けない。`:` を含むもの（`part: …` などの対応付け）は外す。
 */
const docPath = (item) => {
  const s = item.trim();
  let value;
  let rest = '';
  const quoted = /^"((?:[^"\\]|\\.)*)"(.*)$/.exec(s) ?? /^'((?:[^']|'')*)'(.*)$/.exec(s);
  if (quoted) {
    value = s.startsWith('"')
      ? quoted[1].replace(/\\(.)/g, (whole, c) => (c === '\\' || c === '"' ? c : whole))
      : quoted[1].replace(/''/g, "'");
    rest = quoted[2];
  } else if (/^["']/.test(s)) {
    return undefined;                     // 閉じていない引用
  } else {
    value = s.replace(/(?:^|\s)#.*$/, '').trimEnd();
  }
  rest = rest.trimStart();
  if (!(rest === '' || rest.startsWith('#')) || value.includes(':')) return undefined;
  return /\.(?:qmd|md)$/i.test(value) ? value.replace(/\\/g, '/') : undefined;
};

/** `<key>: <値>` の値（`key:` の後ろが空白か行末のとき。`part:x` は YAML では 1 つの文字列） */
const valueOf = (s, key) => (s.startsWith(`${key}:`) && /^(?:\s|$)/.test(s.slice(key.length + 1))
  ? s.slice(key.length + 1)
  : undefined);

/** 対応付けの行 `キー: 値` の [キー, 値]（キーは英数字・`-`・`_`。`valueOf` と同じく `part:x` は 1 つの文字列） */
const mapKey = (s) => {
  const m = /^([A-Za-z0-9_-]+):(?:\s|$)/.exec(s);
  return m ? [m[1], s.slice(m[1].length + 1)] : undefined;
};

/** 引用が始まれるか（その前の、引用の外の空白でない文字が無いか `[`・`{`・`,`・`:`。Rust の `quote_may_open`） */
const quoteMayOpen = (last) => last === undefined || '[{,:'.includes(last);

/**
 * フロー形式の値の中身（開きの `[`・`{` の後ろから、対応する閉じの前まで。Rust の `flow_text` と同じ）。
 * 閉じるまで次の行も読み（`rest` は残りの行の反復子。行の継ぎ目は空白）、空白の後の `#` から行末のコメントは
 * 落とす。閉じないまま終わったら（正しい YAML ではない）、読めた分に入れ子の閉じを補って返す。
 */
const flowText = (first, rest) => {
  let text = '';
  const nested = [];                      // 開いている入れ子の閉じ（外側の閉じは含めない）
  let quote;
  let last;
  for (let line = first; ; ) {
    let afterSpace = true;
    for (let i = 0; i < line.length; i++) {
      const c = line[i];
      if (quote) {
        text += c;
        if (quote === '"' && c === '\\' && i + 1 < line.length) text += line[++i];
        else if (quote === "'" && c === "'" && line[i + 1] === "'") text += line[++i];   // `''` は `'`
        else if (c === quote) {
          quote = undefined;
          last = c;
        }
      } else {
        if (c === '#' && afterSpace) break;
        if ((c === '"' || c === "'") && quoteMayOpen(last)) quote = c;
        else if (c === '[') nested.push(']');
        else if (c === '{') nested.push('}');
        else if ((c === ']' || c === '}') && nested.length === 0) return text;
        else if (c === ']' || c === '}') nested.pop();
        text += c;
        if (!/\s/.test(c)) last = c;
      }
      afterSpace = /\s/.test(c);
    }
    text += ' ';
    const next = rest.next();
    if (next.done) break;
    line = next.value;
  }
  return text + nested.reverse().join('');
};

/** 引用の外の文字の位置（Rust の `unquoted`）。二重引用の中の `\x` と単一引用の中の `''` は閉じではない */
const unquoted = (s) => {
  const out = [];
  let quote;
  let last;
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (quote === '"' && c === '\\') {
      i++;
    } else if (quote && c === quote) {
      if (quote === "'" && s[i + 1] === "'") {
        i++;                              // `''` は `'`
      } else {
        quote = undefined;
        last = c;
      }
    } else if (quote) {
      // 引用の中
    } else if ((c === '"' || c === "'") && quoteMayOpen(last)) {
      quote = c;
    } else {
      out.push(i);
      if (!/\s/.test(c)) last = c;
    }
  }
  return out;
};

/** フロー形式の値の中身を、引用と入れ子の外の `,` で区切る（Rust の `split_flow`） */
const splitFlow = (s) => {
  const pieces = [];
  let from = 0;
  let depth = 0;
  for (const i of unquoted(s)) {
    const c = s[i];
    if (c === '[' || c === '{') depth += 1;
    else if (c === ']' || c === '}') depth -= 1;
    else if (c === ',' && depth === 0) {
      pieces.push(s.slice(from, i));
      from = i + 1;
    }
  }
  pieces.push(s.slice(from));
  return pieces;
};

/**
 * フロー形式の 1 つの値の形（Rust の `flow_node`）: 文字列 { scalar }（引用は付いたまま）・列 { seq: 中身 }・
 * 対応付け { map: 中身 }。括弧を含むのに 1 つの列・対応付けになっていないもの（`a [b]` など）は undefined
 */
const flowNode = (s) => {
  const t = s.trim();
  let depth = 0;
  let bracket = false;
  let closed;
  for (const i of unquoted(t)) {
    const c = t[i];
    if (c === '[' || c === '{') {
      depth += 1;
      bracket = true;
    } else if (c === ']' || c === '}') {
      depth -= 1;
      bracket = true;
      if (depth === 0 && closed === undefined) closed = i;
    }
  }
  if (!bracket) return { scalar: t };
  if (closed !== t.length - 1) return undefined;
  const inner = t.slice(1, -1);
  if (t[0] === '[') return { seq: inner };
  return t[0] === '{' ? { map: inner } : undefined;
};

/** 対応付けの 1 項目 `キー: 値` の [キー, 値]（`:` の後ろが空白か終わりか括弧。Rust の `key_value`） */
const keyValue = (s) => {
  let depth = 0;
  for (const i of unquoted(s)) {
    const c = s[i];
    if (c === '[' || c === '{') depth += 1;
    else if (c === ']' || c === '}') depth -= 1;
    else if (c === ':' && depth === 0 && /^(?:[\s[\]{}]|$)/.test(s.slice(i + 1, i + 2))) {
      return [s.slice(0, i).trim(), s.slice(i + 1)];
    }
  }
  return undefined;
};

/**
 * 項目のページになるキーの強さ（Rust の `page_rank`）。Quarto は 1 つの項目から、部の扉ページ（`part:` の値が
 * ファイル）→ `href:` → `file:` の順に 1 つだけ使う。部の項目（`part:` がある）のページは、部の題名の項目で
 * `href:`・`file:` から取ったものも部のページ（番号・付録の記号を付けない）
 */
const PAGE_RANK = new Map([['part', 3], ['href', 2], ['file', 1]]);

/**
 * フロー形式の値が並べる文書ファイルのパスを `out` に足す（Rust の `flow_docs` と同じ）。列は要素を順に読み、
 * 入れ子の列と括弧の混じった値は外す。対応付け（`{part: …, chapters: […]}`・`{href: …}`）は、項目のページ
 * （`PAGE_RANK` の順に 1 つ）→ `chapters:` の列の順（キーを書いた順にかかわらない）。部の項目のページは `parts` にも入れる
 */
const flowDocs = (node, isFile, out, parts) => {
  if (node?.seq !== undefined) {
    for (const piece of splitFlow(node.seq)) {
      const item = flowNode(piece);
      if (item?.scalar !== undefined) {
        const p = docPath(item.scalar);
        if (p !== undefined) out.push(p);
      } else if (item?.map !== undefined) {
        flowDocs(item, isFile, out, parts);
      }
    }
  } else if (node?.map !== undefined) {
    const entries = splitFlow(node.map).map(keyValue).filter(Boolean);
    const get = (key) => {
      const entry = entries.find(([k]) => k === key);
      return entry ? flowNode(entry[1]) : undefined;
    };
    for (const key of PAGE_RANK.keys()) {
      const value = get(key)?.scalar;
      const p = value === undefined ? undefined : docPath(value);
      if (p !== undefined && (key !== 'part' || isFile(p))) {
        out.push(p);
        if (entries.some(([k]) => k === 'part')) parts.add(p);
        break;
      }
    }
    const chapters = get('chapters');
    if (chapters?.seq !== undefined) flowDocs(chapters, isFile, out, parts);
  }
};

/** フロー形式の値（`[`・`{` の後ろから）が並べる文書ファイルのパス（Rust の `flow_items`） */
const flowItems = (open, first, rest, isFile, parts) => {
  const text = flowText(first, rest);
  const out = [];
  flowDocs(open === '[' ? { seq: text } : { map: text }, isFile, out, parts);
  return out;
};

/**
 * `chapters:` と `appendices:` の下に続く「その行より深い字下げ」の範囲から、文書のパスを
 * 拾う（Rust の `chapters_of` と同じ規則）。キーと同じ字下げの `- ` もそのキーの列。
 * `part:` で束ねた入れ子の `chapters:` も同じ範囲に入るので、外側の列に続けて拾う。
 * 対応付けの項目（`- part:`・`- href:`・`- file:`・`- text:` など。後ろの行のキーも同じ項目）からは、項目の
 * ページを `PAGE_RANK` の順に 1 つ、どの行に書いても項目の始まり（子の章の前）に入れる。`part:` の値は
 * ファイル（`isFile`）なら部の扉ページで、無ければ部の題名なので入れない（Quarto と同じ。H-01）。部の項目の
 * ページは `parts` にも入れる。値はフロー形式 `[a, b]` でもよく、項目もフロー形式の対応付け
 * `{part: …, chapters: […]}` で書ける（`flowDocs`）。`book.references` は `references` に入れる。
 * 章・references・付録は別々に返し、使う側が章 → references → 付録の順に並べる（書いた順にかかわらず）。
 */
const docsOf = (text, isFile) => {
  const lists = { chapters: [], appendices: [], references: undefined, parts: new Set() };
  let inside;                             // { indent, appendix }
  let items = [];                         // 対応付けの項目: { content（中身の字下げ）, at（始まった列の位置）, page, part }
  let book;                               // { indent, child（直下のキーの字下げ） }
  const lines = text.split(/\r?\n/)[Symbol.iterator]();
  // 対応付けの項目のキー 1 つ（項目の行か、後ろの行。Rust の `item_key`）
  const itemKey = (key, value, item, list) => {
    if (key === 'chapters') {
      const flow = value.trimStart();
      if (flow.startsWith('[')) list.push(...flowItems('[', flow.slice(1), lines, isFile, lists.parts));
      return;
    }
    if (key === 'part') item.part = true;
    const rank = PAGE_RANK.get(key);
    const p = rank > item.page ? docPath(value) : undefined;
    if (p !== undefined && (key !== 'part' || isFile(p))) {
      if (item.page === 0) {
        list.splice(item.at, 0, p);
      } else {
        lists.parts.delete(list[item.at]);
        list[item.at] = p;
      }
      item.page = rank;
    }
    if (item.part && item.page > 0) lists.parts.add(list[item.at]);
  };
  for (const line of lines) {
    const trimmed = line.trimStart();
    const indent = line.length - trimmed.length;
    if (trimmed === '' || trimmed.startsWith('#')) continue;
    if (inside && (indent < inside.indent || (indent === inside.indent && !trimmed.startsWith('- ')))) {
      inside = undefined;
    }
    items = items.filter((it) => it.content <= indent);
    if (book && indent <= book.indent) book = undefined;
    else if (book && book.child === undefined) book.child = indent;
    const list = inside?.appendix ? lists.appendices : lists.chapters;

    if (inside && trimmed.startsWith('- ')) {
      const body = trimmed.slice(2).trimStart();
      const entry = mapKey(body);
      if (entry) {
        const item = { content: indent + trimmed.length - body.length, at: list.length, page: 0, part: false };
        itemKey(entry[0], entry[1], item, list);
        items.push(item);
      } else if (body.startsWith('{')) {
        list.push(...flowItems('{', body.slice(1), lines, isFile, lists.parts));
      } else {
        const p = docPath(body);
        if (p !== undefined) list.push(p);
      }
      continue;
    }
    // 対応付けの項目の後ろの行のキー（`part:`・`href:`・`chapters:` など）
    const owner = items[items.length - 1];
    const entry = mapKey(trimmed);
    if (inside && owner?.content === indent && entry) {
      itemKey(entry[0], entry[1], owner, list);
      continue;
    }
    const key = /^(chapters|appendices):(?:\s|$)/.exec(trimmed);
    if (key) {
      const appendix = Boolean(inside?.appendix) || key[1] === 'appendices';
      const flow = trimmed.slice(key[1].length + 1).trimStart();
      if (flow.startsWith('[')) {
        (appendix ? lists.appendices : lists.chapters).push(...flowItems('[', flow.slice(1), lines, isFile, lists.parts));
      } else if (!inside) {
        inside = { indent, appendix };
      }
      continue;
    }
    const refs = valueOf(trimmed, 'references');
    if (refs !== undefined) {
      if (!inside && book?.child === indent) lists.references = docPath(refs);
      continue;
    }
    if (!inside && valueOf(trimmed, 'book') !== undefined) book = { indent, child: undefined };
  }
  return lists;
};

// Quarto と同じく、ファイルであれば部の扉ページ（フォルダや無い名前は部の題名）。cwd は執筆フォルダ
const isFile = (p) => {
  try {
    return fs.statSync(p).isFile();
  } catch {
    return false;
  }
};

const toHtml = (f) => f.replace(/\.(?:qmd|md)$/i, '.html');
const docs = docsOf(conf, isFile);
// book.references は章の後ろ・付録の前（Quarto の bookRenderItems。番号は本文の章番号から取るので並びの位置だけ）
const chapters = [...docs.chapters, ...(docs.references ? [docs.references] : [])].map(toHtml);
const appendices = docs.appendices.map(toHtml);
// 付録（book.appendices）。Quarto は章番号を A・B… と振るが、**HTML の章タイトルには
// chapter-number を出さず**「付録 A — 解析根拠」という文にしてしまう（id を付けた章では
// 特に）。語は言語設定で変わるので、記号は _quarto.yml の並び順から決める。
// 付録の中の部の扉ページは記号を使わない（Quarto も番号なしで出す。番号なしの章と同じく 0）。
const appendixLetter = new Map(
  docs.appendices.filter((f) => !docs.parts.has(f)).map(toHtml).map((f, i) => [f, String.fromCharCode(65 + i)])
);
const order = [...chapters, ...appendices.filter((f) => !chapters.includes(f))];
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
// 付録（book.appendices）では Quarto が章番号を英字（A・B…）、節を "A.1" で出すので、
// 数字だけでなく英大文字も拾う（図表番号は「表 A-1」「表 A.1-1」になる）。
const CHAP = String.raw`<h1 class="title">(?:<span(?![^>]*\bclass="chapter-number")[^>]*>)*(?:<span class="chapter-number">([0-9A-Z]+)</span>)?`;
const HEAD = String.raw`<h[2-6][^>]*\bdata-number="([0-9A-Z][0-9A-Z.]*)"`;
// 「図 1.1: 」（Quarto 既定＝未処理）と「図 3.2-1　」（前回この後処理が書いた形）の
// どちらにも一致させる（冪等性のため。ヘッダのコメント参照）。
// 付録の番号は英字で始まる（表 A-1 / 表 A.1-1）ので、数字だけに限定しない。
const NUM = String.raw`[0-9A-Z][\d.A-Z]*(?:-\d+)?(?::${SP}|　)`;
// コード（lst-。キャプション付きのコードブロック）も図表と同じ「章.節-連番」にする
// （PDF は lib.typ の figure 採番に乗るので「コード 3.2-1」。連番の列は図・表と別）。
const CAP = String.raw`<figcaption[^>]*\bid="((?:fig|tbl|lst)-[^"]*?)-caption-[^"]*"[^>]*>${SP}(図|表|コード)${SP}${NUM}`;
const CAP_TAIL = new RegExp(String.raw`(図|表|コード)${SP}${NUM}$`);
// 自前採番の表(.tbl/.ipo)のキャプション div（design-doc.lua が各パートの上に置く）。
// 先頭パート(data-split-first)で1つの表番号を確定し、続くパートも同じ番号を共有する。
// 番号は本文の表と同じ連番列に載せる（走査順に採番）。
// class は "split-caption" の後ろに Quarto が id 付き要素へ付与する " anchored" 等が
// 続くことがあるので、追加クラスを許容して**そのまま書き戻す**。
const SPLIT = String.raw`<div class="(split-caption[^"]*)"([^>]*)>([\s\S]*?)</div>`;
// 前回この後処理が前置したラベル（「表 3-1　」）。剥がしてから付け直す。
const SPLIT_LABEL = /^(?:図|表)\s*[0-9A-Z][\d.A-Z]*-\d+　/;

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
  // 付録はファイル単位で記号が決まる（章タイトルから拾えないため）
  const fileChap = appendixLetter.get(rel) ?? '0';
  let chap = fileChap;
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
      // 付録は "A" のような英字なので数値化しない（文字列のまま接頭辞に使う）。
      // 章番号が無い（= 付録、または番号なし章）ときはファイル由来の値に戻す。
      chap = m[1] || fileChap;
      secKey = '';
      continue;
    }
    if (m[2]) {                                 // 節以下の見出し
      // data-number="3.3.2" をそのまま接頭辞に使う（レベル5まで、以降は切る）。
      const parts = m[2].split('.').slice(0, 5);
      chap = parts[0] || fileChap;
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
  // 番号付きリストの項目（design-doc.lua が <span id="num-x" data-num-label="3"> を置く）。
  // 番号は Lua がリストの並びから決め済みなので、ここでは所在を控えるだけ。別の章ファイル
  // からの @num-x（Lua では番号が分からず「?」）を、パス2で図表と同じ経路で解決する。
  for (const t of html.matchAll(/<span\b[^>]*>/g)) {
    const id = t[0].match(/\bid="(num-[^"]+)"/);
    const lab = t[0].match(/\bdata-num-label="([^"]*)"/);
    if (id && lab && !numberOf.has(id[1])) { numberOf.set(id[1], lab[1]); fileOf.set(id[1], rel); }
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
  // 自前採番の表(.tbl/.ipo)と番号付きリストの項目への参照は design-doc.lua が
  // href="#tbl-x" / "#num-x"（同一ページ内）で出す。表が別の章ファイルにあるとリンク切れになるので、パス1で控えた所在へ相対パスを
  // 足す（Quarto 標準の相互参照は最初から "../x.html#fig-x" の形なので触らない）。
  const relFile = path.relative(root, file).split(path.sep).join('/');
  out = out.replace(
    /(<a href=")(#(?:fig|tbl|lst|num)-[^"]+)("[^>]*class="[^"]*quarto-xref[^"]*"[^>]*>)/g,
    (whole, open, frag, close) => {
      const target = fileOf.get(frag.slice(1));
      if (!target || target === relFile) return whole;
      const href = path.relative(path.dirname(relFile), target).split(path.sep).join('/');
      return `${open}${href}${frag}${close}`;
    });
  out = out.replace(
    /(<a href="[^"]*#((?:fig|tbl|lst|num)-[^"]+)"[^>]*class="[^"]*quarto-xref[^"]*"[^>]*>)([\s\S]*?)(<\/a>)/g,
    (whole, open, id, _text, close) => {
      const label = numberOf.get(id);
      if (!label) { missing += 1; return whole; }
      refCount += 1;
      return `${open}${label}${close}`;
    });
  // 数式の参照(@eq-x): Quarto 既定の「式 1.1」を、式の右に出る番号と同じ「式 (1.1)」にする
  // （PDF は typst の既定で「式 (1.1)」）。番号は Quarto の採番（章.連番）のまま変えない。
  // 冪等性: 変換後は <span>(1.1)</span> となり数字で始まらないので、二度目は一致しない。
  out = out.replace(
    /(<a href="[^"]*#eq-[^"]+"[^>]*class="[^"]*quarto-xref[^"]*"[^>]*>[^<]*)<span>([0-9A-Z][\d.A-Z]*)<\/span>(<\/a>)/g,
    (whole, open, num, close) => {
      refCount += 1;
      return `${open}<span>(${num})</span>${close}`;
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
    /(<a href="([^"]*)"[^>]*class="[^"]*quarto-xref[^"]*"[^>]*>)<span>[^0-9A-Z<]*([0-9A-Z][\d.A-Z]*)<\/span>(<\/a>)/g,
    (whole, open, href, num, close) => {
      const frag = href.split('#')[1];
      if (frag && !frag.startsWith('sec-')) return whole;   // 図表など見出し以外の参照
      // 付録（番号が英字で始まる）は「付録B」「B.1節」、本文は「5章」「5.3節」。
      const label = num.includes('.')
        ? `${num}節`
        : /^[A-Z]/.test(num) ? `付録${num}` : `${num}章`;
      refCount += 1;
      return `${open}<span>${label}</span>${close}`;
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
