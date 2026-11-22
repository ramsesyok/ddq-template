/**
 * ddq との契約試験（QA-01）。期待値は ddq 側の試験（cli/tests/contract.rs・e2e.rs）と**同じファイル**
 * （cli/tests/fixtures/contract/）を読む。ddq がその形を出すことは Rust 側が、拡張がその形を
 * 読めて同じ結果を書くことはここが確かめる。モックの ddq は使わない（実 exe の出力から採った期待値）。
 */

import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import { describe, expect, it, vi } from 'vitest';

// build.ts の結果行の読み取りだけを使う（VSCode API には触れない）
vi.mock('vscode', () => ({}));

import { htmlIndexOf, isResultLine, warningOf } from '../build';
import { NO_ORIGINAL, characterOffset, checkRows, lacksOriginal, sortForApply, staleReason } from '../tagging/labels';
import { ContractError, parseRevNext, parseTagList } from './contract';
import type { TagList } from './types';
import { TEMPLATE_MISMATCH_EXIT_CODE } from './version';

const dir = join(__dirname, '..', '..', '..', '..', 'cli', 'tests', 'fixtures', 'contract');
const load = (name: string): unknown => JSON.parse(readFileSync(join(dir, name), 'utf8'));

const manuscript = load('manuscript.json') as { files: Record<string, string> };
const applied = load('tag-applied.json') as { files: Record<string, string> };
const cli = load('cli.json') as {
    exit: Record<string, number>;
    stdout: Record<string, string>;
};

const stripBom = (s: string) => s.replace(/^\uFEFF/, '');
const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;

/**
 * 拡張の書き戻し（activateRevision.ts の applyRows）と同じ手順を、文書の文字列に対して行う。
 * VSCode の TextDocument は BOM を文字コード側で持つので、本文は BOM を除いた文字列で比べる。
 */
function applyLikeTheExtension(list: TagList, files: Record<string, string>): Record<string, string> {
    const rows = list.items
        .filter((item) => item.suggested && item.edit)
        .map((item) => ({ item, label: item.suggested as string, checked: true }));
    const { edits, errors } = checkRows(rows, list.items);
    expect([...errors]).toEqual([]);
    const out: Record<string, string> = {};
    for (const [rel, body] of Object.entries(files)) out[rel] = stripBom(body);
    for (const e of sortForApply(edits)) {
        const text = out[e.file];
        const eol = text.includes('\r\n') ? '\r\n' : '\n';
        const lines = text.split(eol);
        const line = lines[e.line - 1];
        expect(staleReason(line, e), `${e.file}:${e.line}`).toBeUndefined();
        const at = characterOffset(line, e.col);
        lines[e.line - 1] = line.slice(0, at) + e.insert + line.slice(at);
        out[e.file] = lines.join(eol);
    }
    return out;
}

describe('tag list --json（現行の ddq）', () => {
    const golden = load('tag-list.json');

    it('形を確かめて、そのまま受け取る', () => {
        expect(parseTagList(golden)).toEqual(golden);
    });

    it('同じ編集指示を拡張が当てると、ddq tag apply --all と同じ本文になる', () => {
        const list = parseTagList(golden);
        const ours = applyLikeTheExtension(list, manuscript.files);
        for (const [rel, want] of Object.entries(applied.files)) {
            expect(ours[rel], rel).toBe(stripBom(want));
        }
    });

    it('col は文字（コードポイント）の数で、サロゲートペアと結合文字を跨いでも位置がずれない', () => {
        const list = parseTagList(golden);
        const purpose = list.items.find((i) => i.file === 'chapters/01/01-purpose.qmd');
        const line = stripBom(manuscript.files['chapters/01/01-purpose.qmd']).split('\n')[0];
        expect(purpose?.edit?.original).toBe(line);
        // 𠮷 は UTF-16 で 2 単位、か＋濁点は 2 コードポイント。col は行末（10 文字）
        expect(purpose?.edit?.col).toBe([...line].length);
        expect(characterOffset(line, purpose!.edit!.col)).toBe(line.length);
    });
});

describe('tag list --json（旧版 2.4.4。edit.original が無い）', () => {
    const old = load('tag-list.2.4.4.json');

    it('一覧としては受け取る（省略形を許す）', () => {
        expect(() => parseTagList(old)).not.toThrow();
        expect(lacksOriginal(parseTagList(old).items)).toBe(true);
    });

    it('照合できないので、どの行にも書き戻さない', () => {
        const list = parseTagList(old);
        const files = manuscript.files;
        for (const item of list.items) {
            if (!item.edit) continue;
            const line = stripBom(files[item.edit.file]).split(/\r?\n/)[item.edit.line - 1];
            expect(staleReason(line, item.edit)).toBe(NO_ORIGINAL);
        }
    });
});

