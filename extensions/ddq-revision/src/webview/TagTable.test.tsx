/**
 * @vitest-environment jsdom
 */
/**
 * Webview の描画（jsdom）。
 *
 * 「パネルが真っ白」は実拡張ホストの検証でも気付きにくい（タブは開くため）ので、
 * ここで表が実際に組まれることと、書き戻しの押下で何が拡張へ渡るかを見る。
 *
 * jsdom は 26 に固定してある。27 以降は Node 22 以上を要求し、CI の Node 20 で
 * 「webidl.util.markAsUncloneable is not a function」になる（実測）。上げるときは
 * .github/workflows/extension.yml の matrix.node から 20 を外すかどうかを一緒に決める。
 */
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { TagList } from '../ddq/types';
import type { FromWebviewMessage } from '../tagging/messages';
import { TagTable } from './TagTable';

const posted: FromWebviewMessage[] = [];

vi.mock('./vscodeApi', () => ({
    vscode: () => ({
        postMessage: (m: FromWebviewMessage) => posted.push(m),
        getState: () => undefined,
        setState: () => undefined
    })
}));

const list: TagList = {
    folder: 'C:/work/order-design/docs',
    order: ['index.qmd', 'chapters/01-overview/index.qmd'],
    items: [
        {
            kind: 'heading',
            level: 1,
            file: 'index.qmd',
            line: 1,
            title: '本書について',
            label: 'sec-preface'
        },
        {
            kind: 'heading',
            level: 2,
            file: 'chapters/01-overview/index.qmd',
            line: 5,
            title: '目的',
            label: null,
            suggested: 'sec-3f9a1c',
            edit: {
                file: 'chapters/01-overview/index.qmd',
                line: 5,
                col: 5,
                insert: ' {#sec-3f9a1c}'
            }
        },
        {
            kind: 'tbl',
            file: 'chapters/01-overview/index.qmd',
            line: 9,
            title: null,
            label: null,
            warning: 'no-caption'
        }
    ],
    warnings: ['同じファイルが 2 回現れます: chapters/x.qmd']
};

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
    posted.length = 0;
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
});

afterEach(() => {
    act(() => root.unmount());
    container.remove();
});

/** 描画して、拡張から一覧を渡す。 */
function render(withList = true) {
    act(() => root.render(<TagTable />));
    if (withList) {
        act(() => {
            window.dispatchEvent(
                new MessageEvent('message', { data: { type: 'list', list, version: '2.3.0' } })
            );
        });
    }
}

describe('TagTable', () => {
    it('描画したらすぐ一覧を要求する', () => {
        render(false);
        expect(posted).toEqual([{ type: 'ready' }]);
    });

    it('一覧を受け取ると行が組まれる', () => {
        render();
        const rows = container.querySelectorAll('tbody tr');
        expect(rows).toHaveLength(3);
        expect(container.textContent).toContain('本書について');
        expect(container.textContent).toContain('sec-preface');
        // 文書順（index.qmd が先）
        expect(rows[0].textContent).toContain('index.qmd');
        // 件数と警告も出る
        expect(container.textContent).toContain('3 件（ラベル済み 1 / 候補あり 1 / 対象外 1）');
        expect(container.querySelector('.warnings')?.textContent).toContain('2 回現れます');
    });

    it('候補のある行だけが入力欄とチェックを持つ', () => {
        render();
        const inputs = container.querySelectorAll<HTMLInputElement>('td.label input');
        expect(inputs).toHaveLength(1);
        expect(inputs[0].value).toBe('sec-3f9a1c');
        expect(container.querySelectorAll('tbody input[type=checkbox]')).toHaveLength(1);
        // キャプション無しの表は理由を出す（付けない）
        expect(container.textContent).toContain('キャプション無し');
    });

    it('候補を書き換えて書き戻すと、その値が拡張へ渡る', () => {
        render();
        const input = container.querySelector<HTMLInputElement>('td.label input')!;
        act(() => {
            const setter = Object.getOwnPropertyDescriptor(
                window.HTMLInputElement.prototype,
                'value'
            )!.set!;
            setter.call(input, 'sec-purpose');
            input.dispatchEvent(new Event('input', { bubbles: true }));
        });

        const apply = [...container.querySelectorAll('button')].find((b) =>
            b.textContent?.includes('書き戻す')
        )!;
        expect(apply.textContent).toContain('1 件');
        act(() => apply.dispatchEvent(new MouseEvent('click', { bubbles: true })));

        const message = posted.at(-1);
        expect(message?.type).toBe('apply');
        if (message?.type !== 'apply') throw new Error('apply ではない');
        expect(message.rows).toHaveLength(1);
        expect(message.rows[0].label).toBe('sec-purpose');
        expect(message.rows[0].checked).toBe(true);
    });

    it('ファイル名を押すとその行を開くよう伝える', () => {
        render();
        const link = container.querySelectorAll('td.file a')[1];
        act(() => link.dispatchEvent(new MouseEvent('click', { bubbles: true })));
        expect(posted.at(-1)).toEqual({
            type: 'reveal',
            file: 'chapters/01-overview/index.qmd',
            line: 5
        });
    });

    it('検査で弾かれた行は理由を表示する', () => {
        render();
        act(() => {
            window.dispatchEvent(
                new MessageEvent('message', {
                    data: {
                        type: 'errors',
                        errors: [['chapters/01-overview/index.qmd:5', 'sec-… で始めてください']]
                    }
                })
            );
        });
        expect(container.querySelector('tr.bad')).not.toBeNull();
        expect(container.textContent).toContain('sec-… で始めてください');
    });

    it('ddq が起動できないときは理由と再試行を出す', () => {
        render(false);
        act(() => {
            window.dispatchEvent(
                new MessageEvent('message', {
                    data: { type: 'error', message: 'ddq を実行できません。', detail: 'ENOENT' }
                })
            );
        });
        expect(container.textContent).toContain('ddq を実行できません。');
        expect(container.textContent).toContain('ENOENT');
        expect(container.querySelector('table')).toBeNull();
    });
});
