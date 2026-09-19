import { describe, it, expect } from 'vitest';
import { simulateMergeCols, mergedMapToRowspans, parseMergeCols } from './simulateMergeCols';
import { canUseMergeCols } from './canUseMergeCols';
import { expandMergeCols } from './expandMergeCols';
import { parsePipeTable } from '../pipeTable/parsePipeTable';
import { mergeCells } from '../../model/mergeCells';
import type { TableModel } from '../../model/TableModel';

describe('parseMergeCols', () => {
  it('"all" と未指定は全列（null）', () => {
    expect(parseMergeCols('all')).toBeNull();
    expect(parseMergeCols(undefined)).toBeNull();
    expect(parseMergeCols('')).toBeNull();
  });
  it('列番号の並びを読む', () => {
    expect(parseMergeCols('2,3')).toEqual([2, 3]);
  });
});

describe('simulateMergeCols', () => {
  it('連続する同一値を結合する', () => {
    const texts = [
      ['受注', '登録'],
      ['受注', '登録'],
      ['受注', '引当']
    ];
    const spans = mergedMapToRowspans(simulateMergeCols(texts, null));
    expect(spans[0]).toEqual([3, 2]);
    expect(spans[1]).toEqual([0, 0]);
    expect(spans[2]).toEqual([0, 1]);
  });

  it('左の列が結合されていなければ連鎖しない（階層ルール）', () => {
    const texts = [
      ['A', '登録'],
      ['B', '登録']
    ];
    const spans = mergedMapToRowspans(simulateMergeCols(texts, null));
    // 1 列目が違うので 2 列目も結合されない
    expect(spans[0]).toEqual([1, 1]);
    expect(spans[1]).toEqual([1, 1]);
  });

  it('merge-cols で対象外の列は連鎖を断たない', () => {
    const texts = [
      ['1', '受注', '登録'],
      ['2', '受注', '登録']
    ];
    const spans = mergedMapToRowspans(simulateMergeCols(texts, [2, 3]));
    expect(spans[0]).toEqual([1, 2, 2]);
    expect(spans[1]).toEqual([1, 0, 0]);
  });

  it('空セルは結合しない', () => {
    const texts = [
      ['', 'x'],
      ['', 'x']
    ];
    const spans = mergedMapToRowspans(simulateMergeCols(texts, null));
    expect(spans[0][0]).toBe(1);
    expect(spans[1][0]).toBe(1);
  });
});

const HIERARCHY = [
  '| 大分類 | 中分類 | 項目         |',
  '|--------|--------|--------------|',
  '| 受注   | 登録   | 入力チェック |',
  '| 受注   | 登録   | 重複チェック |',
  '| 受注   | 引当   | 在庫確認     |'
].join('\n');

function model(source: string): TableModel {
  const parsed = parsePipeTable(source);
  if (!parsed.ok) throw new Error(parsed.message);
  return parsed.value;
}

