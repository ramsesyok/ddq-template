/**
 * `ddq` を起動して JSON を受け取る。
 *
 * 拡張は qmd を解釈しない。文書の解釈はすべて ddq に任せ、ここはその呼び出し口である
 * （同じ解釈が CLI と拡張で二重にならないようにするため）。
 */

import { execFile, spawn } from 'node:child_process';

import { ContractError } from './contract';
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
 * 起動できない ddq の指定なら、その理由（起動できるなら undefined）。
 *
 * `.cmd` / `.bat` は受け付けない。Windows の Node は CVE-2024-27980 の対応以降バッチを
 * 直接起動できず、`cmd /c` を通すと引数（執筆フォルダのパスなど）の `&` や `%` を
 * cmd.exe が解釈してしまう。同梱の ddq.exe が主になったので、exe の指定に限る。
 */
export function unsupportedCommand(command: string): string | undefined {
    if (!/\.(cmd|bat)$/i.test(command)) return undefined;
    return (
        `${command} はバッチファイルなので起動しません（引数を cmd.exe が解釈してしまうため）。` +
        '設定 ddqTemplate.ddqPath には ddq.exe を指定するか、空にして同梱の ddq を使ってください。'
    );
}

/** 起動中の ddq（JSON を返す短い実行も、pdf / html のような長い実行も載せる）。 */
const live = new Set<{ kill: () => void }>();

/** 起動中の ddq を孫ごと止める（拡張の終了時。docs/ddq-template-study.md §3.4）。 */
export function stopAll(): void {
    for (const child of live) child.kill();
    live.clear();
}

/** いま起動中の ddq の数（試験用）。 */
export function liveCount(): number {
    return live.size;
}

/** 標準出力を返す。終了コードが 0 でなければ `DdqError`。 */
export function run(command: string, args: string[], cwd?: string): Promise<string> {
    const unsupported = unsupportedCommand(command);
    if (unsupported) return Promise.reject(new DdqError(unsupported));
    return new Promise((resolve, reject) => {
        const child = execFile(
            command,
            args,
            // 設計書は 1000 ページ級になりうるので、既定の 1MB では足りない
            { cwd, maxBuffer: 64 * 1024 * 1024, windowsHide: true },
            (error, stdout, stderr) => {
                live.delete(handle);
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
        const handle = { kill: () => killTree(child.pid, () => child.kill()) };
        live.add(handle);
    });
}

/**
 * JSON を返すコマンドを実行し、`parse` で形を確かめてから返す（contract.ts）。
 * 形が違えば、画面に流したり書き戻したりする前に `DdqError` で止める。
 */
export async function runJson<T>(
    command: string,
    args: string[],
    cwd: string | undefined,
    parse: (value: unknown) => T
): Promise<T> {
    const out = await run(command, args, cwd);
    let value: unknown;
    try {
        value = JSON.parse(out);
    } catch {
        throw new DdqError('ddq の出力を JSON として読めません。', out.slice(0, 400));
    }
    try {
        return parse(value);
    } catch (e) {
        if (e instanceof ContractError) {
            throw new DdqError(
                `${e.what} の出力が想定した形ではありません（拡張と ddq の版が合っていない可能性があります。設定 ddqTemplate.ddqPath を確認してください）。`,
                e.problems.join('\n')
            );
        }
        throw e;
    }
}

export type StreamOptions = {
    cwd?: string;
    env?: NodeJS.ProcessEnv;
    /** 標準出力・標準エラーを届いた順に渡す（出力チャネルへ流す） */
    onOutput: (text: string) => void;
    /** 起動した子プロセス。止めるとき（キャンセル）に使う。拡張の終了時は `stopAll` が止める */
    onSpawn?: (child: { pid?: number; kill: () => void }) => void;
    /**
     * 結果として手元に残す行（通知に使う `OK: …` など）。出力の全部は `onOutput` へ流すだけで
     * 溜めない（1000 ページ級の文書では Quarto の進捗が長い）。省略すれば何も残さない
     */
    keepLine?: (line: string) => boolean;
};

/** 実行結果。`code` が null のときは起動できなかったか、止められた。 */
export type StreamResult = {
    code: number | null;
    /** `keepLine` に合った行（行末の改行を除く。届いた順） */
    lines: string[];
    error?: string;
};

/**
 * pdf / html のように時間のかかるコマンドを、出力を流しながら実行する。
 * 失敗しても reject せず、終了コードで呼び出し側が分岐する（3 = 版・機構の不一致）。
 */
export function runStreaming(command: string, args: string[], options: StreamOptions): Promise<StreamResult> {
    const unsupported = unsupportedCommand(command);
    if (unsupported) return Promise.resolve({ code: null, lines: [], error: unsupported });
    return new Promise((resolve) => {
        const lines: string[] = [];
        const keep = options.keepLine ?? (() => false);
        const child = spawn(command, args, { cwd: options.cwd, env: options.env, windowsHide: true });
        const handle = { pid: child.pid, kill: () => killTree(child.pid, () => child.kill()) };
        live.add(handle);
        options.onSpawn?.(handle);
        // 行は流れごとに組み立てる（stdout と stderr の断片が交互に届いても行が混ざらないように）
        const streams = [child.stdout, child.stderr].map((stream) => {
            const splitter = lineSplitter((line) => {
                if (keep(line)) lines.push(line);
            });
            // 文字列で受ける（Buffer の境目で日本語の 1 文字が割れないように）
            stream.setEncoding('utf8');
            stream.on('data', (text: string) => {
                options.onOutput(text);
                splitter.push(text);
            });
            return splitter;
        });
        const finish = (result: Omit<StreamResult, 'lines'>) => {
            live.delete(handle);
            for (const s of streams) s.end();
            resolve({ ...result, lines });
        };
        child.on('error', (e: NodeJS.ErrnoException) => {
            const error =
                e.code === 'ENOENT'
                    ? `${command} を実行できません。設定 ddqTemplate.ddqPath に ddq.exe のパスを指定してください。`
                    : e.message;
            finish({ code: null, error });
        });
        child.on('close', (code) => finish({ code }));
    });
}

/** 届いた断片を行に切り、1 行ずつ `onLine` に渡す（`\r\n` も `\n` も行の終わり）。 */
export function lineSplitter(onLine: (line: string) => void): { push: (text: string) => void; end: () => void } {
    let rest = '';
    let ended = false;
    const emit = (line: string) => onLine(line.endsWith('\r') ? line.slice(0, -1) : line);
    return {
        push(text: string) {
            const parts = (rest + text).split('\n');
            rest = parts.pop() ?? '';
            for (const part of parts) emit(part);
        },
        end() {
            if (ended) return;
            ended = true;
            if (rest !== '') emit(rest);
            rest = '';
        }
    };
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
