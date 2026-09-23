/**
 * DDQ Revision — 見出し・表・図のラベル付け（docs/revision-study.md §4.4）。
 *
 * 画面は Webview の表 1 枚で、中身はすべて `ddq tag list --json` の結果である
 * （拡張は qmd を解釈しない）。書き戻しは ddq に書かせず、**拡張が `WorkspaceEdit` で
 * 当てる**。こうすると Ctrl+Z で戻せて、未保存のバッファにもそのまま当たる。
 */

import * as path from 'node:path';
import * as vscode from 'vscode';

import { DdqError, ddqCommand, run, runJson, versionOf } from './ddq/run';
import type { TagList } from './ddq/types';
import {
    EMPTY_SCHEME,
    RevisionEditorProvider,
    html,
    openDiff,
    reveal as revealEntry,
    write,
    writingFolderOf
} from './revision/provider';
import { parse } from './revision/revfile';
import { characterOffset, checkRows, rowKey, sortForApply, type Row } from './tagging/labels';
import type { FromWebviewMessage, ToWebviewMessage } from './tagging/messages';

/** 開いている一覧。執筆フォルダごとに 1 枚。 */
type Session = {
    panel: vscode.WebviewPanel;
    /** 執筆フォルダ（絶対パス） */
    folder: string;
    list?: TagList;
};

let session: Session | undefined;

export function activate(context: vscode.ExtensionContext) {
    context.subscriptions.push(
        vscode.commands.registerCommand('ddqRevision.tags', (resource?: vscode.Uri) =>
            openTags(context, resource)
        )
    );
    context.subscriptions.push(
        vscode.commands.registerCommand('ddqRevision.newRevision', (resource?: vscode.Uri) =>
            newRevision(resource)
        )
    );
    // 改訂履歴の編集画面（revisions/rev-<記号>.yml）
    context.subscriptions.push(
        vscode.window.registerCustomEditorProvider(
            RevisionEditorProvider.viewType,
            new RevisionEditorProvider(context.extensionUri),
            { webviewOptions: { retainContextWhenHidden: true }, supportsMultipleEditorsPerDocument: false }
        )
    );
    // 追加・削除の側に出す「空の文書」（§5.8）
    context.subscriptions.push(
        vscode.workspace.registerTextDocumentContentProvider(
            EMPTY_SCHEME,
            RevisionEditorProvider.emptyProvider()
        )
    );
    // 書き戻しだけを呼ぶ口。package.json に出していないのでコマンドパレットには現れない。
    // 実拡張ホストで動かす検証（tests/host）がここを使う。
    context.subscriptions.push(
        vscode.commands.registerCommand(
            'ddqRevision.internal.applyRows',
            (folder: string, list: TagList, rows: Row[]) => applyRows(folder, list, rows)
        )
    );
    // 改訂履歴の編集画面が行うことを、検証（tests/host）から直接呼ぶための口。
    // package.json に出していないのでコマンドパレットには現れない。
    context.subscriptions.push(
        vscode.commands.registerCommand(
            'ddqRevision.internal.writeNote',
            async (uri: string, note: string) => {
                const document = await vscode.workspace.openTextDocument(vscode.Uri.parse(uri));
                const revision = parse(document.getText());
                if (revision.entries.length === 0) return false;
                revision.entries[0].note = note;
                await write(document, revision);
                return true;
            }
        )
    );
    context.subscriptions.push(
        vscode.commands.registerCommand(
            'ddqRevision.internal.openDiff',
            async (uri: string, index: number) => {
                const target = vscode.Uri.parse(uri);
                const document = await vscode.workspace.openTextDocument(target);
                const revision = parse(document.getText());
                const entry = revision.entries[index];
                if (!entry) return false;
                await openDiff(writingFolderOf(target), revision, entry.label);
                return true;
            }
        )
    );
    context.subscriptions.push(
        vscode.commands.registerCommand(
            'ddqRevision.internal.reveal',
            async (uri: string, index: number) => {
                const target = vscode.Uri.parse(uri);
                const document = await vscode.workspace.openTextDocument(target);
                const entry = parse(document.getText()).entries[index];
                if (!entry) return false;
                await revealEntry(writingFolderOf(target), entry.file, entry.line);
                return true;
            }
        )
    );
    context.subscriptions.push({ dispose: dispose });
}

export function deactivate() {
    dispose();
}

function dispose() {
    session?.panel.dispose();
    session = undefined;
}

