import { describe, expect, it } from 'vitest';

import { classifyMismatch, compareVersions, parseVersion } from './version';

/**
 * template/VERSION に入ったことのある版（`git log -p -- template/VERSION`）を古い順に並べたもの。
 * 隣どうしが必ず「古い → 新しい」と判定されること（EXT-01）。
 */
const SHIPPED = [
    '1.0.0', '1.1.0', '1.1.1', '1.1.2', '1.1.3', '1.2.0', '1.2.1', '1.2.2', '1.2.3', '1.3.0', '1.4.0',
    '2.0.0', '2.1.0', '2.2.0', '2.2.1', '2.2.2', '2.3.0', '2.4.0', '2.4.1', '2.4.2', '2.4.3', '2.4.4', '2.4.5',
    '3.0.0-pre1', '3.0.0-pre2', '3.0.0-pre3', '3.0.0-pre4', '3.0.0', '3.0.1-pre1', '3.1.0-pre1'
];

describe('compareVersions', () => {
    it('数として比べる（文字列比較ではない）', () => {
        expect(compareVersions('2.10.0', '2.9.9')).toBe(1);
        expect(compareVersions('2.4.5', '2.4.5')).toBe(0);
        expect(compareVersions('2.4.4', '2.4.5')).toBe(-1);
    });

    it('桁数が違っても比べられる（2.4 = 2.4.0 は従来どおり）', () => {
        expect(compareVersions('2.4', '2.4.0')).toBe(0);
        expect(compareVersions('2.4.0', '2.4')).toBe(0);
        expect(compareVersions('3', '2.99.99')).toBe(1);
    });

    it('preN は番号順で、同じ数字の正式版より前（両方向）', () => {
        const order = ['3.0.0-pre1', '3.0.0-pre4', '3.0.0-pre10', '3.0.0'];
        for (let i = 0; i < order.length; i += 1) {
            for (let j = 0; j < order.length; j += 1) {
                expect(compareVersions(order[i], order[j]), `${order[i]} と ${order[j]}`).toBe(Math.sign(i - j));
            }
        }
        // 数字が違えば pre は見ない
        expect(compareVersions('3.0.1-pre1', '3.0.0')).toBe(1);
        expect(compareVersions('2.4.5', '3.0.0-pre1')).toBe(-1);
    });

    it('出荷した版は古い順に並ぶ', () => {
        for (let i = 1; i < SHIPPED.length; i += 1) {
            expect(compareVersions(SHIPPED[i - 1], SHIPPED[i]), `${SHIPPED[i - 1]} < ${SHIPPED[i]}`).toBe(-1);
            expect(compareVersions(SHIPPED[i], SHIPPED[i - 1])).toBe(1);
        }
    });

    it('版の形でなければ比べない（undefined）', () => {
        for (const bad of ['', '  ', 'abc', '3.0.0-rc1', '3.0.0-pre', '3.0.0-pre1-x', 'v3.0.0', '3..0', '3.0.0 4']) {
            expect(parseVersion(bad), bad).toBeUndefined();
            expect(compareVersions(bad, '3.0.0'), bad).toBeUndefined();
            expect(compareVersions('3.0.0', bad), bad).toBeUndefined();
        }
        expect(parseVersion(' 3.1.0-pre1\n')).toEqual({ numbers: [3, 1, 0], pre: 1 });
    });
});

describe('classifyMismatch', () => {
    it('文書の方が古ければ update を出す側', () => {
        expect(classifyMismatch('2.4.4', '2.4.5')).toBe('folder-older');
        expect(classifyMismatch('3.0.0-pre4', '3.0.0')).toBe('folder-older');
        expect(classifyMismatch('3.0.0-pre1', '3.0.0-pre4')).toBe('folder-older');
        expect(classifyMismatch('3.0.0-pre4', '3.0.0-pre10')).toBe('folder-older');
    });

    it('文書の方が新しければ update を出さない側（降格になる）', () => {
        expect(classifyMismatch('2.6.0', '2.4.5')).toBe('folder-newer');
        expect(classifyMismatch('3.0.0', '3.0.0-pre4')).toBe('folder-newer');
        expect(classifyMismatch('3.0.0-pre4', '3.0.0-pre1')).toBe('folder-newer');
    });

    it('同じ版なら機構ファイルの改変（短い版も従来どおり同じ版）', () => {
        expect(classifyMismatch('2.4.5\n', '2.4.5')).toBe('modified');
        expect(classifyMismatch('3.0.0-pre4', '3.0.0-pre4')).toBe('modified');
        expect(classifyMismatch('2.4', '2.4.0')).toBe('modified');
    });

    it('版が分からなければ決めつけない（版ファイルなし・空・不正・ddq の版が不明）', () => {
        expect(classifyMismatch(undefined, '2.4.5')).toBe('unknown');
        expect(classifyMismatch('  ', '2.4.5')).toBe('unknown');
        expect(classifyMismatch('abc', '2.4.5')).toBe('unknown');
        expect(classifyMismatch('2.4.5', undefined)).toBe('unknown');
        expect(classifyMismatch('2.4.5', 'abc')).toBe('unknown');
    });
});
