/**
 * 執筆フォルダの機構の版と ddq の版の比較（docs/ddq-template-study.md §3.2）。
 *
 * ddq は版・機構が合わないと終了コード 3 で終わる。拡張はそのときだけ版を比べ、
 * update を出すか（ddq の方が新しい・同じ版で中身だけ違う）、拡張の更新を促すか
 * （文書の方が新しい。update すると機構を古い版に戻してしまう）を決める。
 */

/** ddq の終了コード: 執筆フォルダの機構が ddq と一致しない（`ddq update` で直る） */
export const TEMPLATE_MISMATCH_EXIT_CODE = 3;

/**
 * 出荷した版名の形（`3.1.0`・`3.0.0-pre4`。数字の桁は `2.4` のように短くてもよい）を読む。
 * それ以外（`abc`・`-rc1`・番号の無い `-pre`）は undefined（EXT-01）。
 */
export function parseVersion(v: string): { numbers: number[]; pre?: number } | undefined {
    const m = /^(\d+(?:\.\d+)*)(?:-pre(\d+))?$/.exec(v.trim());
    if (!m) return undefined;
    return { numbers: m[1].split('.').map(Number), pre: m[2] === undefined ? undefined : Number(m[2]) };
}

/**
 * 版を比べる（a の方が古ければ -1、新しければ 1、同じなら 0）。数字は桁ごとに数として比べ、
 * 足りない桁は 0（`2.4` = `2.4.0`）。数字が同じなら `-preN` は N の順で、正式版より前
 * （pre1 < pre4 < pre10 < 正式版）。どちらかが版の形でなければ undefined。
 */
export function compareVersions(a: string, b: string): number | undefined {
    const x = parseVersion(a);
    const y = parseVersion(b);
    if (!x || !y) return undefined;
    for (let i = 0; i < Math.max(x.numbers.length, y.numbers.length); i += 1) {
        const d = (x.numbers[i] ?? 0) - (y.numbers[i] ?? 0);
        if (d !== 0) return d < 0 ? -1 : 1;
    }
    // 正式版は同じ数字のどの pre よりも新しい
    const px = x.pre ?? Number.POSITIVE_INFINITY;
    const py = y.pre ?? Number.POSITIVE_INFINITY;
    return px === py ? 0 : px < py ? -1 : 1;
}

/**
 * 不一致の種類。
 * - `folder-older` … 文書の方が古い → update を出す
 * - `folder-newer` … 文書の方が新しい → update は出さず、拡張（ddq）の更新を促す
 * - `modified` … 版は同じで機構ファイルの中身だけ違う → update を出す
 * - `unknown` … 版ファイルが無い・空・版の形でない、または ddq の版が分からない → update は出さず、
 *   版を確かめるよう案内する（古い版と決めつけて update すると、機構を古い版に戻しうる）
 */
export type Mismatch = 'folder-older' | 'folder-newer' | 'modified' | 'unknown';

export function classifyMismatch(folderVersion: string | undefined, ddqVersion: string | undefined): Mismatch {
    if (folderVersion === undefined || ddqVersion === undefined) return 'unknown';
    const c = compareVersions(folderVersion, ddqVersion);
    if (c === undefined) return 'unknown';
    if (c < 0) return 'folder-older';
    if (c > 0) return 'folder-newer';
    return 'modified';
}
