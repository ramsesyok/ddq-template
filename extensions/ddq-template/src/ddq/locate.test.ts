import * as path from 'node:path';

import { describe, expect, it } from 'vitest';

import { bundledPath, chooseDdq } from './locate';

describe('chooseDdq', () => {
    const bundled = 'C:\\ext\\bin\\win32-x64\\ddq.exe';

    it('設定があれば同梱より優先する', () => {
        expect(chooseDdq(' C:\\tools\\ddq.exe ', bundled, () => true)).toEqual({
            command: 'C:\\tools\\ddq.exe',
            source: 'setting'
        });
    });

    it('設定が空なら同梱を使う', () => {
        expect(chooseDdq('', bundled, () => true)).toEqual({ command: bundled, source: 'bundled' });
        expect(chooseDdq(undefined, bundled, () => true).source).toBe('bundled');
    });

    it('同梱が無ければ PATH の ddq', () => {
        expect(chooseDdq('  ', bundled, () => false)).toEqual({ command: 'ddq', source: 'path' });
        expect(chooseDdq(undefined, undefined, () => true)).toEqual({ command: 'ddq', source: 'path' });
    });
});

describe('bundledPath', () => {
    it('win32-x64 だけ同梱を探す', () => {
        expect(bundledPath('C:\\ext', 'win32', 'x64')).toBe(path.join('C:\\ext', 'bin', 'win32-x64', 'ddq.exe'));
        expect(bundledPath('C:\\ext', 'win32', 'arm64')).toBeUndefined();
        expect(bundledPath('/ext', 'linux', 'x64')).toBeUndefined();
    });
});
