import * as path from 'node:path';

import { describe, expect, it, vi } from 'vitest';

vi.mock('vscode', () => ({}));

import { isSkipped, nearestWritingFolder, subfolders } from './folders';

describe('subfolders', () => {
    const docs = path.resolve('/repo/docs');
    const tree: Record<string, string[]> = {
        [docs]: ['chapters', '_book', '.quarto', 'images'],
        [path.join(docs, 'chapters')]: ['01-overview', '02-system'],
        [path.join(docs, 'chapters', '01-overview')]: [],
        [path.join(docs, 'chapters', '02-system')]: ['01-hw'],
        [path.join(docs, 'chapters', '02-system', '01-hw')]: [],
        [path.join(docs, 'images')]: [],
        [path.join(docs, '_book')]: ['chapters']
    };
    const readDir = async (dir: string) => tree[dir] ?? [];

    it('配下のフォルダを深さ順に拾い、生成物のフォルダには入らない', async () => {
        expect(await subfolders(docs, readDir)).toEqual([
            path.join(docs, 'chapters'),
            path.join(docs, 'images'),
            path.join(docs, 'chapters', '01-overview'),
            path.join(docs, 'chapters', '02-system'),
            path.join(docs, 'chapters', '02-system', '01-hw')
        ]);
    });

    it('上限で打ち切る', async () => {
        expect(await subfolders(docs, readDir, 2)).toHaveLength(2);
    });
});

describe('isSkipped', () => {
    it('生成物の中のパスを見分ける', () => {
        expect(isSkipped(path.join('/repo', 'docs', '_book', 'index.html'))).toBe(true);
        expect(isSkipped('C:\\repo\\docs\\.quarto\\idx')).toBe(true);
        expect(isSkipped(path.join('/repo', 'docs', 'chapters', 'new'))).toBe(false);
    });
});

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
