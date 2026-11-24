import { beforeEach, describe, expect, it, vi } from 'vitest';

/**
 * ステータスバー（build.ts の updateStatus）。執筆フォルダの問い合わせは非同期なので、
 * エディタを続けて切り替えると答えが逆の順に返りうる。古い答えで新しい表示を上書きしないことを見る。
 * 問い合わせ（activeWritingFolder）と ddq の実行は、答えを返す時期を試験が決める差し替え。
 */

type Deferred<T> = { promise: Promise<T>; resolve: (v: T) => void };
const deferred = <T>(): Deferred<T> => {
    let resolve!: (v: T) => void;
    const promise = new Promise<T>((r) => (resolve = r));
    return { promise, resolve };
};

const state = vi.hoisted(() => ({
    item: { text: '', shown: false } as { text: string; shown: boolean },
    onActiveEditor: undefined as (() => void) | undefined,
    commands: new Map<string, (...args: any[]) => unknown>(),
    /** activeWritingFolder の答え（呼ばれた順に積む） */
    queries: [] as { resolve: (v: string | undefined) => void }[],
    /** runWithProgress の結果 */
    run: undefined as { resolve: (v: { code: number; lines: string[] }) => void } | undefined
}));

vi.mock('vscode', () => ({
    StatusBarAlignment: { Left: 1, Right: 2 },
    Uri: { file: (fsPath: string) => ({ fsPath, scheme: 'file' }) },
    window: {
        createStatusBarItem: () => {
            const item = state.item;
            return {
                set text(v: string) {
                    item.text = v;
                },
                get text() {
                    return item.text;
                },
                show: () => (item.shown = true),
                hide: () => (item.shown = false),
                dispose() {}
            };
        },
        onDidChangeActiveTextEditor: (listener: () => void) => {
            state.onActiveEditor = listener;
            return { dispose() {} };
        },
        showInformationMessage: async () => undefined
    },
    commands: {
        registerCommand: (id: string, handler: (...args: any[]) => unknown) => {
            state.commands.set(id, handler);
            return { dispose() {} };
        }
    }
}));

vi.mock('./folders', () => ({
    TEMPLATE_VERSION_FILE: '_extensions/ddq/_extension.yml',
    activeWritingFolder: () =>
        new Promise<string | undefined>((resolve) => {
            state.queries.push({ resolve });
        }),
    resolveWritingFolder: async () => undefined,
    watchWritingFolders: () => undefined
}));
vi.mock('./ddq/launch', () => ({
    channel: () => ({ appendLine() {}, show() {} }),
    currentDdq: () => ({ command: 'ddq', source: 'path' }),
    settings: () => ({ quartoPath: '' }),
    showError: async () => undefined,
    runWithProgress: () =>
        new Promise((resolve) => {
            state.run = { resolve };
        })
}));
vi.mock('./ddq/lock', () => ({ tryLock: () => () => undefined }));
vi.mock('./quarto', () => ({ envWithQuarto: () => ({}), hasQuarto: async () => true }));
vi.mock('./openFile', () => ({ openWithDefaultApp: async () => undefined }));

import { activateBuild } from './build';

const flush = async () => {
    for (let i = 0; i < 10; i++) await Promise.resolve();
};

describe('ステータスバーの表示（EXT-03 と同じ型の逆転）', () => {
    beforeEach(() => {
        state.item.text = '';
        state.item.shown = false;
        state.queries = [];
        state.run = undefined;
        state.commands.clear();
        activateBuild({ subscriptions: [], extension: { packageJSON: { version: '3.1.0' } } } as never);
    });

    it('エディタを続けて切り替え、答えが逆の順に返っても、後の切り替えの表示が残る', async () => {
        state.onActiveEditor!();
        state.onActiveEditor!();
        expect(state.queries).toHaveLength(2);
        state.queries[1].resolve('/work/後');
        await flush();
        expect(state.item.text).toContain('PDF: 後');
        state.queries[0].resolve('/work/先');
        await flush();
        expect(state.item.text).toContain('PDF: 後');
        expect(state.item.shown).toBe(true);
    });

    it('執筆フォルダの外へ切り替えた後に、前のフォルダの答えが返っても表示しない', async () => {
        state.onActiveEditor!();
        state.onActiveEditor!();
        state.queries[1].resolve(undefined);
        await flush();
        expect(state.item.shown).toBe(false);
        state.queries[0].resolve('/work/先');
        await flush();
        expect(state.item.shown).toBe(false);
    });

    it('発行中の表示を、その前に始めた問い合わせの答えで消さない', async () => {
        state.onActiveEditor!();
        const build = state.commands.get('ddqTemplate.internal.build')!('pdf', '/work/docs') as Promise<string>;
        await flush();
        expect(state.item.text).toContain('sync~spin');
        state.queries[0].resolve('/work/docs');
        await flush();
        expect(state.item.text).toContain('sync~spin');

        // 終われば、改めて問い合わせた答えで通常の表示に戻る
        state.run!.resolve({ code: 0, lines: [] });
        await flush();
        expect(state.queries).toHaveLength(2);
        state.queries[1].resolve('/work/docs');
        await flush();
        expect(await build).toBe('ok');
        expect(state.item.text).toBe('$(file-pdf) PDF: docs');
    });
});
