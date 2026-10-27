/**
 * セル本文と Markdown の相互変換。
 *
 * モデル内では改行を '\n' で持ち、出力時に `<br>` へ変換する（AUTHORING.md §6
 * 「セル内で改行する（`<br>`）」）。グリッド表では行末のバックスラッシュ `\` でも
 * 改行できるため、読み込み時はどちらも '\n' に戻す。
 */

/** ソフト改行（空白）と区別するための一時マーカー。本文には現れない制御文字を使う。 */
const HARD_BREAK = '\u0001';

/** コード span を伏せ字にする一時マーカー（HARD_BREAK と別の制御文字）。 */
const CODE_MARK = String.fromCharCode(2);

/**
 * コード span（`` `…` ``）を伏せ字にして、中身を書き換えないようにする。
 *
 * `` `<br>` `` のように **`<br>` を文字として説明しているセル**があるため
 * （テンプレートの記法例がまさにそれ）、改行への変換をコード span の中へ
 * 効かせると本文が壊れる。
 */
function protectCodeSpans(text: string): { masked: string; restore: (s: string) => string } {
  const spans: string[] = [];
  const masked = text.replace(/`+[^`\n]*`+/g, span => {
    spans.push(span);
    return `${CODE_MARK}${spans.length - 1}${CODE_MARK}`;
  });
  const restore = (s: string) =>
    s.replace(
      new RegExp(`${CODE_MARK}(\\d+)${CODE_MARK}`, 'g'),
      (_, i: string) => spans[Number(i)] ?? ''
    );
  return { masked, restore };
}

/** Markdown のセル文字列 → モデルのテキスト（`<br>` / 行末 `\` を改行へ）。 */
export function cellTextFromMarkdown(raw: string): string {
  return raw
    .replace(/<br\s*\/?>/gi, '\n')
    .replace(/\\\n/g, '\n')
    .split('\n')
    .map(line => line.trim())
    .join('\n')
    .trim();
}

/** モデルのテキスト → Markdown のセル文字列（改行を `<br>` へ）。 */
export function cellTextToMarkdown(text: string): string {
  return text.split('\n').map(l => l.trim()).join('<br>');
}

/** パイプ表のセルに入れるため `|` をエスケープする。 */
export function escapePipeCell(text: string): string {
  return cellTextToMarkdown(text).replace(/\|/g, '\\|');
}

export function unescapePipeCell(text: string): string {
  return text.replace(/\\\|/g, '|');
}

/**
 * グリッド表のセル 1 行ぶんをエスケープする。
 *
 * グリッド表は罫線 `|` で列を区切るため、本文中の `|` はエスケープしないと
 * 列の区切りと区別できない。Pandoc は grid table 内の `\|` を `|` と解釈する。
 */
export function escapeGridCellLine(line: string): string {
  return line.replace(/\|/g, '\\|');
}

export function unescapeGridCell(text: string): string {
  return text.replace(/\\\|/g, '|');
}

/** 箇条書き（`- ` / `* ` / `+ `）・番号付きリスト（`1. ` / `1) `）の項目行か。 */
export function isListItemLine(line: string): boolean {
  return /^\s*(?:[-*+]|\d+[.)])\s+/.test(line);
}

/**
 * 各行が「本当にリストとして解釈される項目行か」を返す。
 *
 * pandoc の markdown はリストで段落を中断できない（`lists_without_preceding_blankline`
 * は既定で無効）。`前置き` の直後に `- 項目` と書いても段落の続きになるだけなので、
 * **セルの先頭か空行の後から始まる並び**だけをリストとみなす。
 *
 * 字下げした行は項目内の折り返しなのでリストの中に留まる（そこで途切れると、
 * 折り返しの後ろの `- 項目` を別ブロックと見なせなくなる）。
 */
function listBlockFlags(lines: string[]): boolean[] {
  const flags: boolean[] = [];
  let inList = false;
  let afterBlank = true; // 先頭はブロックの始まり扱い
  for (const line of lines) {
    if (line.trim() === '') {
      inList = false;
      afterBlank = true;
      flags.push(false);
      continue;
    }
    if (isListItemLine(line) && (inList || afterBlank)) {
      inList = true;
      flags.push(true);
    } else {
      // 字下げのある行は項目の続き。無ければ段落に戻る
      if (!/^\s/.test(line)) inList = false;
      flags.push(false);
    }
    afterBlank = false;
  }
  return flags;
}

/**
 * 段落とリストの境目へ空行を補う（付録「段落とリストの組み合わせ」の書き方に合わせる）。
 *
 * 空行が無いと pandoc は 1 つの段落として読むため、`- 項目` が文字のまま出たり、
 * リストの後の段落が最後の項目に飲み込まれたりする。エディタ上では空行なしで
 * 書けるようにし、書き出すときにここで足す（`collapseInsertedBlanks` が対称に畳む）。
 *
 * 字下げした行は項目内の折り返しなので、そこには入れない。
 */
function insertBlankLinesAroundLists(lines: string[]): string[] {
  const out: string[] = [];
  let inList = false;
  let afterBlank = true;
  for (const line of lines) {
    if (line.trim() === '') {
      out.push(line);
      inList = false;
      afterBlank = true;
      continue;
    }
    // 空行を挟まずに続く行は項目内の折り返しなので、リストの中に留める
    // （段落にしたいときは書き手が空行を入れる）
    if (isListItemLine(line) && !inList && !afterBlank) {
      out.push(''); // 段落 → リスト
      inList = true;
    } else if (isListItemLine(line)) {
      inList = true;
    }
    out.push(line);
    afterBlank = false;
  }
  return out;
}

/**
 * リスト項目の続き（空行を挟まずに続く字下げの無い行）を、項目本文の桁まで下げる。
 *
 * 付録「改行と箇条書きの組み合わせ」の書き方に合わせる（`- ` なら 2 桁、`1. ` なら
 * 3 桁、入れ子ならその子の本文の桁）。字下げ済みの行は書き手の指定なのでそのまま。
 */
function indentListContinuations(lines: string[]): string[] {
  const out: string[] = [];
  let contentIndent = '';
  let inList = false;
  let afterBlank = true;
  for (const line of lines) {
    if (line.trim() === '') {
      out.push(line);
      inList = false;
      afterBlank = true;
      contentIndent = '';
      continue;
    }
    const marker = line.match(/^(\s*(?:[-*+]|\d+[.)])\s+)/);
    if (marker && (inList || afterBlank)) {
      inList = true;
      contentIndent = ' '.repeat(marker[1].length);
      out.push(line);
    } else if (inList && !/^\s/.test(line)) {
      out.push(contentIndent + line);
    } else {
      out.push(line);
    }
    afterBlank = false;
  }
  return out;
}

/**
 * `insertBlankLinesAroundLists` が入れた空行を畳む（エディタ上の見た目を元に戻す）。
 *
 * 「その空行を消しても書き出しで同じ位置に戻るか」で判定するので、必ず対称になる。
 */
function collapseInsertedBlanks(lines: string[]): string[] {
  const out: string[] = [];
  for (let i = 0; i < lines.length; i++) {
    const next = lines[i + 1];
    if (lines[i].trim() === '' && out.length > 0 && next !== undefined) {
      const before = insertBlankLinesAroundLists(out).length;
      const after = insertBlankLinesAroundLists([...out, next]).length;
      if (after > before + 1) continue; // 書き出し時に補われる空行なので持たない
    }
    out.push(lines[i]);
  }
  return out;
}

/**
 * グリッド表のセル本文を物理行に分ける。
 *
 * グリッド表のセルは複数行で書けるが、単に行を分けただけだと pandoc はソフト改行
 * （＝空白）として扱う。実際に改行させるには行末に `<br>` かバックスラッシュ `\` が要る
 * （AUTHORING.md §6）。
 *
 * ここでは**バックスラッシュ**を使う。pandoc 上で raw HTML ではなく native な LineBreak
 * になるため PDF（typst）側でもそのまま改行になり、`<br>` のように後続行の先頭へ
 * 余分な空白が入らない（Quarto 同梱 pandoc で確認済み）。
 *
 * ただし次の行送りには付けない。付けなくても別ブロックになり、付けると余分な改行が
 * 入るため（pandoc で確認済み）:
 *
 * - リスト項目どうしの間（各項目の末尾に `<br>` が入ってしまう）
 * - 空行の前後（段落の区切り。`\` だけの行になってしまう）
 *
 * 段落とリストの境目には、そう解釈されるよう空行を補う。
 *
 * 行頭の字下げは**入れ子リストと項目内の折り返し**に意味があるのでそのまま残す
 * （入れ子は子の行を 4 桁、折り返しは項目本文の桁まで下げる）。
 */
export function gridCellLines(text: string): string[] {
  // 行末のバックスラッシュは改行の印としてここで付けるもの。モデル側に残っていたら
  // 落とす（重ねると `\\` ＝エスケープされたバックスラッシュになり、改行が消える）
  const raw = text
    .split('\n')
    .map(l => escapeGridCellLine(l.replace(/\s+$/, '').replace(/\\+$/, '')));
  if (raw.length === 0) return [''];
  const lines = indentListContinuations(insertBlankLinesAroundLists(raw));
  const inList = listBlockFlags(lines);
  return lines.map((line, i) => {
    if (i === lines.length - 1) return line;
    if (line === '' || lines[i + 1] === '') return line;
    // 次がリスト項目なら行送りだけで別項目になる（項目内の折り返しには付ける）
    if (inList[i + 1]) return line;
    return `${line}\\`;
  });
}

/**
 * グリッド表のセルから読み取った物理行を 1 つのテキストへ戻す。
 *
 * pandoc の解釈に合わせる:
 * - `<br>` / 行末バックスラッシュ `\` … ハード改行 → '\n'
 * - 空行の前後・リスト項目どうし … ブロックの区切り → '\n'
 * - 単なる行の続き … ソフト改行 → 空白
 *
 * 段落とリストの境目の空行は `gridCellLines` が補ったものなので畳んで返す。
 * 行頭の字下げ（入れ子・項目内の折り返し）は意味があるので残す。
 */
export function gridCellTextFromLines(lines: string[]): string {
  const raw = lines.map(l => unescapeGridCell(l).replace(/\s+$/, ''));
  const inList = listBlockFlags(raw);

  // 空行の前後とリスト項目どうしの間は `\` が無くても別ブロック＝改行。
  // gridCellLines がそこに `\` を書かないので、読み取りも対称にしておく。
  let joined = '';
  raw.forEach((line, i) => {
    if (i > 0) {
      const prev = raw[i - 1];
      const hard = !prev.endsWith('\\') && (prev === '' || line === '' || inList[i]);
      joined += hard ? HARD_BREAK : '\n';
    }
    joined += line;
  });

  const code = protectCodeSpans(joined);
  const parts = code.masked
    .replace(/<br\s*\/?>[ \t]*\n?/gi, HARD_BREAK)
    // 行末のバックスラッシュは改行。`\\` と重なっていても 1 つの改行として読む
    // （古い版が二重に書いた表を、開いて書き戻すだけで直せるように）
    .replace(/\\+\n/g, HARD_BREAK)
    // ソフト改行は空白 1 つ。続きの行の字下げまで持ち込まない
    .replace(/\n[ \t]*/g, ' ')
    .split(HARD_BREAK)
    .map(part => code.restore(part).replace(/\s+$/, ''));

  return collapseInsertedBlanks(parts).join('\n').trim();
}
