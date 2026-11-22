/**
 * 子プロセスの起動・出力・終了（run.ts）を**本物の子プロセス**で確かめる（EXT-02）。
 * ddq の代わりに node でスクリプトを走らせ、引数・cwd・PATH の受け渡し、UTF-8 と行の
 * 境目、終了コード、ENOENT、並行実行、取消・拡張の終了で孫まで止まることを見る。
 */

import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';

import { afterAll, describe, expect, it } from 'vitest';

import { DdqError, lineSplitter, liveCount, run, runStreaming, stopAll } from './run';

const node = process.execPath;
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ddq-process-'));
afterAll(() => fs.rmSync(dir, { recursive: true, force: true }));

function script(name: string, body: string): string {
    const file = path.join(dir, name);
    fs.writeFileSync(file, body, 'utf8');
    return file;
}

const isResult = (line: string) => line.startsWith('OK') || line.startsWith('警告: ');

/** pid のプロセスが生きているか */
function alive(pid: number): boolean {
    try {
        process.kill(pid, 0);
        return true;
    } catch {
        return false;
    }
}

async function until(cond: () => boolean, ms = 10000): Promise<void> {
    const end = Date.now() + ms;
    while (!cond()) {
        if (Date.now() > end) throw new Error('時間内に条件を満たしませんでした');
        await new Promise((r) => setTimeout(r, 50));
    }
}

const echo = script(
    'echo.js',
    `process.stdout.write(JSON.stringify({ argv: process.argv.slice(2), cwd: process.cwd(), path: process.env.PATH }) + '\\n');`
);

describe('引数・cwd・環境変数', () => {
    const cwd = fs.mkdtempSync(path.join(dir, '受注 設計-'));
    const args = ['tag', 'list', 'C:\\work\\受注 設計 & 100%\\docs', '--json'];

    it('runStreaming: 引数はそのまま（シェルを通さない）、cwd と PATH が届く', async () => {
        const env = { ...process.env, PATH: `C:\\quarto-test\\bin${path.delimiter}${process.env.PATH ?? ''}` };
        let seen = '';
        const result = await runStreaming(node, [echo, ...args], {
            cwd,
            env,
            onOutput: (t) => (seen += t),
            keepLine: (l) => l.startsWith('{')
        });
        expect(result.code).toBe(0);
        const got = JSON.parse(result.lines[0]);
        expect(got.argv).toEqual(args);
        expect(fs.realpathSync(got.cwd)).toBe(fs.realpathSync(cwd));
        expect(got.path.startsWith('C:\\quarto-test\\bin')).toBe(true);
        expect(seen).toBe(`${result.lines[0]}\n`);
    });

    it('run: JSON の取得も同じ引数・cwd で起動する', async () => {
        const got = JSON.parse(await run(node, [echo, ...args], cwd));
        expect(got.argv).toEqual(args);
        expect(fs.realpathSync(got.cwd)).toBe(fs.realpathSync(cwd));
    });
});

describe('出力の境目', () => {
    // 「警告」の 1 文字目（e8 ad a6）の途中で切り、行も stdout / stderr の断片をまたいで届ける
    const chunks = script(
        'chunks.js',
        `const w = (s, b) => new Promise((r) => s.write(b, r));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
(async () => {
  const warn = Buffer.from('警告: 図がありません\\n', 'utf8');
  await w(process.stdout, warn.subarray(0, 1)); await sleep(30);
  await w(process.stdout, warn.subarray(1)); await sleep(30);
  await w(process.stdout, Buffer.from('OK: C:\\\\受注\\\\_book', 'utf8')); await sleep(30);
  await w(process.stderr, Buffer.from('[quarto] 進捗 1/2\\r\\n途中の', 'utf8')); await sleep(30);
  await w(process.stdout, Buffer.from('\\\\index.html（直接開くと全文検索は使えません）\\r\\n', 'utf8')); await sleep(30);
  await w(process.stderr, Buffer.from('行\\n', 'utf8'));
  await w(process.stdout, Buffer.from('OK -> 改行なしで終わる', 'utf8'));
  process.exitCode = 0;
})();`
    );

    it('日本語の 1 文字が割れず、行は流れごとに組み立てられ、結果行だけが残る', async () => {
        let seen = '';
        const result = await runStreaming(node, [chunks], { onOutput: (t) => (seen += t), keepLine: isResult });
        expect(result.code).toBe(0);
        expect(seen).not.toContain('\uFFFD');
        expect(seen).toContain('警告: 図がありません');
        expect(result.lines).toEqual([
            '警告: 図がありません',
            'OK: C:\\受注\\_book\\index.html（直接開くと全文検索は使えません）',
            'OK -> 改行なしで終わる'
        ]);
    });

    it('lineSplitter: CRLF / LF / 末尾の改行なしを同じに扱う', () => {
        const got: string[] = [];
        const s = lineSplitter((l) => got.push(l));
        for (const piece of ['a\r', '\nb', '\n\nc\r\n', 'd']) s.push(piece);
        s.end();
        s.end();
        expect(got).toEqual(['a', 'b', '', 'c', 'd']);
    });

    it('全出力は溜めない（keepLine を省けば何も残さない）', async () => {
        const many = script('many.js', `for (let i = 0; i < 20000; i++) console.log('[quarto] 進捗 ' + i);`);
        let bytes = 0;
        const result = await runStreaming(node, [many], { onOutput: (t) => (bytes += t.length) });
        expect(result.code).toBe(0);
        expect(result.lines).toEqual([]);
        expect(bytes).toBeGreaterThan(200000);
    });
});