describe('canUseMergeCols', () => {
  it('階層どおりの結合は再現できる', () => {
    const base = model(HIERARCHY);
    const a = mergeCells(base, { startRow: 1, startCol: 0, endRow: 3, endCol: 0 });
    if (!a.ok) throw new Error(a.message);
    const b = mergeCells(a.value, { startRow: 1, startCol: 1, endRow: 2, endCol: 1 });
    if (!b.ok) throw new Error(b.message);

    const check = canUseMergeCols(b.value);
    expect(check.ok).toBe(true);
    if (check.ok) expect(check.cols).toEqual([1, 2]);
  });

  it('colspan があると不可', () => {
    const base = model(HIERARCHY);
    const merged = mergeCells(base, { startRow: 1, startCol: 0, endRow: 1, endCol: 1 });
    if (!merged.ok) throw new Error(merged.message);
    const check = canUseMergeCols(merged.value);
    expect(check.ok).toBe(false);
    if (!check.ok) expect(check.reason).toContain('colspan');
  });

  it('同じ値が続く箇所を結合しないでおくことはできない', () => {
    // 「受注」が 3 行続くのに 2 行だけ結合する → merge-cols では 3 行結合されてしまう
    const base = model(HIERARCHY);
    const merged = mergeCells(base, { startRow: 1, startCol: 0, endRow: 2, endCol: 0 });
    if (!merged.ok) throw new Error(merged.message);
    const check = canUseMergeCols(merged.value);
    expect(check.ok).toBe(false);
    if (!check.ok) expect(check.reason).toContain('意図せず結合');
  });

  it('結合が 1 列だけなら、その列を指定すれば再現できる（連鎖の制約を受けない）', () => {
    const src = [
      '| A    | B    |',
      '|------|------|',
      '| x    | 同じ |',
      '| y    | 同じ |'
    ].join('\n');
    const merged = mergeCells(model(src), { startRow: 1, startCol: 1, endRow: 2, endCol: 1 });
    if (!merged.ok) throw new Error(merged.message);
    const check = canUseMergeCols(merged.value);
    expect(check.ok).toBe(true);
    if (check.ok) expect(check.cols).toEqual([2]);
  });

  it('右の列のほうが粗い階層は再現できない（順序を入れ替えても連鎖しない）', () => {
    // B（同じ）が 3 行、A（p）が 2 行。design-doc.lua は列を左から順に 1 パスで
    // 評価するので、merge-cols="2,1" と並べても「列 1 を見る時点で列 2 は未評価」に
    // なり連鎖しない。したがってこの結合はグリッド表でしか表現できない。
    const src = [
      '| A | B    |',
      '|---|------|',
      '| p | 同じ |',
      '| p | 同じ |',
      '| q | 同じ |'
    ].join('\n');
    const a = mergeCells(model(src), { startRow: 1, startCol: 1, endRow: 3, endCol: 1 });
    if (!a.ok) throw new Error(a.message);
    const b = mergeCells(a.value, { startRow: 1, startCol: 0, endRow: 2, endCol: 0 });
    if (!b.ok) throw new Error(b.message);

    const check = canUseMergeCols(b.value);
    expect(check.ok).toBe(false);
  });

  it('分割表では使えない', () => {
    const base = model(HIERARCHY);
    const merged = mergeCells(base, { startRow: 1, startCol: 0, endRow: 3, endCol: 0 });
    if (!merged.ok) throw new Error(merged.message);
    const check = canUseMergeCols(merged.value, 2);
    expect(check.ok).toBe(false);
    if (!check.ok) expect(check.reason).toContain('分割表');
  });

  it('ヘッダが複数行だと使えない', () => {
    const base = model(HIERARCHY);
    base.headerRows = 2;
    const check = canUseMergeCols(base);
    expect(check.ok).toBe(false);
  });
});

describe('expandMergeCols', () => {
  it('読み込み時に結合を復元し、判定と往復する', () => {
    const expanded = expandMergeCols(model(HIERARCHY), null);
    expect(expanded.rows[1][0].rowspan).toBe(3);
    expect(expanded.rows[2][0].hidden).toBe(true);
    expect(expanded.rows[1][1].rowspan).toBe(2);
    expect(expanded.rows[3][1].rowspan).toBe(1);

    const check = canUseMergeCols(expanded);
    expect(check.ok).toBe(true);
  });

  it('merge-cols="2,3" の指定を尊重する', () => {
    const src = [
      '| No | 大分類 | 中分類 |',
      '|----|--------|--------|',
      '| 1  | 受注   | 登録   |',
      '| 2  | 受注   | 登録   |'
    ].join('\n');
    const expanded = expandMergeCols(model(src), [2, 3]);
    expect(expanded.rows[1][0].rowspan).toBe(1); // 連番の列は結合しない
    expect(expanded.rows[1][1].rowspan).toBe(2);
    expect(expanded.rows[1][2].rowspan).toBe(2);
  });
});
