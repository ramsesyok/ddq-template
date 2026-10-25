import {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useLayoutEffect,
  useRef,
  useState,
  type MouseEvent as ReactMouseEvent,
  type KeyboardEvent as ReactKeyboardEvent
} from 'react';
import type { TableModel, CellAlign } from '../model/TableModel';
import type { CellRange } from '../model/mergeCells';
import { inspectColumnWidths, WIDTH_TOTAL } from '../model/columnWidths';
import { insertLineBreak } from '../model/cellTextEditing';
import {
  anchorOf,
  cellOverlapsRange,
  expandRangeToMerges,
  extendSelection,
  moveActive,
  moveTab,
  moveToEdge,
  selectAll,
  singleCell,
  type CellPos,
  type Direction
} from '../model/selection';

export type GridViewHandle = {
  /** 表にキーボードフォーカスを戻す（ツールバー操作の後など）。 */
  focus(): void;
};

type Props = {
  model: TableModel;
  /** 生の選択。start がアンカー、end がフォーカス。表内に収まっていること。 */
  selection: CellRange | undefined;
  onSelect: (range: CellRange | undefined) => void;
  /** 編集を確定したときに 1 回だけ呼ぶ（入力途中では呼ばない）。 */
  onCommitCell: (row: number, col: number, text: string) => void;
  onClearCells: () => void;
  onOpenMenu: (x: number, y: number) => void;
  onChangeAlign: (col: number, align: CellAlign | undefined) => void;
  onChangeWidth: (col: number, width: number | undefined) => void;
};

const ARROWS: Record<string, Direction> = {
  ArrowUp: 'up',
  ArrowDown: 'down',
  ArrowLeft: 'left',
  ArrowRight: 'right'
};

/**
 * 入力欄（caption など）や編集中のセルにフォーカスがあるか。
 *
 * 表のキーボード操作用の textarea は選択中も常にフォーカスを持っているので、
 * `data-editing` で「編集中か」を見分ける。編集中でなければクリップボードや Undo の
 * ショートカットは表の操作として扱ってよい。
 */
export function inTextField(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el) return false;
  if (el.tagName === 'INPUT' || el.tagName === 'SELECT') return true;
  return el.tagName === 'TEXTAREA' && el.dataset.editing !== 'false';
}

/**
 * 表の描画と、セル単位の操作（選択・移動・編集）。
 *
 * 編集用の textarea は常に 1 つだけ置き、選択中は見えない大きさでアクティブセルの位置に
 * 重ねてフォーカスを持たせる。文字キーや IME の入力がそこへ届いた時点で編集モードへ
 * 切り替え、同じ要素をセルの上へ広げる。要素を差し替えないので IME の変換途中でも
 * 入力が途切れない（Handsontable と同じ作り）。
 */
