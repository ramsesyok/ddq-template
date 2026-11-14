import type { CellAlign } from '../../model/TableModel';

/** 区切り行のセル（`:---` / `:--:` / `---:` / `---`）から揃えを読む。 */
export function separatorToAlign(cell: string): CellAlign | undefined {
  const s = cell.trim();
  const left = s.startsWith(':');
  const right = s.endsWith(':');
  if (left && right) return 'center';
  if (right) return 'right';
  if (left) return 'left';
  return undefined;
}

/** 揃えから区切り行のセル（表示幅 `width` の桁埋め込み）を作る。 */
export function alignToSeparator(align: CellAlign | undefined, width: number): string {
  const w = Math.max(width, 3);
  switch (align) {
    case 'left':
      return ':' + '-'.repeat(w - 1);
    case 'right':
      return '-'.repeat(w - 1) + ':';
    case 'center':
      return ':' + '-'.repeat(w - 2) + ':';
    default:
      return '-'.repeat(w);
  }
}

/** グリッド表のヘッダ終端行（`===` 区間）に揃えのコロンを打つ。 */
export function alignToGridSeparator(
  align: CellAlign | undefined,
  width: number,
  fill: '=' | '-'
): string {
  const w = Math.max(width, 3);
  switch (align) {
    case 'left':
      return ':' + fill.repeat(w - 1);
    case 'right':
      return fill.repeat(w - 1) + ':';
    case 'center':
      return ':' + fill.repeat(w - 2) + ':';
    default:
      return fill.repeat(w);
  }
}

/** グリッド表の罫線区間からの揃え読み取り（`:===:` など）。 */
export function gridSeparatorToAlign(segment: string): CellAlign | undefined {
  const s = segment.trim();
  const left = s.startsWith(':');
  const right = s.endsWith(':');
  if (left && right) return 'center';
  if (right) return 'right';
  if (left) return 'left';
  return undefined;
}
