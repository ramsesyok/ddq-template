import { describe, it, expect } from 'vitest';
import { parseFenceHeader, toTblAttributes } from './parseTblAttributes';
import { buildFenceHeader } from './serializeTblBlock';
import { emptyAttributes, type TableModel } from '../../model/TableModel';

function modelWith(caption: string, extra?: Partial<TableModel['attributes']>): TableModel {
  return {
    id: 't',
    version: 1,
    headerRows: 1,
    columns: [{}, {}],
    rows: [],
    attributes: { ...emptyAttributes(), caption, ...extra },
    outputFormat: 'pipeTable'
  };
}

/** 出力した fence 行を読み直して、属性が元に戻るか。 */
function roundTrip(caption: string): string | undefined {
  const header = `::: ${buildFenceHeader(modelWith(caption))}`;
  const parsed = parseFenceHeader(header);
  if (!parsed) throw new Error(`fence として読めません: ${header}`);
  return toTblAttributes(parsed).caption;
}

describe('属性値のエスケープ', () => {
  it('引用符を含むキャプションが往復する', () => {
    expect(roundTrip('引用"あり"のキャプション')).toBe('引用"あり"のキャプション');
  });

  it('末尾がバックスラッシュでも fence が壊れない', () => {
    // エスケープしないと閉じ引用符が潰れ、.tbl ブロックごと壊れる
    const header = `::: ${buildFenceHeader(modelWith('末尾が円記号\\'))}`;
    expect(header).toContain('\\\\"');
    expect(roundTrip('末尾が円記号\\')).toBe('末尾が円記号\\');
  });

  it('バックスラッシュと引用符が混ざっても往復する', () => {
    expect(roundTrip('a\\"b')).toBe('a\\"b');
  });

  it('改行・タブは空白へ潰す', () => {
    const header = `::: ${buildFenceHeader(modelWith('前\n後\tX'))}`;
    expect(header).not.toMatch(/[\r\n\t]/);
    expect(roundTrip('前\n後\tX')).toBe('前 後 X');
  });

  it('波括弧はそのまま扱える', () => {
    expect(roundTrip('波括弧}を含む')).toBe('波括弧}を含む');
  });

  it('クラス名として成立しないものは出力しない', () => {
    const header = buildFenceHeader(
      modelWith('x', { extraClasses: ['landscape', 'bad class}', ''] })
    );
    expect(header).toContain('.landscape');
    expect(header).not.toContain('bad class');
    expect(header).not.toContain('..');
  });
});

describe('parseFenceHeader', () => {
  it('クラスと属性を読み分ける', () => {
    const h = parseFenceHeader(':::: {.tbl .unnumbered caption="表題" widths="1,2" merge-cols="1"}');
    expect(h?.colons).toBe(4);
    expect(h?.classes).toEqual(['tbl', 'unnumbered']);
    expect(h?.attributes).toEqual([
      ['caption', '表題'],
      ['widths', '1,2'],
      ['merge-cols', '1']
    ]);
  });

  it('label の先頭 # を落とす', () => {
    const h = parseFenceHeader('::: {.tbl label="#tbl-x"}');
    expect(toTblAttributes(h!).label).toBe('tbl-x');
  });

  it('未知の属性とクラスは保持する', () => {
    const h = parseFenceHeader('::: {.tbl .custom data-foo="1"}');
    const a = toTblAttributes(h!);
    expect(a.extraClasses).toEqual(['custom']);
    expect(a.extraAttributes).toEqual([['data-foo', '1']]);
  });
});
