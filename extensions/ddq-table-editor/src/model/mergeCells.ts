import type { TableModel, ParseResult } from './TableModel';
import { normalizeTableModel } from './normalizeTableModel';

export type CellRange = {
  startRow: number;
  startCol: number;
  endRow: number;
  endCol: number;
};

export function normalizeRange(range: CellRange): CellRange {
  return {
    startRow: Math.min(range.startRow, range.endRow),
    startCol: Math.min(range.startCol, range.endCol),
    endRow: Math.max(range.startRow, range.endRow),
    endCol: Math.max(range.startCol, range.endCol)
  };
}

/**
 * 選択範囲を 1 つのセルへ結合する。
 *
 * - 範囲は矩形であること（UI 側で矩形選択しか作らないが、既存の結合が食い込むと崩れる）
 * - 既存の結合を部分的にまたぐ選択は拒否する（範囲を広げる提案はしない）
 * - 左上のセルが表示セルになり、残りは hidden になる
 * - テキストは左上のセルのものを採用し、他の非空セルがあれば改行で連結する
 */
export function mergeCells(
  model: TableModel,
  range: CellRange
): ParseResult<TableModel> {
  const { startRow, startCol, endRow, endCol } = normalizeRange(range);

  if (
    startRow < 0 ||
    startCol < 0 ||
    endRow >= model.rows.length ||
    endCol >= model.columns.length
  ) {
    return { ok: false, message: '選択範囲が表の外にあります。' };
  }
  if (startRow === endRow && startCol === endCol) {
    return { ok: false, message: '結合するには 2 つ以上のセルを選択してください。' };
  }

  // 既存の結合が範囲の境界をまたいでいないか確認する
  for (let r = 0; r < model.rows.length; r++) {
    for (let c = 0; c < model.columns.length; c++) {
      const cell = model.rows[r][c];
      if (cell.hidden || (cell.rowspan === 1 && cell.colspan === 1)) continue;
      const cellEndRow = r + cell.rowspan - 1;
      const cellEndCol = c + cell.colspan - 1;
      const overlaps =
        r <= endRow && cellEndRow >= startRow && c <= endCol && cellEndCol >= startCol;
      if (!overlaps) continue;
      const contained =
        r >= startRow && cellEndRow <= endRow && c >= startCol && cellEndCol <= endCol;
      if (!contained) {
        return {
          ok: false,
          message: '既存の結合セルが選択範囲の境界をまたいでいます。範囲を見直してください。'
        };
      }
    }
  }

  const texts: string[] = [];
  const rows = model.rows.map(row => row.map(cell => ({ ...cell })));

  for (let r = startRow; r <= endRow; r++) {
    for (let c = startCol; c <= endCol; c++) {
      const cell = rows[r][c];
      if (!cell.hidden && cell.text.trim() !== '') texts.push(cell.text);
      if (r === startRow && c === startCol) continue;
      cell.text = '';
      cell.hidden = true;
      cell.rowspan = 1;
      cell.colspan = 1;
    }
  }

  const anchor = rows[startRow][startCol];
  anchor.hidden = false;
  anchor.rowspan = endRow - startRow + 1;
  anchor.colspan = endCol - startCol + 1;
  // 同じ値のセルを結合するのが典型的な使い方なので、重複するテキストはまとめる
  // （そのまま連結すると「受注管理／受注管理」のように重複してしまう）。
  anchor.text = [...new Set(texts)].join('\n');

  return { ok: true, value: normalizeTableModel({ ...model, rows }) };
}
