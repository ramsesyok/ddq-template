import type { TableModel } from './TableModel';

/**
 * 列幅の合計として扱う値。
 *
 * テンプレート（design-doc.lua の `parse_widths`）は合計 1 へ正規化するので比率でも
 * 通るが、UI では％として扱う。空欄 1 列の補完もこの値を基準にする。
 */
export const WIDTH_TOTAL = 100;

/** 浮動小数の誤差で 100.00000000000001 を超過と誤判定しないための許容差。 */
const EPSILON = 1e-9;

export type ColumnWidthStatus = {
  /** 幅が入っている列数。 */
  specifiedCount: number;
  /** 空欄の列数。 */
  blankCount: number;
  /** 入っている幅の合計。 */
  total: number;
  /** 合計が WIDTH_TOTAL を超えている。 */
  overflow: boolean;
  /** 負の幅が入っている列があるか。 */
  hasNegative: boolean;
  /** 空欄が 1 列だけで残りを補完できるとき、その列番号と補完値。 */
  autoFill?: { col: number; width: number };
};

/** 小数の丸め。100 - 33.3 - 33.3 のような誤差を出さない。 */
function roundWidth(value: number): number {
  return Math.round(value * 1000) / 1000;
}

function isWidth(value: number | undefined): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}

/** 列幅の状態を調べる。UI の赤枠表示と検査・補完の判断はすべてこれを見る。 */
export function inspectColumnWidths(model: TableModel): ColumnWidthStatus {
  const widths = model.columns.map(c => c.width);
  const specified = widths.filter(isWidth);
  const total = roundWidth(specified.reduce((a, b) => a + b, 0));
  const blanks = widths
    .map((w, c) => (isWidth(w) ? -1 : c))
    .filter(c => c >= 0);

  const status: ColumnWidthStatus = {
    specifiedCount: specified.length,
    blankCount: blanks.length,
    total,
    overflow: total > WIDTH_TOTAL + EPSILON,
    hasNegative: specified.some(w => w < 0)
  };

  // 空欄が 1 列だけなら、残り幅（100 − 他列の合計）を割り当てられる。
  const remainder = roundWidth(WIDTH_TOTAL - total);
  if (blanks.length === 1 && specified.length > 0 && remainder > 0) {
    status.autoFill = { col: blanks[0], width: remainder };
  }

  return status;
}

/**
 * 空欄が 1 列だけのときに残り幅を埋めたモデルを返す（補完できなければそのまま）。
 *
 * 編集中のモデルには触れず、Apply とプレビューの直前だけ通す。入力欄が勝手に
 * 埋まると、消して入れ直すことができなくなるため。
 */
export function completeColumnWidths(model: TableModel): TableModel {
  const { autoFill } = inspectColumnWidths(model);
  if (!autoFill) return model;
  const columns = model.columns.map((col, c) =>
    c === autoFill.col ? { ...col, width: autoFill.width } : col
  );
  return { ...model, columns };
}
