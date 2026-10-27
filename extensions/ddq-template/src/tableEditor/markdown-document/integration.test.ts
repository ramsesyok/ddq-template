import { describe, it, expect } from 'vitest';
import { findEditTarget } from './findEditTarget';
import { replaceLines } from './replaceRange';
import { serializeTblBlock, serializeTableBody } from '../formats/tbl/serializeTblBlock';
import { mergeCells } from '../model/mergeCells';
import { validateTableModel } from '../model/validateTableModel';

/** 拡張ホストが Apply でやることと同じ手順。 */
function apply(lines: string[], cursorLine: number, edit: (m: any) => any): string[] {
  const found = findEditTarget(lines, cursorLine, 'id');
  if (!found.ok) throw new Error(found.message);
  const t = found.value;
  const model = edit(t.model);

  if (t.kind === 'tblPart' && t.partCount === 1 && t.divRange) {
    return replaceLines(lines, t.divRange, serializeTblBlock(model, { colons: t.colons, partCount: 1 }));
  }
  if (t.kind === 'tblPart') {
    return replaceLines(lines, t.range, serializeTableBody(model, t.partCount));
  }
  return replaceLines(lines, t.range, serializeTblBlock(model, { colons: t.colons }));
}

describe('通常の .tbl（結合なし）', () => {
  it('パイプ表のまま出力し、属性を保つ', () => {
    const src = [
      '::: {.tbl caption="ユーザ属性一覧" label="tbl-user" widths="20,20,60"}',
      '| 区分 | 属性 | 説明 |',
      '|------|------|------|',
      '| 基本 | 氏名 | 名前 |',
      ':::'
    ];
    const out = apply(src, 2, m => m).join('\n');

    expect(out).toContain('::: {.tbl caption="ユーザ属性一覧" label="tbl-user" widths="20,20,60"}');
    expect(out).toContain('| 区分 | 属性 | 説明 |');
    expect(out).not.toContain('+---');
    expect(out.trimEnd().endsWith(':::')).toBe(true);
  });
});

describe('結合あり → グリッド表', () => {
  it('既定でグリッド表として出力する', () => {
    const src = [
      '::: {.tbl caption="機能分類"}',
      '| 大分類 | 中分類 | 項目 |',
      '|--------|--------|------|',
      '| 受注   | 登録   | 入力 |',
      '| 受注   | 引当   | 在庫 |',
      ':::'
    ];
    const out = apply(src, 3, m => {
      const merged = mergeCells(m, { startRow: 1, startCol: 0, endRow: 2, endCol: 0 });
      if (!merged.ok) throw new Error(merged.message);
      return merged.value;
    }).join('\n');

    expect(out).toContain('+===');
    expect(out).toMatch(/\+ +\+/); // rowspan の空白区間
    expect(out).toContain('caption="機能分類"');
  });

  it('merge-cols を選べば属性付きのパイプ表として出力する', () => {
    const src = [
      '::: {.tbl caption="機能分類"}',
      '| 大分類 | 中分類 | 項目 |',
      '|--------|--------|------|',
      '| 受注   | 登録   | 入力 |',
      '| 受注   | 引当   | 在庫 |',
      ':::'
    ];
    const out = apply(src, 3, m => {
      const merged = mergeCells(m, { startRow: 1, startCol: 0, endRow: 2, endCol: 0 });
      if (!merged.ok) throw new Error(merged.message);
      return { ...merged.value, outputFormat: 'mergeCols' as const };
    }).join('\n');

    expect(out).toContain('merge-cols="1"');
    expect(out).not.toContain('+---');
    // 結合された値は縦に繰り返して書く（テンプレート側で結合される）
    expect(out.match(/受注/g)?.length).toBe(2);
  });

  it('merge-cols で再現できない結合は選んでもグリッド表になる', () => {
    const src = [
      '::: {.tbl}',
      '| A | B |',
      '|---|---|',
      '| 1 | 2 |',
      '| 3 | 4 |',
      ':::'
    ];
    const out = apply(src, 3, m => {
      // 横結合は merge-cols では表現できない
      const merged = mergeCells(m, { startRow: 1, startCol: 0, endRow: 1, endCol: 1 });
      if (!merged.ok) throw new Error(merged.message);
      return { ...merged.value, outputFormat: 'mergeCols' as const };
    }).join('\n');

    expect(out).toContain('+---');
    expect(out).not.toContain('merge-cols');
  });
});

describe('分割表のパート編集', () => {
  const src = [
    '前書き',
    '',
    '::: {.tbl caption="一覧" label="tbl-x" widths="30,70"}',
    '| 属性 | 説明 |',
    '|------|------|',
    '| 氏名 | 名前 |',
    '',
    '| 属性 | 説明 |',
    '|------|------|',
    '| 住所 | 現住 |',
    ':::',
    '',
    '後書き'
  ];

  it('カーソルのあるパートだけを差し替える', () => {
    const out = apply(src, 9, m => {
      const rows = m.rows.map((r: any) => r.map((c: any) => ({ ...c })));
      rows[1][0].text = '電話';
      return { ...m, rows };
    });

    expect(out[2]).toBe(src[2]); // fence 行は不変
    expect(out.slice(3, 6)).toEqual(src.slice(3, 6)); // 1 番目のパートは不変
    expect(out[out.length - 1]).toBe('後書き');
    expect(out.join('\n')).toContain('電話');
  });

  it('パート編集ではグリッド表に固定される（merge-cols は全パートに波及するため）', () => {
    const out = apply(src, 9, m => {
      const merged = mergeCells(m, { startRow: 0, startCol: 0, endRow: 1, endCol: 0 });
      if (!merged.ok) throw new Error(merged.message);
      return { ...merged.value, outputFormat: 'mergeCols' as const };
    }).join('\n');

    expect(out).toContain('+---');
    expect(out).not.toContain('merge-cols');
    expect(out).toContain('caption="一覧"'); // fence は元のまま
  });

  it('列数が他パートと違うと警告になる', () => {
    const found = findEditTarget(src, 9, 'id');
    if (!found.ok) throw new Error(found.message);
    expect(found.value.otherPartColumnCounts).toEqual([2]);

    const model = { ...found.value.model, columns: [{}, {}, {}] };
    const v = validateTableModel(model as any, {
      otherPartColumnCounts: found.value.otherPartColumnCounts
    });
    expect(v.warnings.join()).toContain('列数が一致しない');
  });
});

describe('素の表の取り込み', () => {
  it('.tbl で囲んで書き戻す', () => {
    const src = ['前書き', '', '| a | b |', '|---|---|', '| 1 | 2 |', '', '後書き'];
    const out = apply(src, 3, m => m);

    expect(out[2]).toBe('::: {.tbl}');
    expect(out[out.length - 1]).toBe('後書き');
    expect(out.filter(l => l === ':::').length).toBe(1);
  });
});

describe('label の検証', () => {
  it('tbl- で始まらない label はエラーにする', () => {
    const found = findEditTarget(['::: {.tbl label="user"}', '| a |', '|---|', '| 1 |', ':::'], 1, 'id');
    if (!found.ok) throw new Error(found.message);
    const v = validateTableModel(found.value.model);
    expect(v.errors.join()).toContain('tbl-');
  });
});
