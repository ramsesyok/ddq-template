import type { TableModel } from './TableModel';
import type { CellRange } from './mergeCells';
import { expandRangeToMerges } from './selection';

/**
 * 範囲内のセル本文を空にする（Delete / Backspace）。
 *
 * 結合は崩さない。範囲に掛かる結合セルは全体をクリアの対象にする。
 * 変わるセルが無ければ元のモデルをそのまま返す（履歴に空の操作を積まないため）。
 */
export function clearCells(model: TableModel, range: CellRange): TableModel {
  const { startRow, startCol, endRow, endCol } = expandRangeToMerges(model, range);
  let changed = false;
  const rows = model.rows.map((row, r) =>
    row.map((cell, c) => {
      const inside = startRow <= r && r <= endRow && startCol <= c && c <= endCol;
      if (!inside || cell.hidden || cell.text === '') return cell;
      changed = true;
      return { ...cell, text: '' };
    })
  );
  return changed ? { ...model, rows } : model;
}
