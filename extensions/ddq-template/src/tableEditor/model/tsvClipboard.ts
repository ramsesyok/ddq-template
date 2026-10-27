import type { TableModel, TableCell } from './TableModel';
import { makeCell } from './TableModel';
import { normalizeTableModel } from './normalizeTableModel';
import { normalizeRange, type CellRange } from './mergeCells';

/**
 * 選択範囲と TSV（Excel のクリップボード形式）の相互変換。
 *
 * 表内でのセルの使い回し（○ / × など）と、Excel との往復の両方をこの 1 形式で賄う。
 * 読み取りは `parseTsv.ts` の `splitTsv` と対称にしてある。
 */

/**
 * 選択範囲を TSV にする。
 *
 * 結合に吸収されたセルは空文字にする（値はアンカーだけが持つ）。Excel へ戻したときに
 * 同じ値が並ぶより空のほうが直しやすく、表内で貼り直すときも同じ理由で扱いやすい。
 */
export function buildTsv(model: TableModel, range: CellRange): string {
  const { startRow, startCol, endRow, endCol } = clampRange(model, range);

  const lines: string[] = [];
  for (let r = startRow; r <= endRow; r++) {
    const cells: string[] = [];
    for (let c = startCol; c <= endCol; c++) {
      const cell = model.rows[r]?.[c];
      cells.push(cell && !cell.hidden ? escapeTsvCell(cell.text) : '');
    }
    lines.push(cells.join('\t'));
  }
  return lines.join('\n');
}

/**
 * 改行・タブ・`"` を含むセルを Excel の流儀で囲む。
 *
 * `splitTsv`（parseTsv.ts）がこの形を読むので、書き出しも同じ規則にそろえる。
 */
function escapeTsvCell(text: string): string {
  if (!/[\t\n"]/.test(text)) return text;
  return `"${text.replace(/"/g, '""')}"`;
}

export type PasteResult = {
  model: TableModel;
  /** 実際に書き込んだ範囲（貼り付け後の選択に使う）。 */
  range: CellRange;
  /** 結合に吸収されたセルに当たって捨てた値の数。 */
  skipped: number;
};

/**
 * 選択範囲の左上を起点に貼り付ける。
 *
 * - 値が 1 つだけなら選択範囲すべてを埋める（○ / × の使い回し）
 * - 表からはみ出す分は行・列を足して受け止める
 * - 結合に吸収されたセルには書かない（結合を崩さないため）。捨てた数を返す
 */
export function pasteCellsAt(
  model: TableModel,
  cells: string[][],
  selection: CellRange
): PasteResult {
  const range = clampRange(model, selection);
  const values = cells.map(row => row.map(text => text.trim()));
  const single = values.length === 1 && values[0].length === 1;

  const height = single ? range.endRow - range.startRow + 1 : values.length;
  const width = single
    ? range.endCol - range.startCol + 1
    : Math.max(...values.map(row => row.length));

  const grown = grow(model, range.startRow + height, range.startCol + width);
  const rows = grown.rows.map(row => row.map(cell => ({ ...cell })));

  let skipped = 0;
  for (let dr = 0; dr < height; dr++) {
    for (let dc = 0; dc < width; dc++) {
      const text = single ? values[0][0] : values[dr]?.[dc];
      if (text === undefined) continue;
      const cell = rows[range.startRow + dr]?.[range.startCol + dc];
      if (!cell) continue;
      if (cell.hidden) {
        skipped++;
        continue;
      }
      cell.text = text;
    }
  }

  return {
    model: normalizeTableModel({ ...grown, rows }),
    range: {
      startRow: range.startRow,
      startCol: range.startCol,
      endRow: range.startRow + height - 1,
      endCol: range.startCol + width - 1
    },
    skipped
  };
}

/** 行数・列数を必要なだけ増やす（結合には触れない）。 */
function grow(model: TableModel, rowLen: number, colLen: number): TableModel {
  const rows = Math.max(model.rows.length, rowLen);
  const cols = Math.max(model.columns.length, colLen);
  if (rows === model.rows.length && cols === model.columns.length) return model;

  const columns = Array.from({ length: cols }, (_, c) => ({ ...(model.columns[c] ?? {}) }));
  const grownRows: TableCell[][] = Array.from({ length: rows }, (_, r) =>
    Array.from({ length: cols }, (_, c) => {
      const cell = model.rows[r]?.[c];
      return cell ? { ...cell } : makeCell(r, c);
    })
  );

  return { ...model, columns, rows: grownRows };
}

/** 範囲を表の中に収める。 */
function clampRange(model: TableModel, range: CellRange): CellRange {
  const maxRow = Math.max(0, model.rows.length - 1);
  const maxCol = Math.max(0, model.columns.length - 1);
  const r = normalizeRange(range);
  return {
    startRow: clamp(r.startRow, 0, maxRow),
    startCol: clamp(r.startCol, 0, maxCol),
    endRow: clamp(r.endRow, 0, maxRow),
    endCol: clamp(r.endCol, 0, maxCol)
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
