/**
 * Quarto の有無の検査（docs/ddq-template-study.md §3.3）。
 *
 * ddq は `quarto` を PATH から起動する。拡張から入る人が最初につまずくのはここなので、
 * pdf / html の前に `quarto --version` で在るかだけを確かめる（版の下限は見ない。U1）。
 * 設定 `ddqTemplate.quartoPath` があれば、そのフォルダを子プロセスの PATH の先頭に足す
 * （ddq は変えずに済む）。
 */

import { execFile } from 'node:child_process';
import * as path from 'node:path';

/** 子プロセスに渡す環境変数。`quartoPath` が空ならそのまま。 */
export function envWithQuarto(
    quartoPath: string | undefined,
    base: NodeJS.ProcessEnv = process.env,
    platform = process.platform
): NodeJS.ProcessEnv {
    const trimmed = (quartoPath ?? '').trim();
    if (trimmed === '') return base;
    // quarto.exe を指していればそのフォルダ、フォルダを指していればそのまま
    const dir = /quarto(\.exe|\.cmd)?$/i.test(path.basename(trimmed)) ? path.dirname(trimmed) : trimmed;
    // Windows の環境変数名は大文字小文字を区別しない（Path のことが多い）
    const key = Object.keys(base).find((k) => k.toUpperCase() === 'PATH') ?? 'PATH';
    const sep = platform === 'win32' ? ';' : ':';
    const current = base[key];
    return { ...base, [key]: current ? `${dir}${sep}${current}` : dir };
}

let found = false;

/**
 * Quarto が起動できるか。一度見つかれば覚える。見つからなかったことは覚えない
 * （インストールした直後に、VSCode を開き直さずにもう一度試せるように）。
 */
export function hasQuarto(env: NodeJS.ProcessEnv): Promise<boolean> {
    if (found) return Promise.resolve(true);
    return new Promise((resolve) => {
        execFile('quarto', ['--version'], { env, windowsHide: true, timeout: 30_000 }, (error) => {
            found = !error;
            resolve(found);
        });
    });
}
