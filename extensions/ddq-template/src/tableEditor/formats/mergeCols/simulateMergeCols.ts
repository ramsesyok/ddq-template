/**
 * `design-doc.lua` の `merge_body` と同じ結合アルゴリズムを再現する。
 *
 * テンプレート側の実際の挙動（`template/design-doc.lua` L152-278 を参照）:
 * - `merge-cols` 省略時は全列を左から順に階層とみなす（prevcol[c] = c-1）
 * - `merge-cols="2,3"` のように指定した場合、**指定した順**が階層の左→右になる
 * - 列 c が上の行に吸収されるのは
 *     「その列が上の行と同じテキスト」かつ「prevcol[c] が同じ行で吸収済み」のとき
 *   （prevcol[c] が nil＝階層の先頭なら、その条件は無い）
 * - **空セルは結合しない**
 * - 比較はプレーンテキスト一致（`pandoc.utils.stringify`）
 * - 既に結合を持つ表（グリッド表）は `is_plain_grid` ガードで対象外
 */

/** `merge-cols` 属性の文字列を 1 始まりの列リストへ。`"all"` や未指定は null（＝全列）。 */
export function parseMergeCols(value: string | undefined): number[] | null {
  if (value === undefined || value.trim() === '') return null;
  if (value.trim().toLowerCase() === 'all') return null;
  const cols: number[] = [];
  for (const m of value.matchAll(/\d+/g)) cols.push(Number(m[0]));
  return cols.length > 0 ? cols : null;
}

export function serializeMergeCols(cols: number[] | null): string {
  return cols === null ? 'all' : cols.join(',');
}

/**
 * 素の表（結合なし・テキストのみ）に merge-cols を適用したときの吸収マップを返す。
 *
 * `merged[r][c] === true` は「行 r の列 c が上の行のセルに吸収される」ことを表す。
 *
 * @param texts  ボディ行のテキスト（ヘッダ行は含めない）
 * @param cols   結合対象列（1 始まり・階層の左→右順）。null なら全列
 */
export function simulateMergeCols(
  texts: string[][],
  cols: number[] | null
): boolean[][] {
  const rowLen = texts.length;
  const merged: boolean[][] = texts.map(row => row.map(() => false));
  if (rowLen === 0) return merged;
  const colLen = texts[0].length;

  // eligible / prevcol を design-doc.lua と同じ手順で決める
  const eligible: boolean[] = Array.from({ length: colLen }, () => false);
  const prevcol: Array<number | null> = Array.from({ length: colLen }, () => null);

  const order = cols ?? Array.from({ length: colLen }, (_, i) => i + 1);
  let prev: number | null = null;
  for (const oneBased of order) {
    const c = oneBased - 1;
    if (c >= 0 && c < colLen && !eligible[c]) {
      eligible[c] = true;
      prevcol[c] = prev;
      prev = c;
    }
  }

  // design-doc.lua と同じく「行ごとに列を左から 1 パス」で評価する。
  // そのため prevcol[c] が c より右の列だと、その時点ではまだ未評価（＝結合していない
  // 扱い）になり連鎖しない。merge-cols の並び順を逆にしても階層は作れない。
  for (let r = 0; r < rowLen; r++) {
    for (let c = 0; c < colLen; c++) {
      if (!eligible[c]) continue;
      if (r === 0) continue;
      const text = texts[r][c] ?? '';
      if (text === '') continue; // 空セルは結合しない
      if (text !== (texts[r - 1][c] ?? '')) continue;
      const p = prevcol[c];
      if (p !== null && !merged[r][p]) continue; // 左の列が結合済みでなければ連鎖しない
      merged[r][c] = true;
    }
  }

  return merged;
}

/** 吸収マップから、各セルの rowspan を求める（吸収されるセルは 0）。 */
export function mergedMapToRowspans(merged: boolean[][]): number[][] {
  const rowLen = merged.length;
  if (rowLen === 0) return [];
  const colLen = merged[0].length;
  const spans: number[][] = merged.map(row => row.map(() => 1));

  for (let c = 0; c < colLen; c++) {
    for (let r = 0; r < rowLen; r++) {
      if (merged[r][c]) {
        spans[r][c] = 0;
        continue;
      }
      let span = 1;
      while (r + span < rowLen && merged[r + span][c]) span++;
      spans[r][c] = span;
    }
  }

  return spans;
}
