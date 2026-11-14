import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

/**
 * 通知・ダイアログの操作（設計書リポジトリの作成・執筆フォルダの追加・発行）。
 *
 * 実拡張ホストではダイアログのボタンを押せないので（tests/host は内部コマンドで作成処理だけを通す）、
 * ここで VSCode を差し替え（testing/fakeVscode.ts）、答えを積んで本物のコマンドを最後まで通す。
 * ddq は差し替え（呼ばれた引数を記録し、積んだ結果を返す）。執筆フォルダは一時フォルダに本当に作る。
 */

vi.mock('vscode', () => import('./testing/fakeVscode'));

const ddq = vi.hoisted(() => ({
    /** runStreaming に渡った引数（init / add / pdf …） */
    calls: [] as string[][],
    /** 順に返す結果（尽きたら成功） */
    results: [] as { code: number | null; output?: string; error?: string }[],
    /** 呼ばれたときの副作用（ddq がファイルを作るところ） */
    onRun: undefined as ((args: string[]) => void) | undefined,
    quarto: true,
    version: 'ddq 2.4.6',
    kill: vi.fn()
}));

vi.mock('./ddq/run', async (importOriginal) => ({
    ...(await importOriginal<typeof import('./ddq/run')>()),
    run: async () => ddq.version,
    runStreaming: async (
        _command: string,
        args: string[],
        options: { onOutput: (t: string) => void; onSpawn?: (c: { kill: () => void }) => void }
    ) => {
        ddq.calls.push(args);
        options.onSpawn?.({ kill: ddq.kill });
        ddq.onRun?.(args);
        const result = ddq.results.shift() ?? { code: 0 };
        options.onOutput(result.output ?? '');
        return { output: '', ...result };
    }
}));
vi.mock('./ddq/locate', () => ({ locateDdq: () => ({ command: 'ddq', source: 'path' }) }));
vi.mock('./quarto', () => ({ envWithQuarto: () => ({}), hasQuarto: async () => ddq.quarto }));
vi.mock('./openFile', () => ({ openWithDefaultApp: vi.fn(async () => undefined) }));

import { activateAdd } from './add';
import { activateBuild } from './build';
import { tryLock } from './ddq/lock';
import { activateInit } from './init';
import { openWithDefaultApp } from './openFile';
import { Uri, extensionContext, fake } from './testing/fakeVscode';

const PENDING = 'ddqTemplate.pendingInit';
let context: ReturnType<typeof extensionContext>;
const temps: string[] = [];

function tmp(): string {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ddq-dialogs-'));
    temps.push(dir);
    return dir;
}

function write(file: string, text = '') {
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, text);
}

/** 執筆フォルダ（_quarto.yml と版ファイル）を作り、ワークスペースの検索に載せる */
function writingFolder(dir: string, version = '2.4.6'): string {
    write(path.join(dir, '_quarto.yml'), 'project:\n  type: book\n');
    write(path.join(dir, '.template-version'), `${version}\n`);
    fake.found.push(path.join(dir, '.template-version'));
    return dir;
}

/** ddq init / add が作るもの（_quarto.yml）を、呼ばれたときに作る */
function ddqCreatesQuartoYml() {
    ddq.onRun = (args) => {
        if (args[0] === 'init') write(path.join(args[1], args[2], '_quarto.yml'));
        if (args[0] === 'add') write(path.join(args[1], '_quarto.yml'));
    };
}

function executed(id: string) {
    return fake.executed.filter((e) => e.id === id).map((e) => e.args);
}

beforeEach(() => {
    fake.reset();
    ddq.calls = [];
    ddq.results = [];
    ddq.onRun = undefined;
    ddq.quarto = true;
    ddq.version = 'ddq 2.4.6';
    ddq.kill.mockClear();
    vi.mocked(openWithDefaultApp).mockClear();
    context = extensionContext();
    activateBuild(context as never);
    activateInit(context as never);
    activateAdd(context as never);
});

