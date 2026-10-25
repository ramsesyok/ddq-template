import type {
  TableModel,
  TableCell,
  TableColumn,
  ParseResult
} from '../../model/TableModel';
import { emptyAttributes, makeCell } from '../../model/TableModel';
import { normalizeTableModel } from '../../model/normalizeTableModel';
import { gridSeparatorToAlign } from '../pipeTable/pipeTableAlignment';
import { gridCellTextFromLines } from '../cellText';
import {
  toDisplayLine,
  hasCharAt,
  sliceByDisplay,
  displayColsOf,
  type DisplayLine
} from './displayLine';

/** セル内の全行に共通する字下げを外す（空行は数えない）。 */
function dedent(lines: string[]): string[] {
  const indents = lines
    .filter(l => l.trim() !== '')
    .map(l => l.length - l.trimStart().length);
  if (indents.length === 0) return lines.map(() => '');
  const common = Math.min(...indents);
  return lines.map(l => (l.trim() === '' ? '' : l.slice(common)));
}

/** グリッド表の罫線（`+---+===+`）か。 */
export function isGridBorderLine(line: string): boolean {
  const s = line.trim();
  return /^\+[-=+:\s]*\+$/.test(s) && (s.match(/\+/g)?.length ?? 0) >= 2;
}

/** グリッド表の内容行（`| … |`）か。 */
export function isGridContentLine(line: string): boolean {
  return line.trim().startsWith('|');
}

/** その行がグリッド表を構成しうる行か。 */
export function isGridTableLine(line: string): boolean {
  return isGridBorderLine(line) || isGridContentLine(line);
}

/**
 * グリッド表を TableModel へ。
 *
 * - 列境界は罫線の `+` の**表示桁**の集合から決める
 * - 横結合は、内容行で境界に `|` が無いこと
 * - 縦結合は、行の境界線でその区間が空白であること
 * - ヘッダ終端は `=` を含む罫線。それより上の行グループがヘッダ
 * - 列揃えはヘッダ終端行（無ければ最上段の罫線）のコロン
 */
