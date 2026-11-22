/**
 * 起動口（launch.ts）。VSCode は差し替え、ddq の代わりに**本物の子プロセス**（node）を走らせる。
 * 進捗通知の取消、拡張の終了（deactivate）後に出力・通知を出さないこと、結果行の扱いを見る（EXT-02）。
 */

import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';

import { afterAll, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('vscode', () => import('../testing/fakeVscode'));

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ddq-launch-'));
afterAll(() => fs.rmSync(dir, { recursive: true, force: true }));
const script = path.join(dir, 'fake-ddq.js');
fs.writeFileSync(
    script,
    `const mode = process.argv[2];
if (mode === 'html') { console.log('[quarto] 進捗'); console.log('OK: C:\\\\docs\\\\_book\\\\index.html（直接開くと全文検索は使えません）'); }
else if (mode === 'wait') { console.log('started'); setTimeout(() => {}, 60000); }
else process.exit(1);`,
    'utf8'
);
// 設定の ddqTemplate.ddqPath に node を指定した扱い（引数の先頭にスクリプトを置く）
vi.mock('./locate', () => ({ locateDdq: () => ({ command: process.execPath, source: 'setting' }) }));

import { fake } from '../testing/fakeVscode';
import { isResultLine } from '../build';
import { activateLaunch, deactivateLaunch, runWithProgress, showError } from './launch';
import { liveCount } from './run';

async function until(cond: () => boolean, ms = 10000) {
    const end = Date.now() + ms;
    while (!cond()) {
        if (Date.now() > end) throw new Error('時間内に条件を満たしませんでした');
        await new Promise((r) => setTimeout(r, 30));
    }
}

beforeEach(() => {
    fake.reset();
    activateLaunch();
});

describe('runWithProgress', () => {
    it('出力は全部チャネルへ流し、結果には結果行だけを残す', async () => {
        const result = await runWithProgress('HTML を作成', [script, 'html'], dir, process.env, isResultLine);
        if (result === 'cancelled') throw new Error('cancelled');
        expect(result.code).toBe(0);
        expect(result.lines).toEqual(['OK: C:\\docs\\_book\\index.html（直接開くと全文検索は使えません）']);
        expect(fake.output).toContain('[quarto] 進捗');
        expect(fake.output).toContain('（終了コード 0）');
    });

    it('進捗通知の取消で止まり、cancelled を返す', { timeout: 20000 }, async () => {
        const pending = runWithProgress('PDF を作成', [script, 'wait'], dir, process.env);
        await until(() => fake.output.includes('started'));
        fake.cancel?.();
        expect(await pending).toBe('cancelled');
        expect(liveCount()).toBe(0);
    });

    it('拡張の終了後は、止めた実行の結果を出力にも通知にも出さない', { timeout: 20000 }, async () => {
        const pending = runWithProgress('PDF を作成', [script, 'wait'], dir, process.env);
        await until(() => fake.output.includes('started'));
        deactivateLaunch();
        const before = fake.output;
        expect(await pending).toBe('cancelled');
        expect(liveCount()).toBe(0);
        // 「（終了コード -）」などを書き足さない
        expect(fake.output).toBe(before);
        await showError('ddq pdf が失敗しました');
        expect(fake.shown('showErrorMessage')).toEqual([]);
    });
});
