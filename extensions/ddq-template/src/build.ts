/**
 * PDF / HTML / 図の発行（docs/ddq-template-study.md §3）。
 *
 * ddq を子プロセスで走らせ、出力を「DDQ」出力チャネルへ流し、キャンセルできる進捗通知を出す。
 * 版・機構が合わないとき（終了コード 3）は版を比べ、`ddq update` を案内する（update の最小形）。
 * 同じ執筆フォルダでは 1 本ずつしか走らせない（lock.ts）。
 */

import * as fs from 'node:fs';
import * as path from 'node:path';
import * as vscode from 'vscode';

import { channel, currentDdq, runWithProgress, settings, showError } from './ddq/launch';
import { tryLock } from './ddq/lock';
import { run, versionOf } from './ddq/run';
import { TEMPLATE_MISMATCH_EXIT_CODE, classifyMismatch, compareVersions } from './ddq/version';
import { TEMPLATE_VERSION_FILE, activeWritingFolder, resolveWritingFolder, watchWritingFolders } from './folders';
import { openWithDefaultApp } from './openFile';
import { envWithQuarto, hasQuarto } from './quarto';

export type BuildKind = 'pdf' | 'html' | 'diagrams';

const TITLES: Record<BuildKind | 'update', string> = {
    pdf: 'PDF を作成',
    html: 'HTML を作成',
    diagrams: '図を変換',
    update: '機構ファイルを更新（ddq update）'
};

/** Quarto を使うもの（diagrams は ddq だけで完結する） */
const NEEDS_QUARTO: ReadonlySet<BuildKind> = new Set(['pdf', 'html']);

/** 実行の結果（実拡張ホストの検証が見る） */
export type BuildOutcome = 'ok' | 'failed' | 'busy' | 'cancelled' | 'no-quarto' | 'mismatch' | 'no-folder';

let statusItem: vscode.StatusBarItem | undefined;

export function activateBuild(context: vscode.ExtensionContext) {
    for (const kind of ['pdf', 'html', 'diagrams'] as const) {
        context.subscriptions.push(
            vscode.commands.registerCommand(`ddqTemplate.${kind}`, (resource?: vscode.Uri) =>
                buildCommand(kind, resource)
            )
        );
    }
    // 実拡張ホストの検証用（通知のボタンを押さずに同じ処理を通す）
    context.subscriptions.push(
        vscode.commands.registerCommand(
            'ddqTemplate.internal.build',
            (kind: BuildKind | 'update', folder: string) => execute(kind, folder, { interactive: false })
        )
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('ddqTemplate.internal.showOutput', () => channel().show(true))
    );
    // 同梱版のスモーク（tests/host/smoke.mjs）が、どの ddq に決まったかを見る
    context.subscriptions.push(
        vscode.commands.registerCommand('ddqTemplate.internal.locate', () => currentDdq())
    );

    statusItem = vscode.window.createStatusBarItem('ddqTemplate.pdf', vscode.StatusBarAlignment.Left, 50);
    statusItem.name = 'DDQ: PDF';
    context.subscriptions.push(statusItem);
    const refreshStatus = () => void updateStatus();
    context.subscriptions.push(vscode.window.onDidChangeActiveTextEditor(refreshStatus));
    watchWritingFolders(context, refreshStatus);

    void checkBundled(context);
}

async function buildCommand(kind: BuildKind, resource?: vscode.Uri): Promise<BuildOutcome> {
    const folder = await resolveWritingFolder(resource);
    if (!folder) return 'no-folder';
    return execute(kind, folder, { interactive: true });
}

/** ステータスバー: 開いているファイルの執筆フォルダと [PDF]。執筆フォルダの外では隠す。 */
async function updateStatus(busyFolder?: string) {
    if (!statusItem) return;
    const folder = busyFolder ?? (await activeWritingFolder());
    if (!folder) {
        statusItem.hide();
        return;
    }
    const name = path.basename(folder);
    statusItem.text = busyFolder ? `$(sync~spin) PDF: ${name}` : `$(file-pdf) PDF: ${name}`;
    statusItem.tooltip = busyFolder ? `${name} を処理しています` : `${name} の PDF を作る（ddq pdf）`;
    statusItem.command = busyFolder
        ? { title: '出力を表示', command: 'ddqTemplate.internal.showOutput' }
        : { title: 'PDF を作成', command: 'ddqTemplate.pdf', arguments: [vscode.Uri.file(folder)] };
    statusItem.show();
}

