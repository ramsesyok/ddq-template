import { describe, expect, it, vi } from 'vitest';

vi.mock('vscode', () => ({}));

import { isNearlyEmpty, validateFolderName } from './init';

describe('isNearlyEmpty', () => {
    it('空のフォルダ', () => {
        expect(isNearlyEmpty([])).toBe(true);
    });

    it('git init 済み・README や .vscode だけならそのまま作ってよい', () => {
        expect(isNearlyEmpty(['.git', '.vscode', 'README.md', 'LICENSE', '.gitignore'])).toBe(true);
    });

    it('ほかのファイルがあれば空とみなさない', () => {
        expect(isNearlyEmpty(['.git', 'src'])).toBe(false);
        expect(isNearlyEmpty(['notes.txt'])).toBe(false);
    });
});

describe('validateFolderName', () => {
    it('ふつうの名前（日本語を含む）は通す', () => {
        expect(validateFolderName('docs')).toBeNull();
        expect(validateFolderName('受注システム設計')).toBeNull();
        expect(validateFolderName('  order-design ')).toBeNull();
    });

    it('空・. / .. は不可', () => {
        expect(validateFolderName('  ')).not.toBeNull();
        expect(validateFolderName('.')).not.toBeNull();
        expect(validateFolderName('..')).not.toBeNull();
    });

    it('区切り文字と Windows で作れない文字は不可', () => {
        expect(validateFolderName('a/b')).not.toBeNull();
        expect(validateFolderName('a\\b')).not.toBeNull();
        expect(validateFolderName('a:b')).not.toBeNull();
        expect(validateFolderName('a?')).not.toBeNull();
        expect(validateFolderName('docs.')).not.toBeNull();
    });
});
