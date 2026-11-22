/**
 * 改訂履歴エディタ（RevisionEditorProvider）の状態の扱い（EXT-03）。VSCode と ddq の起動は
 * 差し替え、`ddq rev next --json` の応答の順番をテストが決める。
 *
 * - メモの入力（文書の変更）では本文をすぐ送り、タグの状態は入力が止まってから 1 回だけ取り直す
 * - 取り直しの応答が逆順に返っても、新しい結果を古い結果で上書きしない
 * - 閉じたエディタには送らない
 * - 書き戻せなかったら知らせ、画面を文書に戻す
 * - 設定（ddqTemplate.ddqPath）を変えたら次の起動から使う
 */

import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

type Message = { type: string; [key: string]: unknown };
type RevNext = { rev: string; base: string; tags: string[]; drafts: string[] };

const state = {
    changes: [] as ((e: { document: { uri: { toString(): string } } }) => void)[],
    nexts: [] as { command: string; resolve: (v: RevNext) => void; reject: (e: unknown) => void }[],
    runs: [] as { args: string[]; resolve: (out: string) => void }[],
    applyResult: true,
    ddqPath: undefined as string | undefined
};

vi.mock('vscode', () => {
    class Range {
        constructor(...args: number[]) {
            void args;
        }
    }
    class WorkspaceEdit {
        text = '';
        replace(_uri: unknown, _range: unknown, text: string) {
            this.text = text;
        }
    }
    return {
        Range,
        WorkspaceEdit,
        EndOfLine: { LF: 1, CRLF: 2 },
        Uri: { joinPath: (...parts: unknown[]) => parts.join('/') },
        window: { showWarningMessage: async () => undefined, showErrorMessage: async () => undefined },
        workspace: {
            getConfiguration: () => ({ get: (key: string) => (key === 'ddqPath' ? state.ddqPath : undefined) }),
            onDidChangeTextDocument: (cb: (typeof state.changes)[number]) => {
                state.changes.push(cb);
                return { dispose: () => (state.changes = state.changes.filter((c) => c !== cb)) };
            },
            applyEdit: async (edit: WorkspaceEdit) => {
                if (state.applyResult) doc.text = edit.text;
                return state.applyResult;
            }
        }
    };
});
vi.mock('../ddq/locate', () => ({ locateDdq: (p?: string) => ({ command: p || 'ddq', source: 'path' }) }));
vi.mock('../ddq/run', async (importOriginal) => {
    const real = await importOriginal<typeof import('../ddq/run')>();
    return {
        ...real,
        runJson: (command: string) =>
            new Promise((resolve, reject) => state.nexts.push({ command, resolve: resolve as (v: RevNext) => void, reject })),
        run: (_command: string, args: string[]) => new Promise((resolve) => state.runs.push({ args, resolve }))
    };
});

import { RevisionEditorProvider, TAG_REFRESH_DELAY_MS } from './provider';

const golden = readFileSync(join(__dirname, 'rev-B.golden.yml'), 'utf8');
const doc = {
    text: golden,
    uri: { fsPath: 'C:\\docs\\revisions\\rev-B.yml', toString: () => 'file:///C:/docs/revisions/rev-B.yml' },
    eol: 1,
    isDirty: false,
    isClosed: false,
    lineCount: golden.split('\n').length,
    getText: () => doc.text
};

class FakePanel {
    posted: Message[] = [];
    /** 閉じた後に送ろうとした数（本物の VSCode では例外になる） */
    afterDispose = 0;
    disposed = false;
    receive: ((m: Message) => Promise<void>) | undefined;
    private onDispose: (() => void)[] = [];
    webview = {
        options: {},
        html: '',
        cspSource: 'csp',
        asWebviewUri: (u: unknown) => u,
        postMessage: async (m: Message) => {
            if (this.disposed) {
                this.afterDispose += 1;
                return false;
            }
            this.posted.push(m);
            return true;
        },
        onDidReceiveMessage: (cb: (m: Message) => Promise<void>) => {
            this.receive = cb;
            return { dispose() {} };
        }
    };
    onDidDispose(cb: () => void) {
        this.onDispose.push(cb);
        return { dispose() {} };
    }
    dispose() {
        this.disposed = true;
        for (const cb of this.onDispose) cb();
    }
}

const next = (tags: string[]): RevNext => ({ rev: 'C', base: 'rev-B', tags, drafts: [] });
const revisions = (panel: FakePanel) => panel.posted.filter((m) => m.type === 'revision');
const fixedOf = (m: Message | undefined) => (m?.context as { fixed: boolean } | undefined)?.fixed;

async function flush() {
    for (let i = 0; i < 5; i++) await Promise.resolve();
}

async function openEditor(): Promise<FakePanel> {
    const panel = new FakePanel();
    await new RevisionEditorProvider({} as never).resolveCustomTextEditor(doc as never, panel as never, {} as never);
    return panel;
}

