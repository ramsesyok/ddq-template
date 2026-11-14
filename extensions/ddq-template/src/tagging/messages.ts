/** 拡張ホスト ⇄ Webview のやりとり（双方が import する）。 */

import type { Item, TagList } from '../ddq/types';
import type { Row } from './labels';

/** 拡張 → Webview */
export type ToWebviewMessage =
    | { type: 'list'; list: TagList; version: string }
    | { type: 'busy'; busy: boolean }
    | { type: 'errors'; errors: [string, string][] }
    | { type: 'error'; message: string; detail?: string };

/** Webview → 拡張 */
export type FromWebviewMessage =
    | { type: 'ready' }
    | { type: 'reload' }
    | { type: 'reveal'; file: string; line: number }
    | { type: 'apply'; rows: Row[] };

/** 表示の並び（文書順）。`order` に無いファイルは後ろにまとめる。 */
export function sortItems(items: Item[], order: string[]): Item[] {
    const at = new Map(order.map((f, i) => [f, i]));
    return [...items].sort((a, b) => {
        const fa = at.get(a.file) ?? Number.MAX_SAFE_INTEGER;
        const fb = at.get(b.file) ?? Number.MAX_SAFE_INTEGER;
        if (fa !== fb) return fa - fb;
        return a.line - b.line;
    });
}

/** 一覧の件数（画面の下に出す）。 */
export function counts(items: Item[]): {
    total: number;
    labelled: number;
    suggested: number;
    blocked: number;
} {
    let labelled = 0;
    let suggested = 0;
    let blocked = 0;
    for (const item of items) {
        if (item.label) labelled += 1;
        else if (item.suggested) suggested += 1;
        else blocked += 1;
    }
    return { total: items.length, labelled, suggested, blocked };
}
