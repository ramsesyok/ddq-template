/**
 * 執筆フォルダの決定（docs/ddq-template-study.md §6）。
 *
 * Quarto の設定と、DDQ の拡張または project.type: ddq を持つフォルダを対象にする。
 *
 *   1. エクスプローラで右クリックしたフォルダ（とその親）
 *   2. アクティブエディタのファイルから親をたどる
 *   3. ワークスペースを探し、1 つならそれ、複数なら選ばせる
 */

import * as path from 'node:path';
import * as vscode from 'vscode';

export const QUARTO_YML = '_quarto.yml';
export const QUARTO_CONFIGS = [QUARTO_YML, '_quarto.yaml'] as const;
export const TEMPLATE_VERSION_FILE = '_extensions/ddq/_extension.yml';

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

export async function quartoConfig(dir: string): Promise<string | undefined> {
    for (const name of QUARTO_CONFIGS) {
        const file = path.join(dir, name);
        if (await exists(file)) return file;
    }
    return undefined;
}

export async function isWritingFolder(dir: string): Promise<boolean> {
    const config = await quartoConfig(dir);
    if (!config) return false;
    if (await exists(path.join(dir, TEMPLATE_VERSION_FILE))) return true;

    // 配線済みで、更新前の執筆フォルダもコマンドの対象にできるようにする。
    try {
        const content = new TextDecoder().decode(await vscode.workspace.fs.readFile(vscode.Uri.file(config)));
        const project = /^project:[ \t]*(.*)$/m.exec(content);
        if (!project) return false;
        const ddqType = /^(?:"ddq"|'ddq'|ddq)[ \t]*(?:#.*)?$/;
        const header = project[1].trim();
        if (header.startsWith('{')) {
            const scalar = /(?:^|,)\s*type:\s*([^,}]+)/.exec(header.slice(1))?.[1]?.trim();
            return scalar !== undefined && ddqType.test(scalar);
        }
        const lines = content.slice(project.index + project[0].length).split(/\r?\n/);
        let indent: number | undefined;
        for (const line of lines) {
            const text = line.trim();
            if (!text || text.startsWith('#')) continue;
            const depth = line.length - line.trimStart().length;
            if (!depth) break;
            indent ??= depth;
            if (depth === indent && text.startsWith('type:')) return ddqType.test(text.slice(5).trim());
        }
    } catch {
        // 構成ファイルが走査の途中で消えた場合は候補から外す。
    }
    return false;
}

/** ワークスペース内の執筆フォルダ（fsPath。並びは安定させる）。 */
export async function listWritingFolders(): Promise<string[]> {
    const found = await vscode.workspace.findFiles('**/{_quarto.yml,_quarto.yaml}', '**/{_book,_freeze,.quarto,node_modules}/**', 200);
    const dirs = [...new Set(found.map(u => path.dirname(u.fsPath)))];
    const checked = await Promise.all(dirs.map(async d => (await isWritingFolder(d)) ? d : undefined));
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
            `執筆フォルダ（${QUARTO_YML} または _quarto.yaml で DDQ を指定したフォルダ）が見つかりません。`
        );
        return undefined;
    }
    if (dirs.length === 1) return dirs[0];
    return await vscode.window.showQuickPick(dirs, { title: '執筆フォルダを選んでください' });
}

/** 配下のフォルダを数えるときに入らないフォルダ（生成物・道具の置き場） */
const SKIP_DIRS = new Set(['_book', '_freeze', '.quarto', '.git', 'node_modules', '_extensions']);

/** 右クリックメニューに載せる配下のフォルダの上限（極端に大きいワークスペースで止まらないように） */
const MAX_SUBFOLDERS = 5000;

/** 配下のフォルダを数えないパスか（生成物の中など）。 */
export function isSkipped(file: string): boolean {
    return file.split(/[\\/]/).some((part) => SKIP_DIRS.has(part));
}

/**
 * `root` の配下のフォルダ（`root` 自身は含まない。`readDir` は差し替えてテストする）。
 * 生成物のフォルダには入らず、`limit` 個で打ち切る。
 */
