import { useEffect, useMemo, useRef, useState } from 'react';

import {
    emptyNotes,
    kindText,
    unitText,
    type EditorContext,
    type ToEditorMessage
} from '../revision/messages';
import { emptyRevision, type RevEntry, type Revision } from '../revision/revfile';
import { editorApi } from './vscodeApi';

/**
 * 改訂履歴の編集画面（docs/revision-study.md §5.8）。
 *
 * 差分そのものは描かない（VSCode 標準の差分エディタに渡す）。ここが担うのは
 * 「どこが変わったか」の一覧と、その 1 行ずつに**修正内容を書く欄**である。
 */
export function RevisionEditor() {
    const [revision, setRevision] = useState<Revision>(emptyRevision);
    const [context, setContext] = useState<EditorContext>({
        folder: '',
        fileName: '',
        fixed: false,
        tags: []
    });
    const [busy, setBusy] = useState(false);
    const [notice, setNotice] = useState<{ text: string; level: string } | undefined>();
    const [base, setBase] = useState('');
    // 自分の編集が文書に返ってきたときに、入力中のカーソルを飛ばさないための番兵
    const editing = useRef(false);

    useEffect(() => {
        const onMessage = (event: MessageEvent<ToEditorMessage>) => {
            const message = event.data;
            switch (message.type) {
                case 'revision':
                    setContext(message.context);
                    if (!editing.current) setRevision(message.revision);
                    editing.current = false;
                    return;
                case 'busy':
                    setBusy(message.busy);
                    return;
                case 'notice':
                    setNotice({ text: message.text, level: message.level });
                    return;
            }
        };
        window.addEventListener('message', onMessage);
        editorApi().postMessage({ type: 'ready' });
        return () => window.removeEventListener('message', onMessage);
    }, []);

    const empty = useMemo(() => emptyNotes(revision), [revision]);
    const live = revision.entries.filter((e) => !e.stale);
    const stale = revision.entries.filter((e) => e.stale);

    const editNote = (label: string, note: string) => {
        editing.current = true;
        const next = {
            ...revision,
            entries: revision.entries.map((e) => (e.label === label ? { ...e, note } : e))
        };
        setRevision(next);
        editorApi().postMessage({ type: 'edit', revision: next });
    };

    const editHeader = (patch: Partial<Revision>) => {
        editing.current = true;
        const next = { ...revision, ...patch };
        setRevision(next);
        editorApi().postMessage({ type: 'edit', revision: next });
    };

    const dropStale = (label: string) => {
        const next = { ...revision, entries: revision.entries.filter((e) => e.label !== label) };
        setRevision(next);
        editorApi().postMessage({ type: 'edit', revision: next });
    };

    return (
        <div className="pane">
            <header>
                <h1>
                    改訂 {revision.rev || '—'}
                    {context.fixed && <span className="badge">確定済み</span>}
                </h1>

                <div className="fields">
                    <label>
                        改訂日
                        <input
                            value={revision.date}
                            disabled={context.fixed}
                            onChange={(e) => editHeader({ date: e.target.value })}
                        />
                    </label>
                    <label>
                        基準
                        <input
                            value={base || revision.base}
                            disabled={context.fixed}
                            list="ddq-tags"
                            onChange={(e) => setBase(e.target.value)}
                            title={revision.baseCommit}
                        />
                        <datalist id="ddq-tags">
                            {context.tags.map((t) => (
                                <option key={t} value={t} />
                            ))}
                        </datalist>
                    </label>
                    <span className="muted commit" title="この SHA で取り直します">
                        {revision.baseCommit ? revision.baseCommit.slice(0, 8) : ''}
                    </span>
                </div>

                <div className="toolbar">
                    <button
                        disabled={busy || context.fixed}
                        onClick={() => editorApi().postMessage({ type: 'refresh', base: base || undefined })}
                    >
                        差分を取り直す
                    </button>
                    <button disabled={busy} onClick={() => editorApi().postMessage({ type: 'build' })}>
                        表を作る
                    </button>
                    <button
                        className="primary"
                        disabled={busy || context.fixed}
                        onClick={() => editorApi().postMessage({ type: 'fix' })}
                    >
                        確定する
                    </button>
                    <span className="grow" />
                    <span className="muted">{context.fileName}</span>
                </div>

                {context.fixed && (
                    <p className="muted">
                        タグ <code>rev-{revision.rev}</code> があるので編集できません。直すには
                        <code>git tag -d rev-{revision.rev}</code> でタグを消してください。
                    </p>
                )}
                {notice && <pre className={`notice ${notice.level}`}>{notice.text}</pre>}
            </header>

            {live.length === 0 ? (
                <p className="muted">
                    変更はありません。本文を直してから「差分を取り直す」を押してください。
                </p>
            ) : (
                <table>
                    <thead>
                        <tr>
                            <th>種別</th>
                            <th>箇所</th>
                            <th className="diff" />
                            <th>修正内容</th>
                        </tr>
                    </thead>
                    <tbody>
                        {live.map((entry) => (
                            <EntryRow
                                key={entry.label}
                                entry={entry}
                                fixed={context.fixed}
                                onNote={(note) => editNote(entry.label, note)}
                            />
                        ))}
                    </tbody>
                </table>
            )}

            {stale.length > 0 && (
                <section className="stale">
                    <h2>差分から消えたもの（メモだけ残っています）</h2>
                    <table>
                        <tbody>
                            {stale.map((entry) => (
                                <tr key={entry.label} className="gone">
                                    <td>{unitText(entry.unit)}</td>
                                    <td>{entry.title || entry.label}</td>
                                    <td className="note">{entry.note}</td>
                                    <td>
                                        <button disabled={context.fixed} onClick={() => dropStale(entry.label)}>
                                            消す
                                        </button>
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </section>
            )}

            <footer className="muted">
                {live.length} 件
                {empty > 0 && <strong className="warn">（修正内容が空: {empty} 件）</strong>}。
                保存すると `{context.fileName}` に書かれます。表は「表を作る」で
                revisions/history.qmd に出ます。
            </footer>
        </div>
    );
}

function EntryRow(props: { entry: RevEntry; fixed: boolean; onNote: (note: string) => void }) {
    const { entry, fixed, onNote } = props;
    const gone = entry.kind === 'removed';
    return (
        <tr>
            <td className={`kind ${entry.kind}`}>{kindText(entry.kind)}</td>
            <td className="place">
                <a
                    onClick={() =>
                        !gone && editorApi().postMessage({ type: 'reveal', file: entry.file })
                    }
                    className={gone ? 'gone' : ''}
                    title={gone ? '新版には無い（削除）' : 'その場所を開く'}
                >
                    {entry.title || entry.label}
                </a>
                <span className="muted"> ({unitText(entry.unit)})</span>
                <div className="label muted">{entry.label}</div>
            </td>
            <td className="diff">
                <button onClick={() => editorApi().postMessage({ type: 'diff', label: entry.label })}>
                    差分
                </button>
            </td>
            <td>
                <textarea
                    value={entry.note}
                    rows={Math.max(2, entry.note.split('\n').length)}
                    disabled={fixed}
                    placeholder="何のために、どこを直したか"
                    onChange={(e) => onNote(e.target.value)}
                />
            </td>
        </tr>
    );
}
