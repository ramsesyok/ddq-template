import { describe, expect, it, vi } from 'vitest';

vi.mock('vscode', () => ({}));

import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';

import { checkAddTarget, suggestFolderName } from './add';

describe('checkAddTarget', () => {
    it('無いフォルダ・文書の無いフォルダには足せる（ddq add は無いものだけ置く）', () => {
        const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ddq-add-'));
        try {
            expect(checkAddTarget(path.join(root, '設計書2'))).toBeNull();
            fs.mkdirSync(path.join(root, 'notes'));
            fs.writeFileSync(path.join(root, 'notes', 'memo.txt'), '');
            expect(checkAddTarget(path.join(root, 'notes'))).toBeNull();
        } finally {
            fs.rmSync(root, { recursive: true, force: true });
        }
    });

    it('同じ名前のファイル・既存の文書（_quarto.yml）があれば理由を返す', () => {
        const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ddq-add-'));
        try {
            fs.writeFileSync(path.join(root, 'file'), '');
            expect(checkAddTarget(path.join(root, 'file'))).toMatch(/ファイルとしてあります/);
            fs.mkdirSync(path.join(root, 'docs'));
            fs.writeFileSync(path.join(root, 'docs', '_quarto.yml'), '');
            expect(checkAddTarget(path.join(root, 'docs'))).toMatch(/既に文書/);
        } finally {
            fs.rmSync(root, { recursive: true, force: true });
        }
    });
});

describe('suggestFolderName', () => {
    it('docs2 から順に、まだ無い名前を返す', () => {
        const root = path.join('C:', 'repo');
        const taken = new Set([path.join(root, 'docs2'), path.join(root, 'docs3')]);
        expect(suggestFolderName(root, (p) => taken.has(p))).toBe('docs4');
        expect(suggestFolderName(root, () => false)).toBe('docs2');
    });
});
