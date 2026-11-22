/**
 * 表エディタのパネルと編集セッション（EXT-03）。VSCode は差し替え、確認ダイアログの答えを
 * テストが止めておく。ダイアログの間に別の表を開き直す・パネルを閉じる・拡張が終わる、
 * 閉じてから開き直す、未保存の編集で行がずれる、を見る。パネルは 1 枚を使い回す（現行どおり）。
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';

type Message = { type: string; [key: string]: unknown };

class FakePanel {
    posted: Message[] = [];
    disposed = false;
    receive: ((m: Message) => Promise<void>) | undefined;
    private onDispose: (() => void)[] = [];
    reveal = vi.fn();
    webview = {
        html: '',
        cspSource: 'csp',
        asWebviewUri: (u: unknown) => u,
        postMessage: async (m: Message) => {
            if (this.disposed) throw new Error('Webview is disposed');
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
        if (this.disposed) return;
        this.disposed = true;
        for (const cb of this.onDispose) cb();
    }
}

/** 行の配列を持つ文書。applyEdit で本当に書き換わる */
class FakeDocument {
    version = 1;
    isClosed = false;
    uri = { fsPath: 'C:\\docs\\chapters\\01.qmd' };
    constructor(public lines: string[]) {}
    getText() {
        return this.lines.join('\n');
    }
    get lineCount() {
        return this.lines.length;
    }
    lineAt(n: number) {
        return { text: this.lines[n] };
    }
}

const state = {
    panels: [] as FakePanel[],
    commands: new Map<string, () => unknown>(),
    editor: undefined as { document: FakeDocument; selection: { active: { line: number } } } | undefined,
    warning: undefined as ((answer: string | undefined) => void) | undefined,
    warnings: [] as string[],
    errors: [] as string[],
    edits: 0
};

vi.mock('vscode', () => {
    class Position {
        constructor(
            public line: number,
            public character: number
        ) {}
    }
    class Range {
        constructor(
            public start: Position,
            public end: Position
        ) {}
    }
    class WorkspaceEdit {
        ops: { range?: Range; at?: Position; text: string }[] = [];
        replace(_uri: unknown, range: Range, text: string) {
            this.ops.push({ range, text });
        }
        insert(_uri: unknown, at: Position, text: string) {
            this.ops.push({ at, text });
        }
    }
    return {
        Position,
        Range,
        WorkspaceEdit,
        ViewColumn: { Beside: -2 },
        Uri: { joinPath: (...parts: unknown[]) => parts.join('/') },
        commands: {
            registerCommand: (id: string, cb: () => unknown) => {
                state.commands.set(id, cb);
                return { dispose() {} };
            }
        },
        window: {
            get activeTextEditor() {
                return state.editor;
            },
            createWebviewPanel: () => {
                const panel = new FakePanel();
                state.panels.push(panel);
                return panel;
            },
            showErrorMessage: async (m: string) => {
                state.errors.push(m);
            },
            showWarningMessage: (m: string) => {
                state.warnings.push(m);
                return new Promise((resolve) => (state.warning = resolve));
            }
        },
        workspace: {
            applyEdit: async (edit: WorkspaceEdit) => {
                state.edits += 1;
                const doc = state.editor!.document;
                for (const op of edit.ops) {
                    if (!op.range) throw new Error('この試験では挿入を使わない');
                    const { start, end } = op.range;
                    doc.lines.splice(start.line, end.line - start.line + 1, ...op.text.split('\n'));
                }
                doc.version += 1;
                return true;
            }
        }
    };
});

import { activateTableEditor, deactivateTableEditor } from './activateTableEditor';

// 表 A は label と .unnumbered を併せ持つので、書き戻しの前に確認ダイアログが出る
const DOC = [
    '# 見出し',
    '',
    '::: {.tbl .unnumbered label="tbl-a" caption="表A"}',
    '| a | b |',
    '|---|---|',
    '| 1 | 2 |',
    ':::',
    '',
    '::: {.tbl caption="表B"}',
    '| c | d |',
    '|---|---|',
    '| 3 | 4 |',
    ':::'
];
const TABLE_A = 3;
const TABLE_B = 9;

let context: { subscriptions: { dispose(): void }[]; extensionUri: string };

function open(document: FakeDocument, line: number) {
    state.editor = { document, selection: { active: { line } } };
    return state.commands.get('ddqTemplate.openTableEditor')!();
}

/** パネルに最後に送られた init の表 */
function lastInit(panel: FakePanel) {
    const init = panel.posted.filter((m) => m.type === 'init').at(-1);
    return init?.model as { rows: { text: string }[][] } | undefined;
}

/** 表エディタで最初のデータセルを書き換えて Apply する（応答を待たない） */
function applyFirstCell(panel: FakePanel, text: string) {
    const model = structuredClone(lastInit(panel)) as { rows: { text: string }[][] };
    model.rows[1][0].text = text;
    return panel.receive!({ type: 'apply', model });
}

