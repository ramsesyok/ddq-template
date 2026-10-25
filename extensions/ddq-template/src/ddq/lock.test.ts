import { describe, expect, it } from 'vitest';

import { isLocked, lockKey, tryLock } from './lock';

describe('tryLock', () => {
    it('同じフォルダの 2 本目は取れず、解放すれば取れる', () => {
        const release = tryLock('/work/docs');
        expect(release).toBeDefined();
        expect(tryLock('/work/docs')).toBeUndefined();
        expect(isLocked('/work/docs')).toBe(true);
        release?.();
        expect(isLocked('/work/docs')).toBe(false);
        const again = tryLock('/work/docs');
        expect(again).toBeDefined();
        again?.();
    });

    it('別のフォルダは並行して取れる', () => {
        const a = tryLock('/work/a');
        const b = tryLock('/work/b');
        expect(a).toBeDefined();
        expect(b).toBeDefined();
        a?.();
        b?.();
    });

    it('解放を 2 回呼んでも、ほかの実行のロックを外さない', () => {
        const first = tryLock('/work/c');
        first?.();
        const second = tryLock('/work/c');
        first?.();
        expect(isLocked('/work/c')).toBe(true);
        second?.();
    });
});

describe('lockKey', () => {
    it('Windows は大文字小文字・区切り・末尾の区切りを揃える', () => {
        expect(lockKey('C:\\Work\\Docs\\', 'win32')).toBe(lockKey('c:/work/docs', 'win32'));
    });

    it('それ以外は大文字小文字を区別する', () => {
        expect(lockKey('/Work/docs', 'linux')).not.toBe(lockKey('/work/docs', 'linux'));
    });
});