async function openTags(context: vscode.ExtensionContext, resource?: vscode.Uri) {
    const folder = await resolveWritingFolder(resource);
    if (!folder) return;

    if (session) {
        session.folder = folder;
        session.panel.reveal(vscode.ViewColumn.Active);
    } else {
        const panel = vscode.window.createWebviewPanel(
            'ddqRevision.tags',
            `ラベル一覧: ${path.basename(folder)}`,
            vscode.ViewColumn.Active,
            { enableScripts: true, retainContextWhenHidden: true }
        );
        panel.webview.html = html(panel.webview, context.extensionUri, 'tags');
        panel.onDidDispose(() => {
            session = undefined;
        });
        panel.webview.onDidReceiveMessage((m: FromWebviewMessage) => onMessage(m));
        session = { panel, folder };
    }
    await refresh();
}

/**
 * 執筆フォルダ（`_quarto.yml` のあるフォルダ）を決める。
 *
 * エクスプローラの右クリックならそのフォルダ、そうでなければ開いているファイルから
 * 上へたどる。どちらも当たらなければワークスペースから探して選ばせる。
 */
async function resolveWritingFolder(resource?: vscode.Uri): Promise<string | undefined> {
    const isWritingFolder = async (dir: string) => {
        try {
            await vscode.workspace.fs.stat(vscode.Uri.file(path.join(dir, '_quarto.yml')));
            return true;
        } catch {
            return false;
        }
    };

    if (resource && (await isWritingFolder(resource.fsPath))) return resource.fsPath;

    const active = vscode.window.activeTextEditor?.document.uri;
    if (active?.scheme === 'file') {
        let dir = path.dirname(active.fsPath);
        for (let i = 0; i < 12; i += 1) {
            if (await isWritingFolder(dir)) return dir;
            const up = path.dirname(dir);
            if (up === dir) break;
            dir = up;
        }
    }

    const found = await vscode.workspace.findFiles('**/_quarto.yml', '**/{_book,node_modules}/**', 20);
    const dirs = [...new Set(found.map((u) => path.dirname(u.fsPath)))];
    if (dirs.length === 0) {
        vscode.window.showErrorMessage(
            '執筆フォルダ（_quarto.yml のあるフォルダ）が見つかりません。'
        );
        return undefined;
    }
    if (dirs.length === 1) return dirs[0];
    return await vscode.window.showQuickPick(dirs, { title: '執筆フォルダを選んでください' });
}

/** `ddq tag list --json` を取り直して画面に流す。 */
async function refresh() {
    if (!session) return;
    const { folder } = session;
    post({ type: 'busy', busy: true });
    try {
        const command = ddqCommand(
            vscode.workspace.getConfiguration('ddqRevision').get<string>('ddqPath')
        );
        const version = versionOf(await run(command, ['--version'], folder));
        const list = await runJson<TagList>(command, ['tag', 'list', folder, '--json'], folder);
        session.list = list;
        post({ type: 'list', list, version });
    } catch (e) {
        const error = e instanceof DdqError ? e : new DdqError(String(e));
        post({ type: 'error', message: error.message, detail: error.detail });
    } finally {
        post({ type: 'busy', busy: false });
    }
}

function post(message: ToWebviewMessage) {
    void session?.panel.webview.postMessage(message);
}

async function onMessage(message: FromWebviewMessage) {
    if (!session) return;
    switch (message.type) {
        case 'ready':
        case 'reload':
            await refresh();
            return;
        case 'reveal':
            await reveal(path.join(session.folder, message.file), message.line);
            return;
        case 'apply':
            await apply(message.rows);
            return;
    }
}

/** 一覧の行から本文の該当行へ飛ぶ。 */
async function reveal(file: string, line: number) {
    try {
        const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(file));
        const at = new vscode.Position(Math.max(0, line - 1), 0);
        await vscode.window.showTextDocument(doc, {
            viewColumn: vscode.ViewColumn.Beside,
            selection: new vscode.Selection(at, at),
            preserveFocus: true
        });
    } catch {
        vscode.window.showErrorMessage(`${file} を開けません。`);
    }
}

/**
 * チェックの付いた行のラベルを書き戻す。
 *
 * `WorkspaceEdit` 1 つにまとめるので、Ctrl+Z 一回で全部戻る。当てる前に、一覧を
 * 取った時点から行が動いていないかを実際のバッファで確かめる（別の場所を書き潰さない）。
 */
async function apply(rows: Row[]) {
    if (!session?.list) return;
    const result = await applyRows(session.folder, session.list, rows);
    if (result.errors.length > 0) {
        post({ type: 'errors', errors: result.errors });
        return;
    }
    await refresh();
}

