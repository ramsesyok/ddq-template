/**
 * 候補ラベルの検査と、編集指示の組み立て（VSCode に依存しない部分）。
 *
 * 人は一覧の上で候補を書き換えられる（`sec-3f9a1c` → `sec-purpose` のように）。
 * 書き戻す前に、形と一意性をここで確かめる。CLI 側（`ddq tag apply --from`）にも
 * 同じ検査があるが、拡張は `WorkspaceEdit` で自分で当てる（Undo を効かせるため）ので、
 * CLI を通らない。したがってこちら側にも検査が要る。
 */

import type { Edit, Item, Kind } from '../ddq/types';

/** 種別ごとのラベルの接頭辞。 */
export function prefixOf(kind: Kind): string {
    switch (kind) {
        case 'heading':
            return 'sec-';
        case 'fig':
            return 'fig-';
        default:
            return 'tbl-';
    }
}

/** ラベルとして使える形か（英数字・`-`・`_` だけで、種別に合った接頭辞を持つ）。 */
export function isValidLabel(label: string, kind: Kind): boolean {
    const prefix = prefixOf(kind);
    return (
        label.startsWith(prefix) &&
        label.length > prefix.length &&
        /^[A-Za-z0-9_-]+$/.test(label)
    );
}

/** 一覧の 1 行の状態（Webview が持ち、書き戻しのときに拡張へ返す）。 */
export type Row = {
    item: Item;
    /** 人が書き換えたあとの候補（未付与のものだけ） */
    label: string;
    /** 書き戻す対象か */
    checked: boolean;
};

/** 検査の結果。`edits` は当てる順に整えていない生の指示。 */
export type Checked = {
    edits: Edit[];
    /** 行ごとのエラー（キーは `file:line`） */
    errors: Map<string, string>;
};

/** `file:line` の形。行の同定に使う。 */
export function rowKey(item: Pick<Item, 'file' | 'line'>): string {
    return `${item.file}:${item.line}`;
}

/**
 * チェックの付いた行を書き戻せるか調べ、編集指示に直す。
 *
 * `ddq tag list` が返した `edit.insert` は候補ラベルを含んだ文字列なので、
 * 人が書き換えた `row.label` で作り直す（挿入位置 `col` はそのまま使える）。
 */
export function checkRows(rows: Row[], allItems: Item[]): Checked {
    const errors = new Map<string, string>();
    const edits: Edit[] = [];

    // 既にある全ラベル + これから付けるラベルで一意性を見る
    const used = new Set<string>();
    for (const item of allItems) {
        if (item.label) used.add(item.label);
    }

    for (const row of rows) {
        if (!row.checked || !row.item.edit) continue;
        const key = rowKey(row.item);
        const label = row.label.trim();
        if (label === '') {
            errors.set(key, 'ラベルが空です');
            continue;
        }
        if (!isValidLabel(label, row.item.kind)) {
            errors.set(
                key,
                `${prefixOf(row.item.kind)}… で始まる英数字・- ・_ だけにしてください`
            );
            continue;
        }
        if (used.has(label)) {
            errors.set(key, `${label} は既に使われています`);
            continue;
        }
        used.add(label);
        edits.push({ ...row.item.edit, insert: insertFor(row.item, label) });
    }
    return { edits, errors };
}

/**
 * 挿入する文字列を候補ラベルから作り直す。
 *
 * `ddq tag list` が返した `insert` の形（`#sec-x ` / ` {#sec-x}` / ` label="tbl-x"`）を
 * そのまま保ち、ラベルの部分だけ差し替える。形を決めているのは CLI 側なので、
 * ここでは**元の形に合わせる**（新しい形を作らない）。
 */
export function insertFor(item: Item, label: string): string {
    const original = item.edit?.insert ?? '';
    const old = item.suggested ?? '';
    if (old !== '' && original.includes(old)) {
        return original.split(old).join(label);
    }
    // 候補が無い（あり得ないが）ときの保険
    return item.kind === 'tbl' || item.kind === 'ipo' ? ` label="${label}"` : ` {#${label}}`;
}

/**
 * 同じファイルの中は**行番号の大きい順**に当てる。
 * 前から当てると、同じ行に 2 つ足すときに 2 つ目の位置がずれる。
 */
export function sortForApply(edits: Edit[]): Edit[] {
    return [...edits].sort((a, b) => {
        if (a.file !== b.file) return a.file < b.file ? -1 : 1;
        if (a.line !== b.line) return b.line - a.line;
        return b.col - a.col;
    });
}

/** 文字数の位置を、その行の UTF-16 オフセット（VSCode の character）に直す。 */
export function characterOffset(line: string, col: number): number {
    let chars = 0;
    for (let i = 0; i < line.length; ) {
        if (chars === col) return i;
        const code = line.codePointAt(i) ?? 0;
        i += code > 0xffff ? 2 : 1;
        chars += 1;
    }
    return line.length;
}
