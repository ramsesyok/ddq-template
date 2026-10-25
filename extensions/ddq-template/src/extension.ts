/**
 * DDQ Template — design-doc-quarto-template（ddq）の VSCode 拡張（docs/ddq-template-study.md）。
 *
 * 旧 ddq-revision（ラベル付け・改訂履歴）と旧 ddq-table-editor（表エディタ）を 1 本にまとめたもの。
 * 機能ごとの登録はそれぞれのファイルに置き、ここは束ねるだけにする。
 */

import * as vscode from 'vscode';

import { activateRevision, deactivateRevision } from './activateRevision';
import { warnLegacyExtensions } from './legacy';
import { activateTableEditor, deactivateTableEditor } from './tableEditor/activateTableEditor';

export function activate(context: vscode.ExtensionContext) {
    activateRevision(context);
    activateTableEditor(context);
    warnLegacyExtensions();
}

export function deactivate() {
    deactivateRevision();
    deactivateTableEditor();
}
