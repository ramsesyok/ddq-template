/**
 * 既存の設計書リポジトリに執筆フォルダ（2 つ目以降の文書）を足す（`ddq add`）。
 *
 * 入口はエクスプローラのルートフォルダの右クリック、またはコマンドパレット。
 * 足す場所はワークスペースのルート直下（入れ子に置きたいときはコマンドの `ddq add` を使う）。
 * 設計書リポジトリでない（執筆フォルダが 1 つも無い）ルートには足さず、作成を案内する。
 */

import * as fs from 'node:fs';
import * as path from 'node:path';
import * as vscode from 'vscode';

import { channel, runWithProgress, settings, showError } from './build';
import { tryLock } from './ddq/lock';
import { QUARTO_YML, listWritingFolders } from './folders';
import { validateFolderName, writingFoldersIn } from './init';
import { envWithQuarto, hasQuarto } from './quarto';

export type AddOutcome = 'ok' | 'failed' | 'cancelled' | 'busy' | 'notRepository';

/** 足す先 `target` を使えるか。null なら可（無いフォルダ・`_quarto.yml` の無いフォルダ）。 */
export function checkAddTarget(target: string): string | null {
    let stat: fs.Stats;
    try {
        stat = fs.statSync(target);
    } catch {
        return null;
    }
    if (!stat.isDirectory()) return `${target} は既にファイルとしてあります。別の名前にしてください。`;
    if (fs.existsSync(path.join(target, QUARTO_YML))) {
        return `${path.basename(target)} には既に文書（${QUARTO_YML}）があります。別の名前にしてください。`;
    }
    return null;
}

/** 入力欄の初期値。`docs2`, `docs3` … のうち `root` にまだ無い最初の名前。 */
export function suggestFolderName(root: string, exists: (p: string) => boolean = fs.existsSync): string {
    for (let i = 2; ; i++) {
        const name = `docs${i}`;
        if (!exists(path.join(root, name))) return name;
    }
}

export function activateAdd(context: vscode.ExtensionContext) {
    context.subscriptions.push(
        vscode.commands.registerCommand('ddqTemplate.add', (uri?: vscode.Uri) => addCommand(uri))
    );
    // 実拡張ホストの検証用（ダイアログを通さずに同じ追加処理を通す）
    context.subscriptions.push(
        vscode.commands.registerCommand('ddqTemplate.internal.add', (target: string) => addWritingFolder(target))
    );
}

async function addCommand(uri?: vscode.Uri): Promise<AddOutcome | undefined> {
    const root = uri?.fsPath ?? (await pickRoot());
    if (!root) return undefined;

    if (writingFoldersIn(root, await listWritingFolders()).length === 0) {
        const create = '設計書リポジトリを作成';
        const choice = await vscode.window.showInformationMessage(
            `${path.basename(root)} は設計書リポジトリではありません（執筆フォルダがありません）。` +
                '先に設計書リポジトリを作成してください。',
            create
        );
        if (choice === create) await vscode.commands.executeCommand('ddqTemplate.init');
        return 'notRepository';
    }

    const name = await vscode.window.showInputBox({
        title: '追加する執筆フォルダの名前',
        prompt: `${root} の下に、文書 1 つ分のフォルダを作ります`,
        value: suggestFolderName(root),
        validateInput: (value) => validateFolderName(value) ?? checkAddTarget(path.join(root, value.trim()))
    });
    if (!name?.trim()) return undefined;
    const target = path.join(root, name.trim());

    const outcome = await addWritingFolder(target);
    if (outcome === 'ok') await showAdded(target);
    return outcome;
}

async function pickRoot(): Promise<string | undefined> {
    const folders = vscode.workspace.workspaceFolders ?? [];
    if (folders.length === 0) {
        void vscode.window.showInformationMessage(
            '設計書リポジトリのフォルダを開いてから実行してください（新しく作るときは「設計書リポジトリを作成」）。'
        );
        return undefined;
    }
    if (folders.length === 1) return folders[0].uri.fsPath;
    const picked = await vscode.window.showQuickPick(
        folders.map((f) => ({ label: f.name, description: f.uri.fsPath, path: f.uri.fsPath })),
        { title: '執筆フォルダを追加する設計書リポジトリを選んでください' }
    );
    return picked?.path;
}

/**
 * `ddq add <執筆フォルダ>` を走らせる。Quarto が無ければ最後の疎通確認
 * （`quarto render`）を省いて作る（init と同じ）。
 */
export async function addWritingFolder(target: string): Promise<AddOutcome> {
    const release = tryLock(target);
    if (!release) {
        void vscode.window.showInformationMessage(`${path.basename(target)} では ddq を実行中です。`);
        return 'busy';
    }
    try {
        const env = envWithQuarto(settings().quartoPath);
        const quarto = await hasQuarto(env);
        const args = ['add', target, ...(quarto ? [] : ['--no-render'])];
        const result = await runWithProgress(`執筆フォルダを追加: ${path.basename(target)}`, args, undefined, env);
        if (result === 'cancelled') return 'cancelled';
        if (result.code !== 0) {
            void showError(result.error ?? `ddq add が失敗しました（終了コード ${result.code}）。`);
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

/** 足し終えた案内: `_quarto.yml` を開く。 */
async function showAdded(target: string) {
    const yml = path.join(target, QUARTO_YML);
    if (fs.existsSync(yml)) {
        await vscode.window.showTextDocument(vscode.Uri.file(yml), { preview: false });
    }
    void vscode.window.showInformationMessage(
        `執筆フォルダ ${path.basename(target)} を追加しました。` +
            `${QUARTO_YML} の表題・資料番号・会社名・章立てを直してから書き始めてください。`
    );
}
