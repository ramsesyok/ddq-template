import type { TableModel } from './TableModel';
import { emptyAttributes, makeCell } from './TableModel';
import { normalizeTableModel } from './normalizeTableModel';

/**
 * Excel からコピーした TSV（`text/plain`）を TableModel へ。
 *
 * Excel 側の結合は復元しない（初版の非対応事項）。結合は VSCode 側で行う。
 * ダブルクォートで囲まれたセルは Excel の仕様に合わせて中の改行・タブを保つ。
 */
export function parseTsv(tsv: string, tableId = ''): TableModel {
  const cells = splitTsv(tsv.replace(/\r\n?/g, '\n').replace(/\n$/, ''));
  const colLen = Math.max(1, ...cells.map(row => row.length));

  const rows = cells.map((row, r) =>
    Array.from({ length: colLen }, (_, c) => {
      const cell = makeCell(r, c);
      cell.text = (row[c] ?? '').trim();
      return cell;
    })
  );

  return normalizeTableModel({
    id: tableId,
    version: 1,
    headerRows: rows.length > 1 ? 1 : 0,
    columns: Array.from({ length: colLen }, () => ({})),
    rows: rows.length > 0 ? rows : [[makeCell(0, 0)]],
    attributes: emptyAttributes(),
    outputFormat: 'gridTable'
  });
}

/** TSV を行×セルへ。`"…"` で囲まれたセル内の改行・タブはセルの一部として扱う。 */
export function splitTsv(text: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let cell = '';
  let quoted = false;

  for (let i = 0; i < text.length; i++) {
    const ch = text[i];

    if (quoted) {
      if (ch === '"') {
        if (text[i + 1] === '"') {
          cell += '"';
          i++;
        } else {
          quoted = false;
        }
      } else {
        cell += ch;
      }
      continue;
    }

    if (ch === '"' && cell === '') {
      quoted = true;
    } else if (ch === '\t') {
      row.push(cell);
      cell = '';
    } else if (ch === '\n') {
      row.push(cell);
      rows.push(row);
      row = [];
      cell = '';
    } else {
      cell += ch;
    }
  }

  row.push(cell);
  rows.push(row);
  return rows;
}
