#!/usr/bin/env node
/**
 * 同梱版のスモーク（docs/ddq-template-study.md §2 Q15）。配る VSIX そのものを確かめる。
 *
 *   npm run test:smoke [-- <VSIX のパス>]
 *
 * 1. ターゲット指定の VSIX（ddq-template-win32-x64-<版>.vsix）を一時フォルダに展開する
 * 2. 展開した extension/ を拡張として VSCode を起動する（ソースではなく配る中身で動かす）
 * 3. tests/host/smoke-index.js が、設定も PATH も無しで同梱の ddq に決まること、
 *    その ddq で init → html が通ることを見る
 *
 * 前提: Windows x64・Quarto がある・VSIX は `npm run package:win32-x64` で作ってある。
 * VSCode の用意は run.mjs と同じ（インストール済み → 無ければ取得）。
 */
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { downloadAndUnzipVSCode, runTests } from '@vscode/test-electron';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../..');
const version = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).version;
const vsix = path.resolve(process.argv[2] ?? path.join(root, `ddq-template-win32-x64-${version}.vsix`));
if (!fs.existsSync(vsix)) {
    console.error(`${vsix} がありません。先に npm run package:win32-x64 を実行してください。`);
    process.exit(1);
}

const work = fs.mkdtempSync(path.join(os.tmpdir(), 'ddq-template-smoke-'));
const unpacked = path.join(work, 'vsix');
fs.mkdirSync(unpacked);
// VSIX は zip。Windows 10 以降の System32\tar.exe（bsdtar）は zip を展開できる。名前だけで呼ぶと
// Git for Windows の GNU tar に当たることがある（zip を読めず、C: をホスト名と解釈する）ので、場所を指定する
const tar = path.join(process.env.SystemRoot ?? 'C:\\Windows', 'System32', 'tar.exe');
execFileSync(tar, ['-xf', vsix, '-C', unpacked]);
const extension = path.join(unpacked, 'extension');
const bundled = path.join(extension, 'bin', 'win32-x64', 'ddq.exe');
if (!fs.existsSync(bundled)) {
    console.error(`VSIX に ddq.exe が入っていません: ${bundled}`);
    process.exit(1);
}
console.log(`VSIX: ${vsix}`);
console.log(`同梱 ddq: ${execFileSync(bundled, ['--version'], { encoding: 'utf8' }).trim()}`);

// PATH から ddq を外す（同梱に決まることを見るため）。Quarto は残す
const pathKey = Object.keys(process.env).find((k) => k.toUpperCase() === 'PATH') ?? 'PATH';
const cleanPath = (process.env[pathKey] ?? '')
    .split(path.delimiter)
    .filter((dir) => dir && !fs.existsSync(path.join(dir, 'ddq.exe')) && !fs.existsSync(path.join(dir, 'ddq')))
    .join(path.delimiter);

const vscodeExecutablePath = await resolveVsCode();
console.log(`VSCode: ${vscodeExecutablePath}`);
const workspace = path.join(work, 'workspace');
fs.mkdirSync(workspace);

try {
    await runTests({
        vscodeExecutablePath,
        extensionDevelopmentPath: extension,
        extensionTestsPath: path.join(here, 'smoke-index.js'),
        extensionTestsEnv: { HOST_OUT: work, BUNDLED_DDQ: bundled, [pathKey]: cleanPath },
        launchArgs: [
            workspace,
            '--disable-workspace-trust',
            '--user-data-dir',
            path.join(work, 'user-data'),
            '--extensions-dir',
            path.join(work, 'extensions')
        ]
    });
} finally {
    const report = path.join(work, 'host-result.txt');
    if (fs.existsSync(report)) console.log(`\n${fs.readFileSync(report, 'utf8')}`);
}
console.log('\n同梱版のスモークに合格しました。');

/** run.mjs と同じ: インストール済みの VSCode を優先し、無ければダウンロードする。 */
async function resolveVsCode() {
    const candidates = [
        process.env.VSCODE_BIN,
        path.join(process.env.LOCALAPPDATA ?? '', 'Programs', 'Microsoft VS Code', 'Code.exe'),
        'C:\\Program Files\\Microsoft VS Code\\Code.exe'
    ].filter((p) => typeof p === 'string' && p !== '');
    for (const candidate of candidates) {
        if (fs.existsSync(candidate)) return candidate;
    }
    console.log('インストール済みの VSCode が見つからないので、テスト用の VSCode を取得します…');
    return await downloadAndUnzipVSCode();
}
