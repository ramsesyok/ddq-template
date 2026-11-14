import { describe, it, expect } from 'vitest';
import { unmergeCell } from './unmergeCell';
import { mergeCells, type CellRange } from './mergeCells';
import { normalizeTableModel } from './normalizeTableModel';
import type { TableModel } from './TableModel';
import { emptyAttributes, hasMerges, makeCell } from './TableModel';

function model(texts: string[][]): TableModel {
  return normalizeTableModel({
    id: 't',
    version: 1,
    headerRows: 1,
    columns: texts[0].map(() => ({})),
    rows: texts.map((row, r) => row.map((text, c) => makeCell(r, c, text))),
    attributes: emptyAttributes(),
    outputFormat: 'gridTable'
  });
}

function merged(m: TableModel, range: CellRange): TableModel {
  const result = mergeCells(m, range);
  if (!result.ok) throw new Error(result.message);
  return result.value;
}

function unmerged(m: TableModel, row: number, col: number): TableModel {
  const result = unmergeCell(m, row, col);
  if (!result.ok) throw new Error(result.message);
  return result.value;
}

const view = (m: TableModel) =>
  m.rows.map(row =>
    row.map(c =>
      c.hidden ? '·' : c.rowspan > 1 || c.colspan > 1 ? `${c.text}[${c.rowspan}x${c.colspan}]` : c.text
    )
  );

/**
 * 3 列 4 行。2〜3 行目の 1〜2 列目を縦横に結合し、4 行目の 2〜3 列目を横に結合した表。
 *
 *   h1 | h2 | h3
 *   X  | ·  | c1
 *   ·  | ·  | c2
 *   a  | Y  | ·
 */
function table(): TableModel {
  const m = merged(
    model([
      ['h1', 'h2', 'h3'],
      ['X', '', 'c1'],
      ['', '', 'c2'],
      ['a', 'Y', '']
    ]),
    { startRow: 1, startCol: 0, endRow: 2, endCol: 1 }
  );
  return merged(m, { startRow: 3, startCol: 1, endRow: 3, endCol: 2 });
}

describe('unmergeCell', () => {
  it('結合元を選べば、本文は左上に残り、覆われていたセルは空の表示セルに戻る', () => {
    expect(view(unmerged(table(), 1, 0))).toEqual([
      ['h1', 'h2', 'h3'],
      ['X', '', 'c1'],
      ['', '', 'c2'],
      ['a', 'Y[1x2]', '·']
    ]);
  });

  it('覆われたセルを選んでも、そのセルを覆う結合が解ける', () => {
    expect(view(unmerged(table(), 2, 1))).toEqual(view(unmerged(table(), 1, 0)));
    expect(view(unmerged(table(), 3, 2))).toEqual([
      ['h1', 'h2', 'h3'],
      ['X[2x2]', '·', 'c1'],
      ['·', '·', 'c2'],
      ['a', 'Y', '']
    ]);
  });

  it('すべての結合を解けば結合の無い表になる', () => {
    const out = unmerged(unmerged(table(), 1, 1), 3, 1);
    expect(hasMerges(out)).toBe(false);
    expect(out.rows.flat().every(c => !c.hidden && c.rowspan === 1 && c.colspan === 1)).toBe(true);
  });

  it('結合していないセルや表の外を選べば、理由を添えて断る', () => {
    for (const [row, col] of [
      [0, 0],
      [1, 2],
      [3, 0],
      [9, 9]
    ]) {
      const result = unmergeCell(table(), row, col);
      expect(result.ok).toBe(false);
      if (!result.ok) expect(result.message).toMatch(/結合されたセル/);
    }
  });

  it('結合して解くと、本文と形は元の表に戻る（本文は結合元に集まったまま）', () => {
    const m = model([
      ['h1', 'h2'],
      ['同じ', '同じ'],
      ['b1', 'b2']
    ]);
    const out = unmerged(merged(m, { startRow: 1, startCol: 0, endRow: 1, endCol: 1 }), 1, 1);
    expect(view(out)).toEqual([
      ['h1', 'h2'],
      ['同じ', ''],
      ['b1', 'b2']
    ]);
  });

  it('元のモデルを書き換えない', () => {
    const m = table();
    const before = JSON.stringify(m);
    unmergeCell(m, 1, 0);
    expect(JSON.stringify(m)).toBe(before);
  });
});
