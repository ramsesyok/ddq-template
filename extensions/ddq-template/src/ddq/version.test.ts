import { describe, expect, it } from 'vitest';

import { classifyMismatch, compareVersions } from './version';

describe('compareVersions', () => {
    it('数として比べる（文字列比較ではない）', () => {
        expect(compareVersions('2.10.0', '2.9.9')).toBe(1);
        expect(compareVersions('2.4.5', '2.4.5')).toBe(0);
        expect(compareVersions('2.4.4', '2.4.5')).toBe(-1);
    });

    it('桁数が違っても比べられる', () => {
        expect(compareVersions('2.4', '2.4.0')).toBe(0);
        expect(compareVersions('3', '2.99.99')).toBe(1);
    });
});

describe('classifyMismatch', () => {
    it('文書の方が古い（版ファイルなしを含む）なら update を出す側', () => {
        expect(classifyMismatch('2.4.4', '2.4.5')).toBe('folder-older');
        expect(classifyMismatch(undefined, '2.4.5')).toBe('folder-older');
        expect(classifyMismatch('  ', '2.4.5')).toBe('folder-older');
    });

    it('文書の方が新しければ update を出さない側（降格になる）', () => {
        expect(classifyMismatch('2.6.0', '2.4.5')).toBe('folder-newer');
    });

    it('同じ版なら機構ファイルの改変', () => {
        expect(classifyMismatch('2.4.5\n', '2.4.5')).toBe('modified');
    });
});
