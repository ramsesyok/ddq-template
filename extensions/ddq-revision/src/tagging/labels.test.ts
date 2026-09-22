import { describe, expect, it } from 'vitest';

import type { Item } from '../ddq/types';
import {
    characterOffset,
    checkRows,
    insertFor,
    isValidLabel,
    sortForApply,
    type Row
} from './labels';

/** 見出し 1 件（ラベル未付与）。 */
function heading(line: number, title: string, suggested: string): Item {
    return {
        kind: 'heading',
        level: 2,
        file: 'chapters/01/index.qmd',
        line,
        title,
        label: null,
        suggested,
        edit: {
            file: 'chapters/01/index.qmd',
            line,
            col: title.length + 3,
            insert: ` {#${suggested}}`
        }
    };
}

function row(item: Item, label?: string, checked = true): Row {
    return { item, label: label ?? item.suggested ?? '', checked };
}

describe('isValidLabel', () => {
    it('種別に合った接頭辞だけを通す', () => {
        expect(isValidLabel('sec-purpose', 'heading')).toBe(true);
        expect(isValidLabel('tbl-cond', 'tbl')).toBe(true);
        expect(isValidLabel('tbl-ipo', 'ipo')).toBe(true);
        expect(isValidLabel('tbl-x', 'heading')).toBe(false);
    });

    it('接頭辞だけ・記号・日本語は通さない', () => {
        expect(isValidLabel('sec-', 'heading')).toBe(false);
        expect(isValidLabel('sec-目的', 'heading')).toBe(false);
        expect(isValidLabel('sec-a b', 'heading')).toBe(false);
        expect(isValidLabel('sec-a.b', 'heading')).toBe(false);
    });
});

describe('checkRows', () => {
    it('チェックした行だけを編集指示にする', () => {
        const a = heading(1, '目的', 'sec-aaa111');
        const b = heading(5, '範囲', 'sec-bbb222');
        const { edits, errors } = checkRows([row(a), row(b, undefined, false)], [a, b]);
        expect(errors.size).toBe(0);
        expect(edits).toHaveLength(1);
        expect(edits[0].insert).toBe(' {#sec-aaa111}');
    });

    it('人が書き換えたラベルを使う', () => {
        const a = heading(1, '目的', 'sec-aaa111');
        const { edits } = checkRows([row(a, 'sec-purpose')], [a]);
        expect(edits[0].insert).toBe(' {#sec-purpose}');
        expect(edits[0].line).toBe(1);
        expect(edits[0].col).toBe(a.edit?.col);
    });

    it('形の違うラベルは当てずに理由を返す', () => {
        const a = heading(1, '目的', 'sec-aaa111');
        const { edits, errors } = checkRows([row(a, 'tbl-oops')], [a]);
        expect(edits).toHaveLength(0);
        expect(errors.get('chapters/01/index.qmd:1')).toContain('sec-');
    });

    it('空のラベルは当てない', () => {
        const a = heading(1, '目的', 'sec-aaa111');
        const { edits, errors } = checkRows([row(a, '   ')], [a]);
        expect(edits).toHaveLength(0);
        expect(errors.get('chapters/01/index.qmd:1')).toContain('空');
    });

    it('既にある他のラベルとの重複を弾く', () => {
        const a = heading(1, '目的', 'sec-aaa111');
        const existing: Item = {
            kind: 'heading',
            file: 'index.qmd',
            line: 1,
            title: '本書について',
            label: 'sec-preface'
        };
        const { edits, errors } = checkRows([row(a, 'sec-preface')], [a, existing]);
        expect(edits).toHaveLength(0);
        expect(errors.get('chapters/01/index.qmd:1')).toContain('既に使われています');
    });

    it('同じ改訂の中の重複も弾く（先に来た方を通す）', () => {
        const a = heading(1, '目的', 'sec-aaa111');
        const b = heading(5, '範囲', 'sec-bbb222');
        const { edits, errors } = checkRows([row(a, 'sec-same'), row(b, 'sec-same')], [a, b]);
        expect(edits).toHaveLength(1);
        expect(errors.get('chapters/01/index.qmd:5')).toContain('既に使われています');
    });
});

describe('insertFor', () => {
    it('.tbl の label= の形を保つ', () => {
        const tbl: Item = {
            kind: 'tbl',
            file: 'a.qmd',
            line: 3,
            title: '設計条件',
            label: null,
            suggested: 'tbl-aaa111',
            edit: { file: 'a.qmd', line: 3, col: 40, insert: ' label="tbl-aaa111"' }
        };
        expect(insertFor(tbl, 'tbl-cond')).toBe(' label="tbl-cond"');
    });

    it('属性が既にある見出しの形（#label + 空白）を保つ', () => {
        const withAttrs: Item = {
            kind: 'heading',
            level: 1,
            file: 'index.qmd',
            line: 1,
            title: '本書について',
            label: null,
            suggested: 'sec-aaa111',
            edit: { file: 'index.qmd', line: 1, col: 9, insert: '#sec-aaa111 ' }
        };
        expect(insertFor(withAttrs, 'sec-preface')).toBe('#sec-preface ');
    });
});

describe('sortForApply', () => {
    it('同じファイルでは行番号の大きい順に当てる', () => {
        const e = (file: string, line: number, col = 0) => ({ file, line, col, insert: 'x' });
        const sorted = sortForApply([e('a.qmd', 1), e('b.qmd', 2), e('a.qmd', 9), e('a.qmd', 5)]);
        expect(sorted.map((x) => `${x.file}:${x.line}`)).toEqual([
            'a.qmd:9',
            'a.qmd:5',
            'a.qmd:1',
            'b.qmd:2'
        ]);
    });
});

describe('characterOffset', () => {
    it('日本語の行でも文字数どおりの位置を返す', () => {
        // ddq は「行頭からの文字数」を返すので、UTF-16 の位置に直して使う
        expect(characterOffset('## 目的', 5)).toBe(5);
        expect(characterOffset('## 目的', 3)).toBe(3);
    });

    it('サロゲートペアを 1 文字として数える', () => {
        const line = '## 🚀 見出し';
        // 「🚀」は UTF-16 では 2 単位。col=4（`## 🚀 `の直後）は offset=5
        expect(characterOffset(line, 4)).toBe(5);
    });

    it('行末より後ろを指されたら行末に丸める', () => {
        expect(characterOffset('## 目的', 99)).toBe('## 目的'.length);
    });
});
