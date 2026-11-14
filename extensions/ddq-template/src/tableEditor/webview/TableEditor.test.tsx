/**
 * @vitest-environment jsdom
 */
/**
 * 表エディタの Webview の描画と操作（jsdom）。
 *
 * モデルの操作（結合・行列・選択・貼り付け）はそれぞれの単体テストで見ている。ここでは
 * 画面がそれを正しく呼び、結果を表・ツールバー・属性欄・プレビューに出し、Apply で
 * 拡張へ何を渡すかを見る。「パネルが真っ白」「ボタンが効かない」は実拡張ホストの
 * 検証でもパネルが開くだけで気付けないため。
 *
 * jsdom は 26 に固定してある（理由は src/revisionWebview/TagTable.test.tsx の冒頭）。
 */
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { parsePipeTable } from '../formats/pipeTable/parsePipeTable';
import type { TableModel } from '../model/TableModel';
import type { EditorContext, FromWebviewMessage } from '../model/WebviewMessages';
import { TableEditor } from './TableEditor';

const posted: FromWebviewMessage[] = [];

vi.mock('./vscodeApi', () => ({
  vscode: () => ({
    postMessage: (m: FromWebviewMessage) => posted.push(m),
    getState: () => undefined,
    setState: () => undefined
  })
}));

const SOURCE = [
  '| 項目 | 型     | 説明     |',
  '|------|--------|----------|',
  '| 氏名 | 文字列 | 利用者名 |',
  '| 年齢 | 数値   | 満年齢   |'
].join('\n');

function tableModel(): TableModel {
  const parsed = parsePipeTable(SOURCE, 't1');
  if (!parsed.ok) throw new Error(parsed.message);
  return {
    ...parsed.value,
    attributes: { ...parsed.value.attributes, caption: '利用者の属性', label: 'tbl-user' }
  };
}

const CONTEXT: EditorContext = {
  kind: 'tblPart',
  partCount: 1,
  partIndex: 0,
  attributesEditable: true,
  enclosingClasses: [],
  warnings: []
};

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  posted.length = 0;
  // jsdom は scrollIntoView を持たない（アクティブセルを見える位置へ動かすのに使う）
  Element.prototype.scrollIntoView = () => undefined;
  container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

// --- 操作の道具 ---

function send(data: unknown) {
  act(() => {
    window.dispatchEvent(new MessageEvent('message', { data }));
  });
}

/** 描画して、拡張から表を渡す。 */
function render(context: Partial<EditorContext> = {}, model: TableModel = tableModel()) {
  act(() => root.render(<TableEditor />));
  send({ type: 'init', model, context: { ...CONTEXT, ...context } });
}

const q = <T extends Element = HTMLElement>(sel: string) => container.querySelector<T>(sel);
const qa = <T extends Element = HTMLElement>(sel: string) => [...container.querySelectorAll<T>(sel)];
const td = (row: number, col: number) => q<HTMLTableCellElement>(`td[data-row="${row}"][data-col="${col}"]`)!;
const editor = () => q<HTMLTextAreaElement>('textarea.cell-editor')!;
const status = () => q('.status')!.textContent;
const message = () => q('.message')?.textContent;
/** 表の本文（行ごとのセルの文字。結合で隠れたセルは出ない）。 */
const cells = () => qa('tbody tr').map(tr => [...tr.querySelectorAll('td')].map(c => c.textContent));

function button(label: string, scope: ParentNode = container): HTMLButtonElement {
  // メニューの項目は「名前」と「ショートカット」の 2 つの span を持つので、先頭の span でも照合する
  const b = [...scope.querySelectorAll<HTMLButtonElement>('button')].find(
    x => x.textContent?.trim() === label || x.firstElementChild?.textContent === label
  );
  if (!b) throw new Error(`ボタン「${label}」がありません`);
  return b;
}

function click(el: Element) {
  act(() => {
    el.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  });
}

function mouseDown(el: Element, init: MouseEventInit = {}) {
  act(() => {
    el.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0, buttons: 1, ...init }));
  });
  act(() => {
    window.dispatchEvent(new MouseEvent('mouseup'));
  });
}

function key(el: EventTarget, k: string, init: KeyboardEventInit = {}) {
  act(() => {
    el.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...init }));
  });
}

