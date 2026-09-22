#!/usr/bin/env node
/**
 * 実拡張ホストでの検証を起動する（見るものは tests/host/index.js）。
 *
 *   npm run test:host            … 手元でも CI でも同じ
 *   node tests/host/run.mjs <ddq のパス>
 *
 * VSCode の用意（この順に探す）:
 *   1. 環境変数 `VSCODE_BIN`
 *   2. インストール済みの VSCode（手元の保守者はこれ。**通信しない**）
 *   3. `@vscode/test-electron` でダウンロード（CI 用。`.vscode-test/` に入る）
 *
 * ddq の用意:
 *   1. 引数 / 環境変数 `DDQ_BIN`
 *   2. `../../cli/target/release/ddq(.exe)`（手元でビルドしてあるもの）
 *   3. `tests/host/fake-ddq.mjs`（CI 用。実物の出力を採取した golden を返す）
 *
 * 専用の `--user-data-dir` で起動するので、実際に使っているプロファイルは汚さない。
 * `--disable-workspace-trust` が要る: 新しいプロファイルだと信頼されていない扱いになり、
 * 拡張が読み込まれない。
 */
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { downloadAndUnzipVSCode, runTests } from '@vscode/test-electron';

const here = path.dirname(fileURLToPath(import.meta.url));
const extensionDevelopmentPath = path.resolve(here, '../..');
const repo = path.resolve(extensionDevelopmentPath, '../..');

const work = fs.mkdtempSync(path.join(os.tmpdir(), 'ddq-revision-host-'));
const docs = makeWorkspace(work);
const ddq = resolveDdq(work);
console.log(`ddq: ${ddq}`);

// 拡張には設定 ddqRevision.ddqPath 経由で渡す（設定の経路もここで確かめる）
fs.mkdirSync(path.join(work, '.vscode'), { recursive: true });
fs.writeFileSync(
    path.join(work, '.vscode', 'settings.json'),
    JSON.stringify({ 'ddqRevision.ddqPath': ddq }, null, 2)
);

const vscodeExecutablePath = await resolveVsCode();
console.log(`VSCode: ${vscodeExecutablePath}`);

try {
    await runTests({
        vscodeExecutablePath,
        extensionDevelopmentPath,
        extensionTestsPath: path.join(here, 'index.js'),
        extensionTestsEnv: { DDQ_BIN: ddq, HOST_OUT: work, DOCS_DIR: docs },
        launchArgs: [
            work,
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
console.log('\n実拡張ホストでの検証に合格しました。');

/** 検証用の執筆フォルダを作る（tag-list.golden.json と同じ中身にする）。 */
function makeWorkspace(root) {
    const dir = path.join(root, 'docs');
    fs.mkdirSync(path.join(dir, 'chapters', '01-overview'), { recursive: true });
    fs.writeFileSync(
        path.join(dir, '_quarto.yml'),
        'project:\n  type: book\nbook:\n  chapters:\n    - index.qmd\n    - chapters/01-overview/index.qmd\n'
    );
    fs.writeFileSync(path.join(dir, 'index.qmd'), '# 本書について {.unnumbered}\n\n前書き。\n');
    fs.writeFileSync(
        path.join(dir, 'chapters', '01-overview', 'index.qmd'),
        '# 概要\n\n導入の本文。\n\n## 目的\n\n目的の本文。\n\n## 対象範囲\n\n範囲の本文。\n'
    );
    return dir;
}

/** 実物の ddq があればそれを、無ければ差し替え（golden を返す）を使う。 */
function resolveDdq(root) {
    const explicit = process.argv[2] || process.env.DDQ_BIN;
    if (explicit && fs.existsSync(explicit)) return explicit;
    const built = path.join(
        repo,
        'cli',
        'target',
        'release',
        process.platform === 'win32' ? 'ddq.exe' : 'ddq'
    );
    if (fs.existsSync(built)) return built;

    // execFile から直接呼べる形にする（.mjs はそのままでは起動できない）
    const fake = path.join(here, 'fake-ddq.mjs');
    if (process.platform === 'win32') {
        const shim = path.join(root, 'ddq.cmd');
        fs.writeFileSync(shim, `@echo off\r\n"${process.execPath}" "${fake}" %*\r\n`);
        return shim;
    }
    const shim = path.join(root, 'ddq');
    fs.writeFileSync(shim, `#!/bin/sh\nexec "${process.execPath}" "${fake}" "$@"\n`, { mode: 0o755 });
    return shim;
}

/** インストール済みの VSCode を優先し、無ければダウンロードする。 */
async function resolveVsCode() {
    const candidates = [
        process.env.VSCODE_BIN,
        process.platform === 'win32'
            ? path.join(process.env.LOCALAPPDATA ?? '', 'Programs', 'Microsoft VS Code', 'Code.exe')
            : undefined,
        process.platform === 'win32' ? 'C:\\Program Files\\Microsoft VS Code\\Code.exe' : undefined,
        process.platform === 'darwin'
            ? '/Applications/Visual Studio Code.app/Contents/MacOS/Electron'
            : undefined
    ].filter((p) => typeof p === 'string' && p !== '');
    for (const candidate of candidates) {
        if (fs.existsSync(candidate)) return candidate;
    }
    try {
        const which = process.platform === 'win32' ? 'where' : 'which';
        const found = execFileSync(which, ['code'], { encoding: 'utf8' }).split(/\r?\n/)[0];
        if (found && fs.existsSync(found)) return found;
    } catch {
        /* インストールされていないだけ */
    }
    console.log('インストール済みの VSCode が見つからないので、テスト用の VSCode を取得します…');
    return await downloadAndUnzipVSCode();
}
