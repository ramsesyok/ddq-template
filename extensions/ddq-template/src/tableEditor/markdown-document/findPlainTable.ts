import type { LineRange } from './findTblBlock';
import { isPipeTableLine, isSeparatorRow } from '../formats/pipeTable/parsePipeTable';
import { isGridTableLine } from '../formats/gridTable/parseGridTable';

/**
 * `.tbl` div の外にある素の表（パイプ表／グリッド表）をカーソル位置から探す。
 *
 * 検出優先度②。見つかった表は Apply 時に `::: {.tbl}` で囲んで書き戻す。
 */
export function findPlainTableAt(lines: string[], line: number): LineRange | undefined {
  if (!isTableLine(lines[line] ?? '')) return undefined;

  let start = line;
  while (start > 0 && isTableLine(lines[start - 1])) start--;
  let end = line;
  while (end < lines.length - 1 && isTableLine(lines[end + 1])) end++;

  const block = lines.slice(start, end + 1);
  if (!looksLikeTable(block)) return undefined;

  return { start, end };
}

function isTableLine(line: string): boolean {
  const s = (line ?? '').trim();
  if (s === '') return false;
  return isPipeTableLine(s) || isGridTableLine(s);
}

/** 表として最低限成立しているか（パイプ表は区切り行が要る）。 */
function looksLikeTable(block: string[]): boolean {
  if (block.length < 2) return false;
  if (block.some(l => isGridTableLine(l.trim()) && l.trim().startsWith('+'))) return true;
  return block.some(l => isSeparatorRow(l));
}
