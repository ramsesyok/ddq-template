/**
 * 旧拡張（ddq-revision / ddq-table-editor）の検出（docs/ddq-template-study.md §4）。
 *
 * 旧拡張が残っていると `revisions/rev-*.yml` に既定の Custom Editor が 2 つ登録され、
 * 右クリックメニューも重複する。VSCode には「同時に入れない」宣言が無いので、
 * 起動時に見つけてアンインストールを案内する。
 */

import * as vscode from 'vscode';

/** ddq-template に統合した旧拡張の ID */
export const LEGACY_EXTENSIONS = ['ramsesyok.ddq-revision', 'ramsesyok.ddq-table-editor'] as const;

/** 入っている旧拡張の ID（`isInstalled` は差し替えてテストする） */
export function installedLegacy(isInstalled: (id: string) => boolean): string[] {
    return LEGACY_EXTENSIONS.filter((id) => isInstalled(id));
}

export function warnLegacyExtensions(): void {
    const found = installedLegacy((id) => vscode.extensions.getExtension(id) !== undefined);
    if (found.length === 0) return;
    const uninstall = '旧拡張をアンインストール';
    void vscode.window
        .showWarningMessage(
            `${found.join(' / ')} は DDQ Template に統合されました。同じ機能が重複するので、旧拡張をアンインストールしてください。`,
            uninstall
        )
        .then(async (choice) => {
            if (choice !== uninstall) return;
            for (const id of found) {
                await vscode.commands.executeCommand('workbench.extensions.uninstallExtension', id);
            }
            const reload = 'ウィンドウを再読み込み';
            const again = await vscode.window.showInformationMessage(
                '旧拡張をアンインストールしました。反映するにはウィンドウを再読み込みしてください。',
                reload
            );
            if (again === reload) await vscode.commands.executeCommand('workbench.action.reloadWindow');
        });
}
