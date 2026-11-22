/**
 * Webview の HTML と、読ませてよい資産の範囲（EXT-04）。表エディタと、ラベル一覧・改訂履歴の
 * 画面が共有する。画面ごとのメッセージの型・モデルはそれぞれの機能に置く（ここは器だけ）。
 *
 * - 資産は `out/webview/<バンドル>/` だけ（`localResourceRoots`）。ワークスペースや拡張の他の
 *   ファイルは Webview から読めない
 * - CSP は `default-src 'none'` で、`connect-src` を書かない（通信は一切できない。
 *   scripts/check-offline.mjs が拡張の束ねた JS で確かめる）。スクリプトとスタイルは nonce つき
 */

import { randomBytes } from 'node:crypto';
import * as vscode from 'vscode';

/** Webview のバンドル（vite.config の出力先 `out/webview/<名前>/`） */
export type Bundle = 'tableEditor' | 'revision';

/**
 * 画面ごとに、Webview のスクリプトが動いて `ready` が届いた回数。実拡張ホストの検証（tests/host）が
 * `ddqTemplate.internal.webviewReadies` で読み、バンドルが CSP と資産の範囲の中で読み込めて、
 * メッセージが拡張まで届くことを確かめる（タブが開くだけでは、画面が真っ白でも気付けない）。
 */
const readies = { tableEditor: 0, tags: 0, revision: 0 };

/** Webview から `ready` が届いた（画面のスクリプトが動いた） */
export function noteReady(view: keyof typeof readies): void {
    readies[view] += 1;
}

export function activateWebviewProbe(context: vscode.ExtensionContext): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('ddqTemplate.internal.webviewReadies', () => ({ ...readies }))
    );
}

/** そのバンドルの置き場所。Webview が読める範囲はここだけにする */
export function bundleRoot(extensionUri: vscode.Uri, bundle: Bundle): vscode.Uri {
    return vscode.Uri.joinPath(extensionUri, 'out', 'webview', bundle);
}

/** スクリプトを許し、読める資産をそのバンドルに限る */
export function webviewOptions(extensionUri: vscode.Uri, bundle: Bundle): vscode.WebviewOptions {
    return { enableScripts: true, localResourceRoots: [bundleRoot(extensionUri, bundle)] };
}

/**
 * バンドルの `assets/main.js` / `main.css` を読み込む HTML。`view` を渡すと `<body data-view>` に入れる
 * （ラベル一覧と改訂履歴は 1 バンドルを data-view で切り替える）。
 */
export function webviewHtml(
    webview: vscode.Webview,
    extensionUri: vscode.Uri,
    bundle: Bundle,
    page: { title: string; view?: string }
): string {
    const nonce = randomBytes(16).toString('base64');
    const assets = vscode.Uri.joinPath(bundleRoot(extensionUri, bundle), 'assets');
    const scriptUri = webview.asWebviewUri(vscode.Uri.joinPath(assets, 'main.js'));
    const styleUri = webview.asWebviewUri(vscode.Uri.joinPath(assets, 'main.css'));
    const body = page.view === undefined ? '<body>' : `<body data-view="${page.view}">`;
    // connect-src を書かないので default-src 'none' が効き、通信は一切できない
    return `<!DOCTYPE html>
<html lang="ja">
<head>
  <meta charset="UTF-8">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${webview.cspSource}; script-src 'nonce-${nonce}'; style-src ${webview.cspSource} 'nonce-${nonce}';">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <link rel="stylesheet" nonce="${nonce}" href="${styleUri}">
  <title>${page.title}</title>
</head>
${body}
  <div id="root"></div>
  <script nonce="${nonce}" src="${scriptUri}"></script>
</body>
</html>`;
}
