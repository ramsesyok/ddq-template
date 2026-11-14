import { describe, it, expect } from 'vitest';
import { insertRow, deleteRow, insertColumn, deleteColumn } from './rowColOps';
import { mergeCells, type CellRange } from './mergeCells';
import { normalizeTableModel } from './normalizeTableModel';
import type { TableModel } from './TableModel';
import { emptyAttributes, makeCell } from './TableModel';

function model(texts: string[][], headerRows = 1): TableModel {
  return normalizeTableModel({
    id: 't',
    version: 1,
    headerRows,
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

/** 表示セルは本文（結合なら `本文[行数x列数]`）、覆われたセルは `·` で並べる。 */
const view = (m: TableModel) =>
  m.rows.map(row =>
    row.map(c =>
      c.hidden ? '·' : c.rowspan > 1 || c.colspan > 1 ? `${c.text}[${c.rowspan}x${c.colspan}]` : c.text
    )
  );

const snapshot = (m: TableModel) => JSON.stringify(m);

/**
 * 3 列 4 行。1 列目の 2〜4 行目（本文 3 行）を縦に結合した表。
 *
 *   h1 | h2 | h3
 *   A  | b1 | c1
 *   ·  | b2 | c2
 *   ·  | b3 | c3
 */
function withRowspan(): TableModel {
  return merged(
    model([
      ['h1', 'h2', 'h3'],
      ['A', 'b1', 'c1'],
      ['', 'b2', 'c2'],
      ['', 'b3', 'c3']
    ]),
    { startRow: 1, startCol: 0, endRow: 3, endCol: 0 }
  );
}

/**
 * 4 列 3 行。見出しの 2〜4 列目を横に結合した表。
 *
 *   k  | 実績 | ·  | ·
 *   a1 | a2   | a3 | a4
 *   b1 | b2   | b3 | b4
 */
function withColspan(): TableModel {
  return merged(
    model([
      ['k', '実績', '', ''],
      ['a1', 'a2', 'a3', 'a4'],
      ['b1', 'b2', 'b3', 'b4']
    ]),
    { startRow: 0, startCol: 1, endRow: 0, endCol: 3 }
  );
}

describe('insertRow', () => {
  it('指定位置に空の行を挿し、下の行を 1 つずつ下げる', () => {
    const out = insertRow(model([['h'], ['a'], ['b']]), 2);
    expect(view(out)).toEqual([['h'], ['a'], [''], ['b']]);
    expect(out.rows.map(r => r[0].id)).toEqual(['r0c0', 'r1c0', 'r2c0', 'r3c0']);
  });

  it('範囲外の位置は先頭・末尾に丸める', () => {
    expect(view(insertRow(model([['h'], ['a']]), -5))).toEqual([[''], ['h'], ['a']]);
    expect(view(insertRow(model([['h'], ['a']]), 99))).toEqual([['h'], ['a'], ['']]);
  });

  it('ヘッダの中に挿せばヘッダ行数が増え、ヘッダの直後なら本文の行になる', () => {
    const m = model([['h'], ['a']], 1);
    expect(insertRow(m, 0).headerRows).toBe(2);
    expect(insertRow(m, 1).headerRows).toBe(1);
  });

  it('縦結合の内側に挿すと結合が 1 行伸び、新しい行のその列は覆われる', () => {
    const out = insertRow(withRowspan(), 2);
    expect(view(out)).toEqual([
      ['h1', 'h2', 'h3'],
      ['A[4x1]', 'b1', 'c1'],
      ['·', '', ''],
      ['·', 'b2', 'c2'],
      ['·', 'b3', 'c3']
    ]);
  });

  it('縦結合の境界（結合元の上・結合の直後）に挿しても結合は伸びない', () => {
    expect(view(insertRow(withRowspan(), 1))).toEqual([
      ['h1', 'h2', 'h3'],
      ['', '', ''],
      ['A[3x1]', 'b1', 'c1'],
      ['·', 'b2', 'c2'],
      ['·', 'b3', 'c3']
    ]);
    expect(view(insertRow(withRowspan(), 4))).toEqual([
      ['h1', 'h2', 'h3'],
      ['A[3x1]', 'b1', 'c1'],
      ['·', 'b2', 'c2'],
      ['·', 'b3', 'c3'],
      ['', '', '']
    ]);
  });

  it('縦横に結合したセルの内側に挿すと、新しい行の結合の幅ぶんが覆われる', () => {
    const m = merged(
      model([
        ['h1', 'h2', 'h3'],
        ['X', '', 'c1'],
        ['', '', 'c2']
      ]),
      { startRow: 1, startCol: 0, endRow: 2, endCol: 1 }
    );
    expect(view(insertRow(m, 2))).toEqual([
      ['h1', 'h2', 'h3'],
      ['X[3x2]', '·', 'c1'],
      ['·', '·', ''],
      ['·', '·', 'c2']
    ]);
  });

  it('元のモデルを書き換えない', () => {
    const m = withRowspan();
    const before = snapshot(m);
    insertRow(m, 2);
    expect(snapshot(m)).toBe(before);
  });
});

describe('deleteRow', () => {
  it('指定した行を消し、下の行を詰める', () => {
    const out = deleteRow(model([['h'], ['a'], ['b']]), 1);
    expect(view(out)).toEqual([['h'], ['b']]);
    expect(out.rows[1][0].id).toBe('r1c0');
  });

  it('最後の 1 行は消さない（同じモデルを返す）', () => {
    const m = model([['h']]);
    expect(deleteRow(m, 0)).toBe(m);
  });

  it('範囲外の位置は先頭・末尾に丸める', () => {
    expect(view(deleteRow(model([['h'], ['a'], ['b']]), 99))).toEqual([['h'], ['a']]);
    expect(view(deleteRow(model([['h'], ['a'], ['b']]), -1))).toEqual([['a'], ['b']]);
  });

  it('ヘッダの行を消せばヘッダ行数が減り、本文の行なら変わらない', () => {
    const m = model([['h1'], ['h2'], ['a']], 2);
    expect(deleteRow(m, 0).headerRows).toBe(1);
    expect(deleteRow(m, 2).headerRows).toBe(2);
  });

  it('縦結合の内側の行を消すと結合が 1 行縮む', () => {
    expect(view(deleteRow(withRowspan(), 2))).toEqual([
      ['h1', 'h2', 'h3'],
      ['A[2x1]', 'b1', 'c1'],
      ['·', 'b3', 'c3']
    ]);
  });

  it('結合元の行を消すと、直下の行が本文と残りの結合を引き継ぐ', () => {
    expect(view(deleteRow(withRowspan(), 1))).toEqual([
      ['h1', 'h2', 'h3'],
      ['A[2x1]', 'b2', 'c2'],
      ['·', 'b3', 'c3']
    ]);
  });

  it('2 行の結合の結合元を消すと、引き継いだセルは通常のセルに戻る（下の行を覆わない）', () => {
    const m = merged(
      model([
        ['h1', 'h2'],
        ['A', 'b1'],
        ['', 'b2'],
        ['z1', 'z2']
      ]),
      { startRow: 1, startCol: 0, endRow: 2, endCol: 0 }
    );
    expect(view(deleteRow(m, 1))).toEqual([
      ['h1', 'h2'],
      ['A', 'b2'],
      ['z1', 'z2']
    ]);
  });

  it('縦横に結合したセルの結合元の行を消すと、横の幅も引き継ぐ', () => {
    const m = merged(
      model([
        ['h1', 'h2', 'h3'],
        ['X', '', 'c1'],
        ['', '', 'c2'],
        ['', '', 'c3']
      ]),
      { startRow: 1, startCol: 0, endRow: 3, endCol: 1 }
    );
    expect(view(deleteRow(m, 1))).toEqual([
      ['h1', 'h2', 'h3'],
      ['X[2x2]', '·', 'c2'],
      ['·', '·', 'c3']
    ]);
  });

  it('結合の内側に挿した行を消すと元の表に戻る', () => {
    const m = withRowspan();
    expect(view(deleteRow(insertRow(m, 2), 2))).toEqual(view(m));
  });

  it('元のモデルを書き換えない', () => {
    const m = withRowspan();
    const before = snapshot(m);
    deleteRow(m, 1);
    deleteRow(m, 2);
    expect(snapshot(m)).toBe(before);
  });
});

describe('insertColumn', () => {
  it('指定位置に空の列を挿し、列の設定も同じ位置に足す', () => {
    const m = model([
      ['h1', 'h2'],
      ['a', 'b']
    ]);
    m.columns[0] = { align: 'right', width: 30 };
    m.columns[1] = { align: 'center', width: 70 };
    const out = insertColumn(m, 1);
    expect(view(out)).toEqual([
      ['h1', '', 'h2'],
      ['a', '', 'b']
    ]);
    expect(out.columns).toEqual([{ align: 'right', width: 30 }, {}, { align: 'center', width: 70 }]);
  });

  it('範囲外の位置は先頭・末尾に丸める', () => {
    expect(view(insertColumn(model([['a', 'b']]), -1))).toEqual([['', 'a', 'b']]);
    expect(view(insertColumn(model([['a', 'b']]), 9))).toEqual([['a', 'b', '']]);
  });

  it('横結合の内側に挿すと結合が 1 列伸び、新しい列のその行は覆われる', () => {
    expect(view(insertColumn(withColspan(), 2))).toEqual([
      ['k', '実績[1x4]', '·', '·', '·'],
      ['a1', 'a2', '', 'a3', 'a4'],
      ['b1', 'b2', '', 'b3', 'b4']
    ]);
  });

  it('横結合の境界（結合元の左・結合の直後）に挿しても結合は伸びない', () => {
    expect(view(insertColumn(withColspan(), 1))[0]).toEqual(['k', '', '実績[1x3]', '·', '·']);
    expect(view(insertColumn(withColspan(), 4))[0]).toEqual(['k', '実績[1x3]', '·', '·', '']);
  });

  it('元のモデルを書き換えない', () => {
    const m = withColspan();
    const before = snapshot(m);
    insertColumn(m, 2);
    expect(snapshot(m)).toBe(before);
  });
});

describe('deleteColumn', () => {
  it('指定した列を消し、列の設定も一緒に消す', () => {
    const m = model([
      ['h1', 'h2', 'h3'],
      ['a', 'b', 'c']
    ]);
    m.columns[1] = { align: 'center', width: 50 };
    const out = deleteColumn(m, 1);
    expect(view(out)).toEqual([
      ['h1', 'h3'],
      ['a', 'c']
    ]);
    expect(out.columns).toEqual([{}, {}]);
    expect(out.rows[0][1].id).toBe('r0c1');
  });

  it('最後の 1 列は消さない（同じモデルを返す）', () => {
    const m = model([['a'], ['b']]);
    expect(deleteColumn(m, 0)).toBe(m);
  });

  it('横結合の内側の列を消すと結合が 1 列縮む', () => {
    expect(view(deleteColumn(withColspan(), 2))).toEqual([
      ['k', '実績[1x2]', '·'],
      ['a1', 'a2', 'a4'],
      ['b1', 'b2', 'b4']
    ]);
  });

  it('結合元の列を消すと、右の列が本文と残りの結合を引き継ぐ', () => {
    expect(view(deleteColumn(withColspan(), 1))).toEqual([
      ['k', '実績[1x2]', '·'],
      ['a1', 'a3', 'a4'],
      ['b1', 'b3', 'b4']
    ]);
  });

  it('2 列の結合の結合元を消すと、引き継いだセルは通常のセルに戻る（右の列を覆わない）', () => {
    const m = merged(
      model([
        ['A', '', 'z'],
        ['a1', 'a2', 'a3']
      ]),
      { startRow: 0, startCol: 0, endRow: 0, endCol: 1 }
    );
    expect(view(deleteColumn(m, 0))).toEqual([
      ['A', 'z'],
      ['a2', 'a3']
    ]);
  });

  it('縦横に結合したセルの結合元の列を消すと、縦の幅も引き継ぐ', () => {
    const m = merged(
      model([
        ['h1', 'h2', 'h3'],
        ['X', '', ''],
        ['', '', ''],
        ['a', 'b', 'c']
      ]),
      { startRow: 1, startCol: 0, endRow: 2, endCol: 2 }
    );
    expect(view(deleteColumn(m, 0))).toEqual([
      ['h2', 'h3'],
      ['X[2x2]', '·'],
      ['·', '·'],
      ['b', 'c']
    ]);
  });

  it('結合の内側に挿した列を消すと元の表に戻る', () => {
    const m = withColspan();
    expect(view(deleteColumn(insertColumn(m, 2), 2))).toEqual(view(m));
  });

  it('元のモデルを書き換えない', () => {
    const m = withColspan();
    const before = snapshot(m);
    deleteColumn(m, 1);
    deleteColumn(m, 2);
    expect(snapshot(m)).toBe(before);
  });
});