/**
 * 1 本実行する。`interactive` が false のときは通知のボタンを待たない（検証用）。
 * 版・機構の不一致は interactive なら update を案内し、update に成功すれば元のコマンドをやり直す。
 */
async function execute(
    kind: BuildKind | 'update',
    folder: string,
    options: { interactive: boolean }
): Promise<BuildOutcome> {
    const release = tryLock(folder);
    if (!release) {
        void vscode.window.showInformationMessage(`${path.basename(folder)} では ddq を実行中です。終わってから実行してください。`);
        return 'busy';
    }
    try {
        const env = envWithQuarto(settings().quartoPath);
        if (kind !== 'update' && NEEDS_QUARTO.has(kind) && !(await hasQuarto(env))) {
            if (options.interactive) void notifyNoQuarto();
            return 'no-quarto';
        }
        if (kind === 'pdf') void updateStatus(folder);
        const result = await runWithProgress(
            `${TITLES[kind]}: ${path.basename(folder)}`,
            [kind, folder],
            folder,
            env,
            isResultLine
        );
        if (result === 'cancelled') return 'cancelled';
        if (result.code === 0) {
            if (options.interactive) void notifyDone(kind, folder, result.lines.join('\n'));
            return 'ok';
        }
        if (result.code === TEMPLATE_MISMATCH_EXIT_CODE && kind !== 'update') {
            if (!options.interactive) return 'mismatch';
            release();
            return await offerUpdate(kind, folder);
        }
        const reason = result.error ?? `ddq ${kind} が失敗しました（終了コード ${result.code}）。`;
        if (options.interactive) void showError(reason);
        return 'failed';
    } finally {
        release();
        void updateStatus();
    }
}

async function notifyDone(kind: BuildKind | 'update', folder: string, stdout: string) {
    const name = path.basename(folder);
    if (kind === 'pdf') {
        const pdf = path.join(folder, 'design-doc.pdf');
        const open = 'PDF を開く';
        const choice = await vscode.window.showInformationMessage(`${name} の PDF を作りました。`, open, '出力を表示');
        if (choice === open) await openWithDefaultApp(pdf);
        else if (choice) channel().show(true);
        return;
    }
    if (kind === 'html') {
        const index = htmlIndexOf(stdout);
        const open = 'HTML を開く';
        const buttons = index ? [open, '出力を表示'] : ['出力を表示'];
        const choice = await vscode.window.showInformationMessage(`${name} の HTML を作りました。`, ...buttons);
        if (choice === open && index) await openWithDefaultApp(index);
        else if (choice) channel().show(true);
        return;
    }
    const warning = kind === 'diagrams' ? warningOf(stdout) : undefined;
    if (warning) {
        // 「変換する図が無い」は終了コード 0 なので、ここで見せないと「終わりました」とだけ出て気付けない
        const choice = await vscode.window.showWarningMessage(`${name}: ${warning}`, '出力を表示');
        if (choice) channel().show(true);
        return;
    }
    void vscode.window.showInformationMessage(`${name}: ${TITLES[kind]}が終わりました。`);
}

/**
 * 通知に使う結果行か（`OK -> …` / `OK: …` / `警告: …`。cli/tests/fixtures/contract/cli.json）。
 * 出力チャネルには全部を流すが、手元にはこの行だけを残す。
 */
export function isResultLine(line: string): boolean {
    return line.startsWith('OK') || line.startsWith('警告: ');
}

/** ddq の出力から最初の警告（`警告: …`）の本文を取る。 */
export function warningOf(stdout: string): string | undefined {
    const m = /^警告: (.+)$/m.exec(stdout);
    return m?.[1].trim();
}

/** `ddq html` の出力から index.html のパスを取る（`OK: <パス>（…）`）。 */
export function htmlIndexOf(stdout: string): string | undefined {
    const m = /^OK: (.+?index\.html)/m.exec(stdout);
    return m?.[1];
}

/**
 * Quarto が無い。ダウンロードページへのリンクは出さない（オフライン環境が前提で、
 * 拡張に外部 URL を持たない。check:offline）。入手方法は利用マニュアルの環境構築の章にある。
 */
