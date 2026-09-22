import { describe, it, expect } from 'vitest';
import { serializeGridTable } from './gridTable/serializeGridTable';
import { parseGridTable } from './gridTable/parseGridTable';
import { normalizeTableModel } from '../model/normalizeTableModel';
import { emptyAttributes, makeCell, type TableModel } from '../model/TableModel';
import { findEditTarget } from '../markdown-document/findEditTarget';
import { parseFenceHeader } from './tbl/parseTblAttributes';

function bigModel(rowLen: number, colLen: number): TableModel {
  const rows = Array.from({ length: rowLen }, (_, r) =>
    Array.from({ length: colLen }, (_, c) => {
      const cell = makeCell(r, c);
      cell.text = `セル${r}-${c}`;
      return cell;
    })
  );
  return normalizeTableModel({
    id: 'big',
    version: 1,
    headerRows: 1,
    columns: Array.from({ length: colLen }, () => ({})),
    rows,
    attributes: emptyAttributes(),
    outputFormat: 'gridTable'
  });
}

function ms(fn: () => void): number {
  const start = performance.now();
  fn();
  return performance.now() - start;
}

describe('大きな表の処理時間', () => {
  it('500 行 × 10 列のグリッド表を現実的な時間で出力できる', () => {
    const model = bigModel(500, 10);
    let out = '';
    const elapsed = ms(() => {
      out = serializeGridTable(model);
    });
    expect(out.split('\n').length).toBeGreaterThan(1000);
    expect(elapsed).toBeLessThan(2000);
  });

  it('500 行 × 10 列のグリッド表を現実的な時間で読み込める', () => {
    const source = serializeGridTable(bigModel(500, 10));
    let ok = false;
    const elapsed = ms(() => {
      ok = parseGridTable(source).ok;
    });
    expect(ok).toBe(true);
    expect(elapsed).toBeLessThan(2000);
  });

  it('結合だらけの最悪ケース（表全体が 1 セル）でも現実的な時間で出力できる', () => {
    // 覆っているセルを後ろから探す実装だと、隠れセルが多いほど探索が伸びる
    const model = bigModel(300, 10);
    const rows = model.rows.map(r => r.map(c => ({ ...c, hidden: true })));
    rows[0][0] = { ...rows[0][0], hidden: false, rowspan: 300, colspan: 10 };
    const merged = { ...model, headerRows: 0, rows };

    let out = '';
    const elapsed = ms(() => {
      out = serializeGridTable(merged);
    });
    expect(out.length).toBeGreaterThan(0);
    expect(elapsed).toBeLessThan(2000);
  });
});

describe('壊れた入力・極端な入力', () => {
  it('長大な空白だけの fence 行で停止しない', () => {
    const line = ':::' + ' '.repeat(50000);
    const elapsed = ms(() => parseFenceHeader(line));
    expect(elapsed).toBeLessThan(1000);
  });

  it('閉じない引用符の属性で停止しない', () => {
    const line = '::: {.tbl caption="' + 'あ'.repeat(20000);
    const elapsed = ms(() => parseFenceHeader(line));
    expect(elapsed).toBeLessThan(1000);
  });

  it('罫線に見える長大な行で停止しない', () => {
    const lines = ['+' + '-'.repeat(50000), '| x |'];
    const elapsed = ms(() => findEditTarget(lines, 0, 'id'));
    expect(elapsed).toBeLessThan(1000);
  });

  it('大量の fence がネストしても停止しない', () => {
    const lines = [
      ...Array.from({ length: 2000 }, () => ':::: {.landscape}'),
      ...Array.from({ length: 2000 }, () => '::::')
    ];
    const elapsed = ms(() => findEditTarget(lines, 0, 'id'));
    expect(elapsed).toBeLessThan(2000);
  });

  it('表でない大量の行でも新規作成として即座に返る', () => {
    const lines = Array.from({ length: 20000 }, (_, i) => `本文 ${i}`);
    let kind = '';
    const elapsed = ms(() => {
      const r = findEditTarget(lines, 10000, 'id');
      if (r.ok) kind = r.value.kind;
    });
    expect(kind).toBe('new');
    expect(elapsed).toBeLessThan(1000);
  });
});
