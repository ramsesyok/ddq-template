import { describe, expect, it, vi } from 'vitest';

vi.mock('vscode', () => ({}));

import { htmlIndexOf } from './build';

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
