import type { TableModel } from '../model/TableModel';
import type { EditTarget } from './findEditTarget';
import type { LineRange } from './findTblBlock';
import { serializeTableBody, serializeTblBlock } from '../formats/tbl/serializeTblBlock';
import { withSurroundingBlankLines } from './replaceRange';

export type Replacement = {
  /** 置換する行範囲。start > end は空範囲＝挿入。 */
  range: LineRange;
  text: string;
};

/**
 * 編集対象とモデルから、ドキュメントへ書き戻す行範囲とテキストを決める。
 *
 * 置換範囲は必要最小限にする:
 * - 分割表（M>1）… カーソルのあるパートの行だけ。div 行・他パートには触れない
 * - 単一表の `.tbl` … 属性も編集できるので div ごと
 * - 素の表 … その表の行を `.tbl` で包む
 * - 新規 … カーソル位置へ挿入（前後に必要なぶんだけ空行を足す）
 */
export function buildReplacement(
  lines: string[],
  target: EditTarget,
  model: TableModel
): Replacement {
  if (target.kind === 'new') {
    const inserted = withSurroundingBlankLines(
      lines,
      target.range.start,
      serializeTblBlock(model, { colons: target.colons })
    );
    return { range: inserted.range, text: inserted.text };
  }

  if (target.kind === 'plainTable') {
    return { range: target.range, text: serializeTblBlock(model, { colons: target.colons }) };
  }

  if (target.partCount === 1 && target.divRange) {
    return {
      range: target.divRange,
      text: serializeTblBlock(model, { colons: target.colons, partCount: 1 })
    };
  }

  return { range: target.range, text: serializeTableBody(model, target.partCount) };
}

/**
 * 書き戻した直後に、再解決の起点となる行を求める。
 *
 * Apply を続けて押せるようにするには、書き戻すたびに編集対象を今のドキュメントから
 * 取り直す必要がある（行番号がずれるため）。その起点として、いま書いた範囲の
 * 最初の非空行を返す。
 */
export function anchorLineOf(replacement: Replacement): number {
  const written = replacement.text.split('\n');
  const first = written.findIndex(l => l.trim() !== '');
  return replacement.range.start + Math.max(0, first);
}
