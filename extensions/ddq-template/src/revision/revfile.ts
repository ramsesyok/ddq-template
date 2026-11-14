/**
 * 改訂 1 回分のファイル `revisions/rev-<記号>.yml` の読み書き（TS 側）。
 *
 * **正本の形を決めているのは ddq（`cli/src/doc/revfile.rs`）**で、ここはそれと同じ形を
 * 保つための写しである。汎用の YAML ライブラリを使わないのは、引用の付け方や
 * キーの順が変わると `ddq rev diff --write` と拡張の保存で毎回差分が出てしまうため。
 * 形が食い違っていないことは `revfile.test.ts` が golden（ddq が実際に書いたもの）で見る。
 */

/** 改訂 1 回分。 */
export type Revision = {
    /** 改訂記号（`A` / `1` など。ファイル名と一致させる） */
    rev: string;
    date: string;
    /** 人が選んだ比較基準の ref */
    base: string;
    /** 解決したコミット ID。再取得はこちらを使う */
    baseCommit: string;
    scheme: string;
    entries: RevEntry[];
};

/** 改訂履歴の 1 行。 */
export type RevEntry = {
    label: string;
    kind: 'changed' | 'added' | 'removed' | 'renamed' | string;
    unit: string;
    title: string;
    file: string;
    /** `file` での行（1 始まり）。removed は旧版での行。取り直した時点の値で、場所を開くのに使う */
    line?: number;
    /** 修正内容（人が書く） */
    note: string;
    /** 再取得で差分から消えたが、メモが残っているもの */
    stale: boolean;
};

export function emptyRevision(): Revision {
    return { rev: '', date: '', base: '', baseCommit: '', scheme: '', entries: [] };
}

/** 決まった形の YAML を読む。 */
export function parse(text: string): Revision {
    const rev = emptyRevision();
    const lines = text.split('\n').map((l) => l.replace(/\r$/, ''));
    let i = 0;
    let inEntries = false;

    while (i < lines.length) {
        const line = lines[i];
        const trimmed = line.trim();
        if (trimmed === '' || trimmed.startsWith('#')) {
            i += 1;
            continue;
        }
        const indent = line.length - line.trimStart().length;

        if (indent === 0) {
            inEntries = false;
            const kv = splitKv(trimmed);
            if (!kv) {
                i += 1;
                continue;
            }
            if (kv.key === 'entries') {
                inEntries = true;
                i += 1;
                continue;
            }
            const [value, used] = scalar(kv.value, lines, i, indent);
            switch (kv.key) {
                case 'rev':
                    rev.rev = value;
                    break;
                case 'date':
                    rev.date = value;
                    break;
                case 'base':
                    rev.base = value;
                    break;
                case 'base_commit':
                    rev.baseCommit = value;
                    break;
                case 'scheme':
                    rev.scheme = value;
                    break;
            }
            i = used;
            continue;
        }

        if (inEntries && trimmed.startsWith('- ')) {
            const entry: RevEntry = {
                label: '',
                kind: '',
                unit: '',
                title: '',
                file: '',
                note: '',
                stale: false
            };
            const itemIndent = indent;
            let first = true;
            while (i < lines.length) {
                const l = lines[i];
                const t = l.trim();
                if (t === '' || t.startsWith('#')) {
                    i += 1;
                    continue;
                }
                const ind = l.length - l.trimStart().length;
                let body: string;
                if (first) {
                    if (!t.startsWith('- ')) break;
                    body = t.slice(2);
                } else {
                    if (ind <= itemIndent || t.startsWith('- ')) break;
                    body = t;
                }
                const kv = splitKv(body);
                if (!kv) {
                    i += 1;
                    first = false;
                    continue;
                }
                const keyIndent = first ? itemIndent + 2 : ind;
                const [value, used] = scalar(kv.value, lines, i, keyIndent);
                switch (kv.key) {
                    case 'label':
                        entry.label = value;
                        break;
                    case 'kind':
                        entry.kind = value;
                        break;
                    case 'unit':
                        entry.unit = value;
                        break;
                    case 'title':
                        entry.title = value;
                        break;
                    case 'file':
                        entry.file = value;
                        break;
                    case 'line': {
                        const n = Number.parseInt(value, 10);
                        if (Number.isInteger(n) && n > 0) entry.line = n;
                        break;
                    }
                    case 'note':
                        entry.note = value;
                        break;
                    case 'stale':
                        entry.stale = value === 'true';
                        break;
                }
                i = used;
                first = false;
            }
            if (entry.label !== '') rev.entries.push(entry);
            continue;
        }
        i += 1;
    }
    return rev;
}