/** React の制御された入力欄に値を入れる（value の setter を通さないと onChange が来ない）。 */
function typeInto(el: HTMLInputElement | HTMLTextAreaElement, value: string) {
  const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
  act(() => {
    Object.getOwnPropertyDescriptor(proto, 'value')!.set!.call(el, value);
    el.dispatchEvent(new Event('input', { bubbles: true }));
  });
}

function choose(el: HTMLSelectElement, value: string) {
  act(() => {
    Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value')!.set!.call(el, value);
    el.dispatchEvent(new Event('change', { bubbles: true }));
  });
}

function check(el: HTMLInputElement) {
  act(() => {
    el.click();
  });
}

/** クリップボードの事件。jsdom の ClipboardEvent は clipboardData を持たないので足す。 */
function clipboard(type: 'copy' | 'paste', text = '') {
  const data = new Map<string, string>([['text/plain', text]]);
  const event = new Event(type, { bubbles: true, cancelable: true });
  Object.defineProperty(event, 'clipboardData', {
    value: { getData: (t: string) => data.get(t) ?? '', setData: (t: string, v: string) => data.set(t, v) }
  });
  act(() => {
    window.dispatchEvent(event);
  });
  return data.get('text/plain');
}

/** Apply を押して、拡張へ渡ったモデルを返す。 */
function apply(): TableModel {
  click(button('Apply'));
  const last = posted.at(-1);
  if (last?.type !== 'apply') throw new Error(`apply ではない: ${JSON.stringify(last)}`);
  return last.model;
}

const texts = (m: TableModel) => m.rows.map(r => r.filter(c => !c.hidden).map(c => c.text));

// --- テスト ---

describe('TableEditor の読み込み', () => {
  it('描画したらすぐ表を要求し、届くまでは読み込み中を出す', () => {
    act(() => root.render(<TableEditor />));
    expect(posted).toEqual([{ type: 'ready' }]);
    expect(container.textContent).toContain('読み込み中');
    expect(q('table')).toBeNull();
  });

  it('届いた表を組み、ヘッダ行・件数・出力形式を出す', () => {
    render();
    expect(cells()).toEqual([
      ['項目', '型', '説明'],
      ['氏名', '文字列', '利用者名'],
      ['年齢', '数値', '満年齢']
    ]);
    expect(qa('tbody tr')[0].className).toBe('header-row');
    expect(td(0, 0).className).toContain('is-header');
    expect(status()).toContain('3 行 × 3 列 ／ ヘッダ 1 行 ／ 結合なし');
    expect(container.textContent).toContain('結合なし → 通常のパイプ表として出力します');
    // 列ごとの幅と揃えの欄
    expect(qa('input.width-input')).toHaveLength(3);
    expect(qa('select.align-select')).toHaveLength(3);
    // 属性欄に caption と label が入る
    expect(q<HTMLInputElement>('.attributes input[placeholder="付けると採番されます"]')!.value).toBe('利用者の属性');
    expect(q<HTMLInputElement>('.attributes input[placeholder="tbl-xxx"]')!.value).toBe('tbl-user');
    // 何もしていないので元に戻せない
    expect(button('元に戻す').disabled).toBe(true);
    expect(button('やり直す').disabled).toBe(true);
  });

  it('拡張からの警告・反映・エラーを知らせる', () => {
    render({ warnings: ['列数がパートごとに違います'] });
    expect(message()).toBe('列数がパートごとに違います');
    send({ type: 'applied' });
    expect(message()).toBe('ドキュメントへ反映しました。');
    send({ type: 'error', message: '表の範囲が見つかりません' });
    expect(message()).toBe('表の範囲が見つかりません');
  });
});

