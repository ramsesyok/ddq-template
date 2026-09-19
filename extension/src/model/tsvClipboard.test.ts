import { describe, it, expect } from 'vitest';
import { buildTsv, pasteCellsAt } from './tsvClipboard';
import { splitTsv } from './parseTsv';
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

const BASE = () =>
  model([
    ['区分', '氏名', '備考'],
    ['A', '田中', '○'],
    ['B', '鈴木', '×']
  ]);

describe('buildTsv', () => {
  it('選択範囲をタブと改行で並べる', () => {
    const tsv = buildTsv(BASE(), { startRow: 1, startCol: 0, endRow: 2, endCol: 1 });
    expect(tsv).toBe('A\t田中\nB\t鈴木');
  });

  it('1 セルならその値だけ', () => {
    expect(buildTsv(BASE(), { startRow: 1, startCol: 2, endRow: 1, endCol: 2 })).toBe('○');
  });

  it('改行・タブ・引用符を含むセルは Excel の流儀で囲む', () => {
    const m = model([['a', 'b'], ['改行\nあり', '"引用" と\tタブ']]);
    const tsv = buildTsv(m, { startRow: 1, startCol: 0, endRow: 1, endCol: 1 });
    expect(tsv).toBe('"改行\nあり"\t"""引用"" と\tタブ"');
    // 読み取り側（parseTsv の splitTsv）と対称であること
    expect(splitTsv(tsv)).toEqual([['改行\nあり', '"引用" と\tタブ']]);
  });

  it('結合に吸収されたセルは空にする', () => {
    const m = model([
      ['区分', '氏名', '備考'],
      ['A', '田中', '○'],
      ['', '鈴木', '×']
    ]);
    const merged = mergeCells(m, { startRow: 1, startCol: 0, endRow: 2, endCol: 0 });
    if (!merged.ok) throw new Error(merged.message);
    const tsv = buildTsv(merged.value, { startRow: 1, startCol: 0, endRow: 2, endCol: 1 });
    expect(tsv).toBe('A\t田中\n\t鈴木');
  });

  it('範囲が逆向き・表外でも表の中に収める', () => {
    const tsv = buildTsv(BASE(), { startRow: 9, startCol: 9, endRow: 1, endCol: 1 });
    expect(tsv).toBe('田中\t○\n鈴木\t×');
  });
});

describe('pasteCellsAt', () => {
  it('選択の左上を起点に上書きする', () => {
    const out = pasteCellsAt(BASE(), [['X', 'Y']], { startRow: 1, startCol: 0, endRow: 1, endCol: 0 });
    expect(texts(out.model)[1]).toEqual(['X', 'Y', '○']);
    expect(out.range).toEqual({ startRow: 1, startCol: 0, endRow: 1, endCol: 1 });
    expect(out.skipped).toBe(0);
  });

  it('値が 1 つなら選択範囲すべてを埋める', () => {
    const out = pasteCellsAt(BASE(), [['○']], { startRow: 1, startCol: 2, endRow: 2, endCol: 2 });
    expect(texts(out.model).map(row => row[2])).toEqual(['備考', '○', '○']);
    expect(out.range).toEqual({ startRow: 1, startCol: 2, endRow: 2, endCol: 2 });
  });

  it('はみ出す分だけ行と列を足す', () => {
    const out = pasteCellsAt(
      BASE(),
      [
        ['a', 'b', 'c'],
        ['d', 'e', 'f']
      ],
      { startRow: 2, startCol: 2, endRow: 2, endCol: 2 }
    );
    expect(out.model.rows).toHaveLength(4);
    expect(out.model.columns).toHaveLength(5);
    expect(texts(out.model)[2]).toEqual(['B', '鈴木', 'a', 'b', 'c']);
    expect(texts(out.model)[3]).toEqual(['', '', 'd', 'e', 'f']);
  });

  it('結合に吸収されたセルには書かず、捨てた数を返す', () => {
    const merged = mergeCells(BASE(), { startRow: 1, startCol: 0, endRow: 2, endCol: 0 });
    if (!merged.ok) throw new Error(merged.message);
    const out = pasteCellsAt(merged.value, [['P'], ['Q']], {
      startRow: 1,
      startCol: 0,
      endRow: 1,
      endCol: 0
    });
    // アンカーには書き、吸収されたセルは元のまま
    expect(out.model.rows[1][0].text).toBe('P');
    expect(out.model.rows[2][0].hidden).toBe(true);
    expect(out.skipped).toBe(1);
    // 結合は保たれる
    expect(out.model.rows[1][0].rowspan).toBe(2);
  });

  it('セル内改行を含む値も 1 セルとして貼れる', () => {
    const out = pasteCellsAt(BASE(), [['一行目\n二行目']], {
      startRow: 1,
      startCol: 1,
      endRow: 1,
      endCol: 1
    });
    expect(out.model.rows[1][1].text).toBe('一行目\n二行目');
  });

  it('元のモデルを壊さない', () => {
    const base = BASE();
    const before = texts(base);
    pasteCellsAt(base, [['X']], { startRow: 1, startCol: 0, endRow: 1, endCol: 0 });
    expect(texts(base)).toEqual(before);
  });
});
