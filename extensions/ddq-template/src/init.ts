/**
 * 設計書リポジトリの新規作成（`ddq init`。docs/ddq-template-study.md §5）。
 *
 * コマンドは 1 つで、状況により入口を変える。
 *   - フォルダを開いていない     … 親フォルダとリポジトリ名を選ぶ → 作る → そのフォルダを開く
 *   - 空（ほぼ空）のフォルダを開いている … そこに作る → `_quarto.yml` を開く
 *   - 既に設計書リポジトリ       … 作らない（執筆フォルダの追加は `ddq add`。段階 3）
 * 作り終えたら「Git リポジトリとして初期化」を出す（組み込みの `git.init`。ddq は git を扱わない）。
 * フォルダを開き直すとウィンドウが読み直されるので、その案内は開き直した先で出す（globalState）。
 */

import * as fs from 'node:fs';
import * as path from 'node:path';
import * as vscode from 'vscode';

import { channel, runWithProgress, settings, showError } from './build';
import { tryLock } from './ddq/lock';
import { QUARTO_YML, listWritingFolders } from './folders';
import { envWithQuarto, hasQuarto } from './quarto';

/** 「ほぼ空」とみなして、そのまま作ってよい中身（ddq init は既存ファイルを触らない） */
const NEGLIGIBLE = new Set(['.git', '.vscode', '.gitignore', '.gitattributes', 'readme.md', 'license', 'license.md']);

/** フォルダの中身の名前から、そのまま作ってよいか。 */
export function isNearlyEmpty(entries: string[]): boolean {
    return entries.every((name) => NEGLIGIBLE.has(name.toLowerCase()));
}

