import { describe, it, expect } from 'vitest';
import { clearCells } from './clearCells';
import { mergeCells } from './mergeCells';
import { normalizeTableModel } from './normalizeTableModel';
import type { TableModel } from './TableModel';
import { emptyAttributes, makeCell } from './TableModel';

function model(texts: string[][]): TableModel {
  return normalizeTableModel({
    id: 't',
    version: 1,
    headerRows: 1,
    columns: texts[0].map(() => ({})),
    rows: texts.map((row, r) => row.map((text, c) => makeCell(r, c, text))),
    attributes: emptyAttributes(),
    outputFormat: 'pipeTable'
  });
}

const texts = (m: TableModel) => m.rows.map(row => row.map(c => (c.hidden ? null : c.text)));

describe('clearCells', () => {
  it('範囲の本文を空にし、外は触らない', () => {
    const m = model([
      ['a', 'b', 'c'],
      ['d', 'e', 'f']
    ]);
    const out = clearCells(m, { startRow: 1, startCol: 1, endRow: 0, endCol: 2 });
    expect(texts(out)).toEqual([
      ['a', '', ''],
      ['d', '', '']
    ]);
  });

  it('結合セルに掛かればその全体を対象にし、結合は崩さない', () => {
    const base = model([
      ['a', 'b'],
      ['a', 'c'],
      ['d', 'e']
    ]);
    const merged = mergeCells(base, { startRow: 0, startCol: 0, endRow: 1, endCol: 0 });
    if (!merged.ok) throw new Error(merged.message);
    const out = clearCells(merged.value, { startRow: 1, startCol: 0, endRow: 1, endCol: 0 });
    expect(texts(out)).toEqual([
      ['', 'b'],
      [null, 'c'],
      ['d', 'e']
    ]);
    expect(out.rows[0][0].rowspan).toBe(2);
  });

  it('変わるセルが無ければ同じモデルを返す', () => {
    const m = model([['', '']]);
    expect(clearCells(m, { startRow: 0, startCol: 0, endRow: 0, endCol: 1 })).toBe(m);
  });
});
