/**
 * DDQ Template — design-doc-quarto-template（ddq）の VSCode 拡張（docs/ddq-template-study.md）。
 *
 * 旧 ddq-revision（ラベル付け・改訂履歴）と旧 ddq-table-editor（表エディタ）を 1 本にまとめ、
 * ddq による発行（pdf / html / diagrams）を足したもの。
 * 機能ごとの登録はそれぞれのファイルに置き、ここは束ねるだけにする。
 */

import * as vscode from 'vscode';

import { activateRevision, deactivateRevision } from './activateRevision';
import { activateBuild, deactivateBuild } from './build';
import { initLocate } from './ddq/locate';
import { warnLegacyExtensions } from './legacy';
import { activateTableEditor, deactivateTableEditor } from './tableEditor/activateTableEditor';

export function activate(context: vscode.ExtensionContext) {
    initLocate(context.extensionPath);
    activateBuild(context);
    activateRevision(context);
    activateTableEditor(context);
    warnLegacyExtensions();
}

export function deactivate() {
    deactivateBuild();
    deactivateRevision();
    deactivateTableEditor();
}
