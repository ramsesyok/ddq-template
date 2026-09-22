/** 改訂履歴エディタ（Custom Editor）の 拡張ホスト ⇄ Webview。 */

import type { Revision } from './revfile';

/** 表示に要る、ファイル本体の外側の情報。 */
export type EditorContext = {
    /** 執筆フォルダ（表示用） */
    folder: string;
    /** このファイルの名前（rev-B.yml） */
    fileName: string;
    /** `rev-<記号>` のタグがある = 確定済み（編集させない） */
    fixed: boolean;
    /** 既にあるタグ（基準の選び直しに使う） */
    tags: string[];
};

/** 拡張 → Webview */
export type ToEditorMessage =
    | { type: 'revision'; revision: Revision; context: EditorContext }
    | { type: 'busy'; busy: boolean }
    | { type: 'notice'; text: string; level: 'info' | 'warn' | 'error' };

/** Webview → 拡張 */
export type FromEditorMessage =
    | { type: 'ready' }
    /** メモなどを直した（拡張が YAML に直して文書へ書く） */
    | { type: 'edit'; revision: Revision }
    /** その行の差分を VSCode の差分エディタで開く */
    | { type: 'diff'; label: string }
    /** その行の場所を本文で開く */
    | { type: 'reveal'; file: string; line?: number }
    /** 基準の版と作業ツリーを取り直す（ddq rev diff --write） */
    | { type: 'refresh'; base?: string }
    /** 改訂履歴の表を作る（ddq rev build） */
    | { type: 'build' }
    /** 確定（表を作り、次に打つ Git コマンドを案内する） */
    | { type: 'fix' };

/** 種別の表示名。 */
export function kindText(kind: string): string {
    switch (kind) {
        case 'changed':
            return '変更';
        case 'added':
            return '追加';
        case 'removed':
            return '削除';
        case 'renamed':
            return '改名';
        default:
            return kind;
    }
}

/** 単位の表示名（改訂履歴の「箇所」に添える）。 */
export function unitText(unit: string): string {
    switch (unit) {
        case 'heading':
            return '見出し';
        case 'fig':
            return '図';
        case 'ipo':
            return 'IPO';
        case 'pipe':
        case 'tbl':
            return '表';
        default:
            return unit;
    }
}

/** メモが空の件数（stale は数えない）。 */
export function emptyNotes(revision: Revision): number {
    return revision.entries.filter((e) => !e.stale && e.note.trim() === '').length;
}
