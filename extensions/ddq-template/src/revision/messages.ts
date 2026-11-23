/** 改訂履歴エディタ（Custom Editor）の 拡張ホスト ⇄ Webview。 */

import { storedDate, storedNote, type Revision } from './revfile';

/** 表示に要る、ファイル本体の外側の情報。 */
export type EditorContext = {
    /** 執筆フォルダ（表示用） */
    folder: string;
    /** このファイルの名前（rev-B.yml） */
    fileName: string;
    /**
     * タグの状態を確かめ終えたか。開いてから最初の `ddq rev next` が返るまでは偽で、そのあいだは
     * 確定済みかどうかが分からないので編集させない（M-01）
     */
    checked: boolean;
    /** `rev-<記号>` のタグがある = 確定済み（編集させない） */
    fixed: boolean;
    /** 既にあるタグ（基準の選び直しに使う） */
    tags: string[];
};

/** 拡張 → Webview */
export type ToEditorMessage =
    | {
          type: 'revision';
          /** 文書のいまの内容 */
          revision: Revision;
          context: EditorContext;
          /** この番号までの edit を処理し終えた（当てた・当てなかった）。H-02 */
          applied?: number;
          /** 画面の未反映の変更を捨て、文書の内容をそのまま出す（当てなかった・編集できない。H-02） */
          reset?: boolean;
      }
    | { type: 'busy'; busy: boolean }
    | { type: 'notice'; text: string; level: 'info' | 'warn' | 'error' };

/**
 * 画面で直した 1 か所（H-02）。`before` は画面が直す前に出していた値で、拡張は文書のいまの値と照らしてから当てる。
 * 改訂全体を送らないのは、ほかの行の変更（別のエディタ・Undo・ddq の書き換え）を、古い画面の値で戻さないため。
 */
export type RevisionChange =
    /** 行のメモ */
    | { field: 'note'; label: string; before: string; after: string }
    /** 改訂日 */
    | { field: 'date'; before: string; after: string }
    /** stale の行を消す（`before` はその行のメモ） */
    | { field: 'drop'; label: string; before: string };

/** Webview → 拡張 */
export type FromEditorMessage =
    | { type: 'ready' }
    /** 1 か所を直した（拡張が文書のいまの内容に当てる）。`id` は画面が振る通し番号 */
    | { type: 'edit'; id: number; change: RevisionChange }
    /** その行の差分を VSCode の差分エディタで開く */
    | { type: 'diff'; label: string }
    /** その行の場所を本文で開く */
    | { type: 'reveal'; file: string; line?: number }
    /** 基準の版と作業ツリーを取り直す（ddq rev diff --write） */
    | { type: 'refresh'; base?: string }
    /** 改訂履歴の表を作る（ddq rev build） */
    | { type: 'build' }
    /** 確定（表を作り、次に打つ Git コマンドを案内する） */
    | { type: 'fix' };

/** 種別の表示名。 */
export function kindText(kind: string): string {
    switch (kind) {
        case 'changed':
            return '変更';
        case 'added':
            return '追加';
        case 'removed':
            return '削除';
        case 'renamed':
            return '改名';
        default:
            return kind;
    }
}

/** 単位の表示名（改訂履歴の「箇所」に添える）。 */
export function unitText(unit: string): string {
    switch (unit) {
        case 'heading':
            return '見出し';
        case 'fig':
            return '図';
        case 'ipo':
            return 'IPO';
        case 'pipe':
        case 'tbl':
            return '表';
        default:
            return unit;
    }
}

/** メモが空の件数（stale は数えない）。 */
export function emptyNotes(revision: Revision): number {
    return revision.entries.filter((e) => !e.stale && e.note.trim() === '').length;
}

/** 当てた結果。`changed` が偽なら文書は既にその値（書かなくてよい）。当てられなければ理由 */
export type ChangeResult = { ok: true; changed: boolean; revision: Revision } | { ok: false; reason: string };

const AGAIN = 'いまの内容を出し直したので、確かめてからもう一度直してください。';

/**
 * 直した 1 か所を改訂に当てる（拡張が、文書の**いまの**内容に対して呼ぶ。H-02）。
 *
 * 画面が直す前に出していた値（`before`）が文書のいまの値と違えば、ほかの所（別のエディタ・Undo・ddq の書き換え）で
 * 変わったので当てない（同じ値を両方で直したときだけは、そのまま通す）。値は書いて読み戻した形で比べる
 * （メモの末尾の空行などは文書に残らないため）。消すのは stale の行だけで、差分に戻った行は消さない。
 */