describe('セルの選択と編集', () => {
  it('セルを選んで文字を打ち、Enter で確定すると表と Apply の中身が変わる', () => {
    render();
    mouseDown(td(1, 0));
    expect(td(1, 0).className).toContain('active');

    // 選んだだけでは編集にならない。文字を打つと置き換えで編集が始まる（Excel と同じ）
    expect(editor().dataset.editing).toBe('false');
    typeInto(editor(), '名前');
    expect(editor().dataset.editing).toBe('true');
    // 確定するまで表には書かない
    expect(td(1, 0).textContent).toBe('氏名');

    key(editor(), 'Enter');
    expect(td(1, 0).textContent).toBe('名前');
    // 確定すると 1 つ下へ移る
    expect(td(2, 0).className).toContain('active');
    expect(editor().dataset.editing).toBe('false');

    expect(texts(apply())[1]).toEqual(['名前', '文字列', '利用者名']);
  });

  it('Escape で編集を取り消す。Enter / F2 / ダブルクリックは今の文字から編集する', () => {
    render();
    mouseDown(td(1, 1));
    typeInto(editor(), '書きかけ');
    key(editor(), 'Escape');
    expect(td(1, 1).textContent).toBe('文字列');
    expect(editor().dataset.editing).toBe('false');

    key(editor(), 'F2');
    expect(editor().dataset.editing).toBe('true');
    expect(editor().value).toBe('文字列');
    key(editor(), 'Escape');

    act(() => {
      td(2, 2).dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    });
    expect(editor().value).toBe('満年齢');
    typeInto(editor(), '満年齢（歳）');
    // Tab で確定して右へ（行末なら次の行の先頭へ）
    key(editor(), 'Tab');
    expect(td(2, 2).textContent).toBe('満年齢（歳）');
  });

  it('Alt+Enter でセル内に改行を入れられ、改行のある表はグリッド表で出すと案内する', () => {
    render();
    mouseDown(td(1, 2));
    key(editor(), 'Enter');
    key(editor(), 'Enter', { altKey: true });
    expect(editor().value).toBe('利用者名\n');
    typeInto(editor(), '利用者名\n（姓と名）');
    key(editor(), 'Enter');
    expect(qa('td[data-row="1"][data-col="2"] .cell-line').map(s => s.textContent)).toEqual([
      '利用者名',
      '（姓と名）'
    ]);
    expect(container.textContent).toContain('セル内改行あり → グリッド表として出力します');
  });

  it('矢印・Tab・Home/End・Escape・Ctrl+A で選択が動く', () => {
    render();
    mouseDown(td(1, 0));
    key(editor(), 'ArrowRight');
    expect(td(1, 1).className).toContain('active');
    key(editor(), 'ArrowDown');
    expect(td(2, 1).className).toContain('active');
    key(editor(), 'End');
    expect(td(2, 2).className).toContain('active');
    key(editor(), 'Home', { ctrlKey: true });
    expect(td(0, 0).className).toContain('active');
    key(editor(), 'Tab');
    expect(td(0, 1).className).toContain('active');

    // Shift+矢印で範囲を伸ばす
    key(editor(), 'ArrowDown', { shiftKey: true });
    expect(qa('td.selected').map(c => c.textContent)).toEqual(['型', '文字列']);

    key(editor(), 'a', { ctrlKey: true });
    expect(qa('td.selected')).toHaveLength(9);

    key(editor(), 'Escape');
    expect(qa('td.selected')).toHaveLength(0);
  });

  it('Shift+クリックで範囲を選び、Delete で内容を消す', () => {
    render();
    mouseDown(td(1, 1));
    mouseDown(td(2, 2), { shiftKey: true });
    expect(qa('td.selected')).toHaveLength(4);
    key(editor(), 'Delete');
    expect(cells().slice(1)).toEqual([
      ['氏名', '', ''],
      ['年齢', '', '']
    ]);
    // 元に戻す（ツールバー）
    click(button('元に戻す'));
    expect(cells()[1]).toEqual(['氏名', '文字列', '利用者名']);
    expect(button('やり直す').disabled).toBe(false);
  });
});

