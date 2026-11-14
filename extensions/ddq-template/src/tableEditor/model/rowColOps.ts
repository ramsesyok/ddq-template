import type { TableModel, TableCell, TableColumn } from './TableModel';
import { makeCell } from './TableModel';
import { normalizeTableModel } from './normalizeTableModel';

/**
 * 行・列の追加／削除。
 *
 * 結合セルの扱いは「またぐ結合は伸縮させる」方針にする。
 * - 挿入位置が結合の内側なら、その結合の rowspan / colspan を 1 増やす
 * - 削除する行／列を覆う結合は 1 減らす（1 になれば通常セルへ戻る）
 */

export function insertRow(model: TableModel, at: number): TableModel {
  const index = clamp(at, 0, model.rows.length);
  const rows = model.rows.map(r => r.map(cell => ({ ...cell })));

  // 挿入位置をまたぐ結合を伸ばす（境界に挿す場合は伸ばさない）
  for (let r = 0; r < rows.length; r++) {
    for (let c = 0; c < rows[r].length; c++) {
      const cell = rows[r][c];
      if (cell.hidden || cell.rowspan <= 1) continue;
      if (r < index && index < r + cell.rowspan) cell.rowspan += 1;
    }
  }

  const newRow: TableCell[] = model.columns.map((_, c) => makeCell(index, c));
  rows.splice(index, 0, newRow);

  // 伸ばした結合に覆われる位置は normalize が hidden にしてくれる
  const headerRows = index < model.headerRows ? model.headerRows + 1 : model.headerRows;
  return normalizeTableModel({ ...model, rows, headerRows });
}

export function deleteRow(model: TableModel, at: number): TableModel {
  if (model.rows.length <= 1) return model;
  const index = clamp(at, 0, model.rows.length - 1);
  const rows = model.rows.map(r => r.map(cell => ({ ...cell })));

  // 削除行を覆う結合を縮める。結合元そのものが消える場合は、
  // 直下の行へテキストと残りの span を引き継ぐ。
  for (let r = 0; r < rows.length; r++) {
    for (let c = 0; c < rows[r].length; c++) {
      const cell = rows[r][c];
      if (cell.hidden || cell.rowspan <= 1) continue;
      if (r === index) {
        const below = rows[r + 1]?.[c];
        if (below) {
          below.text = cell.text;
          below.hidden = false;
          below.rowspan = cell.rowspan - 1;
          below.colspan = cell.colspan;
        }
      } else if (r < index && index < r + cell.rowspan) {
        cell.rowspan -= 1;
      }
    }
  }

  rows.splice(index, 1);
  const headerRows = index < model.headerRows ? model.headerRows - 1 : model.headerRows;
  return normalizeTableModel({ ...model, rows, headerRows });
}

export function insertColumn(model: TableModel, at: number): TableModel {
  const index = clamp(at, 0, model.columns.length);
  const rows = model.rows.map(r => r.map(cell => ({ ...cell })));

  for (let r = 0; r < rows.length; r++) {
    for (let c = 0; c < rows[r].length; c++) {
      const cell = rows[r][c];
      if (cell.hidden || cell.colspan <= 1) continue;
      if (c < index && index < c + cell.colspan) cell.colspan += 1;
    }
  }

  for (let r = 0; r < rows.length; r++) {
    rows[r].splice(index, 0, makeCell(r, index));
  }

  const columns: TableColumn[] = [...model.columns];
  columns.splice(index, 0, {});

  return normalizeTableModel({ ...model, rows, columns });
}

export function deleteColumn(model: TableModel, at: number): TableModel {
  if (model.columns.length <= 1) return model;
  const index = clamp(at, 0, model.columns.length - 1);
  const rows = model.rows.map(r => r.map(cell => ({ ...cell })));

  for (let r = 0; r < rows.length; r++) {
    for (let c = 0; c < rows[r].length; c++) {
      const cell = rows[r][c];
      if (cell.hidden || cell.colspan <= 1) continue;
      if (c === index) {
        const right = rows[r][c + 1];
        if (right) {
          right.text = cell.text;
          right.hidden = false;
          right.colspan = cell.colspan - 1;
          right.rowspan = cell.rowspan;
        }
      } else if (c < index && index < c + cell.colspan) {
        cell.colspan -= 1;
      }
    }
  }

  for (let r = 0; r < rows.length; r++) rows[r].splice(index, 1);
  const columns = model.columns.filter((_, c) => c !== index);

  return normalizeTableModel({ ...model, rows, columns });
}

function clamp(v: number, lo: number, hi: number): number {
  return Math.min(Math.max(v, lo), hi);
}