export function parseGridTable(source: string, tableId = ''): ParseResult<TableModel> {
  const rawLines = source
    .replace(/\r\n?/g, '\n')
    .split('\n')
    .map(l => l.replace(/\s+$/, ''))
    .filter(l => l.trim() !== '');

  if (rawLines.length < 3) {
    return { ok: false, message: 'グリッド表として読めません（罫線と内容行が必要です）。' };
  }

  const indent = Math.min(...rawLines.map(l => l.length - l.trimStart().length));
  const lines = rawLines.map(l => toDisplayLine(l.slice(indent)));

  if (!lines.every(l => isGridTableLine(l.raw))) {
    return { ok: false, message: 'グリッド表として読めません（罫線・内容行以外が含まれています）。' };
  }
  if (!isGridBorderLine(lines[0].raw)) {
    return { ok: false, message: 'グリッド表として読めません（1 行目が罫線ではありません）。' };
  }

  // 列境界＝すべての罫線に現れる `+` の表示桁の和集合
  const borderIdx = lines.map((l, i) => (isGridBorderLine(l.raw) ? i : -1)).filter(i => i >= 0);
  const boundarySet = new Set<number>();
  for (const i of borderIdx) for (const col of displayColsOf(lines[i], '+')) boundarySet.add(col);
  const boundaries = [...boundarySet].sort((a, b) => a - b);

  if (boundaries.length < 2) {
    return { ok: false, message: 'グリッド表として読めません（列の境界が見つかりません）。' };
  }
  const colLen = boundaries.length - 1;

  // 内容行を罫線で区切ってグループ（＝1 行）にする
  type Group = { lines: DisplayLine[]; borderAfter: DisplayLine | undefined };
  const groups: Group[] = [];
  let current: DisplayLine[] = [];
  const borderAfterGroup: DisplayLine[] = [];
  const headerEndAfter: number[] = [];

  for (const line of lines) {
    if (isGridBorderLine(line.raw)) {
      if (current.length > 0) {
        groups.push({ lines: current, borderAfter: line });
        borderAfterGroup.push(line);
        if (line.raw.includes('=')) headerEndAfter.push(groups.length - 1);
        current = [];
      }
    } else {
      current.push(line);
    }
  }
  if (current.length > 0) groups.push({ lines: current, borderAfter: undefined });

  if (groups.length === 0) {
    return { ok: false, message: 'グリッド表として読めません（内容行がありません）。' };
  }

  const headerRows = headerEndAfter.length > 0 ? headerEndAfter[0] + 1 : 0;

  // 列揃え: ヘッダ終端の `+===+` 行。無ければ最上段の罫線。
  const alignLine =
    headerEndAfter.length > 0 ? borderAfterGroup[headerEndAfter[0]] : lines[borderIdx[0]];
  const columns: TableColumn[] = Array.from({ length: colLen }, (_, c) => {
    const segment = sliceByDisplay(alignLine, boundaries[c] + 1, boundaries[c + 1]);
    const align = gridSeparatorToAlign(segment);
    return align ? { align } : {};
  });

  // 境界に `|` があるか（無ければ横結合）／罫線区間が空白か（＝縦結合）
  const boundaryClosed: boolean[][] = groups.map(g =>
    Array.from({ length: boundaries.length }, (_, b) =>
      b === 0 || b === boundaries.length - 1
        ? true
        : g.lines.some(l => hasCharAt(l, boundaries[b], '|'))
    )
  );
  const segmentBlank: boolean[][] = groups.map((g, gi) => {
    const border = g.borderAfter;
    return Array.from({ length: colLen }, (_, c) => {
      if (!border || gi === groups.length - 1) return false;
      const segment = sliceByDisplay(border, boundaries[c] + 1, boundaries[c + 1]);
      return segment.trim() === '';
    });
  });

  // セルの組み立て
  const rows: TableCell[][] = groups.map((_, r) =>
    Array.from({ length: colLen }, (_, c) => makeCell(r, c))
  );
  const covered: boolean[][] = groups.map(() => Array.from({ length: colLen }, () => false));

  for (let r = 0; r < groups.length; r++) {
    for (let c = 0; c < colLen; c++) {
      if (covered[r][c]) continue;

      let colspan = 1;
      while (c + colspan < colLen && !boundaryClosed[r][c + colspan]) colspan++;

      let rowspan = 1;
      while (r + rowspan < groups.length) {
        const allBlank = Array.from({ length: colspan }, (_, i) =>
          segmentBlank[r + rowspan - 1][c + i]
        ).every(Boolean);
        if (!allBlank) break;
        rowspan++;
      }

      const texts: string[] = [];
      for (let rr = r; rr < r + rowspan; rr++) {
        for (const line of groups[rr].lines) {
          texts.push(sliceByDisplay(line, boundaries[c] + 1, boundaries[c + colspan]));
        }
      }
      // 行頭の字下げは入れ子リスト・項目内の折り返しに意味があるので、
      // セル共通の字下げ（罫線の内側に置く空白）だけを外して相対関係を残す。
      const cellLines = dedent(texts.map(t => t.replace(/\s+$/, '')));
      // 内側の空行は段落の区切りなので残す。前後の空行はセルを縦に埋めるための
      // 余白（他の列のほうが背が高いだけ）なので落とす。
      while (cellLines.length > 0 && cellLines[0] === '') cellLines.shift();
      while (cellLines.length > 0 && cellLines[cellLines.length - 1] === '') cellLines.pop();
      const text = gridCellTextFromLines(cellLines);

      const cell = rows[r][c];
      cell.text = text;
      cell.rowspan = rowspan;
      cell.colspan = colspan;

      for (let rr = r; rr < r + rowspan; rr++) {
        for (let cc = c; cc < c + colspan; cc++) {
          if (rr === r && cc === c) continue;
          covered[rr][cc] = true;
          rows[rr][cc].hidden = true;
        }
      }
    }
  }

  const model: TableModel = {
    id: tableId,
    version: 1,
    headerRows,
    columns,
    rows,
    attributes: emptyAttributes(),
    outputFormat: 'gridTable'
  };

  return { ok: true, value: normalizeTableModel(model) };
}