describe('ツールバー', () => {
  it('行と列の追加・削除', () => {
    render();
    mouseDown(td(1, 0));
    const groups = qa('.toolbar-group');
    const rowGroup = groups.find(g => g.textContent?.startsWith('行'))!;
    const colGroup = groups.find(g => g.textContent?.startsWith('列'))!;

    click(button('下に追加', rowGroup));
    expect(status()).toContain('4 行 × 3 列');
    expect(cells()[2]).toEqual(['', '', '']);

    click(button('上に追加', rowGroup));
    expect(status()).toContain('5 行 × 3 列');
    // 選んでいたセルも 1 行下へ付いていく
    expect(td(2, 0).textContent).toBe('氏名');
    expect(td(2, 0).className).toContain('active');

    click(button('削除', rowGroup));
    expect(status()).toContain('4 行 × 3 列');
    expect(cells()[1]).toEqual(['', '', '']);

    click(button('右に追加', colGroup));
    expect(status()).toContain('4 行 × 4 列');
    expect(qa('input.width-input')).toHaveLength(4);
    click(button('削除', colGroup));
    expect(status()).toContain('4 行 × 3 列');
  });

  it('範囲を結合すると結合ありになり、出力形式を選べる。解除で元に戻る', () => {
    render();
    mouseDown(td(1, 0));
    mouseDown(td(2, 0), { shiftKey: true });
    click(button('結合'));

    expect(td(1, 0).rowSpan).toBe(2);
    expect(td(1, 0).className).toContain('is-merged');
    expect(q('td[data-row="2"][data-col="0"]')).toBeNull();
    expect(status()).toContain('結合あり');

    const format = q<HTMLSelectElement>('.toolbar select')!;
    expect(format.value).toBe('gridTable');
    // 縦結合だけなので merge-cols も選べる
    expect(format.querySelector<HTMLOptionElement>('option[value="mergeCols"]')!.disabled).toBe(false);
    choose(format, 'mergeCols');
    expect(apply().outputFormat).toBe('mergeCols');

    mouseDown(td(1, 0));
    click(button('結合を解除'));
    expect(status()).toContain('結合なし');
    expect(q('td[data-row="2"][data-col="0"]')).not.toBeNull();
  });

  it('結合できないときは理由を出す', () => {
    render();
    click(button('結合'));
    expect(message()).toBe('結合する範囲を選択してください。');
    mouseDown(td(1, 1));
    click(button('結合を解除'));
    expect(message()).not.toBeUndefined();
    expect(status()).toContain('結合なし');
  });

  it('ヘッダ行数を変えられる', () => {
    render();
    typeInto(q<HTMLInputElement>('.toolbar input[type="number"]')!, '2');
    expect(status()).toContain('ヘッダ 2 行');
    expect(td(1, 0).className).toContain('is-header');
    expect(apply().headerRows).toBe(2);
  });

  it('プレビューに書き戻す Markdown を出し、分割表なら何番目のパートかを示す', () => {
    render();
    click(button('プレビュー'));
    const pre = q('.preview pre')!.textContent!;
    expect(pre).toContain('caption="利用者の属性"');
    expect(pre).toContain('label="tbl-user"');
    expect(pre).toContain('| 氏名');
    click(button('プレビューを閉じる'));
    expect(q('.preview')).toBeNull();

    act(() => root.unmount());
    root = createRoot(container);
    render({ partCount: 3, partIndex: 1, attributesEditable: false });
    expect(q('.badge')!.textContent).toBe('分割表 2／3');
    click(button('プレビュー'));
    expect(q('.preview-title')!.textContent).toContain('分割表の 2 番目のパートのみ');
    // パートだけを出す（div の囲みは書かない）
    expect(q('.preview pre')!.textContent).not.toContain(':::');
  });
});

describe('属性欄', () => {
  it('caption・label・.unnumbered の変更が Apply に入り、label の誤りを示す', () => {
    render();
    const caption = q<HTMLInputElement>('.attributes input[placeholder="付けると採番されます"]')!;
    const label = q<HTMLInputElement>('.attributes input[placeholder="tbl-xxx"]')!;

    typeInto(caption, '利用者');
    typeInto(label, '#user');
    // 先頭の # は取り除く。tbl- で始まらないので誤り
    expect(label.value).toBe('user');
    expect(label.className).toBe('invalid');
    expect(container.textContent).toContain('label は tbl- で始まる必要があります。');

    typeInto(label, 'tbl-user2');
    expect(container.textContent).not.toContain('label は tbl- で始まる必要があります。');

    let m = apply();
    expect(m.attributes.caption).toBe('利用者');
    expect(m.attributes.label).toBe('tbl-user2');

    // .unnumbered にすると label は入力できない
    check(q<HTMLInputElement>('.attributes input[type="checkbox"]')!);
    expect(label.disabled).toBe(true);
    m = apply();
    expect(m.attributes.unnumbered).toBe(true);
  });

  it('分割表では属性を編集できず、外側の div を案内する', () => {
    render({ attributesEditable: false, partCount: 2, enclosingClasses: ['landscape'] });
    for (const input of qa<HTMLInputElement>('.attributes input')) {
      expect(input.disabled).toBe(true);
    }
    expect(container.textContent).toContain('分割表のため div 属性は編集できません');
    expect(container.textContent).toContain('外側の div: .landscape（そのまま保持します）');
  });
});

