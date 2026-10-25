/**
 * 執筆フォルダごとの実行中フラグ。
 *
 * 同じ執筆フォルダで ddq を 2 本走らせると、`_book/` や図のキャッシュを同時に書き換える。
 * そこで同じフォルダの 2 本目は受け付けない（別のフォルダなら並行してよい）。
 */

const busy = new Set<string>();

/** Windows のパスは大文字小文字を区別しないので、比べる前に揃える。 */
export function lockKey(folder: string, platform = process.platform): string {
    const trimmed = folder.replace(/[\\/]+$/, '');
    return platform === 'win32' ? trimmed.toLowerCase().replace(/\//g, '\\') : trimmed;
}

/** 取れたら解放する関数を返す。実行中なら undefined。 */
export function tryLock(folder: string): (() => void) | undefined {
    const key = lockKey(folder);
    if (busy.has(key)) return undefined;
    busy.add(key);
    let released = false;
    return () => {
        if (released) return;
        released = true;
        busy.delete(key);
    };
}

export function isLocked(folder: string): boolean {
    return busy.has(lockKey(folder));
}
