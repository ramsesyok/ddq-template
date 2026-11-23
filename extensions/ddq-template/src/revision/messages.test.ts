/**
 * 改訂履歴の画面で直した 1 か所の当て方と、画面の状態の重ね方（H-02）。拡張は文書のいまの内容に
 * `applyChange` で当て、画面は `withChange` / `withDocument` / `shown` で文書の内容に自分の変更を重ねる。
 */
import { describe, expect, it } from 'vitest';

import {
    applyChange,
    shown,
    withChange,
    withDocument,
    type EditorView,
    type RevisionChange
} from './messages';
import { emptyRevision, storedNote, type RevEntry, type Revision } from './revfile';

const entry = (label: string, note: string, stale = false): RevEntry => ({
    label,
    kind: 'changed',
    unit: 'heading',
    title: label,
    file: 'a.qmd',
    note,
    stale
});

const revision = (...entries: RevEntry[]): Revision => ({ ...emptyRevision(), rev: 'B', date: '2026-09-22', entries });

const notes = (r: Revision) => r.entries.map((e) => [e.label, e.note]);

describe('applyChange（拡張が文書のいまの内容に当てる）', () => {
    const doc = revision(entry('one', '一行目'), entry('two', 'editor B'), entry('old', '残ったメモ', true));

    it('直す前の値が文書と同じなら、その 1 か所だけを変え、ほかの行はいまの文書のまま', () => {
        const result = applyChange(doc, { field: 'note', label: 'one', before: '一行目', after: 'editor A' });
        expect(result).toMatchObject({ ok: true, changed: true });
        if (!result.ok) throw new Error('当たらない');
        expect(notes(result.revision)).toEqual([
            ['one', 'editor A'],
            ['two', 'editor B'],
            ['old', '残ったメモ']
        ]);
    });

    it('直す前の値が文書と違えば（ほかで変わった）当てない。同じ値に直してあれば通す', () => {
        const stale = applyChange(doc, { field: 'note', label: 'two', before: '', after: 'editor A' });
        expect(stale).toMatchObject({ ok: false, reason: expect.stringContaining('two のメモが') });
        expect(applyChange(doc, { field: 'note', label: 'two', before: '', after: 'editor B' })).toEqual({
            ok: true,
            changed: false,
            revision: doc
        });
    });

    it('値は書いて読み戻した形で比べる（打った末尾の改行は文書に残らない）', () => {
        expect(storedNote('一行目\n\n')).toBe('一行目');
        const typed = applyChange(doc, { field: 'note', label: 'one', before: '一行目\n', after: '一行目\n続き' });
        expect(typed).toMatchObject({ ok: true, changed: true });
        expect(applyChange(doc, { field: 'note', label: 'one', before: '前の値', after: '一行目\n' })).toMatchObject({
            ok: true,
            changed: false
        });
    });

    it('文書に無い行のメモは当てない', () => {
        expect(applyChange(doc, { field: 'note', label: 'gone', before: '', after: 'x' })).toMatchObject({
            ok: false,
            reason: expect.stringContaining('gone の行が文書にありません')
        });
    });

    it('改訂日も直す前の値と照らす', () => {
        const ok = applyChange(doc, { field: 'date', before: '2026-09-22', after: '2026-10-01' });
        expect(ok.ok && ok.revision.date).toBe('2026-10-01');
        expect(applyChange(doc, { field: 'date', before: '2026-09-01', after: '2026-10-01' })).toMatchObject({
            ok: false,
            reason: expect.stringContaining('改訂日が')
        });
    });

    it('消すのは stale の行だけ。差分に戻った行・メモが変わった行は消さず、もう無ければそのまま', () => {
        const dropped = applyChange(doc, { field: 'drop', label: 'old', before: '残ったメモ' });
        expect(dropped.ok && dropped.revision.entries.map((e) => e.label)).toEqual(['one', 'two']);
        expect(applyChange(doc, { field: 'drop', label: 'one', before: '一行目' })).toMatchObject({
            ok: false,
            reason: expect.stringContaining('差分に戻った')
        });
        expect(applyChange(doc, { field: 'drop', label: 'old', before: '前のメモ' })).toMatchObject({ ok: false });
        expect(applyChange(doc, { field: 'drop', label: 'none', before: '' })).toMatchObject({ ok: true, changed: false });
    });
});

