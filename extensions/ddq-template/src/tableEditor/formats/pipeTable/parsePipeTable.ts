import type {
  TableModel,
  TableCell,
  TableColumn,
  ParseResult
} from '../../model/TableModel';
import { emptyAttributes } from '../../model/TableModel';
import { separatorToAlign } from './pipeTableAlignment';
import { cellTextFromMarkdown, unescapePipeCell } from '../cellText';

/** 行がパイプ表の区切り行（`|---|:--:|`）か。 */
export function isSeparatorRow(line: string): boolean {
  const s = line.trim();
  if (!s.startsWith('|')) return false;
  const cells = splitPipeCells(s);
  return cells.length > 0 && cells.every(c => /^:?-{1,}:?$/.test(c.trim()));
}

/** 行がパイプ表の行か（`|` で始まる）。 */
export function isPipeTableLine(line: string): boolean {
  return line.trim().startsWith('|');
}

/**
 * パイプ表を TableModel へ。
 *
 * パイプ表はヘッダ 1 行・結合なししか表現できないため、常に headerRows=1・
 * 全セル rowspan=colspan=1 のモデルになる。
 */
export function parsePipeTable(source: string, tableId = ''): ParseResult<TableModel> {
  const lines = source
    .replace(/\r\n?/g, '\n')
    .split('\n')
    .map(l => l.trim())
    .filter(l => l.startsWith('|'));

  if (lines.length < 2) {
    return { ok: false, message: 'パイプ表として読めません（ヘッダ行と区切り行が必要です）。' };
  }
  if (!isSeparatorRow(lines[1])) {
    return { ok: false, message: 'パイプ表として読めません（2 行目が区切り行ではありません）。' };
  }

  const headerCells = splitPipeCells(lines[0]);
  const separatorCells = splitPipeCells(lines[1]);
  const colLen = Math.max(headerCells.length, separatorCells.length);

  const columns: TableColumn[] = Array.from({ length: colLen }, (_, c) => {
    const align = separatorToAlign(separatorCells[c] ?? '');
    return align ? { align } : {};
  });

  const rows: TableCell[][] = [];
  const pushRow = (cells: string[]) => {
    const r = rows.length;
    rows.push(
      Array.from({ length: colLen }, (_, c) => ({
        id: `r${r}c${c}`,
        text: cellTextFromMarkdown(unescapePipeCell(cells[c] ?? '')),
        row: r,
        col: c,
        rowspan: 1,
        colspan: 1,
        hidden: false
      }))
    );
  };

  pushRow(headerCells);
  for (let i = 2; i < lines.length; i++) pushRow(splitPipeCells(lines[i]));

  return {
    ok: true,
    value: {
      id: tableId,
      version: 1,
      headerRows: 1,
      columns,
      rows,
      attributes: emptyAttributes(),
      outputFormat: 'pipeTable'
    }
  };
}

/** `| a | b |` → ['a', 'b']。`\|` はセル内容として扱う。 */
export function splitPipeCells(line: string): string[] {
  let s = line.trim();
  if (s.startsWith('|')) s = s.slice(1);
  if (s.endsWith('|') && !s.endsWith('\\|')) s = s.slice(0, -1);

  const parts: string[] = [];
  let current = '';
  for (let i = 0; i < s.length; i++) {
    if (s[i] === '|' && s[i - 1] !== '\\') {
      parts.push(current.trim());
      current = '';
    } else {
      current += s[i];
    }
  }
  parts.push(current.trim());
  return parts;
}
