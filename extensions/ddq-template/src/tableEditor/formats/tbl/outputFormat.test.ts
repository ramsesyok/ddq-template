import { describe, it, expect } from 'vitest';
import { serializeTableBody, serializeTblBlock } from './serializeTblBlock';
import { parsePipeTable } from '../pipeTable/parsePipeTable';
import { mergeCells } from '../../model/mergeCells';
import type { TableModel } from '../../model/TableModel';

/** 結合なし・ヘッダ 1 行の素の表。 */
function plainModel(): TableModel {
  const parsed = parsePipeTable(
    [
      '| 大分類   | 中分類 | 項目         |',
      '|----------|--------|--------------|',
      '| 受注管理 | 登録   | 入力チェック |',
      '| 受注管理 | 引当   | 在庫確認     |'
    ].join('\n')
  );
  if (!parsed.ok) throw new Error(parsed.message);
  return parsed.value;
}

const isGrid = (out: string) => out.startsWith('+');
const isPipe = (out: string) => out.startsWith('|');

describe('セル内改行があるときの出力形式', () => {
  it('改行が無ければ従来どおりパイプ表', () => {
    expect(isPipe(serializeTableBody(plainModel()))).toBe(true);
  });

  it('改行があればグリッド表にする（パイプ表では <br> になりリストを書けない）', () => {
    const model = plainModel();
    model.rows[1][2].text = '- 桁数\n- 必須';

    const out = serializeTableBody(model);
    expect(isGrid(out)).toBe(true);
    expect(out).not.toContain('<br>');
    expect(out).toContain('- 桁数');
  });

  it('隠れセル（結合に吸収されたセル）の改行では切り替えない', () => {
    const model = plainModel();
    const merged = mergeCells(model, { startRow: 1, startCol: 0, endRow: 2, endCol: 0 });
    if (!merged.ok) throw new Error(merged.message);
    // 表示されないセルのテキストは出力に出ない
    merged.value.rows[2][0].text = 'a\nb';
    merged.value.outputFormat = 'mergeCols';

    expect(isPipe(serializeTableBody(merged.value))).toBe(true);
  });

  it('merge-cols を選んでいても、改行があれば素のグリッド表で出す', () => {
    const model = plainModel();
    const merged = mergeCells(model, { startRow: 1, startCol: 0, endRow: 2, endCol: 0 });
    if (!merged.ok) throw new Error(merged.message);
    merged.value.outputFormat = 'mergeCols';
    merged.value.rows[1][2].text = '- 桁数\n- 必須';

    const block = serializeTblBlock(merged.value);
    // merge-cols 属性は残る（素のグリッド表なら design-doc.lua が結合してくれる）
    expect(block).toContain('merge-cols="1"');
    const body = serializeTableBody(merged.value);
    expect(isGrid(body)).toBe(true);
    // 結合は展開され、同じ値が縦に並ぶ
    expect((body.match(/受注管理/g) ?? []).length).toBe(2);
  });

  it('改行が無い merge-cols は従来どおりパイプ表', () => {
    const model = plainModel();
    const merged = mergeCells(model, { startRow: 1, startCol: 0, endRow: 2, endCol: 0 });
    if (!merged.ok) throw new Error(merged.message);
    merged.value.outputFormat = 'mergeCols';

    expect(isPipe(serializeTableBody(merged.value))).toBe(true);
  });
});
