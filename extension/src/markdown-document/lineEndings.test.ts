import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { splitLines } from './splitLines';
import { findEditTarget } from './findEditTarget';
import { buildReplacement } from './applyTable';
import { replaceLines } from './replaceRange';
import { parseFenceHeader } from '../formats/tbl/parseTblAttributes';

/**
 * 改行コードの違いで挙動が変わらないことを確かめる。
 *
 * Windows では git が CRLF でチェックアウトするため、行末に `\r` が残ったまま
 * 走査すると fence を取りこぼし、`.tbl` の検出が丸ごと失敗していた
 * （`.` は `\r` に一致せず、`$` も真の終端でしか一致しないため）。
 * チェックアウトのされ方に依存しないよう、ここでは CRLF をその場で作って検査する。
 */

const LF_DOC = [
  '前書き',
  '',
  ':::: {.landscape}',
  '::: {.tbl caption="一覧" label="tbl-x" widths="30,70"}',
  '| 属性 | 説明 |',
  '| ---- | ---- |',
  '| 氏名 | 名前 |',
  ':::',
  '::::',
  '',
  '後書き'
].join('\n');

describe('splitLines', () => {
  it('LF / CRLF / CR のどれでも同じ行に分かれる', () => {
    const expected = ['a', 'b', 'c'];
    expect(splitLines('a\nb\nc')).toEqual(expected);
    expect(splitLines('a\r\nb\r\nc')).toEqual(expected);
    expect(splitLines('a\rb\rc')).toEqual(expected);
  });

  it('行末に \\r を残さない', () => {
    expect(splitLines('x\r\ny').every(l => !l.includes('\r'))).toBe(true);
  });
});

describe('parseFenceHeader', () => {
  it('行末に \\r が付いていても fence として読める', () => {
    const header = parseFenceHeader('::: {.tbl caption="x"}\r');
    expect(header).toBeDefined();
    expect(header?.classes).toContain('tbl');
    expect(header?.attributes).toEqual([['caption', 'x']]);
  });

  it('行末に余分な空白があっても読める', () => {
    expect(parseFenceHeader('::: {.tbl}   ')?.classes).toContain('tbl');
  });
});

describe('CRLF のドキュメント', () => {
  const crlf = splitLines(LF_DOC.replace(/\n/g, '\r\n'));
  const lf = splitLines(LF_DOC);

  it('LF と同じ編集対象が得られる', () => {
    const a = findEditTarget(lf, 6, 'id');
    const b = findEditTarget(crlf, 6, 'id');
    expect(a.ok).toBe(true);
    expect(b.ok).toBe(true);
    if (!a.ok || !b.ok) return;

    expect(b.value.kind).toBe(a.value.kind);
    expect(b.value.range).toEqual(a.value.range);
    expect(b.value.model.attributes).toEqual(a.value.model.attributes);
    expect(b.value.enclosingClasses).toEqual(a.value.enclosingClasses);
  });

  it('.tbl として検出され、fence 不整合のエラーにならない', () => {
    const found = findEditTarget(crlf, 6, 'id');
    expect(found.ok).toBe(true);
    if (!found.ok) return;
    expect(found.value.kind).toBe('tblPart');
    expect(found.value.model.attributes.caption).toBe('一覧');
  });

  it('書き戻しても外側 div と前後の本文が保たれる', () => {
    const found = findEditTarget(crlf, 6, 'id');
    if (!found.ok) throw new Error(found.message);
    const replacement = buildReplacement(crlf, found.value, found.value.model);
    const out = replaceLines(crlf, replacement.range, replacement.text);

    expect(out[0]).toBe('前書き');
    expect(out[2]).toBe(':::: {.landscape}');
    expect(out[out.length - 1]).toBe('後書き');
    expect(out.some(l => l.includes('\r'))).toBe(false);
  });
});

describe('sample/tables.qmd', () => {
  it('CRLF でチェックアウトされていても全 fence を走査できる', () => {
    const text = readFileSync(resolve(__dirname, '../../sample/tables.qmd'), 'utf8');
    const asCrlf = splitLines(text.replace(/\r\n/g, '\n').replace(/\n/g, '\r\n'));
    const found = findEditTarget(asCrlf, 0, 'id');
    expect(found.ok).toBe(true);
  });
});
