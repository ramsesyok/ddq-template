/**
 * 端末・等幅フォント上の表示幅（全角=2 / 半角=1）。
 *
 * グリッド表は罫線を桁で揃える必要があり、全角文字が混ざると `String.length` では
 * 揃わない。`design-doc.lua` の `disp_width` と同じ方針で幅を数える。
 */

export function charDisplayWidth(codePoint: number): number {
  if (codePoint === 0) return 0;
  // 結合文字（Combining Diacritical Marks）は幅 0 とみなす
  if (codePoint >= 0x0300 && codePoint <= 0x036f) return 0;
  if (isWide(codePoint)) return 2;
  return 1;
}

export function displayWidth(text: string): number {
  let width = 0;
  for (const ch of text) {
    width += charDisplayWidth(ch.codePointAt(0) ?? 0);
  }
  return width;
}

/** 表示幅が `width` になるよう右側を空白で埋める。溢れる場合はそのまま返す。 */
export function padToWidth(text: string, width: number): string {
  const pad = width - displayWidth(text);
  return pad > 0 ? text + ' '.repeat(pad) : text;
}

function isWide(cp: number): boolean {
  return (
    (cp >= 0x1100 && cp <= 0x115f) || // Hangul Jamo
    (cp >= 0x2e80 && cp <= 0x303e) || // CJK Radicals, Kangxi, CJK Symbols
    (cp >= 0x3041 && cp <= 0x33ff) || // Hiragana / Katakana / Hangul Compat / CJK Compat
    (cp >= 0x3400 && cp <= 0x4dbf) || // CJK Ext A
    (cp >= 0x4e00 && cp <= 0x9fff) || // CJK Unified
    (cp >= 0xa000 && cp <= 0xa4cf) || // Yi
    (cp >= 0xac00 && cp <= 0xd7a3) || // Hangul Syllables
    (cp >= 0xf900 && cp <= 0xfaff) || // CJK Compatibility Ideographs
    (cp >= 0xfe10 && cp <= 0xfe19) || // Vertical forms
    (cp >= 0xfe30 && cp <= 0xfe6f) || // CJK Compatibility Forms
    (cp >= 0xff00 && cp <= 0xff60) || // Fullwidth Forms
    (cp >= 0xffe0 && cp <= 0xffe6) ||
    (cp >= 0x1f300 && cp <= 0x1f64f) || // 絵文字
    (cp >= 0x1f900 && cp <= 0x1f9ff) ||
    (cp >= 0x20000 && cp <= 0x3fffd)
  );
}
