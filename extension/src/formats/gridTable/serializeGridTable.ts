import type { TableModel, TableCell } from '../../model/TableModel';
import { gridCellLines } from '../cellText';
import { displayWidth, padToWidth } from '../displayWidth';
import { alignToGridSeparator } from '../pipeTable/pipeTableAlignment';

/**
 * グリッド表を出力する。
 *
 * 桁は **表示幅**（全角=2 / 半角=1）で揃える。pandoc のグリッド表パーサは文字数では
 * なく表示幅で列位置を決めるため（Quarto 同梱 pandoc で検証済み）、全角文字を 1 文字
 * として揃えると表が 1 セルに潰れる。
 *
 * 記法（AUTHORING.md §6「横結合・見出し結合・マルチヘッダ」）:
 * - 列境界は `+---+---+`、ヘッダの終わりは `+===+===+`
 * - 横結合（colspan）は、内容行でセル間の `|` を書かない
 * - 縦結合（rowspan）は、行の境界線をそのセルの部分だけ空白にする
 * - 列揃えはヘッダ終端行（ヘッダが無ければ最上段の罫線）にコロンを打つ
 */
export function serializeGridTable(model: TableModel): string {
  const colLen = model.columns.length;
  const rowLen = model.rows.length;
  if (colLen === 0 || rowLen === 0) return '';

  const cellLines = model.rows.map(row => row.map(cell => gridCellLines(cell.text)));

  const widths = computeColumnWidths(model, cellLines);
  const heights = computeRowHeights(model, cellLines);

  /** 列 c..c+span-1 をまたぐセルの内容領域の幅。区切り 1 文字＋左右の空白 1 文字ぶんを吸収する。 */
  const spanWidth = (c: number, span: number): number => {
    let w = 0;
    for (let i = c; i < c + span; i++) w += widths[i];
    return w + 3 * (span - 1);
  };

  const lines: string[] = [];
  const headerRows = Math.min(model.headerRows, rowLen);

  // 最上段の罫線。ヘッダが無い表はここに揃えのコロンを打つ。
  lines.push(borderLine(model, widths, -1, headerRows === 0 ? model : undefined, '-'));

  for (let r = 0; r < rowLen; r++) {
    for (let line = 0; line < heights[r]; line++) {
      lines.push(contentLine(model, widths, spanWidth, cellLines, heights, r, line));
    }
    const isHeaderEnd = headerRows > 0 && r === headerRows - 1;
    lines.push(
      borderLine(model, widths, r, isHeaderEnd ? model : undefined, isHeaderEnd ? '=' : '-')
    );
  }

  return lines.join('\n');
}

/**
 * 行 `r` の下（r=-1 なら表の最上段）に引く罫線。
 *
 * `alignModel` が渡された行では列揃えのコロンを打つ。
 */
function borderLine(
  model: TableModel,
  widths: number[],
  r: number,
  alignModel: TableModel | undefined,
  fill: '-' | '='
): string {
  const colLen = widths.length;
  let out = '+';
  for (let c = 0; c < colLen; c++) {
    const continues = spansAcrossBorder(model, r, c);
    const segWidth = widths[c] + 2;
    if (continues) {
      out += ' '.repeat(segWidth);
    } else if (alignModel) {
      out += alignToGridSeparator(alignModel.columns[c]?.align, segWidth, fill);
    } else {
      out += fill.repeat(segWidth);
    }
    // 次の境界。結合セルの内側を横切る境界は、そのセルが縦にも続くときだけ空白にする。
    const nextIsInsideContinuingCell =
      c + 1 < colLen && continues && spansAcrossBorder(model, r, c + 1) && sameCell(model, r, c, c + 1);
    out += nextIsInsideContinuingCell ? ' ' : '+';
  }
  return out;
}

/** 行 r と r+1 の境界を、列 c のセルが縦にまたいでいるか。 */
function spansAcrossBorder(model: TableModel, r: number, c: number): boolean {
  if (r < 0 || r + 1 >= model.rows.length) return false;
  const upper = coveringCell(model, r, c);
  const lower = coveringCell(model, r + 1, c);
  return !!upper && !!lower && upper === lower;
}

/** 行 r において、列 c1 と列 c2 が同一のセルに属するか。 */
function sameCell(model: TableModel, r: number, c1: number, c2: number): boolean {
  if (r < 0) return false;
  const a = coveringCell(model, r, c1);
  const b = coveringCell(model, r, c2);
  return !!a && a === b;
}

