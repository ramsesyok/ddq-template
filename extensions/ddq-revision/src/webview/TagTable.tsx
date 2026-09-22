import { useEffect, useMemo, useState } from 'react';

import { kindLabel, warningLabel, type Item, type TagList } from '../ddq/types';
import { rowKey, type Row } from '../tagging/labels';
import { counts, sortItems, type ToWebviewMessage } from '../tagging/messages';
import { vscode } from './vscodeApi';

/**
 * ラベル一覧（docs/revision-study.md §4.4）。
 *
 * TreeView ではなく表にしているのは、候補ラベルをその場で直せる必要があるため
 * （TreeView は表示専用で、行の中に入力欄を置けない）。
 */
export function TagTable() {
    const [list, setList] = useState<TagList | undefined>();
    const [version, setVersion] = useState('');
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<{ message: string; detail?: string } | undefined>();
    const [labels, setLabels] = useState<Record<string, string>>({});
    const [checked, setChecked] = useState<Record<string, boolean>>({});
    const [errors, setErrors] = useState<Record<string, string>>({});

    useEffect(() => {
        const onMessage = (event: MessageEvent<ToWebviewMessage>) => {
            const message = event.data;
            switch (message.type) {
                case 'list': {
                    setList(message.list);
                    setVersion(message.version);
                    setError(undefined);
                    setErrors({});
                    // 候補は毎回同じ値が返る（内容由来ハッシュ）ので、取り直しても
                    // 人が直した入力を残したいものだけ残す。
                    setLabels((prev) => {
                        const next: Record<string, string> = {};
                        for (const item of message.list.items) {
                            if (!item.suggested) continue;
                            next[rowKey(item)] = prev[rowKey(item)] ?? item.suggested;
                        }
                        return next;
                    });
                    setChecked(() => {
                        const next: Record<string, boolean> = {};
                        for (const item of message.list.items) {
                            if (item.suggested) next[rowKey(item)] = true;
                        }
                        return next;
                    });
                    return;
                }
                case 'busy':
                    setBusy(message.busy);
                    return;
                case 'errors':
                    setErrors(Object.fromEntries(message.errors));
                    return;
                case 'error':
                    setError({ message: message.message, detail: message.detail });
                    return;
            }
        };
        window.addEventListener('message', onMessage);
        vscode().postMessage({ type: 'ready' });
        return () => window.removeEventListener('message', onMessage);
    }, []);

    const items = useMemo(() => (list ? sortItems(list.items, list.order) : []), [list]);
    const stats = useMemo(() => counts(items), [items]);
    const targets = items.filter((i) => i.suggested && checked[rowKey(i)]).length;

    const apply = () => {
        if (!list) return;
        const rows: Row[] = items
            .filter((item) => item.suggested)
            .map((item) => ({
                item,
                label: labels[rowKey(item)] ?? item.suggested ?? '',
                checked: checked[rowKey(item)] ?? false
            }));
        vscode().postMessage({ type: 'apply', rows });
    };

    const toggleAll = (value: boolean) => {
        const next: Record<string, boolean> = {};
        for (const item of items) if (item.suggested) next[rowKey(item)] = value;
        setChecked(next);
    };

    if (error) {
        return (
            <div className="pane">
                <h1>ラベル一覧</h1>
                <p className="error">{error.message}</p>
                {error.detail && <pre className="detail">{error.detail}</pre>}
                <button onClick={() => vscode().postMessage({ type: 'reload' })}>もう一度</button>
            </div>
        );
    }

    return (
        <div className="pane">
            <header>
                <h1>ラベル一覧</h1>
                <div className="toolbar">
                    <button onClick={() => vscode().postMessage({ type: 'reload' })} disabled={busy}>
                        取り直す
                    </button>
                    <button className="primary" onClick={apply} disabled={busy || targets === 0}>
                        チェックした {targets} 件を書き戻す
                    </button>
                    <span className="grow" />
                    <span className="muted">
                        {list ? `${list.folder}（ddq ${version}）` : busy ? '読み込み中…' : ''}
                    </span>
                </div>
            </header>

            {list && list.warnings.length > 0 && (
                <ul className="warnings">
                    {list.warnings.map((w) => (
                        <li key={w}>{w}</li>
                    ))}
                </ul>
            )}

            <table>
                <thead>
                    <tr>
                        <th className="check">
                            <input
                                type="checkbox"
                                title="すべて選ぶ / 外す"
                                checked={targets > 0 && targets === stats.suggested}
                                onChange={(e) => toggleAll(e.target.checked)}
                            />
                        </th>
                        <th>ファイル</th>
                        <th>行</th>
                        <th>種別</th>
                        <th>名称</th>
                        <th>ラベル</th>
                    </tr>
                </thead>
                <tbody>
                    {items.map((item) => (
                        <TagRow
                            key={rowKey(item)}
                            item={item}
                            label={labels[rowKey(item)] ?? ''}
                            checked={checked[rowKey(item)] ?? false}
                            error={errors[rowKey(item)]}
                            onLabel={(v) => setLabels((p) => ({ ...p, [rowKey(item)]: v }))}
                            onCheck={(v) => setChecked((p) => ({ ...p, [rowKey(item)]: v }))}
                        />
                    ))}
                </tbody>
            </table>

            <footer className="muted">
                {stats.total} 件（ラベル済み {stats.labelled} / 候補あり {stats.suggested} / 対象外{' '}
                {stats.blocked}）。書き戻したあとは Ctrl+Z で戻せます。
            </footer>
        </div>
    );
}

function TagRow(props: {
    item: Item;
    label: string;
    checked: boolean;
    error?: string;
    onLabel: (value: string) => void;
    onCheck: (value: boolean) => void;
}) {
    const { item, label, checked, error, onLabel, onCheck } = props;
    const reveal = () =>
        vscode().postMessage({ type: 'reveal', file: item.file, line: item.line });

    return (
        <tr className={error ? 'bad' : item.label ? 'done' : ''}>
            <td className="check">
                {item.suggested && (
                    <input type="checkbox" checked={checked} onChange={(e) => onCheck(e.target.checked)} />
                )}
            </td>
            <td className="file">
                <a onClick={reveal} title="その行を開く">
                    {item.file}
                </a>
            </td>
            <td className="num">{item.line}</td>
            <td className="kind">{kindLabel(item)}</td>
            <td className="title">{item.title ?? ''}</td>
            <td className="label">
                {item.label ? (
                    <span className="have">{item.label}</span>
                ) : item.suggested ? (
                    <>
                        <input
                            value={label}
                            spellCheck={false}
                            onChange={(e) => onLabel(e.target.value)}
                            aria-label="候補ラベル"
                        />
                        {error && <span className="why">{error}</span>}
                    </>
                ) : (
                    <span className="muted">{item.warning ? warningLabel(item.warning) : '—'}</span>
                )}
            </td>
        </tr>
    );
}
