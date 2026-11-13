import { describe, expect, it, vi } from 'vitest';

vi.mock('vscode', () => ({}));

import { defaultAppCommand } from './openFile';

describe('defaultAppCommand', () => {
    it('Windows は explorer.exe に素のパスを渡す（file URL にしない）', () => {
        const file = 'C:\work\設計 書\docs\design-doc.pdf';
        expect(defaultAppCommand(file, 'win32')).toEqual({ command: 'explorer.exe', args: [file] });
    });

    it('Windows 以外は undefined（openExternal を使う）', () => {
        expect(defaultAppCommand('/home/a/design-doc.pdf', 'linux')).toBeUndefined();
        expect(defaultAppCommand('/Users/a/design-doc.pdf', 'darwin')).toBeUndefined();
    });
});
