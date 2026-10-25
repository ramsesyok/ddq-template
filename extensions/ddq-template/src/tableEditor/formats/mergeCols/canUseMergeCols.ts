import type { TableModel } from '../../model/TableModel';
import { simulateMergeCols, mergedMapToRowspans } from './simulateMergeCols';

export type MergeColsCheck =
  | { ok: true; cols: number[] | null }
  | { ok: false; reason: string };

/**
 * 編集中の結合状態を `merge-cols` で**完全に再現できるか**判定する。
 *
 * `merge-cols` は「縦に連続する同一値を自動で rowspan 結合」する再生成型の指定なので、
 * 任意の結合を表現できるとは限らない。ここでは
 *
 *   1. 明らかに不可能な条件（colspan / ヘッダ複数行 / 結合セルの内容不一致）を弾く
 *   2. 結合を展開した素の表に `design-doc.lua` と同じアルゴリズムを適用する
 *   3. 得られた結合が編集中のモデルと完全一致するか比較する
 *
 * という手順で、取りこぼしなく判定する。一致しない場合は、意図しない結合が生じている
 * （同じ値がたまたま連続している）か、連鎖ルールで結合されないかのどちらか。
 */
export function canUseMergeCols(model: TableModel, partCount = 1): MergeColsCheck {
  if (partCount > 1) {
    return {
      ok: false,
      reason: '分割表のため（merge-cols は div 全体＝すべてのパートに影響します）。'
    };
  }
  if (model.headerRows !== 1) {
    return {
      ok: false,
      reason:
        model.headerRows === 0
          ? 'ヘッダ行が無いため（merge-cols はパイプ表が前提で、ヘッダ 1 行が必要です）。'
          : 'ヘッダが複数行のため（パイプ表はヘッダ 1 行しか表現できません）。'
    };
  }

  const colLen = model.columns.length;
  const bodyRows = model.rows.slice(1);
  if (bodyRows.length === 0) {
    return { ok: false, reason: '本体行が無いため。' };
  }

  // 1. colspan があれば不可
  for (const row of model.rows) {
    for (const cell of row) {
      if (!cell.hidden && cell.colspan > 1) {
        return { ok: false, reason: '横方向の結合（colspan）があるため。' };
      }
    }
  }
  // ヘッダ行に縦結合があると本体だけを対象とする merge-cols では表現できない
  for (const cell of model.rows[0]) {
    if (!cell.hidden && cell.rowspan > 1) {
      return { ok: false, reason: 'ヘッダ行がまたがる縦結合があるため。' };
    }
  }

  // 2. 結合セルは全被覆行で同じテキストになるので、展開表を作る
  const texts: string[][] = bodyRows.map(row => row.map(() => ''));
  const actual: number[][] = bodyRows.map(row => row.map(() => 1));

  for (let r = 0; r < bodyRows.length; r++) {
    for (let c = 0; c < colLen; c++) {
      const cell = bodyRows[r][c];
      if (cell.hidden) continue;
      actual[r][c] = cell.rowspan;
      for (let rr = r; rr < r + cell.rowspan; rr++) {
        texts[rr][c] = cell.text;
        if (rr !== r) actual[rr][c] = 0;
      }
    }
  }

  // 空セルの結合は merge-cols では起こせない
  for (let r = 0; r < bodyRows.length; r++) {
    for (let c = 0; c < colLen; c++) {
      if (actual[r][c] > 1 && texts[r][c].trim() === '') {
        return { ok: false, reason: '空セルが結合されているため（merge-cols は空セルを結合しません）。' };
      }
    }
  }

  // 3. 結合を持つ列を左→右の順に並べたものを候補にする
  const mergedCols: number[] = [];
  for (let c = 0; c < colLen; c++) {
    if (actual.some(row => row[c] !== 1)) mergedCols.push(c + 1);
  }
  if (mergedCols.length === 0) {
    return { ok: false, reason: '結合が無いため（通常のパイプ表として出力されます）。' };
  }

  // 4. 候補を順に試す。
  //    merge-cols は「指定した順が階層の左→右」なので、並び順を変えると再現できる場合がある
  //    （例: 右の列のほうが粗い階層になっているとき）。列数は多くないので順列を全部試す。
  const candidates: Array<number[] | null> = [mergedCols];
  if (mergedCols.length === colLen) candidates.push(null); // "all" 相当
  if (mergedCols.length <= 5) {
    for (const p of permutations(mergedCols)) {
      if (p.join(',') !== mergedCols.join(',')) candidates.push(p);
    }
  }

  for (const cols of candidates) {
    const spans = mergedMapToRowspans(simulateMergeCols(texts, cols));
    if (sameSpans(spans, actual)) return { ok: true, cols };
  }

  // 5. 一致しない理由を推定する（自然な左→右順での結果で説明する）
  const spans = mergedMapToRowspans(simulateMergeCols(texts, mergedCols));
  for (let r = 0; r < spans.length; r++) {
    for (let c = 0; c < colLen; c++) {
      const sim = spans[r][c];
      const act = actual[r][c];
      if (sim === act) continue;
      const where = `${r + 2} 行目・${c + 1} 列目`;
      if (sim === 0 || (act !== 0 && sim > act)) {
        return {
          ok: false,
          reason: `同じ値が連続する箇所（${where}）が意図せず結合されるため。`
        };
      }
      return {
        ok: false,
        reason: `左の列が結合されていないため、連鎖ルールで再現できません（${where}）。`
      };
    }
  }

  return { ok: false, reason: 'merge-cols では再現できない結合のため。' };
}

function permutations(items: number[]): number[][] {
  if (items.length <= 1) return [items];
  const out: number[][] = [];
  for (let i = 0; i < items.length; i++) {
    const rest = [...items.slice(0, i), ...items.slice(i + 1)];
    for (const p of permutations(rest)) out.push([items[i], ...p]);
  }
  return out;
}

function sameSpans(a: number[][], b: number[][]): boolean {
  if (a.length !== b.length) return false;
  for (let r = 0; r < a.length; r++) {
    if (a[r].length !== b[r].length) return false;
    for (let c = 0; c < a[r].length; c++) {
      if (a[r][c] !== b[r][c]) return false;
    }
  }
  return true;
}
