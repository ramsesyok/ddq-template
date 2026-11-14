/**
 * ドキュメント本文を行へ分ける唯一の入口。
 *
 * CRLF・CR・LF のどれでも同じ結果になるようにする。行末に `\r` を残したまま
 * 走査すると、JavaScript の `$`（`m` フラグなし）が真の終端でしか一致しないため
 * fence の見出し行を取りこぼし、`.tbl` の検出が丸ごと失敗する。
 * 分割の仕方を 1 か所に集約して、呼び出し側ごとにずれないようにする。
 */
export function splitLines(text: string): string[] {
  return text.split(/\r\n|\r|\n/);
}
