import { beforeEach, describe, expect, it, vi } from 'vitest';

/**
 * ラベル一覧の取り直しが重なったとき（レビュー R-06）。
 *
 * フォルダ A の `ddq tag list` を待っているあいだに B で開き直し、B → A の順に
 * 応答が返る。画面と書き戻しに使う一覧は B だけでなければならない。
 * ddq と VSCode は差し替え、応答の順番をテストが決める。
 */

type Handler = (...args: unknown[]) => unknown;

const commands = new Map<string, Handler>();
const posted: { type: string; [key: string]: unknown }[] = [];
const warnings: string[] = [];
/** フォルダごとの `tag list` の応答待ち */
const pending = new Map<string, { resolve: (v: unknown) => void; reject: (e: unknown) => void }>();
const panel = {
    title: '',
    reveal: vi.fn(),
    dispose: vi.fn(),
    onDidDispose: vi.fn(),
    webview: {
        html: '',
        postMessage: (m: { type: string }) => {
            posted.push(m);
            return Promise.resolve(true);
        },
        onDidReceiveMessage: vi.fn()
    }
};

vi.mock('vscode', () => ({
    ViewColumn: { Active: -1, Beside: -2 },
    Uri: { file: (fsPath: string) => ({ fsPath }) },
    commands: {
        registerCommand: (id: string, fn: Handler) => {
            commands.set(id, fn);
            return { dispose() {} };
        }
    },
    window: {
        createWebviewPanel: () => panel,
        registerCustomEditorProvider: () => ({ dispose() {} }),
        showWarningMessage: (m: string) => warnings.push(m)
    },
    workspace: {
        getConfiguration: () => ({ get: () => '' }),
        registerTextDocumentContentProvider: () => ({ dispose() {} })
    }
}));

vi.mock('./revision/provider', () => ({
    EMPTY_SCHEME: 'ddq-empty',
    RevisionEditorProvider: class {
        static viewType = 'ddqTemplate.revisionEditor';
        static emptyProvider() {
            return {};
        }
    },
    html: () => '',
    openDiff: vi.fn(),
    reveal: vi.fn(),
    write: vi.fn(),
    writingFolderOf: vi.fn()
}));

vi.mock('./folders', () => ({
    resolveWritingFolder: async (resource: { fsPath: string }) => resource.fsPath
}));

vi.mock('./ddq/run', () => ({
    DdqError: class extends Error {},
    ddqCommand: () => 'ddq',
    run: async () => 'ddq 2.4.5',
    versionOf: () => '2.4.5',
    runJson: (_command: string, args: string[]) =>
        new Promise((resolve, reject) => pending.set(args[2], { resolve, reject }))
}));

import { activateRevision, deactivateRevision } from './activateRevision';

const listOf = (folder: string, original = true) => ({
    folder,
    order: ['a.qmd'],
    items: [
        {
            kind: 'heading',
            level: 1,
            file: 'a.qmd',
            line: 1,
            title: folder,
            label: null,
            suggested: 'sec-a',
            edit: { file: 'a.qmd', line: 1, col: 3, insert: ' {#sec-a}', ...(original ? { original: '# a' } : {}) }
        }
    ],
    warnings: []
});

/** そのフォルダの `tag list` が呼ばれるまで待つ（`--version` のあとに呼ばれる）。 */
async function waitFor(folder: string) {
    for (let i = 0; i < 100 && !pending.has(folder); i++) await new Promise((r) => setTimeout(r, 1));
    const p = pending.get(folder);
    if (!p) throw new Error(`${folder} の tag list が呼ばれない`);
    pending.delete(folder);
    return p;
}

const openTags = (folder: string) => commands.get('ddqTemplate.tags')!({ fsPath: folder }) as Promise<void>;
const lists = () => posted.filter((m) => m.type === 'list').map((m) => (m.list as { folder: string }).folder);

describe('ラベル一覧の取り直しが重なったとき（R-06）', () => {
    beforeEach(() => {
        deactivateRevision();
        commands.clear();
        posted.length = 0;
        warnings.length = 0;
        pending.clear();
        panel.title = '';
        activateRevision({ subscriptions: [], extensionUri: { fsPath: '/ext' } } as never);
    });

    it('B → 遅れて A の順に返っても、画面に出すのは B だけ', async () => {
        const a = openTags('A');
        const fromA = await waitFor('A');
        const b = openTags('B');
        const fromB = await waitFor('B');

        fromB.resolve(listOf('B'));
        await b;
        fromA.resolve(listOf('A'));
        await a;

        expect(lists()).toEqual(['B']);
        expect(panel.title).toBe('ラベル一覧: B');
        // 取得中の表示は B の完了で消え、A の遅れた応答では触らない
        expect(posted.filter((m) => m.type === 'busy').at(-1)).toEqual({ type: 'busy', busy: false });
    });

    it('遅れて返った A の失敗も、B の画面にエラーとして出さない', async () => {
        const a = openTags('A');
        const fromA = await waitFor('A');
        const b = openTags('B');
        const fromB = await waitFor('B');

        fromB.resolve(listOf('B'));
        await b;
        fromA.reject(new Error('A の失敗'));
        await a;

        expect(lists()).toEqual(['B']);
        expect(posted.some((m) => m.type === 'error')).toBe(false);
    });

    it('A → B の順に返れば、A は捨てて B を出す', async () => {
        const a = openTags('A');
        const fromA = await waitFor('A');
        const b = openTags('B');
        const fromB = await waitFor('B');

        fromA.resolve(listOf('A'));
        await a;
        fromB.resolve(listOf('B'));
        await b;

        expect(lists()).toEqual(['B']);
    });

    it('照合用の行を返さない古い ddq の一覧なら、書き戻せないことを先に知らせる（R-02）', async () => {
        const a = openTags('A');
        (await waitFor('A')).resolve(listOf('A', false));
        await a;

        expect(lists()).toEqual(['A']);
        expect(warnings).toHaveLength(1);
        expect(warnings[0]).toMatch(/ラベルを書き戻せません/);
    });
});
