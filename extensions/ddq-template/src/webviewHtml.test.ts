/**
 * Webview の器（EXT-04）。読める資産の範囲と CSP、HTML の形を見る。
 * 期待値の HTML は共通化する前の 2 か所（表エディタの buildHtml・改訂の html）の出力を
 * nonce だけ伏せて写したもの。画面の見た目と読み込みが変わらないことを確かめる。
 */

import { describe, expect, it, vi } from 'vitest';

const commands = new Map<string, () => unknown>();

vi.mock('vscode', () => ({
    commands: {
        registerCommand: (id: string, cb: () => unknown) => {
            commands.set(id, cb);
            return { dispose() {} };
        }
    },
    Uri: {
        joinPath: (base: { path: string }, ...parts: string[]) => ({ path: [base.path, ...parts].join('/') })
    }
}));

import { activateWebviewProbe, bundleRoot, noteReady, webviewHtml, webviewOptions } from './webviewHtml';

const ext = { path: '/ext' } as never;
const webview = {
    cspSource: 'vscode-resource:',
    asWebviewUri: (u: { path: string }) => `webview:${u.path}`
} as never;

const shape = (bundle: string, title: string, body: string) => `<!DOCTYPE html>
<html lang="ja">
<head>
  <meta charset="UTF-8">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src vscode-resource:; script-src 'nonce-N'; style-src vscode-resource: 'nonce-N';">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <link rel="stylesheet" nonce="N" href="webview:/ext/out/webview/${bundle}/assets/main.css">
  <title>${title}</title>
</head>
${body}
  <div id="root"></div>
  <script nonce="N" src="webview:/ext/out/webview/${bundle}/assets/main.js"></script>
</body>
</html>`;

/** nonce を伏せる（どこでも同じ値であることは別に見る） */
const masked = (html: string) => html.replace(/nonce-[A-Za-z0-9+/=]+|nonce="[A-Za-z0-9+/=]+"/g, (m) => (m.startsWith('nonce-') ? 'nonce-N' : 'nonce="N"'));

describe('Webview の器', () => {
    it('読める資産はそのバンドルの出力先だけ', () => {
        expect(webviewOptions(ext, 'tableEditor')).toEqual({
            enableScripts: true,
            localResourceRoots: [{ path: '/ext/out/webview/tableEditor' }]
        });
        expect(webviewOptions(ext, 'revision').localResourceRoots).toEqual([bundleRoot(ext, 'revision')]);
        expect(bundleRoot(ext, 'revision')).toEqual({ path: '/ext/out/webview/revision' });
    });

    it('表エディタの HTML は共通化する前と同じ', () => {
        expect(masked(webviewHtml(webview, ext, 'tableEditor', { title: 'DDQ 表エディタ' }))).toBe(
            shape('tableEditor', 'DDQ 表エディタ', '<body>')
        );
    });

    it('ラベル一覧・改訂履歴の HTML は共通化する前と同じ（data-view で画面を切り替える）', () => {
        for (const view of ['tags', 'revision']) {
            expect(masked(webviewHtml(webview, ext, 'revision', { title: 'DDQ', view }))).toBe(
                shape('revision', 'DDQ', `<body data-view="${view}">`)
            );
        }
    });

    it('CSP は通信を許さず、スクリプトとスタイルは同じ nonce で、開くたびに変わる', () => {
        const a = webviewHtml(webview, ext, 'tableEditor', { title: 't' });
        const b = webviewHtml(webview, ext, 'tableEditor', { title: 't' });
        const csp = /content="([^"]+)"/.exec(a.split('\n').find((l) => l.includes('Content-Security-Policy'))!)![1];
        expect(csp).toMatch(/^default-src 'none';/);
        expect(csp).not.toMatch(/connect-src|unsafe-inline|unsafe-eval|https?:/);
        const nonces = [...a.matchAll(/nonce(?:-|=")([A-Za-z0-9+/=]+)/g)].map((m) => m[1]);
        expect(nonces).toHaveLength(4);
        expect(new Set(nonces).size).toBe(1);
        expect(nonces[0].length).toBeGreaterThanOrEqual(22);
        expect(b).not.toContain(nonces[0]);
    });

    it('画面ごとに ready の届いた回数を内部コマンドで読める（実拡張ホストの検証用）', () => {
        activateWebviewProbe({ subscriptions: [] } as never);
        const read = () => commands.get('ddqTemplate.internal.webviewReadies')!() as Record<string, number>;
        const before = read();
        noteReady('tableEditor');
        noteReady('tableEditor');
        noteReady('revision');
        expect(read()).toEqual({ ...before, tableEditor: before.tableEditor + 2, revision: before.revision + 1 });
        // 返した値を書き換えても数には響かない
        read().tags = 99;
        expect(read().tags).toBe(before.tags);
    });
});
