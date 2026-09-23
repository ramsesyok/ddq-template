/**
 * 改訂履歴エディタ（docs/revision-study.md §5.8）。
 *
 * `revisions/rev-<記号>.yml` を `CustomTextEditorProvider` として開く。**文書そのものが
 * 正本**なので、保存・未保存の印・Undo・外部からの変更の検知は VSCode 任せにできる。
 * Webview は差分の一覧とメモの入力だけを持ち、直したら YAML 全体を作り直して
 * `WorkspaceEdit` で置き換える（ddq-table-editor と同じ流儀）。
 *
 * 差分そのものは自前で描かず、VSCode 標準の差分エディタに渡す。旧版は内蔵 Git 拡張の
 * `toGitUri`、旧版に無いファイル（追加）や新版に無いファイル（削除）の側は、空を返す
 * 自前スキーム（`ddq-rev`）で出す。理由は §5.8 のとおりで、旧版に無いファイルの
 * `git:` URI は `openTextDocument` が例外になるため。
 */

import * as path from 'node:path';
import * as vscode from 'vscode';

import { DdqError, ddqCommand, run, runJson } from '../ddq/run';
import { emptyNotes, type EditorContext, type FromEditorMessage, type ToEditorMessage } from './messages';
import { parse, toYaml, type Revision } from './revfile';

/** 空の左右ペインを出すための自前スキーム（§5.8）。 */
export const EMPTY_SCHEME = 'ddq-rev';

/** `ddq rev next --json` の答え（cli/src/commands/rev.rs と対）。 */
type Next = {
    rev: string;
    base: string | null;
    base_commit: string | null;
    scheme: string;
    tags: string[];
    drafts: string[];
};

export class RevisionEditorProvider implements vscode.CustomTextEditorProvider {
    static readonly viewType = 'ddqRevision.editor';

    constructor(private readonly extensionUri: vscode.Uri) {}

    /** 追加・削除の側に出す「空の文書」。 */
    static emptyProvider(): vscode.TextDocumentContentProvider {
        return { provideTextDocumentContent: () => '' };
    }

    async resolveCustomTextEditor(
        document: vscode.TextDocument,
        panel: vscode.WebviewPanel,
        _token: vscode.CancellationToken
    ): Promise<void> {
        panel.webview.options = { enableScripts: true };
        panel.webview.html = html(panel.webview, this.extensionUri, 'revision');

        const post = (message: ToEditorMessage) => void panel.webview.postMessage(message);
        const folder = writingFolderOf(document.uri);

        let context: EditorContext = {
            folder,
            fileName: path.basename(document.uri.fsPath),
            fixed: false,
            tags: []
        };

        const send = async () => {
            context = { ...context, ...(await tagState(folder, document.uri)) };
            post({ type: 'revision', revision: parse(document.getText()), context });
        };

        const changed = vscode.workspace.onDidChangeTextDocument((e) => {
            // ddq rev diff --write で外から書き換わったときも画面を合わせる
            if (e.document.uri.toString() === document.uri.toString()) void send();
        });
        panel.onDidDispose(() => changed.dispose());

        panel.webview.onDidReceiveMessage(async (message: FromEditorMessage) => {
            try {
                switch (message.type) {
                    case 'ready':
                        await send();
                        return;
                    case 'edit':
                        await write(document, message.revision);
                        return;
                    case 'reveal':
                        await reveal(folder, message.file, message.line);
                        return;
                    case 'diff':
                        await openDiff(folder, parse(document.getText()), message.label);
                        return;
                    case 'refresh':
                        await this.refresh(folder, document, message.base, post);
                        return;
                    case 'build':
                        await this.build(folder, post);
                        return;
                    case 'fix':
                        await this.fix(folder, document, post);
                        return;
                }
            } catch (e) {
                const error = e instanceof DdqError ? e : new DdqError(String(e));
                post({
                    type: 'notice',
                    level: 'error',
                    text: [error.message, error.detail].filter(Boolean).join('\n')
                });
            } finally {
                post({ type: 'busy', busy: false });
            }
        });
    }

    /** 基準の版と作業ツリーを取り直す。ddq が yml を書き、外部変更として画面に返ってくる。 */
    private async refresh(
        folder: string,
        document: vscode.TextDocument,
        base: string | undefined,
        post: (m: ToEditorMessage) => void
    ) {
        if (document.isDirty) {
            const save = await vscode.window.showWarningMessage(
                '保存していない変更があります。取り直す前に保存しますか？',
                { modal: true },
                '保存して取り直す'
            );
            if (save !== '保存して取り直す') return;
            await document.save();
        }
        post({ type: 'busy', busy: true });
        const args = ['rev', 'diff', folder, '--write'];
        if (base && base.trim() !== '') args.push('--base', base.trim());
        const out = await run(ddq(), args, folder);
        post({ type: 'notice', level: 'info', text: out.trim() });
    }