export function applyChange(revision: Revision, change: RevisionChange): ChangeResult {
    const unchanged: ChangeResult = { ok: true, changed: false, revision };
    if (change.field === 'date') {
        if (revision.date === storedDate(change.after)) return unchanged;
        if (revision.date !== storedDate(change.before)) {
            return { ok: false, reason: `改訂日が、画面に出した後でほかの所から変わっています。${AGAIN}` };
        }
        return { ok: true, changed: true, revision: { ...revision, date: change.after } };
    }
    const entry = revision.entries.find((e) => e.label === change.label);
    if (change.field === 'drop') {
        if (!entry) return unchanged;
        if (!entry.stale) {
            return { ok: false, reason: `${change.label} は差分に戻ったので消しません。${AGAIN}` };
        }
        if (entry.note !== storedNote(change.before)) {
            return { ok: false, reason: `${change.label} のメモが、画面に出した後でほかの所から変わっています。${AGAIN}` };
        }
        return { ok: true, changed: true, revision: { ...revision, entries: revision.entries.filter((e) => e !== entry) } };
    }
    if (!entry) {
        return { ok: false, reason: `${change.label} の行が文書にありません（差分の取り直しなどで消えた）。${AGAIN}` };
    }
    if (entry.note === storedNote(change.after)) return unchanged;
    if (entry.note !== storedNote(change.before)) {
        return { ok: false, reason: `${change.label} のメモが、画面に出した後でほかの所から変わっています。${AGAIN}` };
    }
    return {
        ok: true,
        changed: true,
        revision: { ...revision, entries: revision.entries.map((e) => (e === entry ? { ...e, note: change.after } : e)) }
    };
}

/** 画面で直した変更。`applied` は拡張が処理し終えた（文書に当たった）印 */
export type PendingChange = { id: number; change: RevisionChange; applied: boolean };

/** 画面の状態: 文書の内容と、その上に重ねる画面の変更（項目ごとに最後のもの。H-02） */
export type EditorView = { doc: Revision; pending: ReadonlyMap<string, PendingChange> };

const keyOf = (change: RevisionChange) => (change.field === 'date' ? 'date' : `${change.field}:${change.label}`);

/** 画面で 1 か所を直した（拡張へ送る edit と同じ番号・中身を覚える） */
export function withChange(view: EditorView, id: number, change: RevisionChange): EditorView {
    const pending = new Map(view.pending);
    pending.set(keyOf(change), { id, change, applied: false });
    return { ...view, pending };
}

/**
 * 拡張から文書の内容が届いた。`reset` なら画面の変更を捨てる。`applied` までの変更は処理済みとし、処理済みの
 * 変更は、文書の値がその値（を書いて読み戻した形）と同じあいだだけ重ねる（打った末尾の改行を消さないため）。
 * ほかで直されて違う値になったら捨てて、文書の値を出す。処理されていない変更は残す（打っている最中の値を、
 * 途中の版で巻き戻さない）。値ではなく番号で処理済みを決めるのは、A → B → A と打ったときに、最初の A が
 * 返った時点で 3 つ目を済んだとみなさないため。
 */
export function withDocument(view: EditorView, doc: Revision, applied?: number, reset?: boolean): EditorView {
    if (reset) return { doc, pending: new Map() };
    const pending = new Map<string, PendingChange>();
    for (const [key, p] of view.pending) {
        const done = p.applied || (applied !== undefined && p.id <= applied);
        if (done && !agrees(doc, p.change)) continue;
        pending.set(key, done ? { ...p, applied: true } : p);
    }
    return { doc, pending };
}

/** 文書がその変更の値になっているか（消すのは、済んだら覚えておく理由が無い） */
function agrees(doc: Revision, change: RevisionChange): boolean {
    switch (change.field) {
        case 'date':
            return doc.date === storedDate(change.after);
        case 'note':
            return doc.entries.find((e) => e.label === change.label)?.note === storedNote(change.after);
        case 'drop':
            return false;
    }
}

/** 画面に出す改訂（文書の内容に画面の変更を重ねたもの） */
export function shown(view: EditorView): Revision {
    if (view.pending.size === 0) return view.doc;
    let { date, entries } = view.doc;
    for (const { change } of view.pending.values()) {
        if (change.field === 'date') date = change.after;
        else if (change.field === 'note') {
            entries = entries.map((e) => (e.label === change.label ? { ...e, note: change.after } : e));
        } else entries = entries.filter((e) => e.label !== change.label);
    }
    return { ...view.doc, date, entries };
}
