import type { TableModel, ParseResult } from './TableModel';
import { normalizeTableModel } from './normalizeTableModel';

/**
 * 指定位置を覆っている結合セルを解除する。
 *
 * 解除後、結合元のテキストは左上のセルに残り、覆われていたセルは空の表示セルに戻る。
 */
export function unmergeCell(
  model: TableModel,
  row: number,
  col: number
): ParseResult<TableModel> {
  const anchor = findAnchor(model, row, col);
  if (!anchor) {
    return { ok: false, message: '結合されたセルが選択されていません。' };
  }

  const rows = model.rows.map(r => r.map(cell => ({ ...cell })));
  const target = rows[anchor.row][anchor.col];
  const { rowspan, colspan } = target;

  for (let r = anchor.row; r < anchor.row + rowspan; r++) {
    for (let c = anchor.col; c < anchor.col + colspan; c++) {
      const cell = rows[r][c];
      cell.hidden = false;
      cell.rowspan = 1;
      cell.colspan = 1;
    }
  }

  return { ok: true, value: normalizeTableModel({ ...model, rows }) };
}

function findAnchor(
  model: TableModel,
  row: number,
  col: number
): { row: number; col: number } | undefined {
  for (let r = 0; r <= row; r++) {
    for (let c = 0; c <= col; c++) {
      const cell = model.rows[r]?.[c];
      if (!cell || cell.hidden) continue;
      if (cell.rowspan === 1 && cell.colspan === 1) continue;
      if (row < r + cell.rowspan && col < c + cell.colspan) {
        return { row: r, col: c };
      }
    }
  }
  return undefined;
}