export const GridView = forwardRef<GridViewHandle, Props>(function GridView(
  {
    model,
    selection,
    onSelect,
    onCommitCell,
    onClearCells,
    onOpenMenu,
    onChangeAlign,
    onChangeWidth
  },
  ref
) {
  const [editing, setEditing] = useState<CellPos | undefined>();
  /** 編集中の本文。確定するまでモデルには書かない（Escape で戻せるように）。 */
  const [draft, setDraft] = useState('');

  const wrapRef = useRef<HTMLDivElement | null>(null);
  const tableRef = useRef<HTMLTableElement | null>(null);
  const editorRef = useRef<HTMLTextAreaElement | null>(null);
  /** 次の描画でキャレットを置く位置。textarea は再描画で末尾へ飛ぶので自分で戻す。 */
  const caretRef = useRef<number | undefined>(undefined);
  /** マウスボタンを押したまま範囲を伸ばしている最中か。 */
  const dragging = useRef(false);

  useImperativeHandle(ref, () => ({ focus: () => editorRef.current?.focus() }), []);

  const active = selection ? anchorOf(model, selection.startRow, selection.startCol) : undefined;
  const range = selection ? expandRangeToMerges(model, selection) : undefined;
  /** フィルハンドルを描くセル（選択の右下を覆う表示セル）。 */
  const handleCell = range ? anchorOf(model, range.endRow, range.endCol) : undefined;
  const widths = inspectColumnWidths(model);

  const findTd = (pos: CellPos) =>
    tableRef.current?.querySelector<HTMLTableCellElement>(
      `td[data-row="${pos.row}"][data-col="${pos.col}"]`
    ) ?? null;

  // 描画のたびに textarea をアクティブセル（編集中なら編集セル）の上へ置く。
  // 編集中は中身の行数に合わせて高さを伸ばす（高さ固定だと下の行が見えなくなる）。
  useLayoutEffect(() => {
    const el = editorRef.current;
    const wrap = wrapRef.current;
    if (!el || !wrap) return;

    const target = editing ?? active;
    const td = target ? findTd(target) : null;
    if (!td) {
      el.style.left = '0';
      el.style.top = '0';
      el.style.width = '1px';
      el.style.height = '1px';
      return;
    }
    const wrapRect = wrap.getBoundingClientRect();
    const tdRect = td.getBoundingClientRect();
    el.style.left = `${tdRect.left - wrapRect.left + wrap.scrollLeft}px`;
    el.style.top = `${tdRect.top - wrapRect.top + wrap.scrollTop}px`;
    if (!editing) {
      el.style.width = '1px';
      el.style.height = '1px';
      return;
    }
    el.style.width = `${tdRect.width}px`;
    el.style.height = 'auto';
    // scrollHeight は枠線を含まないので、border-box での不足ぶんを足す
    const contentHeight = el.scrollHeight + (el.offsetHeight - el.clientHeight);
    el.style.height = `${Math.max(tdRect.height, contentHeight)}px`;

    const at = caretRef.current;
    if (at === undefined) return;
    caretRef.current = undefined;
    el.setSelectionRange(at, at);
  });

  // キーボードで動かしたときに、アクティブセルが見える位置までスクロールする
  useEffect(() => {
    if (!active) return;
    findTd(active)?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  }, [active?.row, active?.col]);

  useEffect(() => {
    const onMouseUp = () => {
      dragging.current = false;
    };
    window.addEventListener('mouseup', onMouseUp);
    window.addEventListener('blur', onMouseUp);
    return () => {
      window.removeEventListener('mouseup', onMouseUp);
      window.removeEventListener('blur', onMouseUp);
    };
  }, []);

  const startEditing = (pos: CellPos, text: string, caret?: number) => {
    caretRef.current = caret;
    setDraft(text);
    setEditing(pos);
    editorRef.current?.focus();
  };

  const commitEditing = () => {
    if (!editing) return;
    onCommitCell(editing.row, editing.col, draft);
    setEditing(undefined);
    setDraft('');
  };

  const cancelEditing = () => {
    setEditing(undefined);
    setDraft('');
  };

  const isEditingCell = (row: number, col: number) =>
    editing?.row === row && editing?.col === col;

  // --- マウス ---

  const onCellMouseDown = (event: ReactMouseEvent<HTMLTableCellElement>, row: number, col: number) => {
    if (event.button === 2) {
      // 右クリック: 選択の外なら、そのセルだけを選び直してからメニューを出す
      if (editing && !isEditingCell(row, col)) commitEditing();
      if (!range || !cellOverlapsRange(model, row, col, range)) onSelect(singleCell({ row, col }));
      return;
    }
    if (event.button !== 0) return;
    // フォーカスを textarea に留め、セルをまたぐドラッグで文字が選択されないようにする
    event.preventDefault();
    if (editing) {
      if (isEditingCell(row, col)) return;
      commitEditing();
    }
    editorRef.current?.focus();
    if (event.shiftKey && selection) {
      onSelect({ ...selection, endRow: row, endCol: col });
    } else {
      onSelect(singleCell({ row, col }));
    }
    dragging.current = true;
  };

  const onCellMouseEnter = (event: ReactMouseEvent<HTMLTableCellElement>, row: number, col: number) => {
    if (!dragging.current || !selection) return;
    // Webview の外でボタンを離すと mouseup が届かないので、ボタンの状態でも見る
    if ((event.buttons & 1) === 0) {
      dragging.current = false;
      return;
    }
    if (selection.endRow === row && selection.endCol === col) return;
    onSelect({ ...selection, endRow: row, endCol: col });
  };

  // フィルハンドル: 選択の左上をアンカーにして、ドラッグで範囲を伸ばす（値は変えない）
  const onHandleMouseDown = (event: ReactMouseEvent<HTMLSpanElement>) => {
    if (event.button !== 0 || !range) return;
    event.preventDefault();
    event.stopPropagation();
    if (editing) commitEditing();
    editorRef.current?.focus();
    onSelect({ ...range });
    dragging.current = true;
  };

  const onCellContextMenu = (event: ReactMouseEvent<HTMLTableCellElement>) => {
    event.preventDefault();
    editorRef.current?.focus();
    onOpenMenu(event.clientX, event.clientY);
  };

  // --- キーボード（常に textarea が受ける） ---

  const onEditorKeyDown = (event: ReactKeyboardEvent<HTMLTextAreaElement>) => {
    // IME の変換中はブラウザに任せる
    if (event.nativeEvent.isComposing || event.key === 'Process') return;
    const ctrl = event.ctrlKey || event.metaKey;

    if (editing) {
      if (event.key === 'Escape') {
        event.preventDefault();
        cancelEditing();
        return;
      }
      if (event.key === 'Enter') {
        event.preventDefault();
        if (event.altKey) {
          // Alt+Enter で改行。textarea は Alt 付きの Enter では改行しないので、
          // 自分で入れてキャレットも戻す
          const el = event.currentTarget;
          const edit = insertLineBreak(draft, el.selectionStart, el.selectionEnd);
          caretRef.current = edit.caret;
          setDraft(edit.text);
          return;
        }
        const pos = editing;
        commitEditing();
        onSelect(singleCell(moveActive(model, pos, event.shiftKey ? 'up' : 'down')));
        return;
      }
      if (event.key === 'Tab') {
        event.preventDefault();
        const pos = editing;
        commitEditing();
        onSelect(singleCell(moveTab(model, pos, event.shiftKey)));
        return;
      }
      return; // それ以外は普通の文字入力
    }

    if (event.key === 'Escape') {
      event.preventDefault();
      onSelect(undefined);
      return;
    }
    if (ctrl && !event.altKey && event.key.toLowerCase() === 'a') {
      event.preventDefault();
      onSelect(selectAll(model));
      return;
    }
    if (!active || !selection) return;

    const dir = ARROWS[event.key];
    if (dir) {
      event.preventDefault();
      onSelect(
        event.shiftKey
          ? extendSelection(model, selection, dir)
          : singleCell(moveActive(model, active, dir))
      );
      return;
    }
    if (event.key === 'Tab') {
      event.preventDefault();
      onSelect(singleCell(moveTab(model, active, event.shiftKey)));
      return;
    }
    if (event.key === 'Home' || event.key === 'End') {
      event.preventDefault();
      onSelect(singleCell(moveToEdge(model, active, event.key === 'End', ctrl)));
      return;
    }
    if (event.key === 'Enter' || event.key === 'F2') {
      event.preventDefault();
      const text = model.rows[active.row][active.col].text;
      startEditing(active, text, text.length);
      return;
    }
    if (event.key === 'Delete' || event.key === 'Backspace') {
      event.preventDefault();
      onClearCells();
      return;
    }
    // 文字キーはそのまま通す。textarea に入った時点で onChange が編集モードへ切り替える。
    // Ctrl+C / Ctrl+V / Ctrl+Z は TableEditor の window ハンドラが受ける
  };

  const onEditorChange = (value: string) => {
    if (editing) {
      setDraft(value);
      return;
    }
    if (!active) return;
    // 選択中に文字を打った: 内容を置き換えて編集開始（Excel と同じ）
    setDraft(value);
    setEditing(active);
  };

  return (
    <div className="grid-wrap" ref={wrapRef}>
      <table className="grid" ref={tableRef}>
        <thead>
          <tr className="col-controls">
            {model.columns.map((col, c) => {
              const filled = widths.autoFill?.col === c ? widths.autoFill.width : undefined;
              // 超過はどの列が悪いか決められないので、幅の入った欄をまとめて赤くする
              const invalid =
                (widths.overflow && col.width !== undefined) ||
                (col.width !== undefined && col.width < 0);
              return (
                <th key={`w${c}`}>
                  <input
                    className={invalid ? 'width-input invalid' : 'width-input'}
                    type="number"
                    min={0}
                    step="any"
                    value={col.width ?? ''}
                    placeholder={filled !== undefined ? `${filled}` : '自動'}
                    title={
                      filled !== undefined
                        ? `列幅（widths）。空欄のまま適用すると残りの ${filled} が入ります`
                        : '列幅（widths）。空欄なら自動幅'
                    }
                    onChange={e =>
                      onChangeWidth(c, e.target.value === '' ? undefined : Number(e.target.value))
                    }
                  />
                  <select
                    className="align-select"
                    value={col.align ?? ''}
                    title="列の揃え"
                    onChange={e =>
                      onChangeAlign(c, (e.target.value || undefined) as CellAlign | undefined)
                    }
                  >
                    <option value="">既定</option>
                    <option value="left">左</option>
                    <option value="center">中央</option>
                    <option value="right">右</option>
                  </select>
                </th>
              );
            })}
          </tr>
        </thead>
        <tbody>
          {model.rows.map((row, r) => (
            <tr key={r} className={r < model.headerRows ? 'header-row' : undefined}>
              {row.map((cell, c) => {
                if (cell.hidden) return null;
                const isActive = active?.row === r && active?.col === c;
                const isSelected = !!range && cellOverlapsRange(model, r, c, range);
                return (
                  <td
                    key={cell.id}
                    data-row={r}
                    data-col={c}
                    rowSpan={cell.rowspan}
                    colSpan={cell.colspan}
                    className={[
                      isSelected ? 'selected' : '',
                      isActive ? 'active' : '',
                      r < model.headerRows ? 'is-header' : '',
                      cell.rowspan > 1 || cell.colspan > 1 ? 'is-merged' : ''
                    ]
                      .filter(Boolean)
                      .join(' ')}
                    style={{ textAlign: model.columns[c]?.align ?? undefined }}
                    onMouseDown={e => onCellMouseDown(e, r, c)}
                    onMouseEnter={e => onCellMouseEnter(e, r, c)}
                    onDoubleClick={() => startEditing({ row: r, col: c }, cell.text, cell.text.length)}
                    onContextMenu={onCellContextMenu}
                  >
                    <span className="cell-text">
                      {cell.text.split('\n').map((line, i) => (
                        <span key={i} className="cell-line">
                          {line}
                        </span>
                      ))}
                    </span>
                    {handleCell?.row === r && handleCell?.col === c && (
                      <span
                        className="fill-handle"
                        title="ドラッグで範囲を広げる"
                        onMouseDown={onHandleMouseDown}
                      />
                    )}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>

      <textarea
        ref={editorRef}
        className={editing ? 'cell-editor editing' : 'cell-editor'}
        data-editing={editing ? 'true' : 'false'}
        aria-label="セルの編集"
        value={draft}
        readOnly={!editing && !active}
        spellCheck={false}
        onChange={e => onEditorChange(e.target.value)}
        onKeyDown={onEditorKeyDown}
        onBlur={() => {
          if (editing) commitEditing();
        }}
      />

      {widths.hasNegative && <p className="hint error">列幅に負の値があります。</p>}
      {widths.overflow && (
        <p className="hint error">
          列幅の合計が {widths.total} です。{WIDTH_TOTAL} 以内に収めてください
          （このままでは適用できません）。
        </p>
      )}
      {!widths.overflow && widths.autoFill && (
        <p className="hint">
          空欄の {widths.autoFill.col + 1} 列目には、適用・プレビュー時に残りの{' '}
          {widths.autoFill.width} が入ります。
        </p>
      )}

      <p className="hint">
        矢印キーで移動・Shift+矢印やドラッグ（セル右下の■も可）で範囲選択・右クリックで
        メニュー。文字を打つか Enter / F2 / ダブルクリックで編集（Alt+Enter で改行・Enter か
        Tab で確定して移動・Escape で取り消し）。Delete で内容を消去、Ctrl+Z / Ctrl+Y で
        元に戻す／やり直す。
      </p>
      <p className="hint">
        選択セルは Ctrl+C でコピー、Ctrl+V で選択位置へ貼り付け（1 つの値なら選択範囲を
        すべて埋めます）。選択していないときの Ctrl+V は Excel の表を丸ごと取り込みます。
      </p>
    </div>
  );
});