afterEach(() => {
    for (const dir of temps.splice(0)) fs.rmSync(dir, { recursive: true, force: true });
});

describe('設計書リポジトリを作成（フォルダを開いていない）', () => {
    it('親フォルダ → リポジトリ名 → 執筆フォルダ名の順に聞いて作り、そのフォルダを開き直す', async () => {
        const parent = tmp();
        write(path.join(parent, 'used', 'notes.txt'));
        fake.answer('showOpenDialog', [Uri.file(parent)]);
        // 区切り文字・空でないフォルダは入力欄で通らない（確定できない）
        fake.answer('showInputBox', 'a/b', 'used', ' order-design ', 'docs');

        expect(await fake.run('ddqTemplate.init')).toBe('ok');

        expect(fake.rejected).toEqual([expect.stringMatching(/区切り文字/), expect.stringMatching(/空ではありません/)]);
        const repo = path.join(parent, 'order-design');
        expect(ddq.calls).toEqual([['init', repo, 'docs']]);
        const [, writingFolderBox] = fake.shown('showInputBox') as { value?: string }[];
        expect(writingFolderBox.value).toBe('docs');
        expect(executed('vscode.openFolder')).toEqual([[Uri.file(repo), { forceReuseWindow: true }]]);
        // 開き直すと拡張も読み直されるので、完了の案内は開き直した先で出す
        expect(context.globalState.get(PENDING)).toEqual({ repo, writingFolder: 'docs' });
    });

    it.each([
        ['親フォルダを選ばない', [], []],
        ['リポジトリ名を入れない', [true], []],
        ['執筆フォルダ名を入れない', [true], ['order-design']]
    ])('%s と何も作らない', async (_what, dialog, names) => {
        const parent = tmp();
        if (dialog.length > 0) fake.answer('showOpenDialog', [Uri.file(parent)]);
        fake.answer('showInputBox', ...names);
        expect(await fake.run('ddqTemplate.init')).toBeUndefined();
        expect(ddq.calls).toEqual([]);
        expect(executed('vscode.openFolder')).toEqual([]);
    });

    it('開き直した先で _quarto.yml を開き、Git の初期化を勧める', async () => {
        const repo = tmp();
        write(path.join(repo, 'docs', '_quarto.yml'));
        const reopened = extensionContext();
        await reopened.globalState.update(PENDING, { repo, writingFolder: 'docs' });
        fake.setFolders(repo);
        fake.answer('showInformationMessage', 'Git リポジトリとして初期化');

        activateInit(reopened as never);

        await vi.waitFor(() => expect(executed('git.init')).toEqual([[true]]));
        expect(fake.shown('showTextDocument')).toEqual([Uri.file(path.join(repo, 'docs', '_quarto.yml'))]);
        expect(fake.shown('showInformationMessage')).toEqual([
            expect.stringContaining('設計書リポジトリを作りました（執筆フォルダ: docs）')
        ]);
        expect(reopened.globalState.get(PENDING)).toBeUndefined();
    });

    it('別のフォルダを開いたときは案内を出さず、記録も残す', async () => {
        const reopened = extensionContext();
        await reopened.globalState.update(PENDING, { repo: tmp(), writingFolder: 'docs' });
        fake.setFolders(tmp());
        activateInit(reopened as never);
        await new Promise((r) => setTimeout(r, 20));
        expect(fake.shown('showInformationMessage')).toEqual([]);
        expect(reopened.globalState.get(PENDING)).toBeDefined();
    });
});