async function untilWarning() {
    for (let i = 0; i < 100 && !state.warning; i++) await new Promise((r) => setTimeout(r, 1));
    expect(state.warning).toBeDefined();
}

beforeEach(() => {
    state.panels = [];
    state.commands.clear();
    state.editor = undefined;
    state.warning = undefined;
    state.warnings = [];
    state.errors = [];
    state.edits = 0;
    context = { subscriptions: [], extensionUri: 'ext' };
    activateTableEditor(context as never);
});

describe('表エディタの編集セッション', () => {
    it('確認ダイアログの間に別の表を開き直したら、書き戻さない（開き直した表はそのまま編集できる）', async () => {
        const doc = new FakeDocument([...DOC]);
        open(doc, TABLE_A);
        const [panel] = state.panels;
        await panel.receive!({ type: 'ready' });
        const pending = applyFirstCell(panel, 'A を直した');
        await untilWarning();
        open(doc, TABLE_B);
        expect(state.panels).toHaveLength(1);
        expect(lastInit(panel)?.rows[0][0].text).toBe('c');
        state.warning!('続行する');
        await pending;
        expect(state.edits).toBe(0);
        expect(doc.lines).toEqual(DOC);
        expect(panel.posted.at(-1)).toMatchObject({ type: 'error', message: expect.stringContaining('別の表を開いた') });
        // 開き直した表 B は書き戻せる（確認の要らない表）
        state.warning = undefined;
        await applyFirstCell(panel, 'B を直した');
        expect(state.edits).toBe(1);
        expect(doc.lines[TABLE_B + 2]).toContain('B を直した');
        expect(doc.lines[TABLE_A + 2]).toBe('| 1 | 2 |');
        expect(panel.posted.at(-1)).toEqual({ type: 'applied' });
    });

    it('確認ダイアログの間にパネルを閉じたら、書き戻さず、閉じたパネルにも送らない', async () => {
        const doc = new FakeDocument([...DOC]);
        open(doc, TABLE_A);
        const [panel] = state.panels;
        await panel.receive!({ type: 'ready' });
        const pending = applyFirstCell(panel, 'A を直した');
        await untilWarning();
        panel.dispose();
        state.warning!('続行する');
        await expect(pending).resolves.toBeUndefined();
        expect(state.edits).toBe(0);
        expect(doc.lines).toEqual(DOC);
    });

    it('確認ダイアログの間に拡張が終わったら、書き戻さない', async () => {
        const doc = new FakeDocument([...DOC]);
        open(doc, TABLE_A);
        const [panel] = state.panels;
        await panel.receive!({ type: 'ready' });
        const pending = applyFirstCell(panel, 'A を直した');
        await untilWarning();
        deactivateTableEditor();
        expect(panel.disposed).toBe(true);
        state.warning!('続行する');
        await expect(pending).resolves.toBeUndefined();
        expect(state.edits).toBe(0);
    });

    it('確認して続行すれば書き戻し、取り直した表を送る', async () => {
        const doc = new FakeDocument([...DOC]);
        open(doc, TABLE_A);
        const [panel] = state.panels;
        await panel.receive!({ type: 'ready' });
        const pending = applyFirstCell(panel, 'A を直した');
        await untilWarning();
        state.warning!('続行する');
        await pending;
        expect(state.edits).toBe(1);
        expect(doc.lines[TABLE_A + 2]).toContain('A を直した');
        expect(panel.posted.slice(-2).map((m) => m.type)).toEqual(['init', 'applied']);
        expect(lastInit(panel)?.rows[1][0].text).toBe('A を直した');
    });

    it('閉じてから開き直すと新しいパネルになり、閉じたパネルから遅れて届いたものは扱わない', async () => {
        const doc = new FakeDocument([...DOC]);
        open(doc, TABLE_A);
        const [first] = state.panels;
        await first.receive!({ type: 'close' });
        expect(first.disposed).toBe(true);
        open(doc, TABLE_B);
        expect(state.panels).toHaveLength(2);
        const second = state.panels[1];
        await second.receive!({ type: 'ready' });
        expect(lastInit(second)?.rows[0][0].text).toBe('c');
        // 閉じたパネルの Webview から遅れて届いた Apply
        await first.receive!({ type: 'apply', model: lastInit(second) });
        expect(state.edits).toBe(0);
        expect(second.posted.map((m) => m.type)).toEqual(['init']);
    });

    it('開いた後に文書が変わっていれば（未保存の編集を含む）書き戻さない', async () => {
        const doc = new FakeDocument([...DOC]);
        open(doc, TABLE_B);
        const [panel] = state.panels;
        await panel.receive!({ type: 'ready' });
        doc.lines.splice(0, 0, '追記した行');
        doc.version += 1;
        await applyFirstCell(panel, 'B を直した');
        expect(state.edits).toBe(0);
        expect(panel.posted.at(-1)).toMatchObject({ type: 'error', message: expect.stringContaining('変更されました') });
    });
});
