import { describe, expect, it } from 'vitest';

import { envWithQuarto } from './quarto';

describe('envWithQuarto', () => {
    it('設定が空なら環境変数はそのまま', () => {
        const base = { Path: 'C:\\Windows' };
        expect(envWithQuarto('', base, 'win32')).toBe(base);
        expect(envWithQuarto(undefined, base, 'win32')).toBe(base);
    });

    it('quarto.exe を指していればそのフォルダを PATH の先頭に足す（Windows の Path 表記のまま）', () => {
        const env = envWithQuarto('C:\\Program Files\\Quarto\\bin\\quarto.exe', { Path: 'C:\\Windows' }, 'win32');
        expect(env.Path).toBe('C:\\Program Files\\Quarto\\bin;C:\\Windows');
        expect(env.PATH).toBeUndefined();
    });

    it('フォルダを指していればそのまま足す', () => {
        const env = envWithQuarto('/opt/quarto/bin', { PATH: '/usr/bin' }, 'linux');
        expect(env.PATH).toBe('/opt/quarto/bin:/usr/bin');
    });

    it('PATH が無ければそれだけにする', () => {
        expect(envWithQuarto('/opt/quarto/bin', {}, 'linux').PATH).toBe('/opt/quarto/bin');
    });
});
