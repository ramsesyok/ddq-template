/**
 * ddq の起動口（発行・作成・追加・改訂が共有する。EXT-02）。
 *
 * 設定の読み取り、起動する ddq の決定、「DDQ」出力チャネル、キャンセルできる進捗通知、
 * 拡張の終了時の後始末をここに集める。機能モジュール（build / init / add / 改訂）は
 * 互いの画面を import せず、ここを使う。子プロセスの起動そのものは run.ts。
 */

import * as vscode from 'vscode';

import { locateDdq, type Located } from './locate';
import { runStreaming, stopAll, type StreamResult } from './run';

export type Settings = {
    /** `ddqTemplate.ddqPath`（空なら同梱 → PATH の ddq） */
    ddqPath: string | undefined;
    /** `ddqTemplate.quartoPath`（空なら PATH の quarto） */
    quartoPath: string | undefined;
};

/** 設定 `ddqTemplate.*`。呼ぶたびに読む（変更をすぐ反映するため）。 */
export function settings(): Settings {
    const config = vscode.workspace.getConfiguration('ddqTemplate');
    return {
        ddqPath: config.get<string>('ddqPath'),
        quartoPath: config.get<string>('quartoPath')
    };
}

/** いま使う ddq（設定 → 同梱 → PATH。locate.ts）。 */
export function currentDdq(): Located {
    return locateDdq(settings().ddqPath);
}

let output: vscode.OutputChannel | undefined;
/** 拡張の終了処理に入ったか。入った後は出力チャネルにも通知にも触れない */
let closing = false;

export function channel(): vscode.OutputChannel {
    output ??= vscode.window.createOutputChannel('DDQ');
    return output;
}

export async function showError(reason: string) {
    if (closing) return;
    const show = '出力を表示';
    if ((await vscode.window.showErrorMessage(reason, show)) === show) channel().show(true);
}

/** 拡張の起動時に呼ぶ（終了処理の印を戻す）。 */
export function activateLaunch(): void {
    closing = false;
}

/** 拡張の終了時に呼ぶ。起動中の ddq を孫ごと止め、以後は出力・通知を出さない。 */
export function deactivateLaunch(): void {
    closing = true;
    stopAll();
}

/**
 * ddq を 1 本、キャンセルできる進捗通知つきで走らせ、出力を「DDQ」に流す（発行・作成・追加）。
 * `keepLine` に合う行（通知に使う結果行）だけを結果に残す。拡張の終了で止められたときは
 * `'cancelled'`（出力チャネルにも書かない）。
 */
export async function runWithProgress(
    title: string,
    args: string[],
    cwd: string | undefined,
    env: NodeJS.ProcessEnv,
    keepLine?: (line: string) => boolean
): Promise<StreamResult | 'cancelled'> {
    const out = channel();
    const command = currentDdq().command;
    out.appendLine(`\n> ${command} ${args.join(' ')}`);
    return vscode.window.withProgress(
        {
            location: vscode.ProgressLocation.Notification,
            title,
            cancellable: true
        },
        async (_progress, token) => {
            let child: { kill: () => void } | undefined;
            let cancelled = false;
            const subscription = token.onCancellationRequested(() => {
                cancelled = true;
                child?.kill();
            });
            try {
                const result = await runStreaming(command, args, {
                    cwd,
                    env,
                    keepLine,
                    onOutput: (text) => {
                        if (!closing) out.append(text);
                    },
                    onSpawn: (c) => {
                        child = c;
                    }
                });
                if (closing) return 'cancelled';
                if (result.error) out.appendLine(result.error);
                out.appendLine(`（終了コード ${result.code ?? '-'}）`);
                return cancelled ? 'cancelled' : result;
            } finally {
                subscription.dispose();
            }
        }
    );
}
