import { describe, it, expect } from 'vitest';
import { inspectColumnWidths, completeColumnWidths } from './columnWidths';
import { validateTableModel } from './validateTableModel';
import type { TableModel } from './TableModel';
import { emptyAttributes, makeCell } from './TableModel';

function model(widths: (number | undefined)[]): TableModel {
  return {
    id: 't',
    version: 1,
    headerRows: 1,
    columns: widths.map(width => ({ width })),
    rows: [widths.map((_, c) => makeCell(0, c, `h${c}`))],
    attributes: emptyAttributes(),
    outputFormat: 'pipeTable'
  };
}

describe('inspectColumnWidths', () => {
  it('合計が 100 以内なら超過ではない', () => {
    const w = inspectColumnWidths(model([20, 20, 60]));
    expect(w.total).toBe(100);
    expect(w.overflow).toBe(false);
    expect(w.autoFill).toBeUndefined();
  });

  it('合計が 100 を超えたら超過', () => {
    const w = inspectColumnWidths(model([50, 60]));
    expect(w.total).toBe(110);
    expect(w.overflow).toBe(true);
  });

  it('小数の誤差で超過扱いにしない', () => {
    const w = inspectColumnWidths(model([33.3, 33.3, 33.4]));
    expect(w.overflow).toBe(false);
  });

  it('空欄が 1 列だけなら残り幅を求める', () => {
    const w = inspectColumnWidths(model([20, undefined, 60]));
    expect(w.autoFill).toEqual({ col: 1, width: 20 });
  });

  it('空欄が 2 列以上なら補完しない', () => {
    expect(inspectColumnWidths(model([20, undefined, undefined])).autoFill).toBeUndefined();
  });

  it('全列が空欄（＝自動幅）なら補完しない', () => {
    expect(inspectColumnWidths(model([undefined, undefined])).autoFill).toBeUndefined();
  });

  it('残りが 0 以下なら補完しない', () => {
    expect(inspectColumnWidths(model([40, 60, undefined])).autoFill).toBeUndefined();
    expect(inspectColumnWidths(model([40, 70, undefined])).autoFill).toBeUndefined();
  });

  it('負の幅を見つける', () => {
    expect(inspectColumnWidths(model([-10, 50])).hasNegative).toBe(true);
  });
});

describe('completeColumnWidths', () => {
  it('空欄 1 列に残り幅を入れる', () => {
    const out = completeColumnWidths(model([30, undefined, 20]));
    expect(out.columns.map(c => c.width)).toEqual([30, 50, 20]);
  });

  it('小数でも誤差を残さない', () => {
    const out = completeColumnWidths(model([33.3, 33.3, undefined]));
    expect(out.columns.map(c => c.width)).toEqual([33.3, 33.3, 33.4]);
  });

  it('補完できないときはモデルをそのまま返す', () => {
    const m = model([20, undefined, undefined]);
    expect(completeColumnWidths(m)).toBe(m);
  });
});

describe('validateTableModel の列幅検査', () => {
  it('合計が 100 を超えるとエラー', () => {
    const { errors } = validateTableModel(model([50, 60]));
    expect(errors.join()).toContain('100 を超えています');
  });

  it('負の幅はエラー', () => {
    const { errors } = validateTableModel(model([-10, 50]));
    expect(errors.join()).toContain('負の値');
  });

  it('一部だけ指定はエラー（補完前）', () => {
    const { errors } = validateTableModel(model([20, undefined, undefined]));
    expect(errors.join()).toContain('全列に指定するか');
  });

  it('空欄 1 列は補完してから通せばエラーにならない', () => {
    const { errors } = validateTableModel(completeColumnWidths(model([20, undefined, 60])));
    expect(errors).toEqual([]);
  });

  it('全列が空欄（自動幅）はエラーにならない', () => {
    const { errors } = validateTableModel(model([undefined, undefined]));
    expect(errors).toEqual([]);
  });

  it('全列 0 はエラー', () => {
    const { errors } = validateTableModel(model([0, 0]));
    expect(errors.join()).toContain('0 以下');
  });
});
