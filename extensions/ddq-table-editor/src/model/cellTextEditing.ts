/**
 * セル本文をキーボードで編集するときの純粋な文字列操作。
 *
 * React 側（GridView）は textarea のキャレット位置を渡すだけにして、
 * 文字列の組み立てはここでテストできるようにしておく。
 */

export type TextEdit = {
  text: string;
  /** 編集後にキャレットを置く位置。 */
  caret: number;
};

/**
 * 選択範囲を改行で置き換える（Alt+Enter）。
 *
 * textarea は Alt+Enter では改行しない（改行を入れるのは修飾キー無しの Enter だけ）。
 * セル確定に Enter を使っている以上、改行はこちらで組み立てるしかない。
 */
export function insertLineBreak(text: string, start: number, end: number): TextEdit {
  const from = clamp(Math.min(start, end), text.length);
  const to = clamp(Math.max(start, end), text.length);
  return { text: text.slice(0, from) + '\n' + text.slice(to), caret: from + 1 };
}

function clamp(value: number, max: number): number {
  if (!Number.isFinite(value) || value < 0) return 0;
  return Math.min(Math.floor(value), max);
}
