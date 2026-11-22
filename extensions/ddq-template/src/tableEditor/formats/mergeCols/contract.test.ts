/**
 * `.tbl` の merge-cols の結合を、テンプレート（design-doc.lua の merge_body）と**同じ期待値**で
 * 確かめる契約試験（GEN-02）。期待値は cli/tests/fixtures/contract/merge-cols.json で、Lua 側は
 * cli/tests/filter_contract.rs が本物のフィルタに同じ原稿を通して確かめる。
 *
 * 表エディタが原稿を開いたときの結合の展開（findEditTarget → expandMergeCols）を通す。
 * `warns` の例は、Lua と同じく解釈できない値として警告が出ることも見る。
 * `tsDiffers` は、表エディタの表示が文書の出力と食い違う既知の差
 * （docs/ddq-template-impl の表エディタ 7 章「merge-cols の本文比較」）。
 */

import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import { describe, expect, it } from 'vitest';

import { findEditTarget } from '../../markdown-document/findEditTarget';

type Part = { header: string[]; rows: string[][]; spans: number[][] };
type Case = {
  name: string;
  doc: string;
  parts: Part[];
  warns?: boolean;
  tsDiffers?: { spans: number[][][]; why: string };
};

const file = join(__dirname, '..', '..', '..', '..', '..', '..', 'cli', 'tests', 'fixtures', 'contract', 'merge-cols.json');
const { cases } = JSON.parse(readFileSync(file, 'utf8')) as { cases: Case[] };

/** 各パートの表の 1 行目（カーソルを置く行） */
function partStarts(lines: string[]): number[] {
  return lines.flatMap((line, i) => (line.startsWith('|') && !lines[i - 1]?.startsWith('|') ? [i] : []));
}

describe('merge-cols（design-doc.lua と同じ期待値）', () => {
  it.each(cases.map((c) => [c.name, c] as const))('%s', (_name, c) => {
    const lines = c.doc.split('\n');
    const starts = partStarts(lines);
    expect(starts).toHaveLength(c.parts.length);
    const got = starts.map((line) => {
      const target = findEditTarget(lines, line, 'tbl-new');
      if (!target.ok) throw new Error(target.message);
      expect(target.value.kind).toBe('tblPart');
      const warned = target.value.warnings.some((w) => w.includes('merge-cols'));
      expect(warned, 'merge-cols の警告').toBe(c.warns === true);
      // 1 行目は見出し
      return target.value.model.rows.slice(1).map((row) => row.map((cell) => (cell.hidden ? 0 : cell.rowspan)));
    });
    const want = c.tsDiffers?.spans ?? c.parts.map((p) => p.spans);
    expect(got).toEqual(want);
    if (c.tsDiffers) {
      // 差が無くなったら（どちらかを直したら）期待値の tsDiffers を外す
      expect(got).not.toEqual(c.parts.map((p) => p.spans));
    }
  });
});
