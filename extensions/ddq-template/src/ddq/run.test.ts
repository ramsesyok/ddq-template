import { describe, expect, it } from 'vitest';

import { DdqError, ddqCommand, run, runStreaming, unsupportedCommand, versionOf } from './run';

describe('ddqCommand', () => {
    it('設定が空なら PATH の ddq を使う', () => {
        expect(ddqCommand(undefined)).toBe('ddq');
        expect(ddqCommand('   ')).toBe('ddq');
        expect(ddqCommand(' C:\tools\ddq.exe ')).toBe('C:\tools\ddq.exe');
    });
});

describe('unsupportedCommand', () => {
    it('実行ファイルは起動する', () => {
        expect(unsupportedCommand('C:\tools\ddq.exe')).toBeUndefined();
        expect(unsupportedCommand('ddq')).toBeUndefined();
        expect(unsupportedCommand('/usr/local/bin/ddq')).toBeUndefined();
    });

    it('.cmd / .bat は起動しない（引数を cmd.exe が解釈するため）', () => {
        expect(unsupportedCommand('C:\tools\ddq.cmd')).toMatch(/バッチファイル/);
        expect(unsupportedCommand('C:\tools\ddq.BAT')).toMatch(/ddqTemplate\.ddqPath/);
    });
});

describe('run / runStreaming', () => {
    it('バッチの指定は起動せずに理由を返す', async () => {
        await expect(run('C:\tools\ddq.cmd', ['tag', 'list', 'a&b'])).rejects.toBeInstanceOf(DdqError);
        const result = await runStreaming('C:\tools\ddq.bat', ['pdf', 'a&b'], { onOutput: () => {} });
        expect(result.code).toBeNull();
        expect(result.error).toMatch(/バッチファイル/);
    });
});

describe('versionOf', () => {
    it('ddq --version の出力から版だけを取る', () => {
        expect(versionOf('ddq 2.3.0\n')).toBe('2.3.0');
        expect(versionOf('')).toBe('');
    });
});