function contentLine(
  model: TableModel,
  widths: number[],
  spanWidth: (c: number, span: number) => number,
  cellLines: string[][][],
  heights: number[],
  r: number,
  lineIndex: number
): string {
  const colLen = widths.length;
  let out = '|';
  let c = 0;
  while (c < colLen) {
    const span = spanAt(model, r, c);
    const width = spanWidth(c, span);
    const text = textLineFor(model, cellLines, heights, r, c, lineIndex);
    out += ' ' + padToWidth(text, width) + ' ';
    c += span;
    out += '|';
  }
  return out;
}

/** 行 r の列 c から始まるセルの列スパン（結合の途中なら残り幅）。 */
function spanAt(model: TableModel, r: number, c: number): number {
  const cell = coveringCell(model, r, c);
  if (!cell) return 1;
  const end = cell.col + cell.colspan;
  return Math.max(1, end - c);
}

/**
 * 行 r・列 c の位置に描く 1 行分のテキスト。
 *
 * 縦結合されたセルの本文は、結合が覆う全行の内容行にまたがって上詰めで流し込む
 * （境界線の行には文字を置けないので、行ブロックを連結したものを 1 本の領域とみなす）。
 */
function textLineFor(
  model: TableModel,
  cellLines: string[][][],
  heights: number[],
  r: number,
  c: number,
  lineIndex: number
): string {
  const cell = coveringCell(model, r, c);
  if (!cell) return '';
  let offset = lineIndex;
  for (let rr = cell.row; rr < r; rr++) offset += heights[rr];
  return cellLines[cell.row][cell.col][offset] ?? '';
}

/** 位置 (r, c) を覆っている表示セル。 */
function coveringCell(model: TableModel, r: number, c: number): TableCell | undefined {
  for (let rr = r; rr >= 0; rr--) {
    for (let cc = c; cc >= 0; cc--) {
      const cell = model.rows[rr]?.[cc];
      if (!cell || cell.hidden) continue;
      if (r < rr + cell.rowspan && c < cc + cell.colspan) return cell;
    }
  }
  return undefined;
}

/** 各列の内容幅（表示幅）。横結合セルは不足ぶんを構成列へ按分する。 */
function computeColumnWidths(model: TableModel, cellLines: string[][][]): number[] {
  const colLen = model.columns.length;
  const widths = Array.from({ length: colLen }, () => 3);

  for (let r = 0; r < model.rows.length; r++) {
    for (let c = 0; c < colLen; c++) {
      const cell = model.rows[r][c];
      if (cell.hidden || cell.colspan > 1) continue;
      const w = maxLineWidth(cellLines[r][c]);
      if (w > widths[c]) widths[c] = w;
    }
  }

  for (let r = 0; r < model.rows.length; r++) {
    for (let c = 0; c < colLen; c++) {
      const cell = model.rows[r][c];
      if (cell.hidden || cell.colspan <= 1) continue;
      const need = maxLineWidth(cellLines[r][c]);
      let have = 3 * (cell.colspan - 1);
      for (let i = c; i < c + cell.colspan; i++) have += widths[i];
      if (need <= have) continue;
      let deficit = need - have;
      // 不足ぶんを構成列へ均等に配る（端数は左の列から）
      for (let i = c; i < c + cell.colspan && deficit > 0; i++) {
        const share = Math.ceil(deficit / (c + cell.colspan - i));
        widths[i] += share;
        deficit -= share;
      }
    }
  }

  return widths;
}

/** 各行の内容行数。縦結合セルが入り切らなければ最終行を伸ばす。 */
function computeRowHeights(model: TableModel, cellLines: string[][][]): number[] {
  const rowLen = model.rows.length;
  const heights = Array.from({ length: rowLen }, () => 1);

  for (let r = 0; r < rowLen; r++) {
    for (const cell of model.rows[r]) {
      if (cell.hidden || cell.rowspan > 1) continue;
      const n = cellLines[r][cell.col].length;
      if (n > heights[r]) heights[r] = n;
    }
  }

  for (let r = 0; r < rowLen; r++) {
    for (const cell of model.rows[r]) {
      if (cell.hidden || cell.rowspan <= 1) continue;
      const n = cellLines[r][cell.col].length;
      let available = 0;
      for (let i = r; i < r + cell.rowspan; i++) available += heights[i];
      if (n > available) heights[r + cell.rowspan - 1] += n - available;
    }
  }

  return heights;
}

function maxLineWidth(lines: string[]): number {
  return lines.reduce((max, l) => Math.max(max, displayWidth(l)), 0);
}
