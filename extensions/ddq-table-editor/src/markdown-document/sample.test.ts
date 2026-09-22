import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { findEditTarget } from './findEditTarget';
import { splitLines } from './splitLines';

/**
 * デバッグ用サンプル（sample/tables.qmd）が、意図したとおりに検出されるかを確かめる。
 * F5 で開いたときの体験を壊さないための回帰テストでもある。
 *
 * 分割は拡張本体と同じ splitLines を使う。Windows では git が CRLF で
 * チェックアウトするため、`\n` だけで割ると行末に `\r` が残って結果が変わる。
 */
const source = readFileSync(resolve(__dirname, '../../sample/tables.qmd'), 'utf8');
const lines = splitLines(source);

/** 見出しの行番号（0 始まり）を探す。 */
function headingLine(text: string): number {
  const i = lines.findIndex(l => l.startsWith('## ') && l.includes(text));
  if (i < 0) throw new Error(`見出しが見つかりません: ${text}`);
  return i;
}

/** 見出しの後ろで最初に条件を満たす行。 */
function lineAfter(heading: string, predicate: (l: string) => boolean): number {
  const start = headingLine(heading);
  for (let i = start; i < lines.length; i++) {
    if (predicate(lines[i])) return i;
  }
  throw new Error(`該当行が見つかりません: ${heading}`);
}

function target(line: number) {
  const found = findEditTarget(lines, line, 'id');
  if (!found.ok) throw new Error(found.message);
  return found.value;
}

describe('sample/tables.qmd', () => {
  it('1. 通常の .tbl は属性を編集できる単一表として開く', () => {
    const t = target(lineAfter('1.', l => l.startsWith('| 基本   | 氏名')));
    expect(t.kind).toBe('tblPart');
    expect(t.partCount).toBe(1);
    expect(t.attributesEditable).toBe(true);
    expect(t.model.attributes.caption).toBe('ユーザ属性一覧');
    expect(t.model.attributes.label).toBe('tbl-user');
    expect(t.model.columns.map(c => c.width)).toEqual([20, 20, 60]);
    expect(t.model.outputFormat).toBe('pipeTable');
  });

  it('2. merge-cols 付きは結合を展開して開く', () => {
    const t = target(lineAfter('2.', l => l.startsWith('| 受注   | 登録   | 入力')));
    expect(t.model.outputFormat).toBe('mergeCols');
    // 「受注」が 3 行、「登録」が 2 行結合される
    expect(t.model.rows[1][0].rowspan).toBe(3);
    expect(t.model.rows[1][1].rowspan).toBe(2);
    expect(t.model.rows[4][0].rowspan).toBe(1);
  });

  it('3. merge-cols="2,3" は 1 列目の連番を結合しない', () => {
    const t = target(lineAfter('3.', l => l.startsWith('| 1   |')));
    expect(t.model.rows[1][0].rowspan).toBe(1);
    expect(t.model.rows[1][1].rowspan).toBe(3);
    expect(t.model.columns[0].align).toBe('center');
  });

  it('4. グリッド表はマルチヘッダと結合を読み取る', () => {
    const t = target(lineAfter('4.', l => l.startsWith('| 売上')));
    expect(t.model.outputFormat).toBe('gridTable');
    expect(t.model.headerRows).toBe(2);
    expect(t.model.rows[0][0].rowspan).toBe(2);
    expect(t.model.rows[0][1].colspan).toBe(2);
  });

  it('5. 分割表は 2 パートで、カーソルのあるパートだけを対象にする', () => {
    const first = target(lineAfter('5.', l => l.startsWith('| 氏名')));
    expect(first.partCount).toBe(2);
    expect(first.partIndex).toBe(0);
    expect(first.attributesEditable).toBe(false);

    const second = target(lineAfter('5.', l => l.startsWith('| 住所')));
    expect(second.partIndex).toBe(1);
    expect(second.range.start).toBeGreaterThan(first.range.end);
    expect(second.otherPartColumnCounts).toEqual([3]);
  });

  it('6. .landscape の入れ子は内側の .tbl だけを対象にし、外側を保持情報として返す', () => {
    const t = target(lineAfter('6.', l => l.startsWith('| P-01')));
    expect(t.kind).toBe('tblPart');
    expect(t.enclosingClasses).toContain('landscape');
    // 置換範囲は内側 .tbl の本文だけ
    expect(lines[t.range.start - 1]).toContain('{.tbl');
    expect(lines[t.range.end + 1].trim()).toBe(':::');
  });

  it('7. .tbl の外の素の表は取り込み対象になる', () => {
    const t = target(lineAfter('7.', l => l.startsWith('| りんご')));
    expect(t.kind).toBe('plainTable');
    expect(t.model.columns.map(c => c.align)).toEqual(['left', 'center', 'right']);
  });

  it('8. 表が無い段落では新規作成になる', () => {
    const t = target(headingLine('8.') + 2);
    expect(t.kind).toBe('new');
  });

  it('9. セル内改行（行末バックスラッシュ）を読み取る', () => {
    const t = target(lineAfter('9.', l => l.startsWith('| 住所')));
    expect(t.model.rows[1][1].text).toBe('東京都千代田区\n1-2-3');
  });

  it('ファイル全体の fence が壊れていない', () => {
    const found = findEditTarget(lines, 0, 'id');
    expect(found.ok).toBe(true);
  });
});
