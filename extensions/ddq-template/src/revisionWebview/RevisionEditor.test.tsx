/**
 * @vitest-environment jsdom
 */
/**
 * 改訂履歴の編集画面の描画。見るのは「表が組まれること」と「直した内容が
 * どういう形で拡張へ渡るか」（直した 1 か所と直す前の値。拡張が文書のいまの内容に当てる）と、
 * 拡張から届いた文書の内容に、まだ処理されていない画面の変更をどう重ねるか（H-02）。
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
            line: 5,
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
    checked: true,
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

/** 拡張から文書の内容が届いた（`mark` は edit を処理した印） */
function receive(doc: Revision, mark: { applied?: number; reset?: boolean } = {}) {
    act(() => {
        window.dispatchEvent(new MessageEvent('message', { data: { type: 'revision', revision: doc, context, ...mark } }));
    });
}

/** `revision` の `label` の行のメモを `note` にしたもの */
function withNote(label: string, note: string, from: Revision = revision): Revision {
    return { ...from, entries: from.entries.map((e) => (e.label === label ? { ...e, note } : e)) };
}

/** その欄に打つ（React が拾う input イベントで） */
function typeInto(field: HTMLTextAreaElement | HTMLInputElement, value: string) {
    act(() => {
        const proto = field instanceof HTMLTextAreaElement ? window.HTMLTextAreaElement : window.HTMLInputElement;
        Object.getOwnPropertyDescriptor(proto.prototype, 'value')!.set!.call(field, value);
        field.dispatchEvent(new Event('input', { bubbles: true }));
    });
}

const textareas = () => [...container.querySelectorAll('textarea')];
const edits = () => posted.filter((m) => m.type === 'edit');

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

    it('メモを書くと、直した 1 か所と直す前の値が拡張へ渡る（改訂全体は送らない。H-02）', () => {
        render();
        typeInto(textareas()[0], '対象システムに○○を追加');
        expect(posted.at(-1)).toEqual({
            type: 'edit',
            id: 1,
            change: { field: 'note', label: 'sec-purpose', before: '', after: '対象システムに○○を追加' }
        });
        // 続けて打つと、前に打った値を直す前の値として送る
        typeInto(textareas()[0], '対象システムに○○を追加した');
        expect(posted.at(-1)).toEqual({
            type: 'edit',
            id: 2,
            change: {
                field: 'note',
                label: 'sec-purpose',
                before: '対象システムに○○を追加',
                after: '対象システムに○○を追加した'
            }
        });
    });

    it('改訂日も 1 か所として渡る', () => {
        render();
        typeInto(container.querySelector('input')!, '2026-10-01');
        expect(posted.at(-1)).toEqual({
            type: 'edit',
            id: 1,
            change: { field: 'date', before: '2026-09-22', after: '2026-10-01' }
        });
    });

    it('打っている最中に届いた文書の内容は、ほかの行の変更を出し、打っている行は巻き戻さない（H-02）', () => {
        render();
        typeInto(textareas()[0], 'a');
        typeInto(textareas()[0], 'ab');
        // ほかのエディタで 2 行目が直り、1 つ目の書き戻しだけが当たった版が届いた
        receive(withNote('sec-purpose', 'a', withNote('sec-gone', 'editor B')), { applied: 1 });
        expect(textareas()[0].value).toBe('ab');
        expect(textareas()[1].value).toBe('editor B');
        // 2 つ目も当たった後に、ほかで 1 行目が直されたら、文書の値を出す
        receive(withNote('sec-purpose', 'ab', withNote('sec-gone', 'editor B')), { applied: 2 });
        receive(withNote('sec-purpose', 'ほかで直した', withNote('sec-gone', 'editor B')));
        expect(textareas()[0].value).toBe('ほかで直した');
    });

    it('A → B → A と打ったとき、途中の版が届いても最後に打った値のまま', () => {
        render();
        typeInto(textareas()[0], 'A');
        typeInto(textareas()[0], 'B');
        typeInto(textareas()[0], 'A');
        receive(withNote('sec-purpose', 'A'), { applied: 1 });
        receive(withNote('sec-purpose', 'B'), { applied: 2 });
        expect(textareas()[0].value).toBe('A');
    });

    it('打った末尾の改行は、文書に当たって読み戻されても消さない', () => {
        render();
        typeInto(textareas()[0], '一行目\n');
        // 文書には末尾の空行が残らない
        receive(withNote('sec-purpose', '一行目'), { applied: 1 });
        expect(textareas()[0].value).toBe('一行目\n');
        typeInto(textareas()[0], '一行目\n二行目');
        expect(edits().at(-1)).toMatchObject({ change: { before: '一行目\n', after: '一行目\n二行目' } });
    });

    it('当てなかった（reset）なら、打っていた値を捨てて文書の内容を出す', () => {
        render();
        typeInto(textareas()[0], '古い画面で書いたメモ');
        receive(withNote('sec-purpose', 'ほかのエディタで書いたメモ'), { applied: 1, reset: true });
        expect(textareas()[0].value).toBe('ほかのエディタで書いたメモ');
        // 次に打つときは、文書の値を直す前の値として送る
        typeInto(textareas()[0], 'ほかのエディタで書いたメモ。追記');
        expect(edits().at(-1)).toMatchObject({ change: { before: 'ほかのエディタで書いたメモ' } });
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
            file: 'chapters/01-overview/01-purpose.qmd',
            line: 5
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

    it('stale の行は消せる（消す前のメモを添える）', () => {
        render();
        act(() => button('消す').dispatchEvent(new MouseEvent('click', { bubbles: true })));
        expect(posted.at(-1)).toEqual({
            type: 'edit',
            id: 1,
            change: { field: 'drop', label: 'tbl-old', before: '書きかけのメモ' }
        });
        // 拡張が処理するまでも、画面からは消える
        expect(container.querySelector('.stale')).toBeNull();
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

    it('タグの状態を確かめ終わるまでは編集させず、確かめたら編集できる（M-01）', () => {
        // 中身が届く前（開いた直後）も編集欄は使えない
        act(() => root.render(<RevisionEditor />));
        expect(container.querySelector('input')?.disabled).toBe(true);

        render({ checked: false });
        expect(container.textContent).toContain('タグの状態を確かめています');
        expect(container.textContent).not.toContain('確定済み');
        expect(container.querySelector('textarea')?.disabled).toBe(true);
        expect(container.querySelector('input')?.disabled).toBe(true);
        expect(button('差分を取り直す').disabled).toBe(true);
        expect(button('確定する').disabled).toBe(true);
        expect(button('消す').disabled).toBe(true);
        expect(button('表を作る').disabled).toBe(false);

        act(() => {
            window.dispatchEvent(
                new MessageEvent('message', { data: { type: 'revision', revision, context } })
            );
        });
        expect(container.textContent).not.toContain('タグの状態を確かめています');
        expect(container.querySelector('textarea')?.disabled).toBe(false);
        expect(button('確定する').disabled).toBe(false);
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
