import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { TableModel, CellAlign } from '../model/TableModel';
import type { EditorContext, ToWebviewMessage } from '../model/WebviewMessages';
import { hasMerges } from '../model/TableModel';
import { normalizeTableModel } from '../model/normalizeTableModel';
import { completeColumnWidths } from '../model/columnWidths';
import { mergeCells, type CellRange } from '../model/mergeCells';
import { unmergeCell } from '../model/unmergeCell';
import { insertRow, deleteRow, insertColumn, deleteColumn } from '../model/rowColOps';
import { parseTsv, splitTsv } from '../model/parseTsv';
import { buildTsv, pasteCellsAt } from '../model/tsvClipboard';
import { clearCells } from '../model/clearCells';
import { clampRange, expandRangeToMerges } from '../model/selection';
import {
  createHistory,
  pushHistory,
  undoHistory,
  redoHistory,
  canUndo,
  canRedo,
  type History
} from '../model/history';
import { canUseMergeCols } from '../formats/mergeCols/canUseMergeCols';
import { serializeTableBody, serializeTblBlock } from '../formats/tbl/serializeTblBlock';
import { vscode } from './vscodeApi';
import { Toolbar } from './Toolbar';
import { AttributesPanel } from './AttributesPanel';
import { GridView, inTextField, type GridViewHandle } from './GridView';
import { ContextMenu, type MenuItem } from './ContextMenu';

