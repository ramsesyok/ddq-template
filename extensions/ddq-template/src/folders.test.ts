import * as path from 'node:path';

import { describe, expect, it, vi } from 'vitest';

vi.mock('vscode', () => ({}));

import { nearestWritingFolder } from './folders';

describe('nearestWritingFolder', () => {
    const root = path.resolve('/repo');
    const docs = path.join(root, 'docs');
    const isWritingFolder = async (dir: string) => dir === docs;

    it('執筆フォルダそのもの', async () => {
        expect(await nearestWritingFolder(docs, isWritingFolder)).toBe(docs);
    });

    it('配下のファイルのフォルダから親へたどる', async () => {
        expect(await nearestWritingFolder(path.join(docs, 'chapters', '01-overview'), isWritingFolder)).toBe(docs);
    });

    it('執筆フォルダの外なら見つからない', async () => {
        expect(await nearestWritingFolder(path.join(root, 'other'), isWritingFolder)).toBeUndefined();
    });
});
