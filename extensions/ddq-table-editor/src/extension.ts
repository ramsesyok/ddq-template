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

let panel: vscode.WebviewPanel | undefined;
let session: Session | undefined;

export function activate(context: vscode.ExtensionContext) {
  context.subscriptions.push(
    vscode.commands.registerCommand('ddqTableEditor.open', () => openTableEditor(context))
  );
  context.subscriptions.push({ dispose: disposePanel });
}

export function deactivate() {
  disposePanel();
}

function disposePanel() {
  panel?.dispose();
  panel = undefined;
  session = undefined;
}

function openTableEditor(context: vscode.ExtensionContext) {
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

  session = { document, target: found.value, documentVersion: document.version };

  // パネルは 1 つだけ使い回す。retainContextWhenHidden は保持コストが高いので、
  // コマンドを実行するたびに増やさない。
  if (panel) {
    panel.reveal(vscode.ViewColumn.Beside, true);
    postInit();
    return;
  }

  panel = vscode.window.createWebviewPanel(
    'ddqTableEditor',
    'DDQ Table Editor',
    vscode.ViewColumn.Beside,
    {
      enableScripts: true,
      retainContextWhenHidden: true,
      localResourceRoots: [vscode.Uri.joinPath(context.extensionUri, 'out', 'webview')]
    }
  );

  panel.webview.html = buildHtml(panel.webview, context.extensionUri);

  panel.onDidDispose(() => {
    panel = undefined;
    session = undefined;
  });

  panel.webview.onDidReceiveMessage(async (message: FromWebviewMessage) => {
    switch (message.type) {
      case 'ready':
        postInit();
        break;
      case 'apply': {
        const result = await applyToDocument(message.model);
        if (result.ok) {
          // 取り直した編集対象を送り直し、Webview とファイルの状態を揃える
          postInit();
          post({ type: 'applied' });
        } else {
          post({ type: 'error', message: result.message });
        }
        break;
      }
      case 'close':
        disposePanel();
        break;
    }
  });
}

function post(message: ToWebviewMessage) {
  panel?.webview.postMessage(message);
}

function postInit() {
  if (!session) return;
  post({ type: 'init', model: session.target.model, context: toContext(session.target) });
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

type ApplyResult = { ok: true } | { ok: false; message: string };

async function applyToDocument(rawModel: TableModel): Promise<ApplyResult> {
  if (!session) return { ok: false, message: '編集セッションが失われました。開き直してください。' };
  const { document, target } = session;

  if (document.isClosed) {
    return { ok: false, message: '対象のファイルが閉じられています。開き直してください。' };
  }
  // 外部で編集されていると行番号がずれており、そのまま書くと別の場所を壊す
  if (document.version !== session.documentVersion) {
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
    // モーダル表示中に変更されていないか、もう一度確かめる
    if (document.isClosed || document.version !== session.documentVersion) {
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

  // 行番号がずれたので、いま書いた場所から編集対象を取り直す。
  // これをしないと 2 回目の Apply が別の範囲を書き潰す。
  const updated = splitLines(document.getText());
  const refound = findEditTarget(updated, anchorLineOf(replacement), generateTableId());
  if (refound.ok) {
    session = { document, target: refound.value, documentVersion: document.version };
  } else {
    session = undefined;
    return { ok: false, message: `書き戻しましたが再読み込みに失敗しました: ${refound.message}` };
  }

  return { ok: true };
}

function buildHtml(webview: vscode.Webview, extensionUri: vscode.Uri): string {
  const nonce = randomBytes(16).toString('base64');
  const scriptUri = webview.asWebviewUri(
    vscode.Uri.joinPath(extensionUri, 'out', 'webview', 'assets', 'main.js')
  );
  const styleUri = webview.asWebviewUri(
    vscode.Uri.joinPath(extensionUri, 'out', 'webview', 'assets', 'main.css')
  );

  // connect-src を書かないので default-src 'none' が効き、通信は一切できない
  return `<!DOCTYPE html>
<html lang="ja">
<head>
  <meta charset="UTF-8">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${webview.cspSource}; script-src 'nonce-${nonce}'; style-src ${webview.cspSource} 'nonce-${nonce}';">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <link rel="stylesheet" nonce="${nonce}" href="${styleUri}">
  <title>DDQ Table Editor</title>
</head>
<body>
  <div id="root"></div>
  <script nonce="${nonce}" src="${scriptUri}"></script>
</body>
</html>`;
}