export function TableEditor() {
  const [history, setHistory] = useState<History<TableModel> | undefined>();
  const [context, setContext] = useState<EditorContext | undefined>();
  const [selection, setSelection] = useState<CellRange | undefined>();
  const [message, setMessage] = useState<string | undefined>();
  const [showPreview, setShowPreview] = useState(false);
  const [menu, setMenu] = useState<{ x: number; y: number } | undefined>();
  const gridRef = useRef<GridViewHandle | null>(null);

  const model = history?.present;

  /** モデルを更新して履歴に積む。変化が無ければ（同じ参照なら）積まない。 */
  const update = useCallback((next: TableModel) => {
    setHistory(h => (!h || next === h.present ? h : pushHistory(h, normalizeTableModel(next))));
  }, []);

  const undo = useCallback(() => setHistory(h => (h ? undoHistory(h) : h)), []);
  const redo = useCallback(() => setHistory(h => (h ? redoHistory(h) : h)), []);

  useEffect(() => {
    const onMessage = (event: MessageEvent<ToWebviewMessage>) => {
      const data = event.data;
      if (data.type === 'init') {
        setHistory(createHistory(normalizeTableModel(data.model)));
        setContext(data.context);
        setMessage(data.context.warnings.join('\n') || undefined);
      } else if (data.type === 'applied') {
        setMessage('ドキュメントへ反映しました。');
      } else if (data.type === 'error') {
        setMessage(data.message);
      }
    };
    window.addEventListener('message', onMessage);
    vscode().postMessage({ type: 'ready' });
    return () => window.removeEventListener('message', onMessage);
  }, []);

  // Ctrl+Z / Ctrl+Y（Ctrl+Shift+Z）。入力欄や編集中のセルでは、その欄自身の Undo に任せる
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
      if (inTextField(event.target)) return;
      const key = event.key.toLowerCase();
      if (key === 'z') {
        event.preventDefault();
        if (event.shiftKey) redo();
        else undo();
      } else if (key === 'y') {
        event.preventDefault();
        redo();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [undo, redo]);

  useEffect(() => {
    const onCopy = (event: ClipboardEvent) => {
      if (inTextField(event.target) || !selection || !model) return;
      event.preventDefault();
      event.clipboardData?.setData('text/plain', buildTsv(model, selection));
      setMessage('選択したセルをコピーしました。');
    };
    window.addEventListener('copy', onCopy);
    return () => window.removeEventListener('copy', onCopy);
  }, [model, selection]);

  useEffect(() => {
    const onPaste = (event: ClipboardEvent) => {
      if (inTextField(event.target)) return;
      const text = event.clipboardData?.getData('text/plain');
      if (!text || !model) return;

      // 選択があれば、その左上を起点に貼る（Excel と同じ）
      if (selection) {
        event.preventDefault();
        const result = pasteCellsAt(
          model,
          splitTsv(text.replace(/\r\n?/g, '\n').replace(/\n$/, '')),
          selection
        );
        update(result.model);
        setSelection(result.range);
        setMessage(
          result.skipped > 0
            ? `貼り付けました（結合セル ${result.skipped} 個ぶんは書き込めないため捨てました）。`
            : '貼り付けました。表全体を置き換えるには Escape で選択を解除してから貼り付けてください。'
        );
        return;
      }

      // 選択が無いときは表全体の置き換え。よそからコピーした文章で表を潰さないよう、
      // 表らしさ（タブ区切り）がある場合だけ受け付ける
      if (!text.includes('\t')) return;
      event.preventDefault();
      const pasted = parseTsv(text, model.id);
      update({ ...pasted, attributes: model.attributes });
      setSelection(undefined);
      setMessage('Excel から貼り付けました（結合は取り込まれません）。');
    };
    window.addEventListener('paste', onPaste);
    return () => window.removeEventListener('paste', onPaste);
  }, [model, selection, update]);

  const mergeColsCheck = useMemo(
    () => (model ? canUseMergeCols(model, context?.partCount ?? 1) : undefined),
    [model, context]
  );

  const preview = useMemo(() => {
    if (!model || !context) return '';
    // 空欄 1 列の列幅は、書き戻しと同じく残り幅で埋めた姿を見せる
    const shown = completeColumnWidths(model);
    const wholeBlock = context.kind !== 'tblPart' || context.partCount === 1;
    return wholeBlock
      ? serializeTblBlock(shown, { partCount: context.partCount })
      : serializeTableBody(shown, context.partCount);
  }, [model, context]);

  if (!model || !history || !context) {
    return <div className="loading">読み込み中…</div>;
  }

  // 行の削除や Undo で表が縮んでも、選択が表の外を指さないようにする
  const raw = selection ? clampRange(model, selection) : undefined;
  const range = raw ? expandRangeToMerges(model, raw) : undefined;

  /** ツールバーやメニューの操作の後、キーボード操作を表に戻す。 */
  const withFocus = (fn: () => void) => () => {
    fn();
    gridRef.current?.focus();
  };

  const insertRowAbove = () => {
    const at = range ? range.startRow : 0;
    update(insertRow(model, at));
    // 選んでいたセルは 1 行下へずれるので、選択も付いていく
    if (raw) setSelection({ ...raw, startRow: raw.startRow + 1, endRow: raw.endRow + 1 });
  };
  const insertRowBelow = () => update(insertRow(model, range ? range.endRow + 1 : model.rows.length));
  const insertColumnLeft = () => {
    const at = range ? range.startCol : 0;
    update(insertColumn(model, at));
    if (raw) setSelection({ ...raw, startCol: raw.startCol + 1, endCol: raw.endCol + 1 });
  };
  const insertColumnRight = () =>
    update(insertColumn(model, range ? range.endCol + 1 : model.columns.length));
  const removeRow = () => update(deleteRow(model, range ? range.startRow : model.rows.length - 1));
  const removeColumn = () =>
    update(deleteColumn(model, range ? range.startCol : model.columns.length - 1));

  const runMerge = () => {
    if (!range) return setMessage('結合する範囲を選択してください。');
    const result = mergeCells(model, range);
    if (!result.ok) return setMessage(result.message);
    update(result.value);
    setMessage(undefined);
  };

  const runUnmerge = () => {
    if (!range) return setMessage('解除するセルを選択してください。');
    const result = unmergeCell(model, range.startRow, range.startCol);
    if (!result.ok) return setMessage(result.message);
    update(result.value);
    setMessage(undefined);
  };

  const runClear = () => {
    if (!range) return;
    update(clearCells(model, range));
  };

  const menuItems: MenuItem[] = [
    { kind: 'item', label: '上に行を挿入', onSelect: insertRowAbove },
    { kind: 'item', label: '下に行を挿入', onSelect: insertRowBelow },
    { kind: 'item', label: '行を削除', disabled: model.rows.length <= 1, onSelect: removeRow },
    { kind: 'separator' },
    { kind: 'item', label: '左に列を挿入', onSelect: insertColumnLeft },
    { kind: 'item', label: '右に列を挿入', onSelect: insertColumnRight },
    {
      kind: 'item',
      label: '列を削除',
      disabled: model.columns.length <= 1,
      onSelect: removeColumn
    },
    { kind: 'separator' },
    {
      kind: 'item',
      label: 'セルを結合',
      disabled: !range || (range.startRow === range.endRow && range.startCol === range.endCol),
      onSelect: runMerge
    },
    { kind: 'item', label: '結合を解除', onSelect: runUnmerge },
    { kind: 'item', label: '内容を消去', shortcut: 'Delete', onSelect: runClear },
    { kind: 'separator' },
    { kind: 'item', label: '元に戻す', shortcut: 'Ctrl+Z', disabled: !canUndo(history), onSelect: undo },
    { kind: 'item', label: 'やり直す', shortcut: 'Ctrl+Y', disabled: !canRedo(history), onSelect: redo }
  ];

  return (
    <div className="editor">
      <Toolbar
        model={model}
        context={context}
        mergeColsCheck={mergeColsCheck}
        canUndo={canUndo(history)}
        canRedo={canRedo(history)}
        onUndo={withFocus(undo)}
        onRedo={withFocus(redo)}
        onInsertRowAbove={withFocus(insertRowAbove)}
        onInsertRowBelow={withFocus(insertRowBelow)}
        onDeleteRow={withFocus(removeRow)}
        onInsertColumnLeft={withFocus(insertColumnLeft)}
        onInsertColumnRight={withFocus(insertColumnRight)}
        onDeleteColumn={withFocus(removeColumn)}
        onMerge={withFocus(runMerge)}
        onUnmerge={withFocus(runUnmerge)}
        onChangeOutputFormat={format => update({ ...model, outputFormat: format })}
        onChangeHeaderRows={n => update({ ...model, headerRows: n })}
        onTogglePreview={() => setShowPreview(v => !v)}
        showPreview={showPreview}
        onApply={() => vscode().postMessage({ type: 'apply', model })}
      />

      <AttributesPanel
        model={model}
        editable={context.attributesEditable}
        enclosingClasses={context.enclosingClasses}
        onChange={update}
      />

      {message && <div className="message">{message}</div>}

      <GridView
        ref={gridRef}
        model={model}
        selection={raw}
        onSelect={setSelection}
        onCommitCell={(row, col, text) => {
          if (model.rows[row]?.[col]?.text === text) return;
          const rows = model.rows.map(r => r.map(c => ({ ...c })));
          rows[row][col].text = text;
          update({ ...model, rows });
        }}
        onClearCells={runClear}
        onOpenMenu={(x, y) => setMenu({ x, y })}
        onChangeAlign={(col, align) => {
          const columns = model.columns.map((c, i) => (i === col ? { ...c, align } : c));
          update({ ...model, columns });
        }}
        onChangeWidth={(col, width) => {
          const columns = model.columns.map((c, i) => (i === col ? { ...c, width } : c));
          update({ ...model, columns });
        }}
      />

      {menu && (
        <ContextMenu x={menu.x} y={menu.y} items={menuItems} onClose={() => setMenu(undefined)} />
      )}

      {showPreview && (
        <div className="preview">
          <div className="preview-title">
            生成される Markdown
            {context.partCount > 1 && `（分割表の ${context.partIndex + 1} 番目のパートのみ）`}
          </div>
          <pre>{preview}</pre>
        </div>
      )}

      <div className="status">
        {model.rows.length} 行 × {model.columns.length} 列 ／ ヘッダ {model.headerRows} 行
        {hasMerges(model) ? ' ／ 結合あり' : ' ／ 結合なし'}
      </div>
    </div>
  );
}

export type { CellAlign };