describe('設計書リポジトリを作成（フォルダを開いている）', () => {
    it('空のフォルダにはそのまま作り、_quarto.yml を開いて Git の初期化を勧める', async () => {
        const here = tmp();
        fake.setFolders(here);
        fake.answer('showInputBox', 'docs');
        ddqCreatesQuartoYml();

        expect(await fake.run('ddqTemplate.init')).toBe('ok');

        expect(ddq.calls).toEqual([['init', here, 'docs']]);
        expect(fake.shown('showWarningMessage')).toEqual([]);
        expect(fake.shown('showTextDocument')).toEqual([Uri.file(path.join(here, 'docs', '_quarto.yml'))]);
        const created = fake.calls.find((c) => c.method === 'showInformationMessage');
        expect(created?.args).toEqual([expect.stringContaining('作りました'), 'Git リポジトリとして初期化']);
        // 押さなければ git.init はしない
        expect(executed('git.init')).toEqual([]);
    });

    it('既に Git リポジトリなら、初期化のボタンを出さない', async () => {
        const here = tmp();
        fs.mkdirSync(path.join(here, '.git'));
        fake.setFolders(here);
        fake.answer('showInputBox', 'docs');
        ddqCreatesQuartoYml();
        await fake.run('ddqTemplate.init');
        const created = fake.calls.find((c) => c.method === 'showInformationMessage');
        expect(created?.args).toEqual([expect.stringContaining('作りました')]);
    });

    it('ファイルのあるフォルダでは確かめてから作る（モーダル）', async () => {
        const here = tmp();
        write(path.join(here, 'notes.txt'));
        fake.setFolders(here);
        fake.answer('showWarningMessage', 'このフォルダに作る');
        fake.answer('showInputBox', 'docs');

        expect(await fake.run('ddqTemplate.init')).toBe('ok');

        const [warning] = fake.calls.filter((c) => c.method === 'showWarningMessage');
        expect(warning.args).toEqual([
            expect.stringContaining('既にファイルがあります'),
            { modal: true },
            'このフォルダに作る',
            '別の場所に新しく作る'
        ]);
        expect(ddq.calls).toEqual([['init', here, 'docs']]);
    });

    it('「別の場所に新しく作る」は親フォルダの選択へ進み、閉じれば何もしない', async () => {
        const here = tmp();
        write(path.join(here, 'notes.txt'));
        fake.setFolders(here);
        fake.answer('showWarningMessage', '別の場所に新しく作る');
        expect(await fake.run('ddqTemplate.init')).toBeUndefined();
        expect(fake.shown('showOpenDialog')).toHaveLength(1);

        fake.calls = [];
        expect(await fake.run('ddqTemplate.init')).toBeUndefined(); // 確かめの通知を閉じた
        expect(fake.shown('showOpenDialog')).toEqual([]);
        expect(ddq.calls).toEqual([]);
    });

    it('既に設計書リポジトリなら作らず、「執筆フォルダを追加」へ案内する', async () => {
        const here = tmp();
        writingFolder(path.join(here, 'docs'));
        fake.setFolders(here);
        fake.answer('showInformationMessage', '執筆フォルダを追加');

        expect(await fake.run('ddqTemplate.init')).toBe('exists');

        expect(executed('ddqTemplate.add')).toEqual([[Uri.file(here)]]);
        // 追加の入力欄が開く（初期値は docs2）。ここでは閉じる
        const [box] = fake.shown('showInputBox') as { value?: string }[];
        expect(box.value).toBe('docs2');
        expect(ddq.calls).toEqual([]);
    });

    it('複数ルートでは作る場所を先に選び、そのルートの中だけで既存の文書を見る', async () => {
        const withDoc = tmp();
        const empty = tmp();
        writingFolder(path.join(withDoc, 'docs'));
        fake.setFolders(withDoc, empty);
        fake.answer('showQuickPick', { label: 'empty', path: empty });
        fake.answer('showInputBox', 'docs');

        expect(await fake.run('ddqTemplate.init')).toBe('ok');
        expect(ddq.calls).toEqual([['init', empty, 'docs']]);
    });

    it('Quarto が無ければ最後の確認を省いて作り、その旨を出力に書く', async () => {
        const here = tmp();
        fake.setFolders(here);
        fake.answer('showInputBox', 'docs');
        ddq.quarto = false;
        expect(await fake.run('ddqTemplate.init')).toBe('ok');
        expect(ddq.calls).toEqual([['init', here, 'docs', '--no-render']]);
        expect(fake.output).toContain('Quarto が見つからない');
    });

    it('ddq init が失敗したら理由を出し、「出力を表示」で出力を開く', async () => {
        const here = tmp();
        fake.setFolders(here);
        fake.answer('showInputBox', 'docs');
        fake.answer('showErrorMessage', '出力を表示');
        ddq.results.push({ code: 2, error: 'ここには作れません' });

        expect(await fake.run('ddqTemplate.init')).toBe('failed');

        expect(fake.calls.find((c) => c.method === 'showErrorMessage')?.args).toEqual(['ここには作れません', '出力を表示']);
        await vi.waitFor(() => expect(fake.outputShown).toBe(1));
        expect(fake.shown('showTextDocument')).toEqual([]);
    });
});

