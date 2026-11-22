/**
 * ddq が返す JSON の形を確かめる（QA-01。期待値は cli/tests/fixtures/contract/）。
 *
 * `JSON.parse` の結果をそのまま型とみなすと、版の違う ddq（設定 `ddqTemplate.ddqPath`）が
 * 欠けた・型の違う値を返したとき、一覧の表示や書き戻しの途中で分かりにくく壊れる。
 * 受け取った時点で確かめ、書き込みより前に「どこが違うか」を出す。
 *
 * - 知らないフィールドは無視する（新しい ddq が足したもの）
 * - 旧版が出さない省略可能なフィールド（`edit.original` など）は無くてよい。
 *   `original` の無い一覧は表示できるが、書き戻しは `staleReason` がすべて拒む
 * - `kind` / `warning` は拡張が知っている値だけを受け付ける（ラベルの接頭辞と
 *   書き戻しの可否がこの値で決まるため、知らない値を推測で扱わない）
 */

import type { Kind, TagList, Warning } from './types';

/** `ddq rev next --json`（cli/src/commands/rev.rs と対）。 */
export type RevNext = {
    /** 次の改訂記号（書きかけがあればその記号） */
    rev: string;
    /** 比較基準の候補（前回の改訂タグ）。無ければ null */
    base: string | null;
    base_commit?: string | null;
    scheme?: string;
    /** 既にある改訂タグ（`rev-A` など） */
    tags: string[];
    /** 書きかけの改訂（タグの無い rev-*.yml） */
    drafts: string[];
};

/** 形の違い。`problems` は「場所: 何が違うか（実際の値）」の並び。 */
export class ContractError extends Error {
    constructor(
        readonly what: string,
        readonly problems: string[]
    ) {
        super(`${what} の出力が想定した形ではありません: ${problems.join(' / ')}`);
        this.name = 'ContractError';
    }
}

const KINDS: readonly Kind[] = ['heading', 'tbl', 'ipo', 'pipe', 'fig'];
const WARNINGS: readonly Warning[] = ['no-caption', 'bare-figure', 'foreign-id', 'multiple-ids'];

/** 一度に出す違いの数の上限（壊れた出力で通知が埋まらないように）。 */
const MAX_PROBLEMS = 8;

type Obj = Record<string, unknown>;

/** 違いを集める。最初の 1 件で止めず、まとめて出す（どこから直すか分かるように）。 */
class Problems {
    readonly list: string[] = [];
    private count = 0;

    add(at: string, what: string, value: unknown) {
        this.count += 1;
        if (this.list.length < MAX_PROBLEMS) this.list.push(`${at || '(全体)'}: ${what}（${show(value)}）`);
    }

    object(value: unknown, at: string): Obj | undefined {
        if (typeof value === 'object' && value !== null && !Array.isArray(value)) return value as Obj;
        this.add(at, 'オブジェクトではありません', value);
        return undefined;
    }

    /** `o[key]` を `ok` で確かめる。`optional` なら無くてよい（あれば確かめる）。 */
    field(o: Obj, key: string, at: string, ok: (v: unknown) => boolean, expected: string, optional = false) {
        const path = at ? `${at}.${key}` : key;
        if (!(key in o) || o[key] === undefined) {
            if (!optional) this.add(path, `${expected}が必要です`, undefined);
            return;
        }
        if (!ok(o[key])) this.add(path, `${expected}ではありません`, o[key]);
    }

    throwIfAny(what: string) {
        if (this.count === 0) return;
        const rest = this.count - this.list.length;
        throw new ContractError(what, rest > 0 ? [...this.list, `ほか ${rest} 件`] : this.list);
    }
}

function show(value: unknown): string {
    if (value === undefined) return '無し';
    const s = JSON.stringify(value) ?? String(value);
    return s.length > 60 ? `${s.slice(0, 57)}...` : s;
}

const isString = (v: unknown) => typeof v === 'string';
const isStringOrNull = (v: unknown) => v === null || typeof v === 'string';
const isStringArray = (v: unknown) => Array.isArray(v) && v.every(isString);
const isInt = (min: number, max = Number.MAX_SAFE_INTEGER) => (v: unknown) =>
    typeof v === 'number' && Number.isInteger(v) && v >= min && v <= max;
const oneOf = (values: readonly string[]) => (v: unknown) => typeof v === 'string' && values.includes(v);

/** `ddq tag list --json` の出力を確かめる。違えば `ContractError`。 */
export function parseTagList(value: unknown): TagList {
    const p = new Problems();
    const top = p.object(value, '');
    if (top) {
        p.field(top, 'folder', '', isString, '文字列');
        p.field(top, 'order', '', isStringArray, '文字列の配列');
        p.field(top, 'warnings', '', isStringArray, '文字列の配列');
        p.field(top, 'items', '', Array.isArray, '配列');
        if (Array.isArray(top.items)) top.items.forEach((item, i) => checkItem(p, item, `items[${i}]`));
    }
    p.throwIfAny('ddq tag list --json');
    return value as TagList;
}

function checkItem(p: Problems, value: unknown, at: string) {
    const item = p.object(value, at);
    if (!item) return;
    p.field(item, 'kind', at, oneOf(KINDS), `${KINDS.join(' / ')} のどれか`);
    p.field(item, 'level', at, isInt(1, 6), '1〜6 の整数', true);
    p.field(item, 'file', at, isString, '文字列');
    p.field(item, 'line', at, isInt(1), '1 以上の整数');
    p.field(item, 'title', at, isStringOrNull, '文字列か null');
    p.field(item, 'label', at, isStringOrNull, '文字列か null');
    p.field(item, 'suggested', at, isString, '文字列', true);
    p.field(item, 'warning', at, oneOf(WARNINGS), `${WARNINGS.join(' / ')} のどれか`, true);
    if (item.edit === undefined) return;
    const edit = p.object(item.edit, `${at}.edit`);
    if (!edit) return;
    const e = `${at}.edit`;
    p.field(edit, 'file', e, isString, '文字列');
    p.field(edit, 'line', e, isInt(1), '1 以上の整数');
    p.field(edit, 'col', e, isInt(0), '0 以上の整数（文字数）');
    p.field(edit, 'insert', e, isString, '文字列');
    p.field(edit, 'original', e, isString, '文字列', true);
}

/** `ddq rev next --json` の出力を確かめる。違えば `ContractError`。 */
export function parseRevNext(value: unknown): RevNext {
    const p = new Problems();
    const top = p.object(value, '');
    if (top) {
        p.field(top, 'rev', '', (v) => typeof v === 'string' && v !== '', '空でない文字列');
        p.field(top, 'base', '', isStringOrNull, '文字列か null');
        p.field(top, 'base_commit', '', isStringOrNull, '文字列か null', true);
        p.field(top, 'scheme', '', isString, '文字列', true);
        p.field(top, 'tags', '', isStringArray, '文字列の配列');
        p.field(top, 'drafts', '', isStringArray, '文字列の配列');
    }
    p.throwIfAny('ddq rev next --json');
    return value as RevNext;
}