/** 文書が変わった（メモを 1 文字打った・ddq が外から書いた） */
function typed(text: string) {
    doc.text = text;
    for (const cb of state.changes) cb({ document: doc });
}

beforeEach(() => {
    vi.useFakeTimers();
    state.changes = [];
    state.nexts = [];
    state.runs = [];
    state.applyResult = true;
    state.ddqPath = undefined;
    doc.text = golden;
});
afterEach(() => vi.useRealTimers());

describe('改訂履歴エディタ', () => {
    it('メモの入力ごとに ddq を起動せず、入力が止まってから 1 回だけタグを取り直す', async () => {
        const panel = await openEditor();
        const ready = panel.receive!({ type: 'ready' });
        expect(state.nexts).toHaveLength(1);
        state.nexts[0].resolve(next([]));
        await ready;
        const before = revisions(panel).length;
        for (let i = 0; i < 5; i++) {
            typed(golden.replace('目的の記述に背景を補足した。', `目的の記述に背景を補足した${'。'.repeat(i + 2)}`));
            vi.advanceTimersByTime(TAG_REFRESH_DELAY_MS / 2);
        }
        // 本文は打つたびに送る
        expect(revisions(panel).length - before).toBe(5);
        expect(state.nexts).toHaveLength(1);
        vi.advanceTimersByTime(TAG_REFRESH_DELAY_MS);
        expect(state.nexts).toHaveLength(2);
    });

    it('取り直しの応答が逆順に返っても、新しい結果を古い結果で上書きしない', async () => {
        const panel = await openEditor();
        const ready = panel.receive!({ type: 'ready' });
        // 開いたときの取り直し（1）が返る前に、入力が止まって取り直す（2）
        typed(golden.replace('目的の記述', '目的の記述を'));
        vi.advanceTimersByTime(TAG_REFRESH_DELAY_MS);
        expect(state.nexts).toHaveLength(2);
        // 2 の間に rev-B のタグが打たれた（確定済み）。2 が先に返り、古い 1 が後から返る
        state.nexts[1].resolve(next(['rev-A', 'rev-B']));
        await flush();
        expect(fixedOf(revisions(panel).at(-1))).toBe(true);
        state.nexts[0].resolve(next(['rev-A']));
        await ready;
        expect(fixedOf(revisions(panel).at(-1))).toBe(true);
    });

    it('閉じたエディタには、遅れて返った取り直しの結果も文書の変更も送らない', async () => {
        const panel = await openEditor();
        const ready = panel.receive!({ type: 'ready' });
        panel.dispose();
        state.nexts[0].resolve(next(['rev-A']));
        await expect(ready).resolves.toBeUndefined();
        typed(golden.replace('目的', '目的2'));
        vi.advanceTimersByTime(TAG_REFRESH_DELAY_MS * 2);
        expect(state.nexts).toHaveLength(1);
        expect(panel.posted.filter((m) => m.type !== 'busy')).toEqual([expect.objectContaining({ type: 'revision' })]);
    });

    it('ddq の実行中に閉じたら、終わった後の結果（お知らせ・処理中の解除）も送らない', async () => {
        const panel = await openEditor();
        const ready = panel.receive!({ type: 'ready' });
        state.nexts[0].resolve(next([]));
        await ready;
        const build = panel.receive!({ type: 'build' });
        expect(state.runs.map((r) => r.args[1])).toEqual(['build']);
        panel.dispose();
        state.runs[0].resolve('OK -> revisions/history.qmd');
        await build;
        expect(panel.afterDispose).toBe(0);
        expect(panel.posted.at(-1)).toEqual({ type: 'busy', busy: true });
    });

    it('書き戻せなかったら知らせ、画面を文書の内容に戻す', async () => {
        const panel = await openEditor();
        const ready = panel.receive!({ type: 'ready' });
        state.nexts[0].resolve(next([]));
        await ready;
        state.applyResult = false;
        const { parse } = await import('./revfile');
        const edited = parse(golden);
        edited.entries[0].note = '書き換えたメモ';
        await panel.receive!({ type: 'edit', revision: edited });
        expect(doc.text).toBe(golden);
        const notice = panel.posted.find((m) => m.type === 'notice');
        expect(notice).toMatchObject({ level: 'error', text: expect.stringContaining('書き込めませんでした') });
        const shown = revisions(panel).at(-1)?.revision as { entries: { note: string }[] };
        expect(shown.entries[0].note).not.toContain('書き換えたメモ');
    });

    it('設定の ddqPath を変えたら、次の取り直しからその ddq を使う', async () => {
        const panel = await openEditor();
        const ready = panel.receive!({ type: 'ready' });
        state.nexts[0].resolve(next([]));
        await ready;
        state.ddqPath = 'C:\\tools\\ddq-new.exe';
        typed(golden.replace('目的', '目的3'));
        vi.advanceTimersByTime(TAG_REFRESH_DELAY_MS);
        expect(state.nexts.map((n) => n.command)).toEqual(['ddq', 'C:\\tools\\ddq-new.exe']);
    });
});