describe('執筆フォルダを追加', () => {
    it('既にある名前は入力欄で通さず、足したら _quarto.yml を開いて案内する', async () => {
        const root = tmp();
        writingFolder(path.join(root, 'docs'));
        fs.mkdirSync(path.join(root, 'docs2'));
        write(path.join(root, 'docs2', '_quarto.yml'));
        fake.answer('showInputBox', 'docs2', 'docs3');
        ddqCreatesQuartoYml();

        expect(await fake.run('ddqTemplate.add', Uri.file(root))).toBe('ok');

        // 初期値は docs2, docs3 … のうちまだ無い最初の名前
        const [box] = fake.shown('showInputBox') as { value?: string }[];
        expect(box.value).toBe('docs3');
        expect(fake.rejected).toEqual([expect.stringMatching(/既に文書/)]);
        expect(ddq.calls).toEqual([['add', path.join(root, 'docs3')]]);
        expect(fake.shown('showTextDocument')).toEqual([Uri.file(path.join(root, 'docs3', '_quarto.yml'))]);
        await vi.waitFor(() =>
            expect(fake.shown('showInformationMessage')).toEqual([expect.stringContaining('docs3 を追加しました')])
        );
    });

    it('設計書リポジトリでなければ足さず、「設計書リポジトリを作成」へ案内する', async () => {
        const root = tmp();
        fake.setFolders(root);
        fake.answer('showInformationMessage', '設計書リポジトリを作成');

        expect(await fake.run('ddqTemplate.add', Uri.file(root))).toBe('notRepository');

        expect(executed('ddqTemplate.init')).toEqual([[]]);
        expect(ddq.calls).toEqual([]);
    });

    it('フォルダを開いていなければ案内だけ出す', async () => {
        expect(await fake.run('ddqTemplate.add')).toBeUndefined();
        expect(fake.shown('showInformationMessage')).toEqual([expect.stringContaining('フォルダを開いてから')]);
    });

    it('複数ルートでは足す先のリポジトリを選ぶ（選ばなければ何もしない）', async () => {
        const a = tmp();
        const b = tmp();
        writingFolder(path.join(b, 'docs'));
        fake.setFolders(a, b);
        fake.answer('showQuickPick', { label: 'b', path: b });
        fake.answer('showInputBox', 'docs2');
        expect(await fake.run('ddqTemplate.add')).toBe('ok');
        expect(ddq.calls).toEqual([['add', path.join(b, 'docs2')]]);

        ddq.calls = [];
        expect(await fake.run('ddqTemplate.add')).toBeUndefined();
        expect(ddq.calls).toEqual([]);
    });
});

