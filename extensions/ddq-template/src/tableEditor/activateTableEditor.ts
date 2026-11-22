import { randomBytes } from 'crypto';
import * as vscode from 'vscode';
import type { TableModel } from './model/TableModel';
import type { EditorContext, FromWebviewMessage, ToWebviewMessage } from './model/WebviewMessages';
import { validateTableModel } from './model/validateTableModel';
import { normalizeTableModel } from './model/normalizeTableModel';
import { completeColumnWidths } from './model/columnWidths';
import { findEditTarget, type EditTarget } from './markdown-document/findEditTarget';
import { buildReplacement, anchorLineOf } from './markdown-document/applyTable';
import { splitLines } from './markdown-document/splitLines';
import { generateTableId } from './markdown-document/generateTableId';

/**
 * 開いている編集セッション。
 *
 * 行範囲はドキュメントが変わるたびに無効になるので、Apply のたびに取り直す
 * （`target` を使い回すと 2 回目以降に別の場所を書き潰してしまう）。
 */
type Session = {
  document: vscode.TextDocument;
  target: EditTarget;
  /** target を解決した時点のドキュメント版。ずれていたら書き戻さない。 */
  documentVersion: number;
};

/**
 * この起動が持つ表エディタ。パネルは 1 枚だけを使い回し、編集セッションはそのパネルに 1 つ。
 * 拡張の終了（context の破棄）で捨てる。
 */
type Host = {
  context: vscode.ExtensionContext;
  panel?: vscode.WebviewPanel;
  session?: Session;
};

/** いまの起動の表エディタ（deactivateTableEditor が捨てる） */
let current: Host | undefined;

export function activateTableEditor(context: vscode.ExtensionContext) {
  const host: Host = { context };
  current = host;
  context.subscriptions.push(
    vscode.commands.registerCommand('ddqTemplate.openTableEditor', () => openTableEditor(host))
  );
  context.subscriptions.push({ dispose: () => disposePanel(host) });
}

export function deactivateTableEditor() {
  if (current) disposePanel(current);
  current = undefined;
}

function disposePanel(host: Host) {
  const panel = host.panel;
  host.panel = undefined;
  host.session = undefined;
  panel?.dispose();
}

function openTableEditor(host: Host) {
  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    vscode.window.showErrorMessage('テーブルを編集する .qmd / .md ファイルを開いてください。');
    return;
  }

  const document = editor.document;
  const lines = splitLines(document.getText());
  const found = findEditTarget(lines, editor.selection.active.line, generateTableId());
  if (!found.ok) {
    vscode.window.showErrorMessage(found.message);
    return;
  }

  host.session = { document, target: found.value, documentVersion: document.version };

  // パネルは 1 つだけ使い回す。retainContextWhenHidden は保持コストが高いので、
  // コマンドを実行するたびに増やさない。
  if (host.panel) {
    host.panel.reveal(vscode.ViewColumn.Beside, true);
    postInit(host, host.panel);
    return;
  }

  const panel = vscode.window.createWebviewPanel(
    'ddqTemplate.tableEditor',
    'DDQ 表エディタ',
    vscode.ViewColumn.Beside,
    {
      enableScripts: true,
      retainContextWhenHidden: true,
      localResourceRoots: [vscode.Uri.joinPath(host.context.extensionUri, 'out', 'webview', 'tableEditor')]
    }
  );
  host.panel = panel;

  panel.webview.html = buildHtml(panel.webview, host.context.extensionUri);

  panel.onDidDispose(() => {
    // 閉じたのがいまのパネルのときだけ捨てる（閉じた後に開き直したパネルは残す）
    if (host.panel !== panel) return;
    host.panel = undefined;
    host.session = undefined;
  });

  panel.webview.onDidReceiveMessage(async (message: FromWebviewMessage) => {
    // 閉じたパネルから遅れて届いたものは扱わない
    if (host.panel !== panel) return;
    switch (message.type) {
      case 'ready':
        postInit(host, panel);
        break;
      case 'apply': {
        const result = await applyToDocument(host, panel, message.model);
        // 書き戻しのあいだにパネルを閉じた・拡張が終わった、または別の表を開き直した
        if (host.panel !== panel || result.superseded) break;
        if (result.ok) {
          // 取り直した編集対象を送り直し、Webview とファイルの状態を揃える
          postInit(host, panel);
          post(panel, { type: 'applied' });
        } else {
          post(panel, { type: 'error', message: result.message });
        }
        break;
      }
      case 'close':
        disposePanel(host);
        break;
    }
  });
}

function post(panel: vscode.WebviewPanel, message: ToWebviewMessage) {
  void panel.webview.postMessage(message);
}

function postInit(host: Host, panel: vscode.WebviewPanel) {
  if (!host.session) return;
  post(panel, { type: 'init', model: host.session.target.model, context: toContext(host.session.target) });
}

