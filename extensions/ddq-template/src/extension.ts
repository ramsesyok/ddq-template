/**
 * DDQ Template — design-doc-quarto-template（ddq）の VSCode 拡張（docs/ddq-template-study.md）。
 *
 * 旧 ddq-revision（ラベル付け・改訂履歴）と旧 ddq-table-editor（表エディタ）を 1 本にまとめ、
 * ddq による作成（init）と発行（pdf / html / diagrams）を足したもの。
 * 機能ごとの登録はそれぞれのファイルに置き、ここは束ねるだけにする。
 */

import * as vscode from 'vscode';

import { activateRevision, deactivateRevision } from './activateRevision';
import { activateBuild } from './build';
import { activateLaunch, deactivateLaunch } from './ddq/launch';
import { initLocate } from './ddq/locate';
import { activateAdd } from './add';
import { activateInit } from './init';
import { warnLegacyExtensions } from './legacy';
import { activateTableEditor, deactivateTableEditor } from './tableEditor/activateTableEditor';

export function activate(context: vscode.ExtensionContext) {
    initLocate(context.extensionPath);
    activateLaunch();
    activateBuild(context);
    activateInit(context);
    activateAdd(context);
    activateRevision(context);
    activateTableEditor(context);
    warnLegacyExtensions();
}

export function deactivate() {
    // 起動中の ddq（発行・JSON の取得とも）を孫ごと止める
    deactivateLaunch();
    deactivateRevision();
    deactivateTableEditor();
}
