/**
 * 執筆フォルダの決定（docs/ddq-template-study.md §6）。
 *
 * 執筆フォルダ = `_quarto.yml` とテンプレートの版ファイル（`.template-version`）がある
 * フォルダ。版ファイルで、ddq の文書と素の Quarto プロジェクトを見分ける。
 *
 *   1. エクスプローラで右クリックしたフォルダ（とその親）
 *   2. アクティブエディタのファイルから親をたどる
 *   3. ワークスペースを探し、1 つならそれ、複数なら選ばせる
 */

import * as path from 'node:path';
import * as vscode from 'vscode';

export const QUARTO_YML = '_quarto.yml';
export const TEMPLATE_VERSION_FILE = '.template-version';

/** 親をたどる上限（ドライブの根まで行かないように） */
const MAX_DEPTH = 12;

/** `start` から親へたどって最初の執筆フォルダ（`isWritingFolder` は差し替えてテストする）。 */
export async function nearestWritingFolder(
    start: string,
    isWritingFolder: (dir: string) => Promise<boolean>
): Promise<string | undefined> {
    let dir = start;
    for (let i = 0; i < MAX_DEPTH; i += 1) {
        if (await isWritingFolder(dir)) return dir;
        const up = path.dirname(dir);
        if (up === dir) break;
        dir = up;
    }
    return undefined;
}

async function exists(file: string): Promise<boolean> {
    try {
        await vscode.workspace.fs.stat(vscode.Uri.file(file));
        return true;
    } catch {
        return false;
    }
}

export async function isWritingFolder(dir: string): Promise<boolean> {
    return (await exists(path.join(dir, QUARTO_YML))) && (await exists(path.join(dir, TEMPLATE_VERSION_FILE)));
}

/** ワークスペース内の執筆フォルダ（fsPath。並びは安定させる）。 */
export async function listWritingFolders(): Promise<string[]> {
    const found = await vscode.workspace.findFiles(`**/${TEMPLATE_VERSION_FILE}`, '**/{_book,node_modules}/**', 200);
    const dirs = [...new Set(found.map((u) => path.dirname(u.fsPath)))];
    const checked = await Promise.all(dirs.map(async (d) => ((await exists(path.join(d, QUARTO_YML))) ? d : undefined)));
    return checked.filter((d): d is string => d !== undefined).sort();
}

/** 開いているファイルが属する執筆フォルダ（無ければ undefined）。 */
export async function activeWritingFolder(): Promise<string | undefined> {
    const active = vscode.window.activeTextEditor?.document.uri;
    if (active?.scheme !== 'file') return undefined;
    return nearestWritingFolder(path.dirname(active.fsPath), isWritingFolder);
}

/** コマンドの対象の執筆フォルダを決める。決まらなければ理由を出して undefined。 */
export async function resolveWritingFolder(resource?: vscode.Uri): Promise<string | undefined> {
    if (resource?.scheme === 'file') {
        const fromResource = await nearestWritingFolder(resource.fsPath, isWritingFolder);
        if (fromResource) return fromResource;
    }
    const fromEditor = await activeWritingFolder();
    if (fromEditor) return fromEditor;

    const dirs = await listWritingFolders();
    if (dirs.length === 0) {
        vscode.window.showErrorMessage(
            `執筆フォルダ（${QUARTO_YML} と ${TEMPLATE_VERSION_FILE} のあるフォルダ）が見つかりません。`
        );
        return undefined;
    }
    if (dirs.length === 1) return dirs[0];
    return await vscode.window.showQuickPick(dirs, { title: '執筆フォルダを選んでください' });
}

/**
 * 右クリックメニューを執筆フォルダだけに出すための一覧（`resourcePath in ddqTemplate.writingFolders`）。
 * `when` 句ではフォルダの中身を見られないので、起動時と版ファイル・_quarto.yml の増減時に作り直す。
 */
export function watchWritingFolders(context: vscode.ExtensionContext, onChange: () => void): void {
    const update = async () => {
        const dirs = await listWritingFolders();
        await vscode.commands.executeCommand(
            'setContext',
            'ddqTemplate.writingFolders',
            dirs.map((d) => vscode.Uri.file(d).fsPath)
        );
        onChange();
    };
    for (const name of [TEMPLATE_VERSION_FILE, QUARTO_YML]) {
        const watcher = vscode.workspace.createFileSystemWatcher(`**/${name}`, false, true, false);
        watcher.onDidCreate(update);
        watcher.onDidDelete(update);
        context.subscriptions.push(watcher);
    }
    context.subscriptions.push(vscode.workspace.onDidChangeWorkspaceFolders(update));
    void update();
}
