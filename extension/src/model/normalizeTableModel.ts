import type { TableModel, TableCell, TableColumn } from './TableModel';
import { makeCell } from './TableModel';

/**
 * モデルの矩形性・結合の整合性を回復する。
 *
 * - 行の長さを列数へ揃える（不足はセル追加、余りは切り捨て）
 * - row / col / id を座標から振り直す
 * - 結合範囲がはみ出していれば表内に収める
 * - hidden フラグを結合範囲から再計算する（結合元が無い hidden セルは表示に戻す）
 * - headerRows を 0〜行数に収める
 */
export function normalizeTableModel(model: TableModel): TableModel {
  const colLen = Math.max(
    model.columns.length,
    ...model.rows.map(r => r.length),
    1
  );

  const columns: TableColumn[] = Array.from({ length: colLen }, (_, c) => ({
    ...(model.columns[c] ?? {})
  }));

  const rows: TableCell[][] = model.rows.map((row, r) =>
    Array.from({ length: colLen }, (_, c) => {
      const src = row[c];
      const cell = src ? { ...src } : makeCell(r, c);
      cell.row = r;
      cell.col = c;
      cell.id = `r${r}c${c}`;
      cell.rowspan = Math.max(1, Math.floor(cell.rowspan || 1));
      cell.colspan = Math.max(1, Math.floor(cell.colspan || 1));
      cell.hidden = false;
      return cell;
    })
  );

  const rowLen = rows.length;

  // 結合範囲を表内に収め、覆われるセルを hidden にする。
  // 上→下・左→右の順に見るので、先に置かれた結合が優先される。
  const covered: boolean[][] = Array.from({ length: rowLen }, () =>
    Array.from({ length: colLen }, () => false)
  );

  for (let r = 0; r < rowLen; r++) {
    for (let c = 0; c < colLen; c++) {
      const cell = rows[r][c];
      if (covered[r][c]) {
        cell.hidden = true;
        cell.rowspan = 1;
        cell.colspan = 1;
        continue;
      }
      cell.rowspan = Math.min(cell.rowspan, rowLen - r);
      cell.colspan = Math.min(cell.colspan, colLen - c);

      // 既に覆われているセルへ食い込まないよう縮める
      for (let rr = r; rr < r + cell.rowspan; rr++) {
        for (let cc = c; cc < c + cell.colspan; cc++) {
          if (rr === r && cc === c) continue;
          if (covered[rr][cc]) {
            cell.rowspan = Math.min(cell.rowspan, rr - r || 1);
            cell.colspan = Math.min(cell.colspan, cc - c || 1);
          }
        }
      }

      for (let rr = r; rr < r + cell.rowspan; rr++) {
        for (let cc = c; cc < c + cell.colspan; cc++) {
          if (rr === r && cc === c) continue;
          covered[rr][cc] = true;
        }
      }
    }
  }

  const headerRows = Math.min(Math.max(0, Math.floor(model.headerRows || 0)), rowLen);

  return { ...model, headerRows, columns, rows };
}
