#!/usr/bin/env node
/**
 * 実拡張ホストでの検証で、`ddq.exe` の代わりに使う差し替え（CI 用）。
 *
 * 拡張の CI は Rust をビルドしない（ddq の CI は別の workflow）。そこで
 * **実物の `ddq tag list --json` の出力を採取した** `tag-list.golden.json` を返す。
 * 検証用の執筆フォルダは `run.mjs` がこの golden と同じ内容で作るので、行番号も
 * 挿入位置も実物と一致する。golden は次のコマンドで採り直す:
 *
 *   ddq tag list <検証用フォルダ> --json   （folder を "{{FOLDER}}" に置き換えて保存）
 *
 * 手元に `ddq.exe` があるときは run.mjs がそちらを使うので、この差し替えは通らない。
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);

if (args[0] === '--version') {
    const version = fs
        .readFileSync(path.resolve(here, '../../../../template/VERSION'), 'utf8')
        .trim();
    console.log(`ddq ${version}`);
    process.exit(0);
}

if (args[0] === 'tag' && args[1] === 'list') {
    const folder = args[2] ?? '';
    const golden = fs.readFileSync(path.join(here, 'tag-list.golden.json'), 'utf8');
    process.stdout.write(golden.replace('{{FOLDER}}', folder.replace(/\\/g, '\\\\')));
    process.exit(0);
}

console.error(`fake-ddq: 知らない呼び出し: ${args.join(' ')}`);
process.exit(1);
