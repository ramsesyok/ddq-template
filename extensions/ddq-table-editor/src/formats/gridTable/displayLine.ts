import { charDisplayWidth } from '../displayWidth';

/**
 * 1 行を「表示桁 → 文字」で引けるようにしたもの。
 *
 * pandoc のグリッド表は文字数ではなく表示幅（全角=2）で列位置を決めるため、
 * 罫線の `+` や `|` の位置は表示桁で数える必要がある。
 */
export type DisplayLine = {
  raw: string;
  /** 文字 i の表示開始桁。 */
  starts: number[];
  /** 行全体の表示幅。 */
  width: number;
};

export function toDisplayLine(raw: string): DisplayLine {
  const starts: number[] = [];
  let col = 0;
  for (const ch of raw) {
    starts.push(col);
    col += charDisplayWidth(ch.codePointAt(0) ?? 0);
  }
  return { raw, starts, width: col };
}

/** 表示桁 `col` から始まる文字。桁が全角文字の途中や行末を超える場合は undefined。 */
export function charAt(line: DisplayLine, col: number): string | undefined {
  const chars = [...line.raw];
  const i = line.starts.indexOf(col);
  return i >= 0 ? chars[i] : undefined;
}

/** 表示桁 `col` に指定文字があるか（エスケープされた `\|` は数えない）。 */
export function hasCharAt(line: DisplayLine, col: number, ch: string): boolean {
  const chars = [...line.raw];
  const i = line.starts.indexOf(col);
  if (i < 0) return false;
  if (chars[i] !== ch) return false;
  if (ch === '|' && chars[i - 1] === '\\') return false;
  return true;
}

/** 表示桁 [from, to) の範囲の文字列を切り出す。 */
export function sliceByDisplay(line: DisplayLine, from: number, to: number): string {
  const chars = [...line.raw];
  let out = '';
  for (let i = 0; i < chars.length; i++) {
    const start = line.starts[i];
    if (start >= from && start < to) out += chars[i];
  }
  return out;
}

/** その行に現れる指定文字の表示桁をすべて返す。 */
export function displayColsOf(line: DisplayLine, ch: string): number[] {
  const chars = [...line.raw];
  const cols: number[] = [];
  for (let i = 0; i < chars.length; i++) {
    if (chars[i] === ch) cols.push(line.starts[i]);
  }
  return cols;
}