describe('終了コードと起動の失敗', () => {
    it('終了コード 3（機構の不一致）をそのまま返す', async () => {
        const three = script('three.js', `console.error('Error: 機構が一致しません。ddq update を実行してください'); process.exit(3);`);
        const result = await runStreaming(node, [three], { onOutput: () => {} });
        expect(result.code).toBe(3);
        await expect(run(node, [three])).rejects.toMatchObject({ detail: expect.stringContaining('ddq update') });
    });

    it('ENOENT は起動できない理由を返す（reject しない）', async () => {
        const missing = path.join(dir, 'no-such-ddq.exe');
        const result = await runStreaming(missing, ['pdf'], { onOutput: () => {} });
        expect(result.code).toBeNull();
        expect(result.error).toMatch(/実行できません.*ddqTemplate\.ddqPath/);
        await expect(run(missing, ['--version'])).rejects.toBeInstanceOf(DdqError);
        expect(liveCount()).toBe(0);
    });
});

describe('並行・取消・拡張の終了', () => {
    // 孫まで止めるのは Windows だけ（killTree が `taskkill /T` を使う。計画 EXT-02 の「取消時の Windows
    // 子孫終了」）。ほかの OS は変更前と同じく ddq（子）だけを止めるので、孫は確かめずに試験の後で片づける
    const stopsGrandchildren = process.platform === 'win32';
    const settle = async (pids: number[]) => {
        if (stopsGrandchildren) {
            await until(() => pids.every((pid) => !alive(pid)));
            return;
        }
        for (const pid of pids) {
            try {
                process.kill(pid);
            } catch {
                // もう終わっている
            }
        }
    };
    const sleeper = script(
        'sleeper.js',
        `const { spawn } = require('node:child_process');
const fs = require('node:fs');
// 孫（quarto → pandoc / typst の代わり）を起動し、その pid を知らせてから待つ。
// detached にする: node の子は libuv のジョブオブジェクトに入り、親の node が死ぬと
// 一緒に消える。それでは「孫まで止めた」ことを確かめられない（実際の ddq → quarto も
// node のジョブには入らない）
const grandchild = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 60000)'], { stdio: 'ignore', detached: true });
grandchild.unref();
fs.writeFileSync(process.argv[2], String(grandchild.pid));
console.log('ready ' + process.pid);
setTimeout(() => {}, 60000);`
    );

    it('別々の実行は互いに待たない', async () => {
        const slow = script('slow.js', `setTimeout(() => console.log('OK: ' + process.argv[2]), 300);`);
        const a = runStreaming(node, [slow, 'a'], { onOutput: () => {}, keepLine: isResult });
        const b = runStreaming(node, [slow, 'b'], { onOutput: () => {}, keepLine: isResult });
        expect(liveCount()).toBe(2);
        const [ra, rb] = await Promise.all([a, b]);
        expect(ra.lines).toEqual(['OK: a']);
        expect(rb.lines).toEqual(['OK: b']);
        expect(liveCount()).toBe(0);
    });

    it('取消（kill）で子が止まる（Windows では孫も）', { timeout: 20000 }, async () => {
        const pidFile = path.join(dir, 'grandchild-cancel.pid');
        let kill: (() => void) | undefined;
        let seen = '';
        const pending = runStreaming(node, [sleeper, pidFile], {
            onOutput: (t) => (seen += t),
            onSpawn: (c) => (kill = c.kill)
        });
        await until(() => seen.includes('ready') && fs.existsSync(pidFile));
        const grandchild = Number(fs.readFileSync(pidFile, 'utf8'));
        expect(alive(grandchild)).toBe(true);
        kill?.();
        const result = await pending;
        expect(result.code).not.toBe(0);
        await settle([grandchild]);
        expect(liveCount()).toBe(0);
    });

    it('stopAll（拡張の終了）は JSON の実行も長い実行も止める（Windows では孫ごと）', { timeout: 20000 }, async () => {
        const pidA = path.join(dir, 'grandchild-a.pid');
        const pidB = path.join(dir, 'grandchild-b.pid');
        const json = run(node, [sleeper, pidA]).catch((e: unknown) => e);
        const stream = runStreaming(node, [sleeper, pidB], { onOutput: () => {} });
        await until(() => fs.existsSync(pidA) && fs.existsSync(pidB));
        const grandchildren = [pidA, pidB].map((f) => Number(fs.readFileSync(f, 'utf8')));
        expect(liveCount()).toBe(2);
        stopAll();
        expect(liveCount()).toBe(0);
        expect(await json).toBeInstanceOf(DdqError);
        expect((await stream).code).not.toBe(0);
        await settle(grandchildren);
    });
});