function toContext(target: EditTarget): EditorContext {
  return {
    kind: target.kind,
    partCount: target.partCount,
    partIndex: target.partIndex,
    attributesEditable: target.attributesEditable,
    enclosingClasses: target.enclosingClasses,
    warnings: target.warnings
  };
}

/** `superseded`: 書き戻しのあいだに別の表を開き直した（画面は開き直した表のまま、何も送らない） */
type ApplyResult = { ok: true; superseded?: boolean } | { ok: false; message: string; superseded?: false };

async function applyToDocument(host: Host, panel: vscode.WebviewPanel, rawModel: TableModel): Promise<ApplyResult> {
  const started = host.session;
  if (!started) return { ok: false, message: '編集セッションが失われました。開き直してください。' };
  const { document, target } = started;
  // 書き戻しを始めたときのパネル・編集セッションのままか（確認ダイアログの間に別の表を開き直せる）
  const unchanged = () => host.panel === panel && host.session === started;

  if (document.isClosed) {
    return { ok: false, message: '対象のファイルが閉じられています。開き直してください。' };
  }
  // 外部で編集されていると行番号がずれており、そのまま書くと別の場所を壊す
  if (document.version !== started.documentVersion) {
    return {
      ok: false,
      message: 'エディタを開いてからファイルが変更されました。表を開き直してください。'
    };
  }

  // 空欄が 1 列だけの列幅は、書き戻す直前に残り幅で埋める
  const model = completeColumnWidths(normalizeTableModel(rawModel));

  const validation = validateTableModel(model, {
    otherPartColumnCounts: target.otherPartColumnCounts
  });
  if (validation.errors.length > 0) {
    return { ok: false, message: validation.errors.join('\n') };
  }
  if (validation.warnings.length > 0) {
    const choice = await vscode.window.showWarningMessage(
      validation.warnings.join('\n'),
      { modal: true },
      '続行する'
    );
    if (choice !== '続行する') return { ok: false, message: '書き戻しを中止しました。' };
    // モーダル表示中に、表エディタで別の表を開き直していないか
    if (!unchanged()) {
      return { ok: false, message: '確認のあいだに表エディタで別の表を開いたため、書き戻しを中止しました。' };
    }
    // モーダル表示中に変更されていないか、もう一度確かめる
    if (document.isClosed || document.version !== started.documentVersion) {
      return { ok: false, message: 'ファイルが変更されました。表を開き直してください。' };
    }
  }

  const lines = splitLines(document.getText());
  const replacement = buildReplacement(lines, target, model);

  const edit = new vscode.WorkspaceEdit();
  const isInsert = replacement.range.start > replacement.range.end;
  if (isInsert) {
    const at = new vscode.Position(Math.min(replacement.range.start, document.lineCount), 0);
    edit.insert(document.uri, at, replacement.text + '\n');
  } else {
    const end = Math.min(replacement.range.end, document.lineCount - 1);
    edit.replace(
      document.uri,
      new vscode.Range(
        new vscode.Position(replacement.range.start, 0),
        new vscode.Position(end, document.lineAt(end).text.length)
      ),
      replacement.text
    );
  }

  if (!(await vscode.workspace.applyEdit(edit))) {
    return { ok: false, message: 'ドキュメントへの反映に失敗しました。' };
  }
  // 反映のあいだに別の表を開き直したなら、その表の編集セッションを上書きしない
  if (!unchanged()) return { ok: true, superseded: true };

  // 行番号がずれたので、いま書いた場所から編集対象を取り直す。
  // これをしないと 2 回目の Apply が別の範囲を書き潰す。
  const updated = splitLines(document.getText());
  const refound = findEditTarget(updated, anchorLineOf(replacement), generateTableId());
  if (refound.ok) {
    host.session = { document, target: refound.value, documentVersion: document.version };
  } else {
    host.session = undefined;
    return { ok: false, message: `書き戻しましたが再読み込みに失敗しました: ${refound.message}` };
  }

  return { ok: true };
}

function buildHtml(webview: vscode.Webview, extensionUri: vscode.Uri): string {
  const nonce = randomBytes(16).toString('base64');
  const scriptUri = webview.asWebviewUri(
    vscode.Uri.joinPath(extensionUri, 'out', 'webview', 'tableEditor', 'assets', 'main.js')
  );
  const styleUri = webview.asWebviewUri(
    vscode.Uri.joinPath(extensionUri, 'out', 'webview', 'tableEditor', 'assets', 'main.css')
  );

  // connect-src を書かないので default-src 'none' が効き、通信は一切できない
  return `<!DOCTYPE html>
<html lang="ja">
<head>
  <meta charset="UTF-8">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${webview.cspSource}; script-src 'nonce-${nonce}'; style-src ${webview.cspSource} 'nonce-${nonce}';">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <link rel="stylesheet" nonce="${nonce}" href="${styleUri}">
  <title>DDQ 表エディタ</title>
</head>
<body>
  <div id="root"></div>
  <script nonce="${nonce}" src="${scriptUri}"></script>
</body>
</html>`;
}