describe('列幅と揃え', () => {
  it('列幅を入れると残りの列幅を案内し、合計が超えると誤りを示す', () => {
    render();
    const widths = qa<HTMLInputElement>('input.width-input');
    typeInto(widths[0], '20');
    typeInto(widths[1], '30');
    // 空欄の 3 列目には残りの 50 が入る
    expect(container.textContent).toContain('空欄の 3 列目には、適用・プレビュー時に残りの 50 が入ります。');
    expect(widths[2].placeholder).toBe('50');

    typeInto(widths[2], '60');
    expect(container.textContent).toContain('列幅の合計が 110 です。');
    expect(widths.every(w => w.className.includes('invalid'))).toBe(true);

    typeInto(widths[2], '');
    expect(apply().columns.map(c => c.width)).toEqual([20, 30, undefined]);
  });

  it('列の揃えを変えるとセルの表示と Apply に入る', () => {
    render();
    choose(qa<HTMLSelectElement>('select.align-select')[1], 'right');
    expect(td(1, 1).style.textAlign).toBe('right');
    expect(apply().columns[1].align).toBe('right');
  });
});

describe('右クリックのメニュー', () => {
  it('右クリックしたセルを選んでメニューを出し、項目で操作する', () => {
    render();
    act(() => {
      td(2, 1).dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 2 }));
      td(2, 1).dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 30, clientY: 40 }));
    });
    const menu = q('[role="menu"]')!;
    expect(menu).not.toBeNull();
    expect(td(2, 1).className).toContain('active');
    // 1 セルだけでは結合できない
    expect(button('セルを結合', menu).disabled).toBe(true);
    expect(button('元に戻す', menu).disabled).toBe(true);

    click(button('上に行を挿入', menu));
    expect(q('[role="menu"]')).toBeNull();
    expect(status()).toContain('4 行 × 3 列');
    expect(cells()[2]).toEqual(['', '', '']);
  });

  it('Escape とメニューの外の押下で閉じる', () => {
    render();
    const open = () =>
      act(() => {
        td(1, 1).dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
      });
    open();
    expect(q('[role="menu"]')).not.toBeNull();
    key(window, 'Escape');
    expect(q('[role="menu"]')).toBeNull();

    open();
    act(() => {
      document.body.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    });
    expect(q('[role="menu"]')).toBeNull();
  });
});

describe('キーボードの Undo とクリップボード', () => {
  it('Ctrl+Z / Ctrl+Y で戻す・やり直す。入力欄の中では表の Undo にしない', () => {
    render();
    mouseDown(td(1, 0));
    key(editor(), 'Delete');
    expect(td(1, 0).textContent).toBe('');

    key(window, 'z', { ctrlKey: true });
    expect(td(1, 0).textContent).toBe('氏名');
    key(window, 'y', { ctrlKey: true });
    expect(td(1, 0).textContent).toBe('');
    key(window, 'z', { ctrlKey: true, shiftKey: true });
    expect(td(1, 0).textContent).toBe('');

    // caption の欄で Ctrl+Z を押しても、表は戻らない
    key(q('.attributes input')!, 'z', { ctrlKey: true });
    expect(td(1, 0).textContent).toBe('');
  });

  it('選択をコピーするとタブ区切りになる', () => {
    render();
    mouseDown(td(1, 0));
    mouseDown(td(2, 1), { shiftKey: true });
    expect(clipboard('copy')).toBe('氏名\t文字列\n年齢\t数値');
    expect(message()).toBe('選択したセルをコピーしました。');
  });

  it('選択があればその位置へ貼り、無ければタブ区切りの表で丸ごと置き換える', () => {
    render();
    mouseDown(td(1, 1));
    clipboard('paste', '文字\t説明文\r\n');
    expect(cells()[1]).toEqual(['氏名', '文字', '説明文']);
    expect(message()).toContain('貼り付けました。');

    key(editor(), 'Escape');
    // タブの無い文章では表を潰さない
    clipboard('paste', 'ただの文章');
    expect(status()).toContain('3 行 × 3 列');

    clipboard('paste', 'A\tB\n1\t2\n');
    expect(cells()).toEqual([
      ['A', 'B'],
      ['1', '2']
    ]);
    expect(message()).toBe('Excel から貼り付けました（結合は取り込まれません）。');
    // 属性はそのまま
    expect(apply().attributes.caption).toBe('利用者の属性');
  });
});