/** 書き戻しの実体。画面（session）に依存しないので、検証から直接呼べる。 */
export async function applyRows(
    folder: string,
    list: TagList,
    rows: Row[]
): Promise<{ applied: number; skipped: string[]; errors: [string, string][] }> {
    const { edits, errors } = checkRows(rows, list.items);
    if (errors.size > 0) {
        vscode.window.showErrorMessage(
            `${errors.size} 件のラベルが使えません。一覧の赤い行を直してください。`
        );
        return { applied: 0, skipped: [], errors: [...errors] };
    }
    if (edits.length === 0) {
        vscode.window.showInformationMessage('書き戻すものがありません。');
        return { applied: 0, skipped: [], errors: [] };
    }

    const edit = new vscode.WorkspaceEdit();
    const stale: string[] = [];
    for (const e of sortForApply(edits)) {
        const uri = vscode.Uri.file(path.join(folder, e.file));
        let doc: vscode.TextDocument;
        try {
            doc = await vscode.workspace.openTextDocument(uri);
        } catch {
            stale.push(`${e.file}（開けません）`);
            continue;
        }
        if (e.line > doc.lineCount) {
            stale.push(`${e.file}:${e.line}（行がありません）`);
            continue;
        }
        const text = doc.lineAt(e.line - 1).text;
        // 一覧を取ったあとに誰かが編集していたら、その行は当てない
        const item = list.items.find((i) => rowKey(i) === `${e.file}:${e.line}`);
        if (item?.label) {
            stale.push(`${e.file}:${e.line}（既にラベルがあります）`);
            continue;
        }
        if (hasLabel(text)) {
            stale.push(`${e.file}:${e.line}（文書が変わりました）`);
            continue;
        }
        edit.insert(uri, new vscode.Position(e.line - 1, characterOffset(text, e.col)), e.insert);
    }

    const ok = await vscode.workspace.applyEdit(edit);
    if (!ok) {
        vscode.window.showErrorMessage('書き戻しに失敗しました。');
        return { applied: 0, skipped: stale, errors: [] };
    }
    const applied = edits.length - stale.length;
    if (stale.length > 0) {
        vscode.window.showWarningMessage(
            `${applied} 件を書き戻しました。${stale.length} 件は飛ばしました: ${stale.join(' / ')}`
        );
    } else {
        vscode.window.showInformationMessage(
            `${applied} 件のラベルを書き戻しました（Ctrl+Z で戻せます。保存すると確定します）。`
        );
    }
    return { applied, skipped: stale, errors: [] };
}

/** その行に既にラベルが書かれているか（書き戻しの前の確認）。 */
function hasLabel(line: string): boolean {
    return /\{[^}]*#(sec|tbl|fig)-|label\s*=\s*"/.test(line);
}



/**
 * 新しい改訂を始める。
 *
 * `ddq rev next` が決めた記号と基準で `rev diff --write` を走らせ、できた
 * `revisions/rev-<記号>.yml` を編集画面で開く。基準のタグが無いときは人に選ばせる。
 */
async function newRevision(resource?: vscode.Uri) {
    const folder = await resolveWritingFolder(resource);
    if (!folder) return;
    const command = ddqCommand(
        vscode.workspace.getConfiguration('ddqRevision').get<string>('ddqPath')
    );
    try {
        const next = await runJson<{ rev: string; base: string | null; drafts: string[] }>(
            command,
            ['rev', 'next', folder, '--json'],
            folder
        );
        let base = next.base ?? undefined;
        if (!base) {
            base = await vscode.window.showInputBox({
                title: '比較の基準',
                prompt: 'rev-<記号> のタグがありません。タグ・ブランチ・コミット ID を入れてください',
                placeHolder: 'HEAD'
            });
            if (!base) return;
        }
        if (next.drafts.length > 0) {
            vscode.window.showInformationMessage(
                `書きかけの改訂 ${next.drafts.join(', ')} を続けます（新しい記号は作りません）。`
            );
        }
        const args = ['rev', 'diff', folder, '--write', '--base', base];
        const out = await run(command, args, folder);
        const file = vscode.Uri.file(path.join(folder, 'revisions', `rev-${next.rev}.yml`));
        await vscode.commands.executeCommand(
            'vscode.openWith',
            file,
            RevisionEditorProvider.viewType
        );
        const warnings = out
            .split(String.fromCharCode(10))
            .filter((l) => l.startsWith('警告'))
            .join(String.fromCharCode(10));
        if (warnings !== '') vscode.window.showWarningMessage(warnings);
    } catch (e) {
        const error = e instanceof DdqError ? e : new DdqError(String(e));
        vscode.window.showErrorMessage([error.message, error.detail].filter(Boolean).join(String.fromCharCode(10)));
    }
}
