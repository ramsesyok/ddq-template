/**
 * 執筆フォルダの機構の版と ddq の版の比較（docs/ddq-template-study.md §3.2）。
 *
 * ddq は版・機構が合わないと終了コード 3 で終わる。拡張はそのときだけ版を比べ、
 * update を出すか（ddq の方が新しい・同じ版で中身だけ違う）、拡張の更新を促すか
 * （文書の方が新しい。update すると機構を古い版に戻してしまう）を決める。
 */

/** ddq の終了コード: 執筆フォルダの機構が ddq と一致しない（`ddq update` で直る） */
export const TEMPLATE_MISMATCH_EXIT_CODE = 3;

/** `2.4.5` のような版を比べる。数字以外の部分（`-rc1` など）は無視する。 */
export function compareVersions(a: string, b: string): number {
    const parts = (v: string) => v.trim().split(/[.-]/).map((p) => Number.parseInt(p, 10) || 0);
    const x = parts(a);
    const y = parts(b);
    for (let i = 0; i < Math.max(x.length, y.length); i += 1) {
        const d = (x[i] ?? 0) - (y[i] ?? 0);
        if (d !== 0) return d < 0 ? -1 : 1;
    }
    return 0;
}

/**
 * 不一致の種類。
 * - `folder-older` … 文書の方が古い（版ファイルが無いときも含む）→ update を出す
 * - `folder-newer` … 文書の方が新しい → update は出さず、拡張（ddq）の更新を促す
 * - `modified` … 版は同じで機構ファイルの中身だけ違う → update を出す
 */
export type Mismatch = 'folder-older' | 'folder-newer' | 'modified';

export function classifyMismatch(folderVersion: string | undefined, ddqVersion: string): Mismatch {
    if (folderVersion === undefined || folderVersion.trim() === '') return 'folder-older';
    const c = compareVersions(folderVersion, ddqVersion);
    if (c < 0) return 'folder-older';
    if (c > 0) return 'folder-newer';
    return 'modified';
}