async function notifyNoQuarto() {
    const setting = '設定でパスを指定';
    const choice = await vscode.window.showErrorMessage(
        'Quarto が見つかりません。利用マニュアル（環境構築）に従ってインストールし、VSCode を開き直してください' +
            '（PATH を読み直すため）。インストール済みなら、設定 ddqTemplate.quartoPath に quarto.exe のパスを指定できます。',
        setting
    );
    if (choice === setting) await vscode.commands.executeCommand('workbench.action.openSettings', 'ddqTemplate.quartoPath');
}

/**
 * 版・機構の不一致（終了コード 3）。版を比べて案内を変える。
 * 文書の方が新しいときは update を出さない（機構を古い版に戻してしまうため）。
 */
async function offerUpdate(kind: BuildKind, folder: string): Promise<BuildOutcome> {
    const folderVersion = readFolderVersion(folder);
    const ddqVersion = await ddqVersionOf(folder);
    const name = path.basename(folder);
    const mismatch = classifyMismatch(folderVersion, ddqVersion);
    const show = '出力を表示';

    if (mismatch === 'unknown') {
        // 比べられないので update を勧めない（文書の方が新しければ機構を古い版に戻してしまう）
        channel().appendLine(
            `版の確認: 執筆フォルダ ${folderVersion ?? `（${TEMPLATE_VERSION_FILE} が無い・空）`}、` +
                `ddq ${ddqVersion ?? '（--version を読めない）'}（${currentDdq().command}）`
        );
        const choice = await vscode.window.showWarningMessage(
            `${name} のテンプレートの版（${folderVersion ?? '版ファイルなし'}）と ddq の版（${ddqVersion ?? '不明'}）を` +
                `比べられません。執筆フォルダの ${TEMPLATE_VERSION_FILE} と ddq の版を確かめてください。`,
            show
        );
        if (choice === show) channel().show(true);
        return 'mismatch';
    }

    if (mismatch === 'folder-newer') {
        const choice = await vscode.window.showWarningMessage(
            `${name} は新しい版のテンプレート（${folderVersion}）で作られています。ddq は ${ddqVersion} です。` +
                'DDQ Template 拡張（または設定 ddqTemplate.ddqPath の ddq）を更新してください。',
            show
        );
        if (choice === show) channel().show(true);
        return 'mismatch';
    }

    const update = 'ddq update を実行';
    const message =
        mismatch === 'modified'
            ? `${name} の機構ファイルがテンプレート ${ddqVersion} と一致しません（手で直された可能性があります）。` +
              'ddq update で元に戻せます（差分は Git で確認してください）。'
            : `${name} のテンプレートは ${folderVersion}、ddq は ${ddqVersion} です。` +
              'ddq update で機構ファイルを新しい版にしてから続けます（変わったファイルはコミットしてください）。';
    const choice = await vscode.window.showWarningMessage(message, update, show);
    if (choice === show) channel().show(true);
    if (choice !== update) return 'mismatch';

    const updated = await execute('update', folder, { interactive: false });
    if (updated !== 'ok') {
        void showError(`ddq update が失敗しました（${updated}）。`);
        return 'failed';
    }
    return execute(kind, folder, { interactive: true });
}

function readFolderVersion(folder: string): string | undefined {
    try {
        return fs.readFileSync(path.join(folder, TEMPLATE_VERSION_FILE), 'utf8').trim() || undefined;
    } catch {
        return undefined;
    }
}

async function ddqVersionOf(cwd?: string): Promise<string | undefined> {
    try {
        return versionOf(await run(currentDdq().command, ['--version'], cwd));
    } catch {
        return undefined;
    }
}

/**
 * 同梱 ddq の版が拡張の版と合っているか（ビルド手順の誤りの検出用。§3.2）。
 * 合っていなくても使えるので、出力チャネルに書くだけにする。
 */
async function checkBundled(context: vscode.ExtensionContext) {
    const located = currentDdq();
    if (located.source !== 'bundled') return;
    const version = await ddqVersionOf();
    const expected = String(context.extension.packageJSON.version ?? '');
    if (version && compareVersions(version, expected) !== 0) {
        channel().appendLine(
            `警告: 同梱の ddq（${version}）が拡張の版（${expected}）と一致しません。VSIX の作り方を確認してください。`
        );
    }
}
