import { describe, it, expect } from 'vitest';
import { parseGridTable } from './parseGridTable';
import { serializeGridTable } from './serializeGridTable';
import { displayWidth } from '../displayWidth';

/** AUTHORING.md §6「横結合・見出し結合・マルチヘッダ」の例。 */
const MULTI_HEADER = [
  '+----------+----------+----------+',
  '| 項目     | 実績                |',
  '+          +----------+----------+',
  '|          | 前期     | 当期     |',
  '+==========+==========+==========+',
  '| 売上     | 100      | 120      |',
  '+----------+----------+----------+'
].join('\n');

describe('parseGridTable', () => {
  it('マルチヘッダ・rowspan・colspan を読み取る', () => {
    const result = parseGridTable(MULTI_HEADER);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    const m = result.value;

    expect(m.columns).toHaveLength(3);
    expect(m.rows).toHaveLength(3);
    expect(m.headerRows).toBe(2);

    // 「項目」が 2 行を縦結合
    expect(m.rows[0][0].text).toBe('項目');
    expect(m.rows[0][0].rowspan).toBe(2);
    expect(m.rows[1][0].hidden).toBe(true);

    // 「実績」が 2 列を横結合
    expect(m.rows[0][1].text).toBe('実績');
    expect(m.rows[0][1].colspan).toBe(2);
    expect(m.rows[0][2].hidden).toBe(true);

    expect(m.rows[1][1].text).toBe('前期');
    expect(m.rows[1][2].text).toBe('当期');
    expect(m.rows[2].map(c => c.text)).toEqual(['売上', '100', '120']);
  });

  it('列揃えをヘッダ終端行から読む', () => {
    const src = [
      '+------+------+------+',
      '| a    | b    | c    |',
      '+:=====+:====:+=====:+',
      '| 1    | 2    | 3    |',
      '+------+------+------+'
    ].join('\n');
    const result = parseGridTable(src);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.columns.map(c => c.align)).toEqual(['left', 'center', 'right']);
  });

  it('ヘッダが無い表は最上段の罫線から揃えを読む', () => {
    const src = [
      '+:-----+-----:+',
      '| a    | b    |',
      '+------+------+'
    ].join('\n');
    const result = parseGridTable(src);
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.headerRows).toBe(0);
    expect(result.value.columns.map(c => c.align)).toEqual(['left', 'right']);
  });
});

