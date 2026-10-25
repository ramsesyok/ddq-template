import { describe, expect, it, vi } from 'vitest';

vi.mock('vscode', () => ({}));

import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';

import { checkTarget, isNearlyEmpty, validateFolderName, writingFoldersIn } from './init';

describe('checkTarget', () => {
    it('無いフォルダ・ほぼ空のフォルダには作れる', () => {
        const parent = fs.mkdtempSync(path.join(os.tmpdir(), 'ddq-init-'));
        try {
            expect(checkTarget(path.join(parent, 'new'))).toBeNull();
            fs.mkdirSync(path.join(parent, 'empty'));
            fs.writeFileSync(path.join(parent, 'empty', 'README.md'), '');
            expect(checkTarget(path.join(parent, 'empty'))).toBeNull();
        } finally {
            fs.rmSync(parent, { recursive: true, force: true });
        }
    });

    it('同じ名前のファイルがあれば例外にせず理由を返す', () => {
        const parent = fs.mkdtempSync(path.join(os.tmpdir(), 'ddq-init-'));
        try {
            fs.writeFileSync(path.join(parent, 'project'), '');
            expect(checkTarget(path.join(parent, 'project'))).toMatch(/ファイルとしてあります/);
            fs.mkdirSync(path.join(parent, 'used'));
            fs.writeFileSync(path.join(parent, 'used', 'notes.txt'), '');
            expect(checkTarget(path.join(parent, 'used'))).toMatch(/空ではありません/);
        } finally {
            fs.rmSync(parent, { recursive: true, force: true });
        }
    });
});

describe('writingFoldersIn', () => {
    const a = path.resolve('/ws/a');
    const b = path.resolve('/ws/b');
    const found = [path.join(a, 'docs'), path.resolve('/ws/ab/docs')];

    it('選んだルートの中の執筆フォルダだけを見る（別ルートの文書で止めない）', () => {
        expect(writingFoldersIn(a, found)).toEqual([path.join(a, 'docs')]);
        expect(writingFoldersIn(b, found)).toEqual([]);
    });

    it('名前の前方一致だけでは中とみなさない', () => {
        expect(writingFoldersIn(path.resolve('/ws/ab'), [path.join(a, 'docs')])).toEqual([]);
    });
});

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