/** 執筆フォルダ名・リポジトリ名の検査（ddq と同じ規則 + Windows で作れない名前）。null なら可。 */
export function validateFolderName(name: string): string | null {
    const trimmed = name.trim();
    if (trimmed === '') return '名前を入れてください。';
    if (trimmed === '.' || trimmed === '..') return '. や .. は使えません。';
    if (/[\\/]/.test(trimmed)) return 'フォルダ名だけを入れてください（区切り文字は使えません）。';
    if (/[<>:"|?*]/.test(trimmed)) return '< > : " | ? * は使えません。';
    if (/[. ]$/.test(trimmed)) return '末尾に . や空白は置けません。';
    return null;
}

/** 新しく作るリポジトリのフォルダ `target` を使えるか。null なら可（無い・ほぼ空のフォルダ）。 */
export function checkTarget(target: string): string | null {
    let stat: fs.Stats;
    try {
        stat = fs.statSync(target);
    } catch {
        return null;
    }
    if (!stat.isDirectory()) return `${target} は既にファイルとしてあります。別の名前にしてください。`;
    return isNearlyEmpty(fs.readdirSync(target)) ? null : `${target} は既にあって空ではありません。`;
}

/** `root` の中（`root` 自身を含む）にある執筆フォルダ。 */
export function writingFoldersIn(root: string, writingFolders: string[]): string[] {
    const prefix = root.endsWith(path.sep) ? root : root + path.sep;
    return writingFolders.filter((dir) => dir === root || dir.startsWith(prefix));
}

/** 開き直した先で完了の案内を出すための記録 */
const PENDING_KEY = 'ddqTemplate.pendingInit';
type Pending = { repo: string; writingFolder: string };

export type InitOutcome = 'ok' | 'failed' | 'cancelled' | 'busy' | 'exists';

export function activateInit(context: vscode.ExtensionContext) {
    context.subscriptions.push(vscode.commands.registerCommand('ddqTemplate.init', () => initCommand(context)));
    // 実拡張ホストの検証用（ダイアログを通さずに同じ作成処理を通す）
    context.subscriptions.push(
        vscode.commands.registerCommand('ddqTemplate.internal.init', (repo: string, writingFolder: string) =>
            createRepository(repo, writingFolder)
        )
    );
    void showPendingNotice(context);
}

async function initCommand(context: vscode.ExtensionContext): Promise<InitOutcome | undefined> {
    const folders = vscode.workspace.workspaceFolders ?? [];
    if (folders.length === 0) return initNew(context);

    // 複数ルートなら先に作る場所を選び、そのルートの中だけで既存の文書を見る
    // （別のルートに文書があっても、空のルートには作れるように）
    const here = folders.length === 1 ? folders[0].uri.fsPath : await pickWorkspaceFolder(folders);
    if (!here) return undefined;
    if (writingFoldersIn(here, await listWritingFolders()).length > 0) {
        void vscode.window.showInformationMessage(
            `${path.basename(here)} は既に設計書リポジトリです（執筆フォルダがあります）。` +
                '文書を増やすときは ddq add を使います（拡張からの実行は今後対応）。'
        );
        return 'exists';
    }
    if (!isNearlyEmpty(fs.readdirSync(here))) {
        const inHere = 'このフォルダに作る';
        const elsewhere = '別の場所に新しく作る';
        const choice = await vscode.window.showWarningMessage(
            `${path.basename(here)} には既にファイルがあります。既存のファイルは上書きしませんが、` +
                'リポジトリの直下に .gitignore や README.md、執筆フォルダが加わります。',
            { modal: true },
            inHere,
            elsewhere
        );
        if (choice === elsewhere) return initNew(context);
        if (choice !== inHere) return undefined;
    }

    const writingFolder = await askWritingFolderName();
    if (!writingFolder) return undefined;
    const outcome = await createRepository(here, writingFolder);
    if (outcome === 'ok') await showCreated(here, writingFolder);
    return outcome;
}

async function pickWorkspaceFolder(folders: readonly vscode.WorkspaceFolder[]): Promise<string | undefined> {
    const picked = await vscode.window.showQuickPick(
        folders.map((f) => ({ label: f.name, description: f.uri.fsPath, path: f.uri.fsPath })),
        { title: '設計書リポジトリにするフォルダを選んでください' }
    );
    return picked?.path;
}

/** フォルダを開いていないとき: 親フォルダ → リポジトリ名 → 執筆フォルダ名 → 作成 → 開き直す */
async function initNew(context: vscode.ExtensionContext): Promise<InitOutcome | undefined> {
    const parent = await vscode.window.showOpenDialog({
        canSelectFolders: true,
        canSelectFiles: false,
        canSelectMany: false,
        title: '設計書リポジトリを作る場所（親フォルダ）を選んでください',
        openLabel: 'ここに作る'
    });
    if (!parent?.[0]) return undefined;
    const parentDir = parent[0].fsPath;

    const name = await vscode.window.showInputBox({
        title: '設計書リポジトリの名前（フォルダ名）',
        prompt: `${parentDir} の下に作ります`,
        placeHolder: '例: order-system-design',
        validateInput: (value) => {
            const invalid = validateFolderName(value);
            if (invalid) return invalid;
            return checkTarget(path.join(parentDir, value.trim()));
        }
    });
    if (!name) return undefined;
    const repo = path.join(parentDir, name.trim());

    const writingFolder = await askWritingFolderName();
    if (!writingFolder) return undefined;

    const outcome = await createRepository(repo, writingFolder);
    if (outcome !== 'ok') return outcome;

    // 開き直すと拡張も読み直されるので、完了の案内は開き直した先で出す
    await context.globalState.update(PENDING_KEY, { repo, writingFolder } satisfies Pending);
    await vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(repo), { forceReuseWindow: true });
    return outcome;
}

async function askWritingFolderName(): Promise<string | undefined> {
    const name = await vscode.window.showInputBox({
        title: '執筆フォルダの名前',
        prompt: '文書 1 つ分のフォルダです。あとから増やせます（ddq add）',
        value: 'docs',
        validateInput: validateFolderName
    });
    return name?.trim() || undefined;
}

/**
 * `ddq init <repo> <執筆フォルダ>` を走らせる。Quarto が無ければ最後の疎通確認
 * （`quarto render`）を省いて作り、その旨を出す（作ること自体は Quarto 無しでできる）。
 */
export async function createRepository(repo: string, writingFolder: string): Promise<InitOutcome> {
    const release = tryLock(repo);
    if (!release) {
        void vscode.window.showInformationMessage(`${path.basename(repo)} では ddq を実行中です。`);
        return 'busy';
    }
    try {
        const env = envWithQuarto(settings().quartoPath);
        const quarto = await hasQuarto(env);
        const args = ['init', repo, writingFolder, ...(quarto ? [] : ['--no-render'])];
        const result = await runWithProgress(`設計書リポジトリを作成: ${path.basename(repo)}`, args, undefined, env);
        if (result === 'cancelled') return 'cancelled';
        if (result.code !== 0) {
            void showError(result.error ?? `ddq init が失敗しました（終了コード ${result.code}）。`);
            return 'failed';
        }
        if (!quarto) {
            channel().appendLine(
                'Quarto が見つからないので、作成後の確認（quarto render）を省きました。Quarto を入れてから PDF / HTML を作ってください。'
            );
        }
        return 'ok';
    } finally {
        release();
    }
}

/** 作り終えた案内: `_quarto.yml` を開き、Git の初期化を勧める。 */
async function showCreated(repo: string, writingFolder: string) {
    const yml = path.join(repo, writingFolder, QUARTO_YML);
    if (fs.existsSync(yml)) {
        await vscode.window.showTextDocument(vscode.Uri.file(yml), { preview: false });
    }
    const hasGit = fs.existsSync(path.join(repo, '.git'));
    const gitInit = 'Git リポジトリとして初期化';
    const message =
        `設計書リポジトリを作りました（執筆フォルダ: ${writingFolder}）。` +
        `${QUARTO_YML} の表題・資料番号・会社名・章立てを直してから書き始めてください。`;
    const choice = hasGit
        ? await vscode.window.showInformationMessage(message)
        : await vscode.window.showInformationMessage(message, gitInit);
    // skipFolderPrompt = true: ワークスペースが 1 フォルダならそこを初期化する
    if (choice === gitInit) await vscode.commands.executeCommand('git.init', true);
}

/** フォルダを開き直した直後に、作成の案内を出す。 */
async function showPendingNotice(context: vscode.ExtensionContext) {
    const pending = context.globalState.get<Pending>(PENDING_KEY);
    if (!pending) return;
    const opened = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    if (!opened || path.resolve(opened).toLowerCase() !== path.resolve(pending.repo).toLowerCase()) return;
    await context.globalState.update(PENDING_KEY, undefined);
    await showCreated(pending.repo, pending.writingFolder);
}