describe('serializeGridTable', () => {
  it('罫線を表示幅（全角=2）で揃える', () => {
    const parsed = parseGridTable(MULTI_HEADER);
    expect(parsed.ok).toBe(true);
    if (!parsed.ok) return;

    const out = serializeGridTable(parsed.value);
    const widths = out.split('\n').map(displayWidth);
    expect(new Set(widths).size).toBe(1);
  });

  it('マルチヘッダの例を往復しても構造が保たれる', () => {
    const first = parseGridTable(MULTI_HEADER);
    expect(first.ok).toBe(true);
    if (!first.ok) return;

    const out = serializeGridTable(first.value);
    const second = parseGridTable(out);
    expect(second.ok).toBe(true);
    if (!second.ok) return;

    expect(second.value.headerRows).toBe(first.value.headerRows);
    expect(second.value.rows.map(r => r.map(c => [c.text, c.rowspan, c.colspan, c.hidden]))).toEqual(
      first.value.rows.map(r => r.map(c => [c.text, c.rowspan, c.colspan, c.hidden]))
    );
  });

  it('rowspan は境界線をそのセルの部分だけ空白にする', () => {
    const parsed = parseGridTable(MULTI_HEADER);
    if (!parsed.ok) throw new Error('parse failed');
    const out = serializeGridTable(parsed.value).split('\n');
    // 1 行目と 2 行目の間の罫線。先頭列は空白、残りは罫線
    expect(out[2].startsWith('+     ')).toBe(true);
    expect(out[2]).toMatch(/-\+$/);
  });

  it('セル内改行を複数行として書き出す', () => {
    const parsed = parseGridTable(MULTI_HEADER);
    if (!parsed.ok) throw new Error('parse failed');
    const model = parsed.value;
    model.rows[2][0].text = '売上\n（税込）';

    const out = serializeGridTable(model);
    expect(out).toContain('（税込）');

    const again = parseGridTable(out);
    expect(again.ok).toBe(true);
    if (!again.ok) return;
    expect(again.value.rows[2][0].text).toBe('売上\n（税込）');
  });

  it('リスト項目の行には `\\` を付けない（項目末尾に余計な改行が入るため）', () => {
    const parsed = parseGridTable(MULTI_HEADER);
    if (!parsed.ok) throw new Error('parse failed');
    const model = parsed.value;
    model.rows[2][0].text = '- 一つ目\n- 二つ目';
    model.rows[2][1].text = '1. 一つ目\n2. 二つ目';
    model.rows[2][2].text = '前置き\n- 一つ目';

    const out = serializeGridTable(model);
    expect(out).toMatch(/\|\s*- 一つ目\s*\|/); // `\` 無し
    expect(out).toMatch(/\|\s*1\. 一つ目\s*\|/);

    const again = parseGridTable(out);
    if (!again.ok) throw new Error('reparse failed');
    expect(again.value.rows[2][0].text).toBe('- 一つ目\n- 二つ目');
    expect(again.value.rows[2][1].text).toBe('1. 一つ目\n2. 二つ目');
    expect(again.value.rows[2][2].text).toBe('前置き\n- 一つ目');
  });

  it('文章行の直後のリストには空行を補う（無いとリストにならないため）', () => {
    const parsed = parseGridTable(MULTI_HEADER);
    if (!parsed.ok) throw new Error('parse failed');
    const model = parsed.value;
    model.rows[2][0].text = '受注入力画面\n1. 一つ目\n2. 二つ目';

    const out = serializeGridTable(model);
    // 文章行にもリスト項目にも `\` は付かない
    expect(out).not.toMatch(/受注入力画面\\/);
    expect(out).not.toMatch(/一つ目\\/);
    // 文章行とリストの間に空のセル行が入る
    const lines = out.split('\n');
    const textLine = lines.findIndex(l => l.includes('受注入力画面'));
    const firstItem = lines.findIndex(l => l.includes('1. 一つ目'));
    expect(firstItem).toBe(textLine + 2);
    expect(lines[textLine + 1]).toMatch(/^\|\s+\|/);

    // 補った空行は読み戻しで畳む（往復で空行が増えていかない）
    const again = parseGridTable(out);
    if (!again.ok) throw new Error('reparse failed');
    expect(again.value.rows[2][0].text).toBe('受注入力画面\n1. 一つ目\n2. 二つ目');
    expect(serializeGridTable(again.value)).toBe(out);
  });

  it('自分で空行を挟んでも結果は同じ（読み戻しでは畳む）', () => {
    const parsed = parseGridTable(MULTI_HEADER);
    if (!parsed.ok) throw new Error('parse failed');

    const withBlank = parsed.value;
    withBlank.rows[2][0].text = '前置き\n\n- 一つ目\n- 二つ目';
    const withoutBlank = parseGridTable(MULTI_HEADER);
    if (!withoutBlank.ok) throw new Error('parse failed');
    withoutBlank.value.rows[2][0].text = '前置き\n- 一つ目\n- 二つ目';

    const out = serializeGridTable(withBlank);
    expect(out).toBe(serializeGridTable(withoutBlank.value));
    expect(out).not.toMatch(/前置き\\/);
    expect(out).not.toMatch(/一つ目\\/);

    const again = parseGridTable(out);
    if (!again.ok) throw new Error('reparse failed');
    expect(again.value.rows[2][0].text).toBe('前置き\n- 一つ目\n- 二つ目');
  });

  it('入れ子リストの字下げを保つ', () => {
    const src = [
      '+----------------+------------------+',
      '| 書き方         | セルに書ける内容 |',
      '+================+==================+',
      '| 入れ子         | - 果物           |',
      '|                |     - りんご     |',
      '|                |     - みかん     |',
      '|                | - 野菜           |',
      '+----------------+------------------+'
    ].join('\n');
    const parsed = parseGridTable(src);
    if (!parsed.ok) throw new Error(parsed.message);
    expect(parsed.value.rows[1][1].text).toBe('- 果物\n    - りんご\n    - みかん\n- 野菜');

    const out = serializeGridTable(parsed.value);
    expect(out).toMatch(/\|\s{5}- りんご\s*\|/); // 字下げが残る
    expect(out).not.toMatch(/りんご\\/); // 項目どうしなので `\` は付かない
    const again = parseGridTable(out);
    if (!again.ok) throw new Error(again.message);
    expect(again.value.rows[1][1].text).toBe(parsed.value.rows[1][1].text);
  });

  it('項目内の折り返しは `\\` を付けて項目本文の桁まで下げる', () => {
    const parsed = parseGridTable(MULTI_HEADER);
    if (!parsed.ok) throw new Error('parse failed');
    const model = parsed.value;
    // `<br>` で書かれた折り返しを読むと字下げの無い続き行になる
    model.rows[2][0].text = '- 産地コード\n（5 桁の数字）\n- 等級';
    model.rows[2][1].text = '1. 計量する\n（0.1 g 単位）\n2. 記録する';

    const out = serializeGridTable(model);
    expect(out).toMatch(/- 産地コード\\/);
    expect(out).toMatch(/\|\s{3}（5 桁の数字）/); // 空白 1 桁＋字下げ 2 桁
    expect(out).toMatch(/1\. 計量する\\/);
    expect(out).toMatch(/\|\s{4}（0\.1 g 単位）/); // 空白 1 桁＋字下げ 3 桁
    // 続き行を段落に切り離さない（切り離すとリストが 2 つに割れる）
    expect(out).not.toMatch(/産地コード\\\n\|\s+\|/);

    const again = parseGridTable(out);
    if (!again.ok) throw new Error('reparse failed');
    expect(again.value.rows[2][0].text).toBe('- 産地コード\n  （5 桁の数字）\n- 等級');
    // 字下げが付いた状態で安定する
    expect(serializeGridTable(again.value)).toBe(out);
  });

  it('リストの後に段落を置くには空行が要る（書いた空行は保つ）', () => {
    const parsed = parseGridTable(MULTI_HEADER);
    if (!parsed.ok) throw new Error('parse failed');
    const model = parsed.value;
    model.rows[2][0].text = '- 糖度 13 度以上\n- 重量 300 g 以上\n\nいずれかを満たすこと。';

    const out = serializeGridTable(model);
    expect(out).not.toMatch(/300 g 以上\\/);

    const again = parseGridTable(out);
    if (!again.ok) throw new Error('reparse failed');
    expect(again.value.rows[2][0].text).toBe(model.rows[2][0].text);
  });

  it('行末の `\\\\`（二重）を 1 つの改行として読み、書き戻しで直す', () => {
    // `\\` はエスケープされたバックスラッシュになり、改行が消えて文字として出る
    const src = [
      '+----------------+--------+',
      '| 内容           | 備考   |',
      '+================+========+',
      '| 受注入力画面\\\\ | 二重   |',
      '| あかさたな     |        |',
      '+----------------+--------+'
    ].join('\n');
    const parsed = parseGridTable(src);
    if (!parsed.ok) throw new Error(parsed.message);
    expect(parsed.value.rows[1][0].text).toBe('受注入力画面\nあかさたな');

    const out = serializeGridTable(parsed.value);
    expect(out).not.toContain('\\\\');
    expect(out).toMatch(/受注入力画面\\/);
  });

  it('モデルの行末にバックスラッシュが残っていても重ねない', () => {
    const parsed = parseGridTable(MULTI_HEADER);
    if (!parsed.ok) throw new Error('parse failed');
    const model = parsed.value;
    model.rows[2][0].text = '受注入力画面\\\nあかさたな';

    const out = serializeGridTable(model);
    expect(out).not.toContain('\\\\');
  });

  it('コード span の中の `<br>` を改行に変えない', () => {
    const src = [
      '+----------+--------------------------------+',
      '| 書き方   | セルの中身                     |',
      '+==========+================================+',
      '| `<br>`   | 入荷は毎週火曜。<br>休祝日は翌 |',
      '+----------+--------------------------------+'
    ].join('\n');
    const parsed = parseGridTable(src);
    if (!parsed.ok) throw new Error(parsed.message);
    // 1 列目は文字としての `<br>`、2 列目は改行として解釈される
    expect(parsed.value.rows[1][0].text).toBe('`<br>`');
    expect(parsed.value.rows[1][1].text).toBe('入荷は毎週火曜。\n休祝日は翌');
    expect(serializeGridTable(parsed.value)).toContain('`<br>`');
  });

  it('リストが絡まない空行（段落の区切り）は往復で保たれる', () => {
    const parsed = parseGridTable(MULTI_HEADER);
    if (!parsed.ok) throw new Error('parse failed');
    const model = parsed.value;
    model.rows[2][0].text = '一段落目\n\n二段落目';

    const out = serializeGridTable(model);
    expect(out).not.toMatch(/一段落目\\/);

    const again = parseGridTable(out);
    if (!again.ok) throw new Error('reparse failed');
    expect(again.value.rows[2][0].text).toBe('一段落目\n\n二段落目');
  });
});
