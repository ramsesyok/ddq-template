import type { TableModel } from './TableModel';
import { isValidLabel } from '../formats/tbl/parseTblAttributes';
import { inspectColumnWidths, WIDTH_TOTAL } from './columnWidths';

export type Validation = {
  errors: string[];
  warnings: string[];
};

/**
 * Apply の直前に走らせる検査。
 *
 * errors があるときは書き戻さない。warnings は表示したうえで続行を選べる。
 */
export function validateTableModel(
  model: TableModel,
  options: { otherPartColumnCounts?: number[] } = {}
): Validation {
  const errors: string[] = [];
  const warnings: string[] = [];

  if (model.rows.length === 0 || model.columns.length === 0) {
    errors.push('表が空です。');
  }

  const label = model.attributes.label?.trim();
  if (label) {
    if (!isValidLabel(label)) {
      errors.push(`label="${label}" は tbl- で始まる必要があります（本文で @${label} と参照するため）。`);
    }
    if (model.attributes.unnumbered) {
      warnings.push('.unnumbered が付いているため、label は無効になり相互参照はできません。');
    }
  }

  // 空欄 1 列の補完（completeColumnWidths）を通した後の値を検査する
  const widths = inspectColumnWidths(model);
  const colLen = model.columns.length;
  if (widths.hasNegative) {
    errors.push('列幅に負の値があります。');
  }
  if (widths.overflow) {
    errors.push(
      `列幅の合計が ${widths.total} で ${WIDTH_TOTAL} を超えています（合計 ${WIDTH_TOTAL} に収めてください）。`
    );
  }
  if (widths.specifiedCount > 0 && widths.blankCount > 0) {
    errors.push(
      `列幅は全列に指定するか、全列とも空にしてください（${widths.specifiedCount}／${colLen} 列のみ指定されています）。` +
        '空欄が 1 列だけなら残り幅を自動で補完します。'
    );
  }
  if (widths.blankCount === 0 && colLen > 0 && widths.total <= 0) {
    errors.push('列幅の合計が 0 以下です。');
  }

  // 分割表では全パートの列数が一致していないと、テンプレート側で列幅の統一がスキップされる
  for (const count of options.otherPartColumnCounts ?? []) {
    if (count > 0 && count !== model.columns.length) {
      warnings.push(
        `分割表の他のパートは ${count} 列ですが、このパートは ${model.columns.length} 列です。` +
          '列数が一致しないと列幅の統一がスキップされます。'
      );
      break;
    }
  }

  if (model.headerRows > model.rows.length) {
    errors.push('ヘッダ行数が行数を超えています。');
  }

  return { errors, warnings };
}
