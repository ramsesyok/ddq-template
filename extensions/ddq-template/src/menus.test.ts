import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

/**
 * 右クリックメニューの `when` 句（package.json の contributes.menus）と、拡張が setContext で渡す値
 * （folders.ts の watchWritingFolders）の突き合わせ。
 *
 * VSCode は `when` を自分で評価し、拡張から結果を問い合わせる API が無いので、実拡張ホストでも
 * 見られない。ここでは package.json の `when` を、使っている範囲の文法（&& || ! ( ) == != in）で
 * 評価し、執筆フォルダ・その配下・生成物の中・ほかのフォルダ・ルートで、どのコマンドが出るかを見る。
 * キーの名前を片方だけ変えた、配下のフォルダを載せ忘れた、といった食い違いをここで拾う。
 * 知らない書き方が `when` に入ったら評価せずに失敗する（黙って素通りしないように）。
 * VSCode の `resourcePath` はエクスプローラの Uri の fsPath で、拡張は同じく Uri.file(…).fsPath を
 * キーにしている（ドライブ文字の大小を揃えるため）。差し替えの Uri はパスをそのまま返す。
 */

vi.mock('vscode', () => import('./testing/fakeVscode'));

import packageJson from '../package.json';
import { watchWritingFolders } from './folders';
import { extensionContext, fake } from './testing/fakeVscode';

type Context = Record<string, unknown>;

/** `when` 句を評価する（VSCode の文法のうち、この拡張が使う範囲だけ） */
function evaluateWhen(expression: string, context: Context): boolean {
    const tokens = expression.match(/\|\||&&|==|!=|!|\(|\)|[^\s()!=&|]+/g) ?? [];
    let i = 0;
    const peek = () => tokens[i];
    const take = (want?: string) => {
        const t = tokens[i++];
        if (t === undefined || (want !== undefined && t !== want)) {
            throw new Error(`when を読めません（${want ?? '値'} が要る）: ${expression}`);
        }
        return t;
    };
    const or = (): boolean => {
        let v = and();
        while (peek() === '||') {
            take();
            const r = and();
            v = v || r;
        }
        return v;
    };
    const and = (): boolean => {
        let v = unary();
        while (peek() === '&&') {
            take();
            const r = unary();
            v = v && r;
        }
        return v;
    };
    const unary = (): boolean => {
        if (peek() === '!') {
            take();
            return !unary();
        }
        if (peek() === '(') {
            take('(');
            const v = or();
            take(')');
            return v;
        }
        const key = take();
        if (!/^[A-Za-z][\w.]*$/.test(key)) throw new Error(`when のキーではありません: ${key}（${expression}）`);
        const op = peek();
        if (op === '==' || op === '!=') {
            take();
            const equal = String(context[key] ?? '') === take();
            return op === '==' ? equal : !equal;
        }
        if (op === 'in') {
            take();
            const container = context[take()];
            const value = String(context[key] ?? '');
            if (Array.isArray(container)) return container.includes(value);
            return typeof container === 'object' && container !== null && value in container;
        }
        return Boolean(context[key]);
    };
    const result = or();
    if (i !== tokens.length) throw new Error(`when を読み切れません: ${expression}`);
    return result;
}

type Menu = { command: string; when: string };
const menus = packageJson.contributes.menus as Record<string, Menu[]>;

/** そのメニューに出るコマンド */
function visible(menu: string, context: Context): string[] {
    return menus[menu].filter((m) => evaluateWhen(m.when, context)).map((m) => m.command);
}

const temps: string[] = [];
let root: string;

function mkdir(rel: string) {
    fs.mkdirSync(path.join(root, rel), { recursive: true });
}

/** watchWritingFolders が setContext に渡した値（起動時の 1 回目） */
async function contextKeys(): Promise<Context> {
    watchWritingFolders(extensionContext() as never, () => undefined);
    await vi.waitFor(() => expect(fake.context).toHaveProperty('ddqTemplate.hasWritingFolders'));
    return fake.context;
}

function explorer(keys: Context, rel: string, folder = true): Context {
    return {
        ...keys,
        explorerResourceIsFolder: folder,
        explorerResourceIsRoot: rel === '',
        resourcePath: path.join(root, rel)
    };
}

const PER_FOLDER = [
    'ddqTemplate.pdf',
    'ddqTemplate.html',
    'ddqTemplate.diagrams',
    'ddqTemplate.tags',
    'ddqTemplate.newRevision'
];

