/**
 * `ddq tag list --json` の出力（cli/src/doc/units.rs・cli/src/doc/mod.rs と対になる型）。
 *
 * この拡張は qmd を解釈しない。表示するものも書き戻すものも、すべて ddq が返す
 * この JSON に由来する（同じ解釈が CLI と拡張で二重にならないようにするため）。
 */

/** 拾った 1 件の種別。 */
export type Kind = 'heading' | 'tbl' | 'ipo' | 'pipe' | 'fig';

/** ラベルを付けられない理由。 */
export type Warning = 'no-caption' | 'bare-figure';

/** 1 行の中への文字列の挿入。`col` は行頭からの**文字数**（バイト数ではない）。 */
export type Edit = {
  file: string;
  /** 1 始まり */
  line: number;
  /** 行頭からの文字数 */
  col: number;
  insert: string;
};

/** 見出し・表・図 1 件。 */
export type Item = {
  kind: Kind;
  /** 見出しの深さ（1〜6）。見出し以外は無い */
  level?: number;
  file: string;
  /** 1 始まり */
  line: number;
  title: string | null;
  /** 既に付いているラベル */
  label: string | null;
  /** ラベルが無いときの候補 */
  suggested?: string;
  /** 候補を書き戻すための編集指示 */
  edit?: Edit;
  warning?: Warning;
};

/** `ddq tag list --json` 全体。 */
export type TagList = {
  folder: string;
  /** 文書順のファイル */
  order: string[];
  items: Item[];
  warnings: string[];
};

/** 種別の表示名。 */
export function kindLabel(item: Item): string {
  switch (item.kind) {
    case 'heading':
      return `見出し${item.level ?? ''}`;
    case 'tbl':
      return '表';
    case 'ipo':
      return 'IPO';
    case 'pipe':
      return '表(pipe)';
    case 'fig':
      return '図';
  }
}

/** 警告の説明。 */
export function warningLabel(warning: Warning): string {
  switch (warning) {
    case 'no-caption':
      return 'キャプション無し（採番されないので対象外）';
    case 'bare-figure':
      return 'ラベル不可（::: {#fig-…} で包むと付けられる）';
  }
}
