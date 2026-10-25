import type { TableModel, ParseResult } from '../model/TableModel';
import { emptyAttributes } from '../model/TableModel';
import { normalizeTableModel } from '../model/normalizeTableModel';
import { scanFences, findTblBlockAt, enclosingClasses, type LineRange, type DivBlock } from './findTblBlock';
import { splitParts, partAt, partIndexOf } from './splitParts';
import { findPlainTableAt } from './findPlainTable';
import { parsePipeTable, isSeparatorRow } from '../formats/pipeTable/parsePipeTable';
import { parseGridTable, isGridBorderLine } from '../formats/gridTable/parseGridTable';
import { expandMergeCols } from '../formats/mergeCols/expandMergeCols';
import { parseMergeCols } from '../formats/mergeCols/simulateMergeCols';
import {
  toTblAttributes,
  attributeValue,
  parseWidths
} from '../formats/tbl/parseTblAttributes';

export type EditTargetKind = 'tblPart' | 'plainTable' | 'new';

export type EditTarget = {
  kind: EditTargetKind;
  /** 置換する行範囲。'new' のときは挿入位置（start === end + 1 の空範囲）。 */
  range: LineRange;
  model: TableModel;
  /** 分割表のパート数。1 なら通常表。 */
  partCount: number;
  /** 何番目のパートか（0 始まり）。'tblPart' 以外は 0。 */
  partIndex: number;
  /** div 属性を編集できるか（partCount === 1 のときだけ）。 */
  attributesEditable: boolean;
  /** 既存 div の fence コロン数（書き戻し用）。 */
  colons: number;
  /** div の fence 行範囲。'tblPart' のときだけ入る。 */
  divRange?: LineRange;
  /** 外側 div のクラス（`.landscape` など）。表示専用。 */
  enclosingClasses: string[];
  /** 他パートの列数（Apply 時の整合チェック用）。 */
  otherPartColumnCounts: number[];
  warnings: string[];
};

/**
 * カーソル位置から編集対象を決める（requirements.md §2 の優先度 ①②③）。
 *
 *   ① `.tbl` div の内側 → カーソルのあるパートだけ
 *   ② 素のパイプ表／グリッド表 → その表だけ
 *   ③ それ以外 → 新規作成
 */
export function findEditTarget(
  lines: string[],
  cursorLine: number,
  newTableId: string
): ParseResult<EditTarget> {
  const scan = scanFences(lines);
  if (!scan.ok) {
    return { ok: false, message: `${scan.message}（${scan.line + 1} 行目）` };
  }

  const block = findTblBlockAt(scan.blocks, cursorLine);
  if (block) return fromTblBlock(lines, cursorLine, block, scan.blocks);

  const plain = findPlainTableAt(lines, cursorLine);
  if (plain) return fromPlainTable(lines, plain, newTableId);

  return {
    ok: true,
    value: {
      kind: 'new',
      range: { start: cursorLine, end: cursorLine - 1 },
      model: newTableModel(newTableId),
      partCount: 1,
      partIndex: 0,
      attributesEditable: true,
      colons: 3,
      enclosingClasses: [],
      otherPartColumnCounts: [],
      warnings: []
    }
  };
}

