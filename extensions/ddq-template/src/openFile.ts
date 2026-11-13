/**
 * 出力した PDF / HTML を OS の既定アプリで開く。
 *
 * Windows では `vscode.env.openExternal(Uri.file(…))` を使わない。VSCode は file URL を
 * パーセント符号化（`設計` → `%E8%A8%AD%E8%A8%88`）して OS に渡し、Windows の ShellExecute は
 * 符号化された非 ASCII のパスを解決できず「指定されたファイルが見つかりません。(0x2)」になる
 * （日本語の執筆フォルダで実測）。代わりに explorer.exe に素のパスを渡す。
 */
import { spawn } from 'node:child_process';
import * as fs from 'node:fs';
import * as vscode from 'vscode';

/** 既定アプリで開くコマンド。Windows 以外は undefined（openExternal を使う）。 */
export function defaultAppCommand(
    file: string,
    platform: NodeJS.Platform = process.platform
): { command: string; args: string[] } | undefined {
    if (platform !== 'win32') return undefined;
    // explorer.exe は開けても終了コード 1 を返すので、終了コードは見ない
    return { command: 'explorer.exe', args: [file] };
}

export async function openWithDefaultApp(file: string): Promise<void> {
    if (!fs.existsSync(file)) {
        void vscode.window.showErrorMessage(`ファイルが見つかりません: ${file}`);
        return;
    }
    const launch = defaultAppCommand(file);
    if (!launch) {
        await vscode.env.openExternal(vscode.Uri.file(file));
        return;
    }
    await new Promise<void>((resolve) => {
        const child = spawn(launch.command, launch.args, { detached: true, stdio: 'ignore', windowsHide: false });
        child.once('error', (e) => {
            void vscode.window.showErrorMessage(`ファイルを開けません: ${file}（${e.message}）`);
            resolve();
        });
        child.once('spawn', () => {
            child.unref();
            resolve();
        });
    });
}
