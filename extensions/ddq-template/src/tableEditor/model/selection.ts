import type { TableModel } from './TableModel';
import { normalizeRange, type CellRange } from './mergeCells';

/**
 * 選択とキーボード移動の計算。
 *
 * `CellRange` は start がアンカー（最初に選んだセル）、end がフォーカス（最後に伸ばした
 * 位置）で、矩形にするときは `normalizeRange` を通す。結合セルは 1 つの単位として扱い、
 * 移動では飛び越え、範囲は結合セル全体を含むまで広げる。
 */

export type CellPos = { row: number; col: number };

export type Direction = 'up' | 'down' | 'left' | 'right';

export function singleCell(pos: CellPos): CellRange {
  return { startRow: pos.row, startCol: pos.col, endRow: pos.row, endCol: pos.col };
}

export function selectAll(model: TableModel): CellRange {
  return {
    startRow: 0,
    startCol: 0,
    endRow: Math.max(0, model.rows.length - 1),
    endCol: Math.max(0, model.columns.length - 1)
  };
}

/** 座標を覆っている表示セル（結合ならその左上）の位置。表の外なら端に寄せる。 */
export function anchorOf(model: TableModel, row: number, col: number): CellPos {
  const r0 = clamp(row, 0, model.rows.length - 1);
  const c0 = clamp(col, 0, model.columns.length - 1);
  for (let r = r0; r >= 0; r--) {
    for (let c = c0; c >= 0; c--) {
      const cell = model.rows[r]?.[c];
      if (!cell || cell.hidden) continue;
      if (r0 < r + cell.rowspan && c0 < c + cell.colspan) return { row: r, col: c };
    }
  }
  return { row: r0, col: c0 };
}

/** 範囲を表の中に収める（アンカーとフォーカスの向きは保つ）。 */
export function clampRange(model: TableModel, range: CellRange): CellRange {
  const maxRow = Math.max(0, model.rows.length - 1);
  const maxCol = Math.max(0, model.columns.length - 1);
  return {
    startRow: clamp(range.startRow, 0, maxRow),
    startCol: clamp(range.startCol, 0, maxCol),
    endRow: clamp(range.endRow, 0, maxRow),
    endCol: clamp(range.endCol, 0, maxCol)
  };
}

/**
 * 結合セルを部分的に含む範囲を、その結合セル全体を含むまで広げる。
 *
 * 広げた先で別の結合セルに掛かることがあるので、変化しなくなるまで繰り返す。
 * 戻り値は矩形に正規化済み。
 */
export function expandRangeToMerges(model: TableModel, range: CellRange): CellRange {
  let cur = normalizeRange(clampRange(model, range));
  for (;;) {
    let next = cur;
    for (let r = 0; r < model.rows.length; r++) {
      for (let c = 0; c < model.columns.length; c++) {
        const cell = model.rows[r][c];
        if (cell.hidden || (cell.rowspan === 1 && cell.colspan === 1)) continue;
        const endRow = r + cell.rowspan - 1;
        const endCol = c + cell.colspan - 1;
        const overlaps =
          r <= next.endRow && endRow >= next.startRow && c <= next.endCol && endCol >= next.startCol;
        if (!overlaps) continue;
        next = {
          startRow: Math.min(next.startRow, r),
          startCol: Math.min(next.startCol, c),
          endRow: Math.max(next.endRow, endRow),
          endCol: Math.max(next.endCol, endCol)
        };
      }
    }
    if (sameRange(next, cur)) return cur;
    cur = next;
  }
}

/** セルの矩形が範囲（正規化済み）と重なるか。結合セルは全体で判定する。 */
export function cellOverlapsRange(model: TableModel, row: number, col: number, range: CellRange): boolean {
  const cell = model.rows[row]?.[col];
  if (!cell || cell.hidden) return false;
  return (
    row <= range.endRow &&
    row + cell.rowspan - 1 >= range.startRow &&
    col <= range.endCol &&
    col + cell.colspan - 1 >= range.startCol
  );
}

/**
 * アクティブセルを 1 つ動かす。結合セルは飛び越え、端では動かない。
 * 戻り値は表示セルの位置（結合なら左上）。
 */
export function moveActive(model: TableModel, pos: CellPos, dir: Direction): CellPos {
  const from = anchorOf(model, pos.row, pos.col);
  const cell = model.rows[from.row][from.col];
  let row = from.row;
  let col = from.col;
  switch (dir) {
    case 'up':
      row = from.row - 1;
      break;
    case 'down':
      row = from.row + cell.rowspan;
      break;
    case 'left':
      col = from.col - 1;
      break;
    case 'right':
      col = from.col + cell.colspan;
      break;
  }
  if (row < 0 || row >= model.rows.length || col < 0 || col >= model.columns.length) return from;
  return anchorOf(model, row, col);
}

/** Tab 移動。行末では次の行の先頭へ、Shift+Tab は行頭で前の行の末尾へ折り返す。 */
export function moveTab(model: TableModel, pos: CellPos, backward: boolean): CellPos {
  const from = anchorOf(model, pos.row, pos.col);
  const next = moveActive(model, from, backward ? 'left' : 'right');
  if (next.row !== from.row || next.col !== from.col) return next;

  if (backward) {
    if (from.row === 0) return from;
    return anchorOf(model, from.row - 1, model.columns.length - 1);
  }
  if (from.row + 1 >= model.rows.length) return from;
  return anchorOf(model, from.row + 1, 0);
}

/** 行頭／行末、Ctrl 付きなら表の左上／右下へ。 */
export function moveToEdge(model: TableModel, pos: CellPos, end: boolean, whole: boolean): CellPos {
  const row = whole ? (end ? model.rows.length - 1 : 0) : pos.row;
  const col = end ? model.columns.length - 1 : 0;
  return anchorOf(model, row, col);
}

/**
 * Shift+矢印。アンカーは動かさず、フォーカスだけを 1 つ動かす。
 *
 * 広げる向きに結合セルがあるときは、その結合セルの端までまとめて飛ぶ（1 つずつ進めると
 * 結合セルの中を通る間は見た目が変わらず、何度も押すことになる）。縮める向きでは飛ばない。
 */
export function extendSelection(model: TableModel, range: CellRange, dir: Direction): CellRange {
  let { endRow, endCol } = range;
  switch (dir) {
    case 'up':
      endRow -= 1;
      break;
    case 'down':
      endRow += 1;
      break;
    case 'left':
      endCol -= 1;
      break;
    case 'right':
      endCol += 1;
      break;
  }
  const moved = clampRange(model, { ...range, endRow, endCol });
  const expanded = expandRangeToMerges(model, moved);
  if (dir === 'down' && moved.endRow >= moved.startRow) moved.endRow = expanded.endRow;
  if (dir === 'up' && moved.endRow <= moved.startRow) moved.endRow = expanded.startRow;
  if (dir === 'right' && moved.endCol >= moved.startCol) moved.endCol = expanded.endCol;
  if (dir === 'left' && moved.endCol <= moved.startCol) moved.endCol = expanded.startCol;
  return moved;
}

export function sameRange(a: CellRange, b: CellRange): boolean {
  return (
    a.startRow === b.startRow &&
    a.startCol === b.startCol &&
    a.endRow === b.endRow &&
    a.endCol === b.endCol
  );
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), Math.max(min, max));
}
