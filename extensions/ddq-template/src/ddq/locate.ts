/**
 * 起動する ddq を決める（docs/ddq-template-study.md §3.2）。
 *
 *   1. 設定 `ddqTemplate.ddqPath`（明示した人の意図を最優先）
 *   2. 拡張に同梱した `bin/win32-x64/ddq.exe`（ターゲット指定 VSIX のときだけある）
 *   3. `PATH` の `ddq`
 */

import * as fs from 'node:fs';
import * as path from 'node:path';

export type DdqSource = 'setting' | 'bundled' | 'path';

export type Located = { command: string; source: DdqSource };

/** 拡張フォルダの中の同梱 ddq（この OS 向けのものが無ければ undefined）。 */
export function bundledPath(extensionPath: string, platform = process.platform, arch = process.arch): string | undefined {
    if (platform !== 'win32' || arch !== 'x64') return undefined;
    return path.join(extensionPath, 'bin', 'win32-x64', 'ddq.exe');
}

/** 解決順に従って ddq を選ぶ（`exists` は差し替えてテストする）。 */
export function chooseDdq(
    configured: string | undefined,
    bundled: string | undefined,
    exists: (file: string) => boolean
): Located {
    const trimmed = (configured ?? '').trim();
    if (trimmed !== '') return { command: trimmed, source: 'setting' };
    if (bundled && exists(bundled)) return { command: bundled, source: 'bundled' };
    return { command: 'ddq', source: 'path' };
}

let extensionPath: string | undefined;

/** 拡張の起動時に 1 回呼ぶ（同梱 ddq の場所を知るため）。 */
export function initLocate(dir: string): void {
    extensionPath = dir;
}

/** いま使う ddq。設定は呼ぶたびに読む（変更をすぐ反映するため）。 */
export function locateDdq(configured: string | undefined): Located {
    const bundled = extensionPath ? bundledPath(extensionPath) : undefined;
    return chooseDdq(configured, bundled, (file) => fs.existsSync(file));
}
