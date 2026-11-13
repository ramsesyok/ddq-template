import { describe, expect, it, vi } from 'vitest';

vi.mock('vscode', () => ({}));

import { htmlIndexOf, warningOf } from './build';

describe('htmlIndexOf', () => {
    it('ddq html の完了行から index.html のパスを取る', () => {
        const out =
            'mermaid: 3 図\r\nOK: C:\\work\\docs\\_book\\index.html（直接開くと全文検索は使えません）\r\n';
        expect(htmlIndexOf(out)).toBe('C:\\work\\docs\\_book\\index.html');
    });

    it('完了行が無ければ undefined', () => {
        expect(htmlIndexOf('Error: quarto render に失敗しました')).toBeUndefined();
    });
});

describe('warningOf', () => {
    it('ddq diagrams の警告行の本文を取る', () => {
        const out = '警告: C:\\work\\docs\\diagrams に変換する図がありません。mmd- / puml- で始まる 2 件は…\r\n';
        expect(warningOf(out)).toBe('C:\\work\\docs\\diagrams に変換する図がありません。mmd- / puml- で始まる 2 件は…');
    });

    it('警告が無ければ undefined', () => {
        expect(warningOf('mermaid: a.mmd -> a.svg\r\nOK: 1 図を変換しました\r\n')).toBeUndefined();
    });
});
