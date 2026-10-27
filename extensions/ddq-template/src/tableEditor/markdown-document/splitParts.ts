import type { LineRange } from './findTblBlock';

/**
 * `.tbl` div の中身を、空行区切りの「パート」へ分割する。
 *
 * AUTHORING.md §6 のとおり、表を 1 つ書けば通常表、空行区切りで複数書けば分割表になる。
 * 本拡張はカーソルのあるパートだけを編集対象にするので、パートの行範囲が必要になる。
 */
export function splitParts(lines: string[], body: LineRange): LineRange[] {
  const parts: LineRange[] = [];
  let start = -1;

  for (let i = body.start; i <= body.end; i++) {
    const blank = (lines[i] ?? '').trim() === '';
    if (blank) {
      if (start >= 0) {
        parts.push({ start, end: i - 1 });
        start = -1;
      }
    } else if (start < 0) {
      start = i;
    }
  }
  if (start >= 0) parts.push({ start, end: body.end });

  return parts;
}

/** カーソル行を含むパート。行が空行なら直前／直後の近いパートを選ぶ。 */
export function partAt(parts: LineRange[], line: number): LineRange | undefined {
  if (parts.length === 0) return undefined;
  const hit = parts.find(p => p.start <= line && line <= p.end);
  if (hit) return hit;

  // 空行や fence 行にカーソルがある場合は最も近いパートを選ぶ
  let best = parts[0];
  let bestDistance = Number.POSITIVE_INFINITY;
  for (const p of parts) {
    const distance = line < p.start ? p.start - line : line - p.end;
    if (distance < bestDistance) {
      best = p;
      bestDistance = distance;
    }
  }
  return best;
}

export function partIndexOf(parts: LineRange[], part: LineRange): number {
  return parts.findIndex(p => p.start === part.start && p.end === part.end);
}
