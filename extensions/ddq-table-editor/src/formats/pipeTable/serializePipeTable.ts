import type { TableModel } from '../../model/TableModel';
import { alignToSeparator } from './pipeTableAlignment';
import { escapePipeCell } from '../cellText';
import { displayWidth, padToWidth } from '../displayWidth';

/**
 * パイプ表を出力する。
 *
 * 呼び出し側は「結合が無く、ヘッダがちょうど 1 行」であることを確認してから使うこと
 * （パイプ表はそれ以外を表現できない）。列幅は表示幅で揃える。
 */
export function serializePipeTable(model: TableModel): string {
  const colLen = model.columns.length;
  const texts = model.rows.map(row =>
    Array.from({ length: colLen }, (_, c) => escapePipeCell(row[c]?.text ?? ''))
  );

  const widths = Array.from({ length: colLen }, (_, c) => {
    const contentMax = texts.reduce((max, row) => Math.max(max, displayWidth(row[c])), 0);
    return Math.max(contentMax, 3);
  });

  const headerRows = Math.min(Math.max(model.headerRows, 1), model.rows.length);
  const lines: string[] = [];

  // ヘッダは 1 行だけ（それ以上は呼び出し側でグリッド表へ振り分ける）
  lines.push(renderRow(texts[0] ?? [], widths));
  lines.push(
    '| ' +
      model.columns
        .map((col, c) => alignToSeparator(col.align, widths[c]))
        .join(' | ') +
      ' |'
  );
  for (let r = headerRows; r < texts.length; r++) {
    lines.push(renderRow(texts[r], widths));
  }

  return lines.join('\n');
}

function renderRow(cells: string[], widths: number[]): string {
  return '| ' + widths.map((w, c) => padToWidth(cells[c] ?? '', w)).join(' | ') + ' |';
}
