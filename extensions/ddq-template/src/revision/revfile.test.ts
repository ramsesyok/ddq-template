import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import { describe, expect, it } from 'vitest';

import { emptyRevision, parse, toYaml, type Revision } from './revfile';

/**
 * ddq が実際に書いたファイル。形が食い違うと `ddq rev diff --write` と拡張の保存で
 * 毎回差分が出るので、読めること・書き戻して**同じ文字列になる**ことを見る。
 * 採り直すときは `ddq rev diff <フォルダ> --write` の出力をそのまま置き換える。
 */
const golden = readFileSync(join(__dirname, 'rev-B.golden.yml'), 'utf8').replace(/\r\n/g, '\n');

describe('ddq が書いたファイル', () => {
    it('読める', () => {
        const rev = parse(golden);
        expect(rev.rev).toBe('B');
        expect(rev.date).toBe('2026-09-22');
        expect(rev.base).toBe('rev-A');
        expect(rev.baseCommit).toHaveLength(40);
        expect(rev.scheme).toBe('alpha');
        expect(rev.entries).toHaveLength(5);

        const first = rev.entries[0];
        expect(first.label).toBe('sec-f348af');
        expect(first.kind).toBe('changed');
        expect(first.unit).toBe('heading');
        expect(first.title).toBe('目的');
        expect(first.file).toBe('chapters/01-overview/01-purpose.qmd');
        expect(first.line).toBe(1);
        expect(first.note).toBe('目的の記述に背景を補足した。');
        expect(first.stale).toBe(false);

        // 削除された見出しは旧版の名称を残す
        const removed = rev.entries.find((e) => e.kind === 'removed');
        expect(removed?.title).toBe('他システムとは疎結合とする');
        expect(rev.entries.find((e) => e.label === 'tbl-cond')?.line).toBe(12);
    });

    it('書き戻すと同じ文字列になる（差分を汚さない）', () => {
        // ddq は LF で書く。手元の Git が CRLF に変換していても同じ結果になるよう、
        // 読み込み時に LF へ揃えてから比べる（CRLF を読めることは別のテストで見る）。
        expect(toYaml(parse(golden))).toBe(golden);
    });

    it('CRLF のファイルでも改行だけの差にならない', () => {
        const crlf = golden.replace(/\n/g, '\r\n');
        expect(toYaml(parse(crlf))).toBe(golden);
    });

    it('複数行のメモも往復する', () => {
        const rev = parse(golden);
        const multi = rev.entries.find((e) => e.note.includes('\n'));
        expect(multi?.note).toBe('標題の言い回しを見直した\n（内容の変更は無い）。');
        expect(parse(toYaml(rev)).entries).toEqual(rev.entries);
    });
});

describe('parse', () => {
    it('人が素直に書いた形も読む（引用なし・1 行メモ・コメント）', () => {
        const text = [
            '# 手で書いた',
            'rev: A',
            'date: 2026-09-22',
            'base: rev--',
            'entries:',
            '  - label: sec-x',
            '    kind: changed',
            '    title: 概要',
            '    note: 文言を直した'
        ].join('\n');
        const rev = parse(text);
        expect(rev.rev).toBe('A');
        expect(rev.base).toBe('rev--');
        expect(rev.entries).toHaveLength(1);
        expect(rev.entries[0].note).toBe('文言を直した');
    });

    it('line の無い古いファイルは line 無しのまま往復する', () => {
        const old = parse(golden.replace(/^ {4}line: \d+\n/gm, ''));
        expect(old.entries.every((e) => e.line === undefined)).toBe(true);
        expect(toYaml(old)).not.toContain('line:');
    });

    it('CRLF でも読める', () => {
        const rev = parse('rev: A\r\ndate: 2026-09-22\r\nentries:\r\n  - label: sec-x\r\n    note: メモ\r\n');
        expect(rev.rev).toBe('A');
        expect(rev.entries[0].note).toBe('メモ');
    });

    it('stale を読む', () => {
        const rev = parse('rev: A\nentries:\n  - label: sec-x\n    stale: true\n    note: 残ったメモ\n');
        expect(rev.entries[0].stale).toBe(true);
    });
});

describe('toYaml', () => {
    const base = (over: Partial<Revision> = {}): Revision => ({
        ...emptyRevision(),
        rev: 'C',
        date: '2026-09-23',
        ...over
    });

    it('空のメモは "" で書く（block scalar にしない）', () => {
        const yaml = toYaml(
            base({
                entries: [
                    {
                        label: 'sec-x',
                        kind: 'changed',
                        unit: 'heading',
                        title: '目的',
                        file: 'a.qmd',
                        note: '',
                        stale: false
                    }
                ]
            })
        );
        expect(yaml).toContain('    note: ""\n');
    });

    it('コロンや # を含む名称は引用する', () => {
        const yaml = toYaml(
            base({
                entries: [
                    {
                        label: 'fig-x',
                        kind: 'removed',
                        unit: 'fig',
                        title: 'ネットワーク: 構成 #2',
                        file: '',
                        note: '統合した',
                        stale: true
                    }
                ]
            })
        );
        expect(yaml).toContain('    title: "ネットワーク: 構成 #2"\n');
        expect(yaml).toContain('    stale: true\n');
        // file が空なら行ごと書かない
        expect(yaml).not.toContain('    file:');
        expect(parse(yaml).entries[0].title).toBe('ネットワーク: 構成 #2');
    });

    it('空の base / scheme は行ごと書かない', () => {
        const yaml = toYaml(base());
        expect(yaml).not.toContain('base:');
        expect(yaml).not.toContain('scheme:');
        expect(yaml).toContain('entries:\n');
    });
});
