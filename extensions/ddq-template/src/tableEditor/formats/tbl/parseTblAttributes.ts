import type { TblAttributes } from '../../model/TableModel';
import { emptyAttributes } from '../../model/TableModel';

export type FenceHeader = {
  /** fence のコロン数（`:::` なら 3）。書き戻すときにそのまま使う。 */
  colons: number;
  classes: string[];
  attributes: Array<[string, string]>;
};

/** `::: {.tbl caption="…" …}` の見出し行を分解する。`.tbl` 以外の div でも使える。 */
export function parseFenceHeader(line: string): FenceHeader | undefined {
  // 行末の空白（CRLF 由来の `\r` を含む）を落としてから見る。
  // `.` は `\r` に一致せず、`$` も真の終端でしか一致しないため、
  // 残したままだと CRLF のドキュメントで fence を取りこぼす。
  const m = /^\s*(:{3,})\s*(.*)$/.exec(line.replace(/\s+$/, ''));
  if (!m) return undefined;

  const colons = m[1].length;
  let rest = m[2].trim();

  // `{...}` 形式と、波括弧なしの `.tbl` 形式の両方を受ける
  const braced = /^\{(.*)\}$/.exec(rest);
  if (braced) rest = braced[1];

  const classes: string[] = [];
  const attributes: Array<[string, string]> = [];

  // 引用符の中では `\"` `\\` をエスケープとして認める（pandoc と同じ扱い）。
  const tokenRe =
    /([.#]?[A-Za-z_][\w:-]*)\s*=\s*("((?:[^"\\]|\\.)*)"|'((?:[^'\\]|\\.)*)'|[^\s}]+)|([.#][\w-]+)|(\S+)/g;
  for (const t of rest.matchAll(tokenRe)) {
    if (t[1] !== undefined) {
      const quoted = t[3] ?? t[4];
      const value = quoted !== undefined ? unescapeAttr(quoted) : t[2];
      attributes.push([t[1], value]);
    } else if (t[5] !== undefined) {
      classes.push(t[5].slice(1));
    }
  }

  return { colons, classes, attributes };
}

/** 引用符付き属性値のエスケープを解く（`\"` → `"`、`\\` → `\`）。 */
function unescapeAttr(value: string): string {
  return value.replace(/\\(.)/g, '$1');
}

export function isTblFence(header: FenceHeader): boolean {
  return header.classes.includes('tbl');
}

/** fence の見出しから `.tbl` の属性モデルを作る。既知以外はそのまま保持する。 */
export function toTblAttributes(header: FenceHeader): TblAttributes {
  const attrs: TblAttributes = emptyAttributes();

  for (const cls of header.classes) {
    if (cls === 'tbl') continue;
    if (cls === 'unnumbered') {
      attrs.unnumbered = true;
      continue;
    }
    attrs.extraClasses.push(cls);
  }

  for (const [key, value] of header.attributes) {
    switch (key) {
      case 'caption':
        attrs.caption = value;
        break;
      case 'label':
        attrs.label = normalizeLabel(value);
        break;
      case 'widths':
      case 'merge-cols':
        // 列情報・出力形式として別途扱うので属性としては保持しない
        break;
      default:
        attrs.extraAttributes.push([key, value]);
    }
  }

  return attrs;
}

/**
 * `label` の正規化。
 *
 * AUTHORING.md §6 のとおり、図の `{#fig-x}` に引きずられて `label="#tbl-x"` と書いても
 * 通るよう、先頭の `#` は取り除く。
 */
export function normalizeLabel(value: string): string {
  return value.trim().replace(/^#/, '');
}

/** `label` が `tbl-` 始まりか。そうでないと相互参照が無効になる。 */
export function isValidLabel(label: string): boolean {
  return /^tbl-/.test(label);
}

/** `widths="20,30,50"` を数値配列へ。個数が列数と合わなければ undefined。 */
export function parseWidths(
  value: string | undefined,
  colLen: number
): number[] | undefined {
  if (!value) return undefined;
  const nums = [...value.matchAll(/[\d.]+/g)].map(m => Number(m[0]));
  if (nums.length !== colLen) return undefined;
  if (nums.some(n => !Number.isFinite(n)) || nums.reduce((a, b) => a + b, 0) <= 0) {
    return undefined;
  }
  return nums;
}

/** fence の属性から値を取り出す。 */
export function attributeValue(
  header: FenceHeader,
  key: string
): string | undefined {
  const found = header.attributes.find(([k]) => k === key);
  return found?.[1];
}
