/**
 * `ddq` を起動して JSON を受け取る。
 *
 * 拡張は qmd を解釈しない。文書の解釈はすべて ddq に任せ、ここはその呼び出し口である
 * （同じ解釈が CLI と拡張で二重にならないようにするため）。
 */

import { execFile, spawn } from 'node:child_process';

import { locateDdq } from './locate';

/** 実行に失敗したときの理由。UI にそのまま出す。 */
export class DdqError extends Error {
    constructor(
        message: string,
        /** ddq が出した説明（あれば） */
        readonly detail?: string
    ) {
        super(message);
        this.name = 'DdqError';
    }
}

/** 実行ファイル。設定 → 同梱 → PATH の `ddq` の順に決める（locate.ts）。 */
export function ddqCommand(configured: string | undefined): string {
    return locateDdq(configured).command;
}

/**
 * 起動するもの（実行ファイルと引数）を決める。
 *
 * Windows の Node は、脆弱性対応（CVE-2024-27980）以降 `.cmd` / `.bat` を
 * `execFile` で直接起動できない（EINVAL になる）。ラッパのバッチを
 * `ddqTemplate.ddqPath` に指定されることはあるので、その場合は `cmd /c` を通す。
 */
export function spawnArgs(command: string, args: string[]): [string, string[]] {
    const isBatch = /\.(cmd|bat)$/i.test(command);
    if (process.platform === 'win32' && isBatch) {
        return ['cmd.exe', ['/c', command, ...args]];
    }
    return [command, args];
}

/** 標準出力を返す。終了コードが 0 でなければ `DdqError`。 */
export function run(command: string, args: string[], cwd?: string): Promise<string> {
    const [file, argv] = spawnArgs(command, args);
    return new Promise((resolve, reject) => {
        execFile(
            file,
            argv,
            // 設計書は 1000 ページ級になりうるので、既定の 1MB では足りない
            { cwd, maxBuffer: 64 * 1024 * 1024, windowsHide: true },
            (error, stdout, stderr) => {
                if (!error) {
                    resolve(stdout);
                    return;
                }
                const detail = (stderr || stdout || '').trim();
                if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
                    reject(
                        new DdqError(
                            `${command} を実行できません。設定 ddqTemplate.ddqPath に ddq.exe のパスを指定してください。`,
                            detail
                        )
                    );
                    return;
                }
                reject(new DdqError(`ddq ${args.join(' ')} に失敗しました。`, detail));
            }
        );
    });
}

/** JSON を返すコマンドを実行する。 */
export async function runJson<T>(command: string, args: string[], cwd?: string): Promise<T> {
    const out = await run(command, args, cwd);
    try {
        return JSON.parse(out) as T;
    } catch {
        throw new DdqError('ddq の出力を JSON として読めません。', out.slice(0, 400));
    }
}

export type StreamOptions = {
    cwd?: string;
    env?: NodeJS.ProcessEnv;
    /** 標準出力・標準エラーを届いた順に渡す（出力チャネルへ流す） */
    onOutput: (text: string) => void;
    /** 起動した子プロセス。止めるとき（キャンセル・拡張の終了）に使う */
    onSpawn?: (child: { pid?: number; kill: () => void }) => void;
};

/** 実行結果。`code` が null のときは起動できなかったか、止められた。 */
export type StreamResult = { code: number | null; output: string; error?: string };

/**
 * pdf / html のように時間のかかるコマンドを、出力を流しながら実行する。
 * 失敗しても reject せず、終了コードで呼び出し側が分岐する（3 = 版・機構の不一致）。
 */
export function runStreaming(command: string, args: string[], options: StreamOptions): Promise<StreamResult> {
    const [file, argv] = spawnArgs(command, args);
    return new Promise((resolve) => {
        let output = '';
        const child = spawn(file, argv, { cwd: options.cwd, env: options.env, windowsHide: true });
        options.onSpawn?.({ pid: child.pid, kill: () => killTree(child.pid, () => child.kill()) });
        const take = (text: string) => {
            output += text;
            options.onOutput(text);
        };
        // 文字列で受ける（Buffer の境目で日本語の 1 文字が割れないように）
        child.stdout.setEncoding('utf8');
        child.stderr.setEncoding('utf8');
        child.stdout.on('data', take);
        child.stderr.on('data', take);
        child.on('error', (e: NodeJS.ErrnoException) => {
            const error =
                e.code === 'ENOENT'
                    ? `${command} を実行できません。設定 ddqTemplate.ddqPath に ddq.exe のパスを指定してください。`
                    : e.message;
            resolve({ code: null, output, error });
        });
        child.on('close', (code) => resolve({ code, output }));
    });
}

/**
 * 子プロセスを孫ごと止める。ddq は quarto を、quarto は pandoc / typst を起動するので、
 * Windows では ddq だけを止めると孫が残る。`taskkill /T` で木ごと止める。
 */
function killTree(pid: number | undefined, fallback: () => void): void {
    if (process.platform === 'win32' && pid !== undefined) {
        execFile('taskkill', ['/pid', String(pid), '/T', '/F'], { windowsHide: true }, (error) => {
            if (error) fallback();
        });
        return;
    }
    fallback();
}

/** `ddq --version` の版（`ddq 2.3.0` → `2.3.0`）。 */
export function versionOf(output: string): string {
    return output.trim().split(/\s+/).pop() ?? '';
}
