/**
 * 通知・ダイアログの操作をテストするための VSCode の差し替え（vitest 用。拡張には入らない）。
 *
 *   vi.mock('vscode', () => import('./testing/fakeVscode'));
 *   import { fake } from './testing/fakeVscode';
 *
 * - ダイアログ（showInformationMessage / showWarningMessage / showErrorMessage / showInputBox /
 *   showOpenDialog / showQuickPick）は `fake.answer(名前, 値…)` で積んだ答えを順に返す。
 *   積んでいなければ undefined（＝閉じた・Esc）。呼ばれたものは `fake.calls` に残る
 * - showInputBox は答えを `validateInput` に通す。通らなければその理由を `fake.rejected` に残し、
 *   次の答えを使う（本物の入力欄は、通らない値では確定できないため）
 * - commands.executeCommand は登録済みのコマンドなら呼び、どちらも `fake.executed` に残す
 *   （`setContext` の値は `fake.context` にも入る）
 * - workspace.fs は本物のファイルシステムを見る。findFiles は `fake.found` を返す
 */

import * as fs from 'node:fs';

type Handler = (...args: any[]) => unknown;
type Call = { method: string; args: unknown[] };

export const fake = {
    calls: [] as Call[],
    answers: new Map<string, unknown[]>(),
    rejected: [] as string[],
    commands: new Map<string, Handler>(),
    executed: [] as { id: string; args: unknown[] }[],
    context: {} as Record<string, unknown>,
    workspaceFolders: undefined as { name: string; uri: Uri }[] | undefined,
    /** workspace.findFiles が返すファイル（fsPath） */
    found: [] as string[],
    /** 出力チャネルに書かれた文字と、show が呼ばれた回数 */
    output: '',
    outputShown: 0,
    cancel: undefined as (() => void) | undefined,

    reset() {
        this.calls = [];
        this.answers.clear();
        this.rejected = [];
        this.commands.clear();
        this.executed = [];
        this.context = {};
        this.workspaceFolders = undefined;
        this.found = [];
        this.output = '';
        this.outputShown = 0;
        this.cancel = undefined;
    },
    answer(method: string, ...values: unknown[]) {
        this.answers.set(method, [...(this.answers.get(method) ?? []), ...values]);
    },
    /** `method` が呼ばれたときの最初の引数（通知なら本文、入力欄なら options） */
    shown(method: string): unknown[] {
        return this.calls.filter((c) => c.method === method).map((c) => c.args[0]);
    },
    /** 登録されたコマンドを呼ぶ（エクスプローラの右クリックやコマンドパレットから選んだのと同じ） */
    run(id: string, ...args: unknown[]) {
        const handler = this.commands.get(id);
        if (!handler) throw new Error(`コマンド ${id} が登録されていない`);
        return handler(...args);
    },
    setFolders(...paths: string[]) {
        this.workspaceFolders = paths.map((p) => ({ name: p.split(/[\\/]/).pop() ?? p, uri: Uri.file(p) }));
    }
};

function next(method: string): unknown {
    return fake.answers.get(method)?.shift();
}

function dialog(method: string) {
    return async (...args: unknown[]) => {
        fake.calls.push({ method, args });
        return next(method);
    };
}

export class Uri {
    readonly scheme = 'file';
    private constructor(readonly fsPath: string) {}
    static file(fsPath: string) {
        return new Uri(fsPath);
    }
    toString() {
        return `file://${this.fsPath}`;
    }
}

export const ProgressLocation = { Notification: 15 };
export const StatusBarAlignment = { Left: 1, Right: 2 };
export const FileType = { File: 1, Directory: 2 };
export const ViewColumn = { Active: -1, Beside: -2 };

const disposable = { dispose() {} };
const event = () => disposable;

export const window = {
    showInformationMessage: dialog('showInformationMessage'),
    showWarningMessage: dialog('showWarningMessage'),
    showErrorMessage: dialog('showErrorMessage'),
    showOpenDialog: dialog('showOpenDialog'),
    showQuickPick: dialog('showQuickPick'),
    showTextDocument: dialog('showTextDocument'),
    async showInputBox(options: { validateInput?: (v: string) => string | null | undefined }) {
        fake.calls.push({ method: 'showInputBox', args: [options] });
        for (;;) {
            const value = next('showInputBox') as string | undefined;
            if (value === undefined) return undefined;
            const invalid = options.validateInput?.(value);
            if (!invalid) return value;
            fake.rejected.push(invalid);
        }
    },
    async withProgress(
        options: unknown,
        task: (progress: unknown, token: { onCancellationRequested: (cb: () => void) => typeof disposable }) => unknown
    ) {
        fake.calls.push({ method: 'withProgress', args: [options] });
        return task(
            { report() {} },
            {
                onCancellationRequested: (cb) => {
                    fake.cancel = cb;
                    return disposable;
                }
            }
        );
    },
    createOutputChannel: () => ({
        append: (text: string) => (fake.output += text),
        appendLine: (text: string) => (fake.output += `${text}\n`),
        show: () => (fake.outputShown += 1),
        dispose() {}
    }),
    createStatusBarItem: () => ({ show() {}, hide() {}, dispose() {} }),
    onDidChangeActiveTextEditor: event,
    activeTextEditor: undefined
};

export const commands = {
    registerCommand(id: string, handler: Handler) {
        fake.commands.set(id, handler);
        return disposable;
    },
    async executeCommand(id: string, ...args: unknown[]) {
        fake.executed.push({ id, args });
        if (id === 'setContext') fake.context[args[0] as string] = args[1];
        return fake.commands.get(id)?.(...args);
    }
};

export const workspace = {
    get workspaceFolders() {
        return fake.workspaceFolders;
    },
    getConfiguration: () => ({ get: () => undefined }),
    async findFiles() {
        return fake.found.map((p) => Uri.file(p));
    },
    fs: {
        async stat(uri: Uri) {
            return fs.statSync(uri.fsPath);
        },
        async readDirectory(uri: Uri) {
            return fs
                .readdirSync(uri.fsPath, { withFileTypes: true })
                .map((e) => [e.name, e.isDirectory() ? FileType.Directory : FileType.File] as const);
        }
    },
    createFileSystemWatcher: () => ({ onDidCreate: event, onDidDelete: event, dispose() {} }),
    onDidChangeWorkspaceFolders: event
};

/** activate* に渡す ExtensionContext（globalState は Map で持つ） */
export function extensionContext() {
    const state = new Map<string, unknown>();
    return {
        subscriptions: [] as { dispose(): void }[],
        globalState: {
            get: (key: string) => state.get(key),
            update: async (key: string, value: unknown) => {
                if (value === undefined) state.delete(key);
                else state.set(key, value);
            }
        },
        extension: { packageJSON: { version: '0.0.0' } }
    };
}