    /** 改訂履歴の表を作る。 */
    private async build(folder: string, post: (m: ToEditorMessage) => void) {
        post({ type: 'busy', busy: true });
        const out = await run(ddq(), ['rev', 'build', folder], folder);
        post({ type: 'notice', level: 'info', text: out.trim() });
    }

    /**
     * 確定。表を作り直し、**次に人が打つ Git コマンドを案内する**（実行はしない）。
     * タグ付けとコミットを人の手に残すのは、運用が回ると確かめられるまでの方針（Q17）。
     */
    private async fix(folder: string, document: vscode.TextDocument, post: (m: ToEditorMessage) => void) {
        const revision = parse(document.getText());
        const empty = emptyNotes(revision);
        if (empty > 0) {
            const go = await vscode.window.showWarningMessage(
                `修正内容（note）が空の行が ${empty} 件あります。このまま確定しますか？`,
                { modal: true },
                '空のまま確定する'
            );
            if (go !== '空のまま確定する') return;
        }
        if (document.isDirty) await document.save();
        await this.build(folder, post);

        const rel = vscode.workspace.asRelativePath(folder, false);
        const tag = `rev-${revision.rev}`;
        const commands = [
            `git add ${rel}/revisions`,
            `git commit -m "改訂 ${revision.rev}"`,
            `git tag ${tag}`
        ].join(' && ');
        const copy = await vscode.window.showInformationMessage(
            `表を作りました。確定するには次を実行してください（コミットとタグは人が打ちます）:\n${commands}`,
            { modal: true },
            'コマンドをコピー'
        );
        if (copy === 'コマンドをコピー') await vscode.env.clipboard.writeText(commands);
        post({ type: 'notice', level: 'info', text: `確定するには: ${commands}` });
    }
}

/**
 * 直した内容を YAML に直して文書へ書く（保存はしない = VSCode の流儀）。
 *
 * ddq は LF で書くが、Git の設定によっては作業ツリーの yml が CRLF になっている。
 * その文書の改行に合わせて書かないと、メモを 1 つ直しただけで全行が書き換わってしまう。
 */
export async function write(document: vscode.TextDocument, revision: Revision) {
    const next =
        document.eol === vscode.EndOfLine.CRLF
            ? toYaml(revision).replace(/\n/g, '\r\n')
            : toYaml(revision);
    if (next === document.getText()) return;
    const edit = new vscode.WorkspaceEdit();
    edit.replace(
        document.uri,
        new vscode.Range(0, 0, document.lineCount, 0),
        next
    );
    await vscode.workspace.applyEdit(edit);
}

/** `revisions/rev-B.yml` から執筆フォルダ（`revisions` の親）を求める。 */
export function writingFolderOf(uri: vscode.Uri): string {
    return path.dirname(path.dirname(uri.fsPath));
}

function ddq(): string {
    return ddqCommand(vscode.workspace.getConfiguration('ddqRevision').get<string>('ddqPath'));
}

/** タグの有無（= 確定済みか）と、既にあるタグ。 */
async function tagState(folder: string, uri: vscode.Uri): Promise<Partial<EditorContext>> {
    try {
        const next = await runJson<Next>(ddq(), ['rev', 'next', folder, '--json'], folder);
        const symbol = path.basename(uri.fsPath).replace(/^rev-/, '').replace(/\.ya?ml$/i, '');
        return { tags: next.tags, fixed: next.tags.includes(`rev-${symbol}`) };
    } catch {
        // Git が無い・タグが読めないだけなら、編集はできてよい
        return { tags: [], fixed: false };
    }
}

async function reveal(folder: string, file: string, line?: number) {
    if (file === '') return;
    try {
        const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(path.join(folder, file)));
        const at = new vscode.Position(Math.max(0, (line ?? 1) - 1), 0);
        await vscode.window.showTextDocument(doc, {
            viewColumn: vscode.ViewColumn.Beside,
            selection: new vscode.Selection(at, at),
            preserveFocus: true
        });
    } catch {
        vscode.window.showErrorMessage(`${file} を開けません。`);
    }
}

/**
 * その行の差分を VSCode の差分エディタで開く。
 *
 * 左右の URI は種別で使い分ける（§5.8 の表）。旧版に無いファイル（added）の `git:` URI は
 * `openTextDocument` が例外になるので、その側は空を返す自前スキームにする。
 */
