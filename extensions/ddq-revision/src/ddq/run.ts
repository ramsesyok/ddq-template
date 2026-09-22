/**
 * `ddq` を起動して JSON を受け取る。
 *
 * 拡張は qmd を解釈しない。文書の解釈はすべて ddq に任せ、ここはその呼び出し口である
 * （同じ解釈が CLI と拡張で二重にならないようにするため）。
 */

import { execFile } from 'node:child_process';

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

/** 実行ファイル。設定が空なら PATH の `ddq` を使う。 */
export function ddqCommand(configured: string | undefined): string {
    const trimmed = (configured ?? '').trim();
    return trimmed === '' ? 'ddq' : trimmed;
}

/** 標準出力を返す。終了コードが 0 でなければ `DdqError`。 */
export function run(command: string, args: string[], cwd?: string): Promise<string> {
    return new Promise((resolve, reject) => {
        execFile(
            command,
            args,
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
                            `${command} を実行できません。設定 ddqRevision.ddqPath に ddq.exe のパスを指定してください。`,
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

/** `ddq --version` の版（`ddq 2.3.0` → `2.3.0`）。 */
export function versionOf(output: string): string {
    return output.trim().split(/\s+/).pop() ?? '';
}
