import type { TableModel } from '../../model/TableModel';
import { hasLineBreaks, hasMerges } from '../../model/TableModel';
import { serializePipeTable } from '../pipeTable/serializePipeTable';
import { serializeGridTable } from '../gridTable/serializeGridTable';
import { canUseMergeCols } from '../mergeCols/canUseMergeCols';
import { serializeMergeCols } from '../mergeCols/simulateMergeCols';

export type SerializeOptions = {
  /** fence のコロン数。既存ブロックを書き戻すときは元の数を渡す。 */
  colons?: number;
  /** 分割表のパート数。1 なら通常表。 */
  partCount?: number;
};

/**
 * 表本体（div の中身）だけを出力する。パート単位の置換で使う。
 *
 * セル内改行があるときは必ずグリッド表にする。パイプ表では改行が `<br>` になり、
 * 箇条書き・番号付きリストのようなブロックをセルに置けないため。
 */
export function serializeTableBody(model: TableModel, partCount = 1): string {
  const merged = hasMerges(model);
  const multiline = hasLineBreaks(model);

  if (!merged && !multiline && model.headerRows === 1) {
    return serializePipeTable(model);
  }
  if (model.outputFormat === 'mergeCols' && canUseMergeCols(model, partCount).ok) {
    // merge-cols は「同じ値が縦に並んだ素の表」に戻して出力する。
    // 素のグリッド表でも design-doc.lua は結合してくれる（pandoc で確認済み）。
    const flat = flattenMerges(model);
    return multiline ? serializeGridTable(flat) : serializePipeTable(flat);
  }
  return serializeGridTable(model);
}

/** `::: {.tbl …}` ブロック全体を出力する（新規作成・M=1 の書き戻しで使う）。 */
export function serializeTblBlock(
  model: TableModel,
  options: SerializeOptions = {}
): string {
  const colons = ':'.repeat(Math.max(3, options.colons ?? 3));
  const partCount = options.partCount ?? 1;
  const body = serializeTableBody(model, partCount);
  return `${colons} ${buildFenceHeader(model, partCount)}\n${body}\n${colons}`;
}

export function buildFenceHeader(model: TableModel, partCount = 1): string {
  const parts: string[] = ['.tbl'];
  if (model.attributes.unnumbered) parts.push('.unnumbered');
  // クラス名として成立しないものは fence を壊すので落とす
  for (const cls of model.attributes.extraClasses) {
    if (/^[\w-]+$/.test(cls)) parts.push(`.${cls}`);
  }

  if (model.attributes.caption) parts.push(`caption="${escapeAttr(model.attributes.caption)}"`);
  if (model.attributes.label && !model.attributes.unnumbered) {
    parts.push(`label="${escapeAttr(model.attributes.label)}"`);
  }

  const widths = model.columns.map(c => c.width);
  if (widths.every(w => typeof w === 'number' && Number.isFinite(w))) {
    parts.push(`widths="${widths.join(',')}"`);
  }

  if (model.outputFormat === 'mergeCols') {
    const check = canUseMergeCols(model, partCount);
    if (check.ok) parts.push(`merge-cols="${serializeMergeCols(check.cols)}"`);
  }

  for (const [key, value] of model.attributes.extraAttributes) {
    parts.push(`${key}="${escapeAttr(value)}"`);
  }

  return `{${parts.join(' ')}}`;
}

/** 結合を解いて「同じ値が縦に並んだ」素の表へ戻す（merge-cols 出力用）。 */
function flattenMerges(model: TableModel): TableModel {
  const rows = model.rows.map(row => row.map(cell => ({ ...cell })));
  for (let r = 0; r < rows.length; r++) {
    for (let c = 0; c < rows[r].length; c++) {
      const cell = rows[r][c];
      if (cell.hidden || cell.rowspan <= 1) continue;
      for (let rr = r + 1; rr < r + cell.rowspan; rr++) {
        rows[rr][c].text = cell.text;
        rows[rr][c].hidden = false;
        rows[rr][c].rowspan = 1;
        rows[rr][c].colspan = 1;
      }
      cell.rowspan = 1;
    }
  }
  return { ...model, rows };
}

/**
 * 属性値のエスケープ。
 *
 * バックスラッシュを先に処理しないと、末尾が `\` のキャプションで閉じ引用符が
 * エスケープされ、`.tbl` ブロックごと壊れる（pandoc で確認済み）。
 * 改行・タブが混ざっても fence が壊れるので空白へ潰す。
 */
function escapeAttr(value: string): string {
  return value
    .replace(/[\r\n\t]+/g, ' ')
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"');
}