export async function openDiff(folder: string, revision: Revision, label: string) {
    const entry = revision.entries.find((e) => e.label === label);
    if (!entry) return;
    if (entry.file === '') {
        vscode.window.showWarningMessage(`${label} は場所が分からないため差分を出せません。`);
        return;
    }
    const fileUri = vscode.Uri.file(path.join(folder, entry.file));
    const empty = (name: string) =>
        vscode.Uri.parse(`${EMPTY_SCHEME}:${encodeURIComponent(name)}?empty`);

    let left: vscode.Uri;
    let right: vscode.Uri;
    if (entry.kind === 'added') {
        left = empty(entry.file);
        right = fileUri;
    } else {
        const git = await gitUri(fileUri, revision.baseCommit || revision.base);
        if (!git) return;
        left = git;
        right = entry.kind === 'removed' ? empty(entry.file) : fileUri;
    }
    const title = `${entry.title || entry.label}（${revision.base || revision.baseCommit} ↔ 作業ツリー）`;
    // 「箇所」（reveal）と同じく編集画面の隣に出す。編集画面に重ねると、メモを書きながら見られない
    await vscode.commands.executeCommand('vscode.diff', left, right, title, {
        preview: true,
        viewColumn: vscode.ViewColumn.Beside,
        preserveFocus: true
    });
}

/** 案内のボタン。 */
const OPEN_REPOSITORY = 'リポジトリを開いて差分を見る';
const OPEN_SETTING = '設定を開く';

/**
 * 内蔵 Git 拡張の API で、その版のファイルを指す URI を作る。
 *
 * `git:` URI は Git 拡張が**そのリポジトリを開いているとき**しか読めない。執筆フォルダだけを
 * VSCode で開くと、リポジトリの本体はその上にあり、設定 `git.openRepositoryInParentFolders`
 * が `never`（または `prompt` の通知を見逃した）だと開かれない。そのまま差分を開くと
 * 「ファイルが見つからない」としか出ないので、先に確かめて案内する。
 */
async function gitUri(fileUri: vscode.Uri, ref: string): Promise<vscode.Uri | undefined> {
    const extension = vscode.extensions.getExtension('vscode.git');
    if (!extension) {
        vscode.window.showErrorMessage('内蔵の Git 拡張が無効です。差分を出せません。');
        return undefined;
    }
    const git = (await extension.activate()).getAPI(1);
    if (!(await repositoryOf(git, fileUri))) return undefined;
    return git.toGitUri(fileUri, ref);
}

/** Git 拡張がそのファイルのリポジトリを開いているか。無ければ案内し、望まれれば開く。 */
async function repositoryOf(git: GitApi, fileUri: vscode.Uri): Promise<boolean> {
    if (git.state !== 'initialized') {
        // 起動直後はリポジトリの探索が終わっていない
        await new Promise<void>((resolve) => {
            const sub = git.onDidChangeState((state) => {
                if (state === 'initialized') {
                    sub.dispose();
                    resolve();
                }
            });
        });
    }
    if (git.getRepository(fileUri)) return true;

    const choice = await vscode.window.showWarningMessage(
        '内蔵の Git 拡張がこの文書のリポジトリを開いていないため、基準の版を読めません。',
        {
            modal: true,
            detail:
                '執筆フォルダだけを VSCode で開いていると、その上にあるリポジトリは設定 ' +
                'git.openRepositoryInParentFolders が "always" でない限り開かれません' +
                '（"prompt" のときは通知で「はい」を押す必要があります）。'
        },
        OPEN_REPOSITORY,
        OPEN_SETTING
    );
    if (choice === OPEN_REPOSITORY) {
        // 設定に関係なく開く（このウィンドウの間だけ。ソース管理ビューにも出る）
        if (await git.openRepository(fileUri)) return true;
        vscode.window.showErrorMessage('リポジトリを開けませんでした。Git の出力を確認してください。');
    } else if (choice === OPEN_SETTING) {
        await vscode.commands.executeCommand(
            'workbench.action.openSettings',
            'git.openRepositoryInParentFolders'
        );
    }
    return false;
}

/** 内蔵 Git 拡張の API のうち、ここで使うもの（vscode/extensions/git/src/api/git.d.ts）。 */
type GitApi = {
    readonly state: 'uninitialized' | 'initialized';
    readonly onDidChangeState: vscode.Event<'uninitialized' | 'initialized'>;
    toGitUri(uri: vscode.Uri, ref: string): vscode.Uri;
    getRepository(uri: vscode.Uri): unknown | null;
    openRepository(root: vscode.Uri): Promise<unknown | null>;
};

/** Webview の HTML（タグ付け画面と共通の 1 バンドルを、data-view で切り替える）。 */
export function html(webview: vscode.Webview, extensionUri: vscode.Uri, view: 'tags' | 'revision'): string {
    const nonce = nonceOf();
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
  <title>DDQ Revision</title>
</head>
<body data-view="${view}">
  <div id="root"></div>
  <script nonce="${nonce}" src="${scriptUri}"></script>
</body>
</html>`;
}

function nonceOf(): string {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    return require('node:crypto').randomBytes(16).toString('base64');
}
