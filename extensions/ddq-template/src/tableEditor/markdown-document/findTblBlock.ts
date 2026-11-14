import { parseFenceHeader, isTblFence, type FenceHeader } from '../formats/tbl/parseTblAttributes';

export type LineRange = { start: number; end: number };

export type DivBlock = {
  header: FenceHeader;
  /** fence 開始行（`::: {.tbl …}`）。 */
  startLine: number;
  /** fence 終了行（`:::`）。 */
  endLine: number;
  /** 中身の行範囲（fence 行を含まない）。中身が無ければ start > end。 */
  body: LineRange;
  /** 入れ子の深さ（0 が最も外側）。 */
  depth: number;
};

export type FenceScanResult =
  | { ok: true; blocks: DivBlock[] }
  | { ok: false; message: string; line: number };

const OPEN_RE = /^\s*:{3,}\s*(\{.*\}|\.\S.*)\s*$/;
const CLOSE_RE = /^\s*(:{3,})\s*$/;

/**
 * ドキュメント全体の div fence を走査する。
 *
 * コロンは 3 個以上（`:::` / `::::` …）で、入れ子（`:::: {.landscape}` の中の
 * `::: {.tbl}`）も正しく扱う。終了 fence は開始 fence 以上のコロン数を要求せず、
 * 直近の開いている fence を閉じる（Pandoc の実運用に合わせる）。
 */
export function scanFences(lines: string[]): FenceScanResult {
  const stack: Array<{ header: FenceHeader; startLine: number; depth: number }> = [];
  const blocks: DivBlock[] = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    if (CLOSE_RE.test(line)) {
      const open = stack.pop();
      if (!open) {
        return {
          ok: false,
          message: '対応する開始 fence が無い `:::` があります。ドキュメントを変更しません。',
          line: i
        };
      }
      blocks.push({
        header: open.header,
        startLine: open.startLine,
        endLine: i,
        body: { start: open.startLine + 1, end: i - 1 },
        depth: open.depth
      });
      continue;
    }

    if (OPEN_RE.test(line)) {
      const header = parseFenceHeader(line);
      if (header) stack.push({ header, startLine: i, depth: stack.length });
    }
  }

  if (stack.length > 0) {
    return {
      ok: false,
      message: '終了 fence `:::` が見つからない div があります。ドキュメントを変更しません。',
      line: stack[stack.length - 1].startLine
    };
  }

  blocks.sort((a, b) => a.startLine - b.startLine);
  return { ok: true, blocks };
}

/** カーソル行を含む最も内側の `.tbl` div を返す。 */
export function findTblBlockAt(blocks: DivBlock[], line: number): DivBlock | undefined {
  const containing = blocks.filter(
    b => isTblFence(b.header) && b.startLine <= line && line <= b.endLine
  );
  if (containing.length === 0) return undefined;
  return containing.reduce((deepest, b) => (b.depth > deepest.depth ? b : deepest));
}

/** その `.tbl` を囲んでいる div のクラス（`.landscape` の判定に使う）。 */
export function enclosingClasses(blocks: DivBlock[], target: DivBlock): string[] {
  const classes: string[] = [];
  for (const b of blocks) {
    if (b === target) continue;
    if (b.startLine < target.startLine && target.endLine < b.endLine) {
      classes.push(...b.header.classes);
    }
  }
  return classes;
}
