/**
 * 右クリックメニュー用の執筆フォルダ一覧（watchWritingFolders）の作り直しが重なったとき（EXT-03）。
 *
 * 走査（findFiles）の応答をテストが止めておき、古い走査が新しい走査より後に終わる順番を作る。
 * いちばん新しい走査の結果だけが setContext に入ること、拡張の終了後は何も書かないこと、
 * ワークスペースのルートの追加・削除で作り直すことを見る。
 */

import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';

import { afterAll, beforeEach, describe, expect, it, vi } from 'vitest';

type Deferred = { resolve: (files: string[]) => void };
const scans: Deferred[] = [];
const contexts: { key: string; value: unknown }[] = [];
const handlers = { folders: undefined as (() => void) | undefined };

vi.mock('vscode', () => {
    const Uri = { file: (fsPath: string) => ({ fsPath, scheme: 'file' }) };
    const disposable = { dispose() {} };
    return {
        Uri,
        FileType: { File: 1, Directory: 2 },
        commands: {
            executeCommand: async (id: string, key: string, value: unknown) => {
                if (id === 'setContext') contexts.push({ key, value });
            }
        },
        workspace: {
            findFiles: () =>
                new Promise<{ fsPath: string }[]>((resolve) =>
                    scans.push({ resolve: (files) => resolve(files.map((f) => Uri.file(f))) })
                ),
            fs: {
                stat: async (uri: { fsPath: string }) => fs.statSync(uri.fsPath),
                readDirectory: async (uri: { fsPath: string }) =>
                    fs
                        .readdirSync(uri.fsPath, { withFileTypes: true })
                        .map((e) => [e.name, e.isDirectory() ? 2 : 1] as const)
            },
            createFileSystemWatcher: () => ({ onDidCreate: () => disposable, onDidDelete: () => disposable, dispose() {} }),
            onDidChangeWorkspaceFolders: (cb: () => void) => {
                handlers.folders = cb;
                return disposable;
            }
        }
    };
});

import { watchWritingFolders } from './folders';

const root = fs.mkdtempSync(path.join(os.tmpdir(), 'ddq-watch-'));
afterAll(() => fs.rmSync(root, { recursive: true, force: true }));

/** 執筆フォルダ（_quarto.yml と版ファイル）を作り、版ファイルのパスを返す */
function writingFolder(name: string): string {
    const dir = path.join(root, name);
    fs.mkdirSync(path.join(dir, 'chapters'), { recursive: true });
    fs.writeFileSync(path.join(dir, '_quarto.yml'), 'project:\n  type: book\n');
    fs.writeFileSync(path.join(dir, '.template-version'), '3.1.0-pre1\n');
    return path.join(dir, '.template-version');
}
const a = writingFolder('a');
const b = writingFolder('b');

async function until(cond: () => boolean) {
    for (let i = 0; i < 200 && !cond(); i++) await new Promise((r) => setTimeout(r, 5));
    expect(cond()).toBe(true);
}

/** 最後に setContext した執筆フォルダの一覧（キー） */
function lastFolders(): string[] | undefined {
    const last = contexts.filter((c) => c.key === 'ddqTemplate.writingFolders').at(-1);
    return last ? Object.keys(last.value as object).sort() : undefined;
}

function context() {
    return { subscriptions: [] as { dispose(): void }[] };
}

beforeEach(() => {
    scans.length = 0;
    contexts.length = 0;
    handlers.folders = undefined;
});

describe('watchWritingFolders', () => {
    it('古い走査が後から終わっても、新しい走査の結果を上書きしない', async () => {
        const changes = vi.fn();
        watchWritingFolders(context() as never, changes);
        // 起動時の走査（1）を待たせたまま、ルートが増えて作り直す（2）
        await until(() => scans.length === 1);
        handlers.folders?.();
        await until(() => scans.length === 2);
        scans[1].resolve([a, b]);
        await until(() => changes.mock.calls.length === 1);
        expect(lastFolders()).toEqual([path.dirname(a), path.join(path.dirname(a), 'chapters'), path.dirname(b), path.join(path.dirname(b), 'chapters')].sort());
        // 古い走査（ルートが増える前。a だけ）が遅れて返る
        scans[0].resolve([a]);
        await new Promise((r) => setTimeout(r, 50));
        expect(lastFolders()).toContain(path.dirname(b));
        expect(changes).toHaveBeenCalledTimes(1);
        expect(contexts.filter((c) => c.key === 'ddqTemplate.hasWritingFolders').map((c) => c.value)).toEqual([true]);
    });

    it('ルートの追加・削除で作り直す（削除で空になれば hasWritingFolders は false）', async () => {
        watchWritingFolders(context() as never, () => {});
        await until(() => scans.length === 1);
        scans[0].resolve([a]);
        await until(() => lastFolders() !== undefined);
        expect(lastFolders()).toContain(path.dirname(a));
        handlers.folders?.();
        await until(() => scans.length === 2);
        scans[1].resolve([]);
        await until(() => contexts.filter((c) => c.key === 'ddqTemplate.hasWritingFolders').length === 2);
        expect(lastFolders()).toEqual([]);
        expect(contexts.filter((c) => c.key === 'ddqTemplate.hasWritingFolders').at(-1)?.value).toBe(false);
    });

    it('拡張の終了後に終わった走査は、メニューの一覧を書かない', async () => {
        const ctx = context();
        const changes = vi.fn();
        watchWritingFolders(ctx as never, changes);
        await until(() => scans.length === 1);
        for (const d of ctx.subscriptions) d.dispose();
        scans[0].resolve([a, b]);
        await new Promise((r) => setTimeout(r, 50));
        expect(contexts).toEqual([]);
        expect(changes).not.toHaveBeenCalled();
    });
});
