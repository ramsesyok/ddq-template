import { describe, expect, it, vi } from 'vitest';

import { ddqCommand, spawnArgs, versionOf } from './run';

describe('ddqCommand', () => {
    it('設定が空なら PATH の ddq を使う', () => {
        expect(ddqCommand(undefined)).toBe('ddq');
        expect(ddqCommand('   ')).toBe('ddq');
        expect(ddqCommand(' C:\tools\ddq.exe ')).toBe('C:\tools\ddq.exe');
    });
});

describe('spawnArgs', () => {
    it('実行ファイルはそのまま起動する', () => {
        expect(spawnArgs('C:\tools\ddq.exe', ['tag', 'list'])).toEqual([
            'C:\tools\ddq.exe',
            ['tag', 'list']
        ]);
    });

    it('Windows の .cmd / .bat は cmd /c を通す', () => {
        // Node は CVE-2024-27980 の対応以降 .cmd を execFile で直接起動できない
        const platform = vi.spyOn(process, 'platform', 'get');
        platform.mockReturnValue('win32');
        try {
            expect(spawnArgs('C:\tools\ddq.cmd', ['--version'])).toEqual([
                'cmd.exe',
                ['/c', 'C:\tools\ddq.cmd', '--version']
            ]);
            expect(spawnArgs('C:\tools\ddq.BAT', [])).toEqual(['cmd.exe', ['/c', 'C:\tools\ddq.BAT']]);
        } finally {
            platform.mockRestore();
        }
    });

    it('Windows 以外では包まない', () => {
        const platform = vi.spyOn(process, 'platform', 'get');
        platform.mockReturnValue('linux');
        try {
            expect(spawnArgs('/usr/local/bin/ddq', ['--version'])).toEqual([
                '/usr/local/bin/ddq',
                ['--version']
            ]);
        } finally {
            platform.mockRestore();
        }
    });
});

describe('versionOf', () => {
    it('ddq --version の出力から版だけを取る', () => {
        expect(versionOf('ddq 2.3.0\n')).toBe('2.3.0');
        expect(versionOf('')).toBe('');
    });
});