describe('形の違いは受け取った時点で止める', () => {
    const golden = load('tag-list.json') as Record<string, unknown> & { items: Record<string, unknown>[] };

    it('知らないフィールドは無視する（新しい ddq が足したもの）', () => {
        const v = clone(golden);
        (v as Record<string, unknown>).schema = 2;
        v.items[0].confidence = 0.9;
        (v.items[0].edit as Record<string, unknown>).hint = 'x';
        expect(() => parseTagList(v)).not.toThrow();
    });

    it.each([
        ['items が無い', (v: typeof golden) => delete (v as Record<string, unknown>).items, 'items'],
        ['order が文字列', (v: typeof golden) => ((v as Record<string, unknown>).order = 'index.qmd'), 'order'],
        ['line が文字列', (v: typeof golden) => (v.items[0].line = '1'), 'items[0].line'],
        ['line が 0', (v: typeof golden) => (v.items[0].line = 0), 'items[0].line'],
        ['col が負', (v: typeof golden) => ((v.items[0].edit as Record<string, unknown>).col = -1), 'items[0].edit.col'],
        ['col が小数', (v: typeof golden) => ((v.items[0].edit as Record<string, unknown>).col = 1.5), 'items[0].edit.col'],
        ['insert が無い', (v: typeof golden) => delete (v.items[0].edit as Record<string, unknown>).insert, 'items[0].edit.insert'],
        ['知らない kind', (v: typeof golden) => (v.items[0].kind = 'lst'), 'items[0].kind'],
        ['知らない warning', (v: typeof golden) => (v.items[4].warning = 'other'), 'items[4].warning'],
        ['title が数', (v: typeof golden) => (v.items[0].title = 1), 'items[0].title']
    ])('%s', (_name, mutate, where) => {
        const v = clone(golden);
        mutate(v);
        let error: unknown;
        try {
            parseTagList(v);
        } catch (e) {
            error = e;
        }
        expect(error).toBeInstanceOf(ContractError);
        expect((error as ContractError).problems.join('\n')).toContain(where);
    });

    it('配列でもオブジェクトでもないものは全体を指して止める', () => {
        expect(() => parseTagList('ok')).toThrow(ContractError);
        expect(() => parseTagList(null)).toThrow(/\(全体\)/);
    });
});

describe('rev next --json', () => {
    const golden = load('rev-next.json');

    it('形を確かめて、そのまま受け取る', () => {
        expect(parseRevNext(golden)).toEqual(golden);
    });

    it('拡張が使う rev / base / tags / drafts が欠けたら止める', () => {
        for (const key of ['rev', 'base', 'tags', 'drafts']) {
            const v = clone(golden) as Record<string, unknown>;
            delete v[key];
            expect(() => parseRevNext(v), key).toThrow(ContractError);
        }
        // base_commit / scheme は使わないので、無くても受け取る
        const v = clone(golden) as Record<string, unknown>;
        delete v.base_commit;
        delete v.scheme;
        expect(() => parseRevNext(v)).not.toThrow();
    });
});

describe('結果行と終了コード', () => {
    const expand = (s: string, folder: string, sep: string) =>
        s.split('{{FOLDER}}').join(folder).split('{{SEP}}').join(sep);

    it.each([
        ['Windows', 'C:\\work\\受注 設計\\docs', '\\'],
        ['Unix', '/home/a/受注 設計/docs', '/']
    ])('ddq html の結果行から index.html を拾う（%s）', (_os, folder, sep) => {
        const line = expand(cli.stdout.html, folder, sep);
        expect(htmlIndexOf(`[quarto の進捗]\n${line}\n`)).toBe(`${folder}${sep}_book${sep}index.html`);
    });

    it('ddq diagrams の「図が無い」警告を拾う', () => {
        const line = expand(cli.stdout.diagramsEmpty, 'C:\\w\\docs', '\\');
        expect(warningOf(`${line}\n`)).toBe('C:\\w\\docs\\diagrams に .mmd / .puml がありません');
    });

    it('結果行（OK -> / OK: / 警告:）は手元に残す行として選ばれる', () => {
        for (const key of ['pdf', 'html', 'diagramsEmpty']) {
            expect(isResultLine(expand(cli.stdout[key], 'C:\\w\\docs', '\\')), key).toBe(true);
        }
        expect(isResultLine('[quarto] 進捗')).toBe(false);
    });

    it('機構の不一致の終了コードは ddq と同じ', () => {
        expect(TEMPLATE_MISMATCH_EXIT_CODE).toBe(cli.exit.templateMismatch);
    });
});