describe('発行（PDF / HTML / 図）', () => {
    let docs: string;
    beforeEach(() => {
        docs = writingFolder(path.join(tmp(), 'docs'));
    });

    it('章のフォルダの右クリックからでも執筆フォルダの PDF を作り、「PDF を開く」で開く', async () => {
        fs.mkdirSync(path.join(docs, 'chapters'));
        fake.answer('showInformationMessage', 'PDF を開く');

        expect(await fake.run('ddqTemplate.pdf', Uri.file(path.join(docs, 'chapters')))).toBe('ok');

        expect(ddq.calls).toEqual([['pdf', docs]]);
        await vi.waitFor(() =>
            expect(openWithDefaultApp).toHaveBeenCalledWith(path.join(docs, 'design-doc.pdf'))
        );
        expect(fake.calls.find((c) => c.method === 'showInformationMessage')?.args).toEqual([
            'docs の PDF を作りました。',
            'PDF を開く',
            '出力を表示'
        ]);
    });

    it('HTML は ddq の出力の index.html を開く（見つからなければ開くボタンを出さない）', async () => {
        const index = path.join(docs, '_book', 'index.html');
        ddq.results.push({ code: 0, output: `OK: ${index}（3 ファイル）\n` }, { code: 0, output: '' });
        fake.answer('showInformationMessage', 'HTML を開く');

        expect(await fake.run('ddqTemplate.html', Uri.file(docs))).toBe('ok');
        await vi.waitFor(() => expect(openWithDefaultApp).toHaveBeenCalledWith(index));

        fake.calls = [];
        expect(await fake.run('ddqTemplate.html', Uri.file(docs))).toBe('ok');
        await vi.waitFor(() =>
            expect(fake.calls.find((c) => c.method === 'showInformationMessage')?.args).toEqual([
                'docs の HTML を作りました。',
                '出力を表示'
            ])
        );
    });

    it('図の変換で警告が出たら、終わったとだけ言わずに警告を見せる', async () => {
        ddq.results.push({ code: 0, output: '警告: 変換する図がありません（diagrams/ が空）\n' });
        fake.answer('showWarningMessage', '出力を表示');

        expect(await fake.run('ddqTemplate.diagrams', Uri.file(docs))).toBe('ok');

        await vi.waitFor(() => expect(fake.outputShown).toBe(1));
        expect(fake.shown('showWarningMessage')).toEqual(['docs: 変換する図がありません（diagrams/ が空）']);
        expect(fake.shown('showInformationMessage')).toEqual([]);
    });

    it('Quarto が無ければ ddq を走らせず、設定を開ける', async () => {
        ddq.quarto = false;
        fake.answer('showErrorMessage', '設定でパスを指定');

        expect(await fake.run('ddqTemplate.pdf', Uri.file(docs))).toBe('no-quarto');

        expect(ddq.calls).toEqual([]);
        await vi.waitFor(() =>
            expect(executed('workbench.action.openSettings')).toEqual([['ddqTemplate.quartoPath']])
        );
        // 図の変換は Quarto を使わないので走る
        expect(await fake.run('ddqTemplate.diagrams', Uri.file(docs))).toBe('ok');
    });

    it('機構が古ければ update を勧め、押せば update してから元のコマンドをやり直す', async () => {
        fs.writeFileSync(path.join(docs, '.template-version'), '2.0.0\n');
        ddq.results.push({ code: 3 }, { code: 0 }, { code: 0 });
        fake.answer('showWarningMessage', 'ddq update を実行');

        expect(await fake.run('ddqTemplate.pdf', Uri.file(docs))).toBe('ok');

        expect(ddq.calls).toEqual([
            ['pdf', docs],
            ['update', docs],
            ['pdf', docs]
        ]);
        expect(fake.calls.find((c) => c.method === 'showWarningMessage')?.args).toEqual([
            expect.stringContaining('テンプレートは 2.0.0、ddq は 2.4.6'),
            'ddq update を実行',
            '出力を表示'
        ]);
    });

    it('update を押さなければ何もしない', async () => {
        fs.writeFileSync(path.join(docs, '.template-version'), '2.0.0\n');
        ddq.results.push({ code: 3 });
        expect(await fake.run('ddqTemplate.html', Uri.file(docs))).toBe('mismatch');
        expect(ddq.calls).toEqual([['html', docs]]);
    });

    it('同じ版で一致しないときは、手で直された可能性を言う', async () => {
        ddq.results.push({ code: 3 });
        expect(await fake.run('ddqTemplate.pdf', Uri.file(docs))).toBe('mismatch');
        expect(fake.shown('showWarningMessage')).toEqual([expect.stringContaining('手で直された可能性があります')]);
    });

    it('文書の方が新しければ update を出さない（機構を古い版に戻さない）', async () => {
        fs.writeFileSync(path.join(docs, '.template-version'), '9.0.0\n');
        ddq.results.push({ code: 3 });

        expect(await fake.run('ddqTemplate.pdf', Uri.file(docs))).toBe('mismatch');

        const warning = fake.calls.find((c) => c.method === 'showWarningMessage');
        expect(warning?.args).toEqual([expect.stringContaining('新しい版のテンプレート（9.0.0）'), '出力を表示']);
        expect(ddq.calls).toEqual([['pdf', docs]]);
    });

    it('update が失敗したら元のコマンドをやり直さない', async () => {
        fs.writeFileSync(path.join(docs, '.template-version'), '2.0.0\n');
        ddq.results.push({ code: 3 }, { code: 1 });
        fake.answer('showWarningMessage', 'ddq update を実行');

        expect(await fake.run('ddqTemplate.pdf', Uri.file(docs))).toBe('failed');

        expect(ddq.calls).toEqual([
            ['pdf', docs],
            ['update', docs]
        ]);
        expect(fake.shown('showErrorMessage')).toEqual(['ddq update が失敗しました（failed）。']);
    });

    it('失敗したら ddq の理由（無ければ終了コード）を出す', async () => {
        ddq.results.push({ code: 1, error: 'typst がエラーを返しました' }, { code: 4 });
        expect(await fake.run('ddqTemplate.pdf', Uri.file(docs))).toBe('failed');
        expect(await fake.run('ddqTemplate.html', Uri.file(docs))).toBe('failed');
        expect(fake.shown('showErrorMessage')).toEqual([
            'typst がエラーを返しました',
            'ddq html が失敗しました（終了コード 4）。'
        ]);
    });

    it('進捗通知のキャンセルで ddq を止め、完了の通知を出さない', async () => {
        ddq.onRun = () => fake.cancel?.();
        expect(await fake.run('ddqTemplate.pdf', Uri.file(docs))).toBe('cancelled');
        expect(ddq.kill).toHaveBeenCalledTimes(1);
        const progress = fake.calls.find((c) => c.method === 'withProgress');
        expect(progress?.args).toEqual([expect.objectContaining({ title: 'PDF を作成: docs', cancellable: true })]);
        await new Promise((r) => setTimeout(r, 20));
        expect(fake.shown('showInformationMessage')).toEqual([]);
    });

    it('同じ執筆フォルダで実行中なら受け付けない', async () => {
        const release = tryLock(docs);
        try {
            expect(await fake.run('ddqTemplate.pdf', Uri.file(docs))).toBe('busy');
            expect(fake.shown('showInformationMessage')).toEqual([expect.stringContaining('docs では ddq を実行中です')]);
            expect(ddq.calls).toEqual([]);
        } finally {
            release?.();
        }
    });

    it('執筆フォルダが決まらなければ選ばせる（無ければ理由を出す）', async () => {
        const other = writingFolder(path.join(tmp(), 'docs-b'));
        fake.answer('showQuickPick', other);
        expect(await fake.run('ddqTemplate.html')).toBe('ok');
        expect(fake.calls.find((c) => c.method === 'showQuickPick')?.args[0]).toEqual([docs, other].sort());
        expect(ddq.calls).toEqual([['html', other]]);

        fake.found = [];
        expect(await fake.run('ddqTemplate.html')).toBe('no-folder');
        expect(fake.shown('showErrorMessage')).toEqual([expect.stringContaining('執筆フォルダ')]);
    });
});
