import { describe, expect, it, vi } from 'vitest';

vi.mock('vscode', () => ({}));

import { LEGACY_EXTENSIONS, installedLegacy } from './legacy';

describe('installedLegacy', () => {
    it('旧拡張が無ければ空', () => {
        expect(installedLegacy(() => false)).toEqual([]);
    });

    it('入っている旧拡張だけを返す', () => {
        expect(installedLegacy((id) => id === 'ramsesyok.ddq-table-editor')).toEqual([
            'ramsesyok.ddq-table-editor'
        ]);
    });

    it('両方入っていれば両方', () => {
        expect(installedLegacy(() => true)).toEqual([...LEGACY_EXTENSIONS]);
    });

    it('自分自身は旧拡張に数えない', () => {
        expect(LEGACY_EXTENSIONS).not.toContain('ramsesyok.ddq-template');
    });
});