/** 書き出す（ddq と同じ形。差分を汚さないためキーの順も固定）。 */
export function toYaml(rev: Revision): string {
    let s = '';
    s += '# ddq rev diff が作り、人が note（修正内容）を書き足すファイル。\n';
    s += '# 改訂履歴の表は `ddq rev build` が revisions/history.qmd に生成する。\n';
    s += `rev: ${quote(rev.rev)}\n`;
    s += `date: ${quote(rev.date)}\n`;
    if (rev.base !== '') s += `base: ${quote(rev.base)}\n`;
    if (rev.baseCommit !== '') s += `base_commit: ${rev.baseCommit}\n`;
    if (rev.scheme !== '') s += `scheme: ${rev.scheme}\n`;
    s += 'entries:\n';
    for (const e of rev.entries) {
        s += `  - label: ${quote(e.label)}\n`;
        s += `    kind: ${e.kind}\n`;
        s += `    unit: ${e.unit}\n`;
        s += `    title: ${quote(e.title)}\n`;
        if (e.file !== '') s += `    file: ${quote(e.file)}\n`;
        if (e.line !== undefined) s += `    line: ${e.line}\n`;
        if (e.stale) s += '    stale: true\n';
        if (e.note === '') {
            s += '    note: ""\n';
        } else {
            s += '    note: |\n';
            for (const l of e.note.split('\n')) s += `      ${l}\n`;
        }
    }
    return s;
}

function splitKv(s: string): { key: string; value: string } | undefined {
    const i = s.indexOf(':');
    if (i < 0) return undefined;
    const key = s.slice(0, i).trim();
    if (key === '' || key.includes(' ')) return undefined;
    return { key, value: s.slice(i + 1).trim() };
}

/** 値を読む。`|` なら続く深い字下げの行をまとめる。戻り値は [値, 次に読む行]。 */
function scalar(value: string, lines: string[], at: number, keyIndent: number): [string, number] {
    if (value === '|' || value === '|-' || value === '>') {
        const body: string[] = [];
        let j = at + 1;
        let strip = Number.MAX_SAFE_INTEGER;
        while (j < lines.length) {
            const l = lines[j];
            if (l.trim() === '') {
                body.push('');
                j += 1;
                continue;
            }
            const ind = l.length - l.trimStart().length;
            if (ind <= keyIndent) break;
            strip = Math.min(strip, ind);
            body.push(l);
            j += 1;
        }
        while (body.length > 0 && body[body.length - 1] === '') body.pop();
        return [body.map((l) => (l.length >= strip ? l.slice(strip) : '')).join('\n'), j];
    }
    return [unquote(value), at + 1];
}

function unquote(v: string): string {
    const s = v.trim();
    if (
        s.length >= 2 &&
        ((s.startsWith('"') && s.endsWith('"')) || (s.startsWith("'") && s.endsWith("'")))
    ) {
        return s.slice(1, -1).split('\\"').join('"');
    }
    return s;
}

/** YAML として素のまま書けない値だけ引用する（ddq の quote と同じ判定）。 */
function quote(v: string): string {
    const first = v.charAt(0);
    const needs =
        v === '' ||
        '-?:&*!|>%@`"\'['.includes(first) ||
        v.includes(': ') ||
        v.endsWith(':') ||
        v.includes('#') ||
        v.includes('\n');
    return needs ? `"${v.split('\\').join('\\\\').split('"').join('\\"')}"` : v;
}
