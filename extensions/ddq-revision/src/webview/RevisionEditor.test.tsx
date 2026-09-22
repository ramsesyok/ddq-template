/**
 * @vitest-environment jsdom
 */
/**
 * 改訂履歴の編集画面の描画。見るのは「表が組まれること」と「直した内容が
 * どういう形で拡張へ渡るか」（拡張はそれをそのまま YAML にして文書へ書く）。
 */
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { EditorContext, FromEditorMessage } from '../revision/messages';
import type { Revision } from '../revision/revfile';
import { RevisionEditor } from './RevisionEditor';

const posted: FromEditorMessage[] = [];

vi.mock('./vscodeApi', () => ({
    vscode: () => ({ postMessage: () => undefined, getState: () => undefined, setState: () => undefined }),
    editorApi: () => ({
        postMessage: (m: FromEditorMessage) => posted.push(m),
        getState: () => undefined,
        setState: () => undefined
    })
}));

const revision: Revision = {
    rev: 'B',
    date: '2026-09-22',
    base: 'rev-A',
    baseCommit: '75468c39af440081ad9a2903cedfc1e1873f24cc',
    scheme: 'alpha',
    entries: [
        {
            label: 'sec-purpose',
            kind: 'changed',
            unit: 'heading',
            title: '目的',
            file: 'chapters/01-overview/01-purpose.qmd',
            note: '',
            stale: false
        },
        {
            label: 'sec-gone',
            kind: 'removed',
            unit: 'heading',
            title: '他システムとは疎結合とする',
            file: 'chapters/02-design-policy/03-loose-coupling.qmd',
            note: '第4章に統合した',
            stale: false
        },
        {
            label: 'tbl-old',
            kind: 'changed',
            unit: 'tbl',
            title: '古い表',
            file: 'chapters/03/index.qmd',
            note: '書きかけのメモ',
            stale: true
        }
    ]
};

const context: EditorContext = {
    folder: 'C:/work/order-design/docs',
    fileName: 'rev-B.yml',
    fixed: false,
    tags: ['rev-A']
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

function render(over: Partial<EditorContext> = {}) {
    act(() => root.render(<RevisionEditor />));
    act(() => {
        window.dispatchEvent(
            new MessageEvent('message', {
                data: { type: 'revision', revision, context: { ...context, ...over } }
            })
        );
    });
}

function button(text: string): HTMLButtonElement {
    const found = [...container.querySelectorAll('button')].find((b) => b.textContent?.includes(text));
    if (!found) throw new Error(`ボタンが無い: ${text}`);
    return found;
}

describe('RevisionEditor', () => {
    it('開いたらすぐ中身を要求する', () => {
        act(() => root.render(<RevisionEditor />));
        expect(posted).toEqual([{ type: 'ready' }]);
    });

    it('差分の一覧が組まれる（stale は別枠）', () => {
        render();
        expect(container.querySelector('h1')?.textContent).toContain('改訂 B');
        const rows = container.querySelectorAll('tbody tr');
        // 生きている 2 件 + stale 1 件（別の表）
        expect(container.querySelectorAll('table')).toHaveLength(2);
        expect(rows.length).toBe(3);
        expect(container.textContent).toContain('目的');
        expect(container.textContent).toContain('第4章に統合した');
        expect(container.querySelector('.stale')?.textContent).toContain('書きかけのメモ');
        // 空のメモの件数を出す
        expect(container.textContent).toContain('修正内容が空: 1 件');
        // 基準の SHA は先頭 8 桁
        expect(container.textContent).toContain('75468c39');
    });

    it('メモを書くと改訂の中身がまるごと拡張へ渡る', () => {
        render();
        const textarea = container.querySelector('textarea')!;
        act(() => {
            const setter = Object.getOwnPropertyDescriptor(
                window.HTMLTextAreaElement.prototype,
                'value'
            )!.set!;
            setter.call(textarea, '対象システムに○○を追加');
            textarea.dispatchEvent(new Event('input', { bubbles: true }));
        });
        const message = posted.at(-1);
        expect(message?.type).toBe('edit');
        if (message?.type !== 'edit') throw new Error('edit ではない');
        expect(message.revision.entries[0].note).toBe('対象システムに○○を追加');
        // 他の行は変えない
        expect(message.revision.entries[1].note).toBe('第4章に統合した');
    });

    it('行の差分ボタンはラベルを渡す', () => {
        render();
        // ヘッダの「差分を取り直す」と紛れないよう、行の中のボタンを取る
        const inRow = container.querySelector<HTMLButtonElement>('td.diff button')!;
        act(() => inRow.dispatchEvent(new MouseEvent('click', { bubbles: true })));
        expect(posted.at(-1)).toEqual({ type: 'diff', label: 'sec-purpose' });
    });

    it('削除された行は場所を開かない（新版に無いため）', () => {
        render();
        const links = container.querySelectorAll('td.place a');
        act(() => links[1].dispatchEvent(new MouseEvent('click', { bubbles: true })));
        expect(posted.at(-1)).toEqual({ type: 'ready' });
        act(() => links[0].dispatchEvent(new MouseEvent('click', { bubbles: true })));
        expect(posted.at(-1)).toEqual({
            type: 'reveal',
            file: 'chapters/01-overview/01-purpose.qmd'
        });
    });

    it('取り直し・表の生成・確定を拡張へ伝える', () => {
        render();
        act(() => button('差分を取り直す').dispatchEvent(new MouseEvent('click', { bubbles: true })));
        expect(posted.at(-1)).toEqual({ type: 'refresh', base: undefined });
        act(() => button('表を作る').dispatchEvent(new MouseEvent('click', { bubbles: true })));
        expect(posted.at(-1)).toEqual({ type: 'build' });
        act(() => button('確定する').dispatchEvent(new MouseEvent('click', { bubbles: true })));
        expect(posted.at(-1)).toEqual({ type: 'fix' });
    });

    it('stale の行は消せる', () => {
        render();
        act(() => button('消す').dispatchEvent(new MouseEvent('click', { bubbles: true })));
        const message = posted.at(-1);
        if (message?.type !== 'edit') throw new Error('edit ではない');
        expect(message.revision.entries.map((e) => e.label)).toEqual(['sec-purpose', 'sec-gone']);
    });

    it('タグ済みなら編集させない', () => {
        render({ fixed: true });
        expect(container.textContent).toContain('確定済み');
        expect(container.querySelector('textarea')?.disabled).toBe(true);
        expect(button('差分を取り直す').disabled).toBe(true);
        expect(button('確定する').disabled).toBe(true);
        // 表を作るのは確定後でもできる
        expect(button('表を作る').disabled).toBe(false);
    });

    it('ddq からの知らせを出す', () => {
        render();
        act(() => {
            window.dispatchEvent(
                new MessageEvent('message', {
                    data: { type: 'notice', level: 'error', text: 'git に失敗しました' }
                })
            );
        });
        expect(container.querySelector('.notice.error')?.textContent).toBe('git に失敗しました');
    });
});
