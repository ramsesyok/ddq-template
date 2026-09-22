import { describe, it, expect } from 'vitest';
import { findEditTarget } from './findEditTarget';
import { replaceLines } from './replaceRange';
import { serializeTableBody, serializeTblBlock } from '../formats/tbl/serializeTblBlock';

function doc(...lines: string[]): string[] {
  return lines;
}

const SPLIT_TABLE = doc(
  '本文の前書き。',
  '',
  ':::: {.landscape}',
  '::: {.tbl caption="ユーザ属性一覧" label="tbl-user" widths="20,20,60"}',
  '| 属性 | 型   | 説明 |',
  '|------|------|------|',
  '| 氏名 | 文字 | 名前 |',
  '',
  '| 属性 | 型   | 説明 |',
  '|------|------|------|',
  '| 住所 | 文字 | 現住 |',
  ':::',
  '::::',
  '',
  '本文の後書き。'
);

describe('findEditTarget — 優先度①（.tbl div）', () => {
  it('カーソルのあるパートだけを編集対象にする', () => {
    const result = findEditTarget(SPLIT_TABLE, 9, 'id');
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const t = result.value;

    expect(t.kind).toBe('tblPart');
    expect(t.partCount).toBe(2);
    expect(t.partIndex).toBe(1);
    // 2 番目のパートの行範囲だけ
    expect(t.range).toEqual({ start: 8, end: 10 });
    expect(t.model.rows[1][0].text).toBe('住所');
  });

  it('分割表では div 属性を編集できない', () => {
    const result = findEditTarget(SPLIT_TABLE, 5, 'id');
    if (!result.ok) throw new Error(result.message);
    expect(result.value.attributesEditable).toBe(false);
    expect(result.value.partCount).toBe(2);
  });

  it('div の属性を読み取り、外側の .landscape を保持情報として返す', () => {
    const result = findEditTarget(SPLIT_TABLE, 5, 'id');
    if (!result.ok) throw new Error(result.message);
    const t = result.value;
    expect(t.model.attributes.caption).toBe('ユーザ属性一覧');
    expect(t.model.attributes.label).toBe('tbl-user');
    expect(t.model.columns.map(c => c.width)).toEqual([20, 20, 60]);
    expect(t.enclosingClasses).toContain('landscape');
    expect(t.colons).toBe(3);
  });

  it('パート置換で div 行・他パート・外側 div を一切変更しない', () => {
    const result = findEditTarget(SPLIT_TABLE, 9, 'id');
    if (!result.ok) throw new Error(result.message);
    const t = result.value;

    t.model.rows[1][0].text = '電話';
    const body = serializeTableBody(t.model, t.partCount);
    const after = replaceLines(SPLIT_TABLE, t.range, body);

    expect(after[2]).toBe(':::: {.landscape}');
    expect(after[3]).toBe(SPLIT_TABLE[3]);
    expect(after.slice(4, 7)).toEqual(SPLIT_TABLE.slice(4, 7));
    expect(after[0]).toBe('本文の前書き。');
    expect(after[after.length - 1]).toBe('本文の後書き。');
    expect(after.join('\n')).toContain('電話');
    expect(after.join('\n')).not.toContain('住所');
  });

  it('単一表の .tbl では属性を編集できる', () => {
    const single = doc(
      '::: {.tbl caption="表題"}',
      '| a | b |',
      '|---|---|',
      '| 1 | 2 |',
      ':::'
    );
    const result = findEditTarget(single, 2, 'id');
    if (!result.ok) throw new Error(result.message);
    expect(result.value.attributesEditable).toBe(true);
    expect(result.value.partCount).toBe(1);
    expect(result.value.range).toEqual({ start: 1, end: 3 });
  });

  it('widths の個数が列数と合わないときは警告して自動幅にする', () => {
    const bad = doc(
      '::: {.tbl widths="10,20"}',
      '| a | b | c |',
      '|---|---|---|',
      '| 1 | 2 | 3 |',
      ':::'
    );
    const result = findEditTarget(bad, 1, 'id');
    if (!result.ok) throw new Error(result.message);
    expect(result.value.model.columns.every(c => c.width === undefined)).toBe(true);
    expect(result.value.warnings.join()).toContain('widths');
  });

  it('merge-cols 付きの表は結合を展開して読み込む', () => {
    const src = doc(
      '::: {.tbl merge-cols="1"}',
      '| 大分類 | 項目 |',
      '|--------|------|',
      '| 受注   | 登録 |',
      '| 受注   | 引当 |',
      ':::'
    );
    const result = findEditTarget(src, 3, 'id');
    if (!result.ok) throw new Error(result.message);
    const m = result.value.model;
    expect(m.outputFormat).toBe('mergeCols');
    expect(m.rows[1][0].rowspan).toBe(2);
    expect(m.rows[2][0].hidden).toBe(true);
  });

  it('グリッド表のパートも読める', () => {
    const src = doc(
      '::: {.tbl}',
      '+------+------+',
      '| 項目 | 実績 |',
      '+======+======+',
      '| 売上 | 100  |',
      '+------+------+',
      ':::'
    );
    const result = findEditTarget(src, 3, 'id');
    if (!result.ok) throw new Error(result.message);
    expect(result.value.model.outputFormat).toBe('gridTable');
    expect(result.value.model.rows[1][0].text).toBe('売上');
  });
});

describe('findEditTarget — 優先度②（素の表）', () => {
  it('.tbl の外のパイプ表を取り込み、Apply で .tbl に包む', () => {
    const src = doc('前書き', '', '| a | b |', '|---|---|', '| 1 | 2 |', '', '後書き');
    const result = findEditTarget(src, 3, 'id');
    if (!result.ok) throw new Error(result.message);
    const t = result.value;

    expect(t.kind).toBe('plainTable');
    expect(t.range).toEqual({ start: 2, end: 4 });

    const after = replaceLines(src, t.range, serializeTblBlock(t.model));
    expect(after[0]).toBe('前書き');
    expect(after[2]).toBe('::: {.tbl}');
    expect(after[after.length - 1]).toBe('後書き');
  });
});

describe('findEditTarget — 優先度③（新規）', () => {
  it('表が無ければ新規モデルを返し、範囲は空になる', () => {
    const src = doc('ただの本文', '');
    const result = findEditTarget(src, 0, 'id');
    if (!result.ok) throw new Error(result.message);
    expect(result.value.kind).toBe('new');
    expect(result.value.range.start).toBeGreaterThan(result.value.range.end);
  });
});

describe('findEditTarget — 壊れた fence', () => {
  it('閉じられていない div があればドキュメントを変更せずエラーにする', () => {
    const src = doc('::: {.tbl}', '| a |', '|---|', '| 1 |');
    const result = findEditTarget(src, 1, 'id');
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.message).toContain('終了 fence');
  });

  it('開始 fence の無い ::: があればエラーにする', () => {
    const src = doc('本文', ':::', '本文');
    const result = findEditTarget(src, 0, 'id');
    expect(result.ok).toBe(false);
  });
});
