#!/usr/bin/env node
/**
 * 実拡張ホストでの検証を起動する（tests/host/index.js の説明を参照）。
 *
 *   node tests/host/run.mjs [<ddq.exe のパス>]
 *
 * インストール済みの VSCode を使う（別の VSCode をダウンロードしない = オフラインで動く）。
 * 専用の --user-data-dir で起動するので、実際に使っているプロファイルは汚さない。
 * --disable-workspace-trust が要る: 新しいプロファイルだと信頼されていない扱いになり、
 * 拡張が読み込まれない。
 *
 * 検証用の執筆フォルダは一時フォルダに作る（リポジトリの examples/ は触らない）。
 */
import { execFileSync, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const extension = path.resolve(here, '../..');
const repo = path.resolve(extension, '../..');

const ddq =
    process.argv[2] ||
    process.env.DDQ_BIN ||
    path.join(repo, 'cli', 'target', 'release', process.platform === 'win32' ? 'ddq.exe' : 'ddq');
if (!fs.existsSync(ddq)) {
    console.error(`ddq がありません: ${ddq}\n  cli/ で cargo build --release してください。`);
    process.exit(1);
}

const code = findVsCode();
if (!code) {
    console.error('VSCode が見つかりません（code コマンドか Code.exe）。');
    process.exit(1);
}

const work = fs.mkdtempSync(path.join(os.tmpdir(), 'ddq-revision-host-'));
const docs = path.join(work, 'docs');
fs.mkdirSync(path.join(docs, 'chapters', '01-overview'), { recursive: true });
fs.writeFileSync(
    path.join(docs, '_quarto.yml'),
    'project:\n  type: book\nbook:\n  chapters:\n    - index.qmd\n    - chapters/01-overview/index.qmd\n'
);
fs.writeFileSync(path.join(docs, 'index.qmd'), '# 本書について {.unnumbered}\n\n前書き。\n');
fs.writeFileSync(
    path.join(docs, 'chapters', '01-overview', 'index.qmd'),
    '# 概要\n\n導入の本文。\n\n## 目的\n\n目的の本文。\n\n## 対象範囲\n\n範囲の本文。\n'
);

const out = work;
fs.rmSync(path.join(out, 'host-result.txt'), { force: true });

const result = spawnSync(
    code,
    [
        '--new-window',
        '--disable-workspace-trust',
        '--user-data-dir',
        path.join(work, 'user-data'),
        '--extensions-dir',
        path.join(work, 'extensions'),
        `--extensionDevelopmentPath=${extension}`,
        `--extensionTestsPath=${path.join(here, 'index.js')}`,
        work
    ],
    { env: { ...process.env, DDQ_BIN: ddq, HOST_OUT: out }, stdio: 'inherit' }
);

const report = path.join(out, 'host-result.txt');
if (fs.existsSync(report)) {
    console.log(`\n${fs.readFileSync(report, 'utf8')}`);
}
if (result.status !== 0 || !fs.existsSync(report)) {
    console.error('\n実拡張ホストでの検証に失敗しました。');
    process.exit(1);
}
console.log('\n実拡張ホストでの検証に合格しました。');

/** インストール済みの VSCode を探す。 */
function findVsCode() {
    const candidates = [
        process.env.VSCODE_BIN,
        process.platform === 'win32'
            ? path.join(process.env.LOCALAPPDATA ?? '', 'Programs', 'Microsoft VS Code', 'Code.exe')
            : undefined,
        process.platform === 'win32' ? 'C:\\Program Files\\Microsoft VS Code\\Code.exe' : undefined,
        process.platform === 'darwin'
            ? '/Applications/Visual Studio Code.app/Contents/MacOS/Electron'
            : undefined
    ].filter(Boolean);
    for (const c of candidates) {
        if (fs.existsSync(c)) return c;
    }
    try {
        const which = process.platform === 'win32' ? 'where' : 'which';
        return execFileSync(which, ['code'], { encoding: 'utf8' }).split(/\r?\n/)[0] || undefined;
    } catch {
        return undefined;
    }
}