export async function subfolders(
    root: string,
    readDir: (dir: string) => Promise<string[]>,
    limit = MAX_SUBFOLDERS
): Promise<string[]> {
    const found: string[] = [];
    const queue = [root];
    while (queue.length > 0 && found.length < limit) {
        const dir = queue.shift() as string;
        for (const name of await readDir(dir)) {
            if (SKIP_DIRS.has(name) || found.length >= limit) continue;
            const child = path.join(dir, name);
            found.push(child);
            queue.push(child);
        }
    }
    return found;
}

async function readSubdirectories(dir: string): Promise<string[]> {
    try {
        const entries = await vscode.workspace.fs.readDirectory(vscode.Uri.file(dir));
        return entries.filter(([, type]) => (type & vscode.FileType.Directory) !== 0).map(([name]) => name);
    } catch {
        return [];
    }
}

/**
 * 右クリックメニューを執筆フォルダとその配下だけに出すための一覧
 * （`resourcePath in ddqTemplate.writingFolders`）。`when` 句ではフォルダの中身も親も
 * 見られないので、執筆フォルダと配下のフォルダのパスをキーに持つオブジェクトを渡す。
 * 起動時と、版ファイル・_quarto.yml・執筆フォルダの中身の増減時に作り直す。
 *
 * 作り直しは重なりうる（起動直後の走査中にワークスペースのフォルダが増えた、など）。
 * 遅れて終わった古い走査で新しい結果を上書きしないよう、いちばん新しい作り直しの結果だけを
 * 使う（通し番号。ラベル一覧の取り直しと同じ流儀）。拡張の終了後は何も書かない。
 */
export function watchWritingFolders(context: vscode.ExtensionContext, onChange: () => void): void {
    let writing: string[] = [];
    let generation = 0;
    let disposed = false;
    const update = async () => {
        const mine = ++generation;
        const latest = () => !disposed && mine === generation;
        const found = await listWritingFolders();
        if (!latest()) return;
        writing = found;
        const keys: Record<string, true> = {};
        for (const dir of found) {
            keys[vscode.Uri.file(dir).fsPath] = true;
            for (const sub of await subfolders(dir, readSubdirectories)) {
                keys[vscode.Uri.file(sub).fsPath] = true;
            }
        }
        if (!latest()) return;
        await vscode.commands.executeCommand('setContext', 'ddqTemplate.writingFolders', keys);
        if (!latest()) return;
        // 「執筆フォルダを追加」をリポジトリ（ルート）の右クリックに出すかどうか
        await vscode.commands.executeCommand('setContext', 'ddqTemplate.hasWritingFolders', found.length > 0);
        if (!latest()) return;
        onChange();
    };
    for (const name of [TEMPLATE_VERSION_FILE, ...QUARTO_CONFIGS]) {
        const watcher = vscode.workspace.createFileSystemWatcher(`**/${name}`, false, true, false);
        watcher.onDidCreate(update);
        watcher.onDidDelete(update);
        context.subscriptions.push(watcher);
    }

    // 執筆フォルダの中でフォルダが増減したら作り直す。ファイルとフォルダを区別できないので
    // 何かが増減したら少し待ってまとめて作り直す（生成物の中の増減は見ない）
    let timer: ReturnType<typeof setTimeout> | undefined;
    const later = (uri: vscode.Uri) => {
        const file = uri.fsPath;
        const inside = writing.some((dir) => file.startsWith(dir + path.sep));
        if (!inside || isSkipped(file)) return;
        if (timer) clearTimeout(timer);
        timer = setTimeout(() => void update(), 500);
    };
    const anything = vscode.workspace.createFileSystemWatcher('**/*', false, true, false);
    anything.onDidCreate(later);
    anything.onDidDelete(later);
    context.subscriptions.push(anything, {
        dispose: () => {
            disposed = true;
            if (timer) clearTimeout(timer);
        }
    });

    context.subscriptions.push(vscode.workspace.onDidChangeWorkspaceFolders(update));
    void update();
}
