import type { TableModel } from '../../model/TableModel';
import { normalizeTableModel } from '../../model/normalizeTableModel';
import { simulateMergeCols, mergedMapToRowspans } from './simulateMergeCols';

/**
 * `merge-cols` 付きのパイプ表を開いたときに、実際にレンダリングされる結合状態へ展開する。
 *
 * ソース上は同じ値が縦に並んでいるだけなので、そのまま表示すると「結合されていない表」に
 * 見えてしまう。テンプレートと同じアルゴリズムで結合を再現し、WYSIWYG に近づける。
 */
export function expandMergeCols(model: TableModel, cols: number[] | null): TableModel {
  if (model.rows.length <= 1) return model;

  const bodyRows = model.rows.slice(1);
  const texts = bodyRows.map(row => row.map(cell => cell.text));
  const spans = mergedMapToRowspans(simulateMergeCols(texts, cols));

  const rows = model.rows.map(row => row.map(cell => ({ ...cell })));

  for (let r = 0; r < bodyRows.length; r++) {
    for (let c = 0; c < model.columns.length; c++) {
      const cell = rows[r + 1][c];
      if (spans[r][c] === 0) {
        cell.hidden = true;
        cell.rowspan = 1;
        cell.text = '';
      } else {
        cell.hidden = false;
        cell.rowspan = spans[r][c];
      }
    }
  }

  return normalizeTableModel({ ...model, rows, outputFormat: 'mergeCols' });
}