describe('画面の状態（文書の内容に、拡張がまだ処理していない画面の変更を重ねる）', () => {
    const doc = revision(entry('one', ''), entry('two', '元のメモ'), entry('old', '残ったメモ', true));
    const start: EditorView = { doc, pending: new Map() };
    const note = (label: string, before: string, after: string): RevisionChange => ({ field: 'note', label, before, after });
    const noteOf = (view: EditorView, label: string) => shown(view).entries.find((e) => e.label === label)?.note;

    it('打っている行は、届いた途中の版で巻き戻さず、ほかの行の変更は出す', () => {
        let view = withChange(start, 1, note('one', '', 'a'));
        view = withChange(view, 2, note('one', 'a', 'ab'));
        // 1 つ目を当てた版が届いた（ほかのエディタで two も直っていた）
        view = withDocument(view, revision(entry('one', 'a'), entry('two', 'editor B'), entry('old', '残ったメモ', true)), 1);
        expect(noteOf(view, 'one')).toBe('ab');
        expect(noteOf(view, 'two')).toBe('editor B');
    });

    it('処理済みの番号で手放す。A → B → A と打っても、最初の A が返った時点では手放さない', () => {
        let view = withChange(start, 1, note('one', '', 'A'));
        view = withChange(view, 2, note('one', 'A', 'B'));
        view = withChange(view, 3, note('one', 'B', 'A'));
        view = withDocument(view, revision(entry('one', 'A'), entry('two', '元のメモ')), 1);
        view = withDocument(view, revision(entry('one', 'B'), entry('two', '元のメモ')), 2);
        expect(noteOf(view, 'one')).toBe('A');
        view = withDocument(view, revision(entry('one', 'A'), entry('two', '元のメモ')), 3);
        expect(noteOf(view, 'one')).toBe('A');
        // 処理済みの後にほかで直されたら、文書の値を出す
        view = withDocument(view, revision(entry('one', 'Z'), entry('two', '元のメモ')));
        expect(noteOf(view, 'one')).toBe('Z');
        expect(view.pending.size).toBe(0);
    });

    it('処理済みでも、文書が同じ値（書いて読み戻した形）のあいだは打った値を出す（末尾の改行を消さない）', () => {
        let view = withChange(start, 1, note('one', '', 'a\n'));
        view = withDocument(view, revision(entry('one', 'a'), entry('two', '元のメモ')), 1);
        expect(noteOf(view, 'one')).toBe('a\n');
    });

    it('reset なら画面の変更を捨てて文書の内容を出す', () => {
        let view = withChange(start, 1, note('one', '', 'a'));
        view = withChange(view, 2, { field: 'date', before: '2026-09-22', after: '2026-10-01' });
        view = withDocument(view, revision(entry('one', 'X'), entry('two', '元のメモ')), 1, true);
        expect(view.pending.size).toBe(0);
        expect(noteOf(view, 'one')).toBe('X');
        expect(shown(view).date).toBe('2026-09-22');
    });

    it('改訂日と stale の削除も重ねる。削除は処理されたら手放す', () => {
        let view = withChange(start, 1, { field: 'date', before: '2026-09-22', after: '2026-10-01' });
        view = withChange(view, 2, { field: 'drop', label: 'old', before: '残ったメモ' });
        expect(shown(view).date).toBe('2026-10-01');
        expect(shown(view).entries.map((e) => e.label)).toEqual(['one', 'two']);
        view = withDocument(view, { ...revision(entry('one', ''), entry('two', '元のメモ')), date: '2026-10-01' }, 2);
        expect([...view.pending.keys()]).toEqual(['date']);
        expect(shown(view).entries.map((e) => e.label)).toEqual(['one', 'two']);
    });
});
