/**
 * 中間テーブルモデル。
 *
 * エディタ（React Webview）はこのモデルだけを操作し、`.tbl` / パイプ表 / グリッド表 /
 * merge-cols といった記法を一切知らない。記法との変換は `src/formats/` 配下が担う。
 */

export type CellAlign = 'left' | 'center' | 'right';

export type TableCell = {
  id: string;
  /** セル本文。改行は '\n'。出力時に `<br>` へ変換する。 */
  text: string;
  row: number;
  col: number;
  rowspan: number;
  colspan: number;
  /** 結合に吸収されて表示されないセル。 */
  hidden: boolean;
};

export type TableColumn = {
  align?: CellAlign;
  /** `widths` 属性の値（％風でも比率でもよい）。未指定なら undefined＝自動幅。 */
  width?: number;
};

/** `::: {.tbl …}` の属性。未知のクラス・属性はそのまま保持して書き戻す。 */
export type TblAttributes = {
  caption?: string;
  /** 相互参照 ID。`tbl-` 始まり。先頭の `#` は取り除いた形で保持する。 */
  label?: string;
  unnumbered?: boolean;
  /** `.tbl` / `.unnumbered` 以外に付いていたクラス。 */
  extraClasses: string[];
  /** caption / label / widths / merge-cols 以外の属性。順序を保って書き戻す。 */
  extraAttributes: Array<[string, string]>;
};

export type OutputFormat = 'pipeTable' | 'gridTable' | 'mergeCols';

export type TableModel = {
  id: string;
  version: number;
  /** ヘッダとして扱う先頭行数（0〜N）。 */
  headerRows: number;
  columns: TableColumn[];
  rows: TableCell[][];
  attributes: TblAttributes;
  /**
   * ユーザーが選んだ出力形式。結合が無ければ常に 'pipeTable' 相当になるが、
   * 結合があるときは 'gridTable'（既定）か 'mergeCols' を選ぶ。
   */
  outputFormat: OutputFormat;
};

export function emptyAttributes(): TblAttributes {
  return { extraClasses: [], extraAttributes: [] };
}

export interface TableParser {
  parse(source: string): TableModel;
}

export interface TableSerializer {
  serialize(table: TableModel): string;
}

export type ParseResult<T> =
  | { ok: true; value: T }
  | { ok: false; message: string };

/** 結合（rowspan/colspan > 1）が 1 つでもあるか。 */
export function hasMerges(model: TableModel): boolean {
  return model.rows.some(row =>
    row.some(cell => !cell.hidden && (cell.rowspan > 1 || cell.colspan > 1))
  );
}

/**
 * セル内改行を含むか。
 *
 * パイプ表は 1 行しか書けず改行が `<br>` になるため、箇条書き・番号付きリストなどの
 * ブロックをセルに置けない。改行があれば必ずグリッド表で出力する判断に使う。
 */
export function hasLineBreaks(model: TableModel): boolean {
  return model.rows.some(row => row.some(cell => !cell.hidden && cell.text.includes('\n')));
}

/** 横方向の結合（colspan > 1）があるか。merge-cols では表現できない。 */
export function hasColspan(model: TableModel): boolean {
  return model.rows.some(row => row.some(cell => !cell.hidden && cell.colspan > 1));
}

export function makeCell(row: number, col: number, text = ''): TableCell {
  return {
    id: `r${row}c${col}`,
    text,
    row,
    col,
    rowspan: 1,
    colspan: 1,
    hidden: false
  };
}

