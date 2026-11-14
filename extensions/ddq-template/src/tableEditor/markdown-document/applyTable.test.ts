import { describe, it, expect } from 'vitest';
import { findEditTarget } from './findEditTarget';
import { buildReplacement, anchorLineOf } from './applyTable';
import { replaceLines } from './replaceRange';
import { mergeCells } from '../model/mergeCells';
import type { TableModel } from '../model/TableModel';

/** 編集対象を取り直さずに 2 回 Apply する（旧実装の挙動）。 */
function applyTwiceStale(lines: string[], cursor: number, edit: (m: TableModel) => TableModel) {
  const found = findEditTarget(lines, cursor, 'id');
  if (!found.ok) throw new Error(found.message);
  const target = found.value;

  const first = buildReplacement(lines, target, edit(target.model));
  let out = replaceLines(lines, first.range, first.text);

  // 同じ target を使い回す＝行番号がずれたまま 2 回目を書き込む
  const second = buildReplacement(out, target, edit(target.model));
  out = replaceLines(out, second.range, second.text);
  return out;
}

/** 書き戻すたびに編集対象を取り直して 2 回 Apply する（現行実装の挙動）。 */
function applyTwiceResolved(lines: string[], cursor: number, edit: (m: TableModel) => TableModel) {
  let out = lines;
  let cursorLine = cursor;

  for (let i = 0; i < 2; i++) {
    const found = findEditTarget(out, cursorLine, 'id');
    if (!found.ok) throw new Error(found.message);
    const replacement = buildReplacement(out, found.value, edit(found.value.model));
    out = replaceLines(out, replacement.range, replacement.text);
    cursorLine = anchorLineOf(replacement);
  }
  return out;
}

const SRC = [
  '前書き',
  '',
  '::: {.tbl caption="機能分類"}',
  '| 大分類 | 中分類 |',
  '| ------ | ------ |',
  '| 受注   | 登録   |',
  '| 受注   | 引当   |',
  ':::',
  '',
  '後書き'
];

/** パイプ表 → グリッド表になるので行数が増える編集。 */
const mergeFirstColumn = (m: TableModel): TableModel => {
  const r = mergeCells(m, { startRow: 1, startCol: 0, endRow: 2, endCol: 0 });
  if (!r.ok) throw new Error(r.message);
  return r.value;
};

describe('Apply を続けて実行したとき', () => {
  it('編集対象を取り直さないとドキュメントが壊れる（回帰の基準）', () => {
    const out = applyTwiceStale(SRC, 5, mergeFirstColumn).join('\n');
    // fence の数が合わなくなる／本文が二重になるなど、元の構造が保てない
    const fences = out.split('\n').filter(l => l.trim() === ':::').length;
    expect(fences).not.toBe(1);
  });

  it('毎回取り直せば、2 回目以降も正しい範囲に書き戻せる', () => {
    const out = applyTwiceResolved(SRC, 5, mergeFirstColumn);
    const text = out.join('\n');

    expect(out[0]).toBe('前書き');
    expect(out[out.length - 1]).toBe('後書き');
    expect(out.filter(l => l.trim() === ':::').length).toBe(1);
    expect(text).toContain('caption="機能分類"');
    expect(text.split('大分類').length - 1).toBe(1);

    // さらに編集せず Apply しても内容が変わらない（べき等）
    const idempotent = applyTwiceResolved(out, 4, m => m);
    expect(idempotent).toEqual(out);
  });

  it('新規作成でも 2 回目に重複挿入されない', () => {
    const src = ['本文だけ', ''];
    const out = applyTwiceResolved(src, 0, m => m);
    expect(out.filter(l => l.trim() === ':::').length).toBe(1);
    expect(out.filter(l => l.includes('{.tbl')).length).toBe(1);
  });

  it('分割表のパートでも 2 回目が正しい範囲に入る', () => {
    const split = [
      '::: {.tbl caption="一覧"}',
      '| a    | b |',
      '| ---- | - |',
      '| 受注 | 1 |',
      '| 受注 | 2 |',
      '',
      '| a    | b |',
      '| ---- | - |',
      '| 出荷 | 3 |',
      '| 出荷 | 4 |',
      ':::'
    ];
    const out = applyTwiceResolved(split, 8, mergeFirstColumn);
    expect(out[0]).toBe('::: {.tbl caption="一覧"}');
    expect(out.filter(l => l.trim() === ':::').length).toBe(1);
    // 1 番目のパートは触られていない
    expect(out.slice(1, 5)).toEqual(split.slice(1, 5));
    // 2 番目のパートだけがグリッド表になっている
    expect(out.filter(l => l.startsWith('+')).length).toBeGreaterThan(0);
  });
});

describe('anchorLineOf', () => {
  it('挿入時に前置きの空行を飛ばして fence 行を指す', () => {
    const src = ['本文', ''];
    const found = findEditTarget(src, 0, 'id');
    if (!found.ok) throw new Error(found.message);
    const replacement = buildReplacement(src, found.value, found.value.model);
    const out = replaceLines(src, replacement.range, replacement.text);
    expect(out[anchorLineOf(replacement)]).toContain('{.tbl');
  });
});