beforeEach(() => {
    fake.reset();
    root = fs.mkdtempSync(path.join(os.tmpdir(), 'ddq-menus-'));
    temps.push(root);
});

afterEach(() => {
    for (const dir of temps.splice(0)) fs.rmSync(dir, { recursive: true, force: true });
});

describe('when の評価', () => {
    it('使っている文法を VSCode と同じ意味で読む', () => {
        const c = { a: true, b: false, ext: '.qmd', set: { x: true }, list: ['y'] };
        expect(evaluateWhen('a && !b', c)).toBe(true);
        expect(evaluateWhen('b || (a && ext == .qmd)', c)).toBe(true);
        expect(evaluateWhen('ext != .qmd', c)).toBe(false);
        expect(evaluateWhen('name in set', { ...c, name: 'x' })).toBe(true);
        expect(evaluateWhen('name in list', { ...c, name: 'x' })).toBe(false);
        expect(evaluateWhen('missing', c)).toBe(false);
    });

    it('知らない書き方は評価しない', () => {
        expect(() => evaluateWhen('a =~ /x/', {})).toThrow();
        expect(() => evaluateWhen('(a && b', {})).toThrow();
    });
});

describe('エクスプローラの右クリック', () => {
    it('執筆フォルダとその配下にだけ発行・ラベル・改訂を出し、ルートにだけ「執筆フォルダを追加」を出す', async () => {
        for (const rel of ['docs/chapters/01-overview', 'docs/_book/chapters', 'docs/.quarto', 'notes']) mkdir(rel);
        fs.writeFileSync(path.join(root, 'docs', '_quarto.yml'), '');
        fs.writeFileSync(path.join(root, 'docs', '.template-version'), '2.4.6\n');
        fs.writeFileSync(path.join(root, 'docs', 'index.qmd'), '');
        fake.found = [path.join(root, 'docs', '.template-version')];
        const keys = await contextKeys();

        const at = (rel: string, folder = true) => visible('explorer/context', explorer(keys, rel, folder));
        expect(at('')).toEqual(['ddqTemplate.add']);
        expect(at('docs')).toEqual(PER_FOLDER);
        expect(at('docs/chapters')).toEqual(PER_FOLDER);
        expect(at('docs/chapters/01-overview')).toEqual(PER_FOLDER);
        // 生成物の中・ほかのフォルダ・ファイルには出さない
        expect(at('docs/_book')).toEqual([]);
        expect(at('docs/_book/chapters')).toEqual([]);
        expect(at('docs/.quarto')).toEqual([]);
        expect(at('notes')).toEqual([]);
        expect(at('docs/index.qmd', false)).toEqual([]);
    });

    it('設計書リポジトリでなければ、ルートにも何も出さない', async () => {
        mkdir('notes');
        const keys = await contextKeys();
        expect(visible('explorer/context', explorer(keys, ''))).toEqual([]);
        expect(visible('explorer/context', explorer(keys, 'notes'))).toEqual([]);
    });

    it('_quarto.yml の無い版ファイルだけのフォルダは執筆フォルダとみなさない', async () => {
        mkdir('half');
        fs.writeFileSync(path.join(root, 'half', '.template-version'), '2.4.6\n');
        fake.found = [path.join(root, 'half', '.template-version')];
        const keys = await contextKeys();
        expect(visible('explorer/context', explorer(keys, 'half'))).toEqual([]);
        expect(visible('explorer/context', explorer(keys, ''))).toEqual([]);
    });
});

describe('エディタの右クリック', () => {
    it('表エディタは .qmd と .md を編集中にだけ出す', () => {
        const at = (ext: string, focus = true) =>
            visible('editor/context', { editorTextFocus: focus, resourceExtname: ext });
        expect(at('.qmd')).toEqual(['ddqTemplate.openTableEditor']);
        expect(at('.md')).toEqual(['ddqTemplate.openTableEditor']);
        expect(at('.yml')).toEqual([]);
        expect(at('.qmd', false)).toEqual([]);
    });
});

describe('メニューのコマンド', () => {
    it('どれも contributes.commands に宣言してある（宣言が無いと名前の無い項目になる）', () => {
        const declared = new Set(packageJson.contributes.commands.map((c: { command: string }) => c.command));
        const used = Object.values(menus).flatMap((list) => list.map((m) => m.command));
        expect(used.filter((c) => !declared.has(c))).toEqual([]);
    });
});
