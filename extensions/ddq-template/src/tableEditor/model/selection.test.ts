import { describe, it, expect } from 'vitest';
import {
  anchorOf,
  clampRange,
  expandRangeToMerges,
  cellOverlapsRange,
  moveActive,
  moveTab,
  moveToEdge,
  extendSelection,
  selectAll
} from './selection';
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

/** 4 行 × 3 列。(1,0)〜(2,0) を縦結合、(0,1)〜(0,2) を横結合。 */
const MERGED = () => {
  const base = model([
    ['h0', 'h1', 'h2'],
    ['a', 'b', 'c'],
    ['a', 'e', 'f'],
    ['g', 'h', 'i']
  ]);
  const v = mergeCells(base, { startRow: 1, startCol: 0, endRow: 2, endCol: 0 });
  if (!v.ok) throw new Error(v.message);
  const h = mergeCells(v.value, { startRow: 0, startCol: 1, endRow: 0, endCol: 2 });
  if (!h.ok) throw new Error(h.message);
  return h.value;
};

describe('anchorOf', () => {
  it('結合に吸収されたセルは結合元の位置になる', () => {
    expect(anchorOf(MERGED(), 2, 0)).toEqual({ row: 1, col: 0 });
    expect(anchorOf(MERGED(), 0, 2)).toEqual({ row: 0, col: 1 });
  });

  it('通常セルはそのまま、表の外は端に寄せる', () => {
    expect(anchorOf(MERGED(), 3, 1)).toEqual({ row: 3, col: 1 });
    expect(anchorOf(MERGED(), 99, -5)).toEqual({ row: 3, col: 0 });
  });
});

describe('clampRange', () => {
  it('向きを保ったまま表内に収める', () => {
    expect(
      clampRange(MERGED(), { startRow: 9, startCol: 2, endRow: -1, endCol: 9 })
    ).toEqual({ startRow: 3, startCol: 2, endRow: 0, endCol: 2 });
  });
});

describe('expandRangeToMerges', () => {
  it('結合セルの一部に掛かる範囲は結合セル全体まで広げる', () => {
    expect(
      expandRangeToMerges(MERGED(), { startRow: 2, startCol: 0, endRow: 2, endCol: 1 })
    ).toEqual({ startRow: 1, startCol: 0, endRow: 2, endCol: 1 });
  });

  it('広げた先で別の結合に掛かれば続けて広げる', () => {
    // (0,2) を含む → 横結合 (0,1)-(0,2) まで広がる
    expect(
      expandRangeToMerges(MERGED(), { startRow: 0, startCol: 2, endRow: 1, endCol: 2 })
    ).toEqual({ startRow: 0, startCol: 1, endRow: 1, endCol: 2 });
  });

  it('結合に掛からなければ正規化するだけ', () => {
    expect(
      expandRangeToMerges(MERGED(), { startRow: 3, startCol: 2, endRow: 3, endCol: 1 })
    ).toEqual({ startRow: 3, startCol: 1, endRow: 3, endCol: 2 });
  });
});

describe('cellOverlapsRange', () => {
  it('結合セルは覆う範囲全体で重なりを見る', () => {
    const range = { startRow: 2, startCol: 0, endRow: 2, endCol: 0 };
    expect(cellOverlapsRange(MERGED(), 1, 0, range)).toBe(true);
    expect(cellOverlapsRange(MERGED(), 2, 0, range)).toBe(false); // hidden
    expect(cellOverlapsRange(MERGED(), 3, 0, range)).toBe(false);
  });
});

describe('moveActive', () => {
  it('結合セルを飛び越える', () => {
    const m = MERGED();
    expect(moveActive(m, { row: 1, col: 0 }, 'down')).toEqual({ row: 3, col: 0 });
    expect(moveActive(m, { row: 3, col: 0 }, 'up')).toEqual({ row: 1, col: 0 });
    expect(moveActive(m, { row: 0, col: 1 }, 'right')).toEqual({ row: 0, col: 1 });
    expect(moveActive(m, { row: 0, col: 0 }, 'right')).toEqual({ row: 0, col: 1 });
  });

  it('端では動かない', () => {
    const m = MERGED();
    expect(moveActive(m, { row: 0, col: 0 }, 'up')).toEqual({ row: 0, col: 0 });
    expect(moveActive(m, { row: 3, col: 2 }, 'right')).toEqual({ row: 3, col: 2 });
  });
});

describe('moveTab', () => {
  it('行末で次の行の先頭へ折り返す', () => {
    const m = MERGED();
    // 次の行の先頭は縦結合の 2 行目なので、結合元 (1,0) に乗る
    expect(moveTab(m, { row: 1, col: 2 }, false)).toEqual({ row: 1, col: 0 });
    expect(moveTab(m, { row: 2, col: 2 }, false)).toEqual({ row: 3, col: 0 });
    expect(moveTab(m, { row: 3, col: 2 }, false)).toEqual({ row: 3, col: 2 });
  });

  it('Shift+Tab は行頭で前の行の末尾へ', () => {
    const m = MERGED();
    expect(moveTab(m, { row: 3, col: 0 }, true)).toEqual({ row: 2, col: 2 });
    expect(moveTab(m, { row: 0, col: 0 }, true)).toEqual({ row: 0, col: 0 });
  });
});

describe('moveToEdge', () => {
  it('Home / End は行内、Ctrl 付きは表全体', () => {
    const m = MERGED();
    expect(moveToEdge(m, { row: 2, col: 1 }, false, false)).toEqual({ row: 1, col: 0 });
    expect(moveToEdge(m, { row: 2, col: 1 }, true, false)).toEqual({ row: 2, col: 2 });
    expect(moveToEdge(m, { row: 2, col: 1 }, false, true)).toEqual({ row: 0, col: 0 });
    expect(moveToEdge(m, { row: 2, col: 1 }, true, true)).toEqual({ row: 3, col: 2 });
  });
});

describe('extendSelection', () => {
  it('アンカーを保ってフォーカスを動かす', () => {
    const m = MERGED();
    const r = { startRow: 3, startCol: 1, endRow: 3, endCol: 1 };
    expect(extendSelection(m, r, 'right')).toEqual({ startRow: 3, startCol: 1, endRow: 3, endCol: 2 });
    expect(extendSelection(m, r, 'left')).toEqual({ startRow: 3, startCol: 1, endRow: 3, endCol: 0 });
  });

  it('広げる向きの結合セルは端までまとめて飛ぶ', () => {
    const m = MERGED();
    const r = { startRow: 0, startCol: 0, endRow: 0, endCol: 0 };
    expect(extendSelection(m, r, 'down')).toEqual({ startRow: 0, startCol: 0, endRow: 2, endCol: 0 });
  });

  it('縮める向きでは 1 つずつ戻る', () => {
    const m = MERGED();
    const r = { startRow: 3, startCol: 2, endRow: 0, endCol: 2 };
    expect(extendSelection(m, r, 'down')).toEqual({ startRow: 3, startCol: 2, endRow: 1, endCol: 2 });
  });

  it('端では止まる', () => {
    const m = MERGED();
    const r = { startRow: 0, startCol: 0, endRow: 0, endCol: 0 };
    expect(extendSelection(m, r, 'up')).toEqual(r);
  });
});

describe('selectAll', () => {
  it('表全体', () => {
    expect(selectAll(MERGED())).toEqual({ startRow: 0, startCol: 0, endRow: 3, endCol: 2 });
  });
});