function fromTblBlock(
  lines: string[],
  cursorLine: number,
  block: DivBlock,
  allBlocks: DivBlock[]
): ParseResult<EditTarget> {
  const parts = splitParts(lines, block.body);
  if (parts.length === 0) {
    return { ok: false, message: '`.tbl` ブロックの中に表がありません。' };
  }

  const part = partAt(parts, cursorLine);
  if (!part) return { ok: false, message: '編集するパートを特定できませんでした。' };
  const partIndex = partIndexOf(parts, part);

  const source = lines.slice(part.start, part.end + 1).join('\n');
  const parsed = parseTableSource(source, `${block.startLine}`);
  if (!parsed.ok) return parsed;

  const warnings: string[] = [];
  const model = parsed.value;
  model.attributes = toTblAttributes(block.header);

  const widthsAttr = attributeValue(block.header, 'widths');
  const widths = parseWidths(widthsAttr, model.columns.length);
  if (widthsAttr && !widths) {
    warnings.push(
      `widths="${widthsAttr}" の個数が列数（${model.columns.length}）と一致しません。自動幅として扱います。`
    );
  }
  if (widths) {
    model.columns = model.columns.map((col, c) => ({ ...col, width: widths[c] }));
  }

  // merge-cols 付きのパイプ表は、テンプレートと同じアルゴリズムで結合を展開して表示する
  const mergeColsAttr = attributeValue(block.header, 'merge-cols');
  const hasMergeRows = block.header.classes.includes('merge-rows');
  let result = model;
  if ((mergeColsAttr !== undefined || hasMergeRows) && model.outputFormat === 'pipeTable') {
    result = expandMergeCols(model, parseMergeCols(mergeColsAttr));
  }

  const otherPartColumnCounts = parts
    .filter((_, i) => i !== partIndex)
    .map(p => countColumns(lines.slice(p.start, p.end + 1)));

  if (parts.length > 1) {
    warnings.push(
      `分割表の ${partIndex + 1}／${parts.length} パートを編集しています。div 属性は変更できません。`
    );
  }

  return {
    ok: true,
    value: {
      kind: 'tblPart',
      range: part,
      model: result,
      partCount: parts.length,
      partIndex,
      attributesEditable: parts.length === 1,
      colons: block.header.colons,
      divRange: { start: block.startLine, end: block.endLine },
      enclosingClasses: enclosingClasses(allBlocks, block),
      otherPartColumnCounts,
      warnings
    }
  };
}

function fromPlainTable(
  lines: string[],
  range: LineRange,
  newTableId: string
): ParseResult<EditTarget> {
  const source = lines.slice(range.start, range.end + 1).join('\n');
  const parsed = parseTableSource(source, newTableId);
  if (!parsed.ok) return parsed;

  return {
    ok: true,
    value: {
      kind: 'plainTable',
      range,
      model: parsed.value,
      partCount: 1,
      partIndex: 0,
      attributesEditable: true,
      colons: 3,
      enclosingClasses: [],
      otherPartColumnCounts: [],
      warnings: ['`.tbl` で囲まれていない表を取り込みました。Apply すると `::: {.tbl}` で囲みます。']
    }
  };
}

/** パイプ表かグリッド表かを判別して読む。 */
export function parseTableSource(source: string, tableId: string): ParseResult<TableModel> {
  const lines = source.split('\n').filter(l => l.trim() !== '');
  if (lines.length === 0) return { ok: false, message: '表が空です。' };

  if (lines.some(l => isGridBorderLine(l.trim()))) {
    return parseGridTable(source, tableId);
  }
  if (lines.some(l => isSeparatorRow(l))) {
    return parsePipeTable(source, tableId);
  }
  return { ok: false, message: 'パイプ表・グリッド表のどちらとしても読めませんでした。' };
}

/** パートの列数を数える（Apply 時の整合チェック用）。 */
function countColumns(partLines: string[]): number {
  const parsed = parseTableSource(partLines.join('\n'), '');
  return parsed.ok ? parsed.value.columns.length : 0;
}

export function newTableModel(id: string): TableModel {
  const rows = Array.from({ length: 3 }, (_, r) =>
    Array.from({ length: 3 }, (_, c) => ({
      id: `r${r}c${c}`,
      text: r === 0 ? `列${c + 1}` : '',
      row: r,
      col: c,
      rowspan: 1,
      colspan: 1,
      hidden: false
    }))
  );

  return normalizeTableModel({
    id,
    version: 1,
    headerRows: 1,
    columns: [{}, {}, {}],
    rows,
    attributes: emptyAttributes(),
    outputFormat: 'gridTable'
  });
}
