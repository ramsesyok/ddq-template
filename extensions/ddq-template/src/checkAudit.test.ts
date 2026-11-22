/**
 * 配布物に入る依存の脆弱性検査（scripts/check-audit.mjs。CI-02）。
 *
 * 模擬の監査結果（`npm audit --json` と同じ形）を与えて、配布物に入る依存の指摘は失敗、
 * 開発用だけの依存の指摘は参考（失敗にしない）になることを見る。npm audit は呼ばない（通信しない）。
 * 最後の 1 件だけは Webview を本当にビルドして、バンドルに入るパッケージを数える。
 */

import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterAll, describe, expect, it } from 'vitest';

const script = join(__dirname, '..', 'scripts', 'check-audit.mjs');
const work = mkdtempSync(join(tmpdir(), 'ddq-check-audit-'));
afterAll(() => rmSync(work, { recursive: true, force: true }));

const BUNDLED = 'react,react-dom,scheduler';

let n = 0;
function file(value: unknown): string {
    const p = join(work, `${n++}.json`);
    writeFileSync(p, JSON.stringify(value));
    return p;
}

/** npm audit --json（auditReportVersion 2）の形の指摘 */
function vulnerability(name: string, severity: string) {
    return {
        name,
        severity,
        isDirect: false,
        via: [{ source: 1, name, title: '模擬の指摘', severity, range: '<99.0.0' }],
        effects: [],
        range: '<99.0.0',
        nodes: [`node_modules/${name}`],
        fixAvailable: true
    };
}

function report(...vulns: [string, string][]) {
    return file({
        auditReportVersion: 2,
        vulnerabilities: Object.fromEntries(vulns.map(([name, sev]) => [name, vulnerability(name, sev)])),
        metadata: {}
    });
}

function run(...args: string[]) {
    const r = spawnSync(process.execPath, [script, ...args], { encoding: 'utf8' });
    return { code: r.status, out: r.stdout, err: r.stderr };
}

describe('check-audit（模擬の監査結果）', () => {
    it('指摘が無ければ通る', () => {
        const r = run('--report', report(), '--bundled', BUNDLED);
        expect(r.err).toBe('');
        expect(r.code).toBe(0);
        expect(r.out).toContain('対象: react, react-dom, scheduler（Webview のバンドル 3 / 本番依存 0）');
        expect(r.out).toContain('配布物に入る依存の指摘 0 件');
    });

    it.each([
        ['react', 'high'],
        ['react-dom', 'critical'],
        ['scheduler', 'low']
    ])('Webview のバンドルに入る %s の指摘（%s）は失敗', (name, severity) => {
        const r = run('--report', report([name, severity], ['vitest', 'moderate']), '--bundled', BUNDLED);
        expect(r.code).toBe(1);
        expect(r.err).toContain(`NG  配布物に入る ${name}（${severity}、<99.0.0）`);
        // 開発用の指摘は失敗の理由にしない
        expect(r.err).not.toContain('vitest');
        expect(r.out).toContain('参考  vitest（moderate、<99.0.0） … 開発用の依存だけ（配布物に入らない）');
    });

    it('開発用だけの依存の指摘は、重くても参考にして通す', () => {
        const r = run('--report', report(['vitest', 'critical'], ['@vscode/vsce', 'high']), '--bundled', BUNDLED);
        expect(r.code).toBe(0);
        expect(r.out).toContain('参考  vitest（critical、<99.0.0） … 開発用の依存だけ');
        expect(r.out).toContain('参考  @vscode/vsce（high、<99.0.0） … 開発用の依存だけ');
        expect(r.out).toContain('脆弱性検査に合格しました。');
    });

    it('配布物に入る依存でも info は参考（従来の --audit-level=low と同じ）', () => {
        const r = run('--report', report(['react', 'info']), '--bundled', BUNDLED);
        expect(r.code).toBe(0);
        expect(r.out).toContain('参考  react（info、<99.0.0）');
    });

    it('本番依存（lock で dev でないもの）ができたら、その指摘も失敗', () => {
        const lock = file({
            lockfileVersion: 3,
            packages: {
                '': { name: 'ddq-template' },
                'node_modules/left-pad': { version: '1.0.0' },
                'node_modules/vitest': { version: '4.0.0', dev: true }
            }
        });
        const r = run('--report', report(['left-pad', 'low']), '--bundled', BUNDLED, '--lock', lock);
        expect(r.code).toBe(1);
        expect(r.out).toContain('（Webview のバンドル 3 / 本番依存 1）');
        expect(r.err).toContain('NG  配布物に入る left-pad（low');
    });

    it('npm audit が失敗した（通信できない）ら、通さない', () => {
        const r = run('--report', file({ error: { code: 'ENOTFOUND', summary: 'request to registry failed' } }), '--bundled', BUNDLED);
        expect(r.code).toBe(1);
        expect(r.err).toContain('npm audit が失敗しました（検査できていません）: request to registry failed');
    });

    it('バンドルに入るものがライセンス表示の一覧と違えば失敗（表示が古い）', () => {
        const r = run('--report', report(), '--bundled', 'react,react-dom');
        expect(r.code).toBe(1);
        expect(r.err).toContain('Webview のバンドルに入るパッケージ（react, react-dom）が、ライセンス表示の一覧');
        expect(r.err).toContain('EXTENSION_BUNDLED: react, react-dom, scheduler');
    });
});

describe('check-audit（実ビルド）', () => {
    it('Webview のバンドルに入るのは、ライセンス表示の一覧の 3 つ', { timeout: 120_000 }, () => {
        const r = run('--report', report(['vitest', 'high']));
        expect(r.err).toBe('');
        expect(r.code).toBe(0);
        expect(r.out).toContain('OK  Webview のバンドル: react, react-dom, scheduler（ライセンス表示の一覧と一致）');
    });

    it('一覧から 1 つ落ちていれば、実際のバンドルとの違いで失敗', { timeout: 120_000 }, () => {
        const list = join(work, 'third_party.py');
        writeFileSync(list, 'INPUTS = ["x"]\nEXTENSION_BUNDLED = ["react", "react-dom"]\n');
        const r = run('--report', report(), '--third-party', list);
        expect(r.code).toBe(1);
        expect(r.err).toContain(
            'Webview のバンドルに入るパッケージ（react, react-dom, scheduler）が、ライセンス表示の一覧' +
                '（cli/tools/third_party.py の EXTENSION_BUNDLED: react, react-dom）と違います'
        );
    });
});
