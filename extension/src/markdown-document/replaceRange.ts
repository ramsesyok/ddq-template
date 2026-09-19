import type { LineRange } from './findTblBlock';

/**
 * 行範囲の置換。編集対象の行だけを差し替え、前後の内容は一切変更しない。
 *
 * `range.start > range.end` は空範囲＝挿入を意味する（新規作成時）。
 */
export function replaceLines(
  lines: string[],
  range: LineRange,
  replacement: string
): string[] {
  const newLines = replacement.split('\n');
  const before = lines.slice(0, range.start);
  const after = lines.slice(range.end + 1);
  return [...before, ...newLines, ...after];
}

/** 新規挿入時に、前後へ必要なぶんだけ空行を足す。 */
export function withSurroundingBlankLines(
  lines: string[],
  at: number,
  block: string
): { range: LineRange; text: string } {
  const needsBefore = at > 0 && (lines[at - 1] ?? '').trim() !== '';
  const needsAfter = at < lines.length && (lines[at] ?? '').trim() !== '';
  const text = `${needsBefore ? '\n' : ''}${block}${needsAfter ? '\n' : ''}`;
  return { range: { start: at, end: at - 1 }, text };
}
