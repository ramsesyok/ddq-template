import type { TableModel, OutputFormat } from '../model/TableModel';
import type { EditorContext } from '../model/WebviewMessages';
import { hasLineBreaks, hasMerges } from '../model/TableModel';
import type { MergeColsCheck } from '../formats/mergeCols/canUseMergeCols';

type Props = {
  model: TableModel;
  context: EditorContext;
  mergeColsCheck: MergeColsCheck | undefined;
  canUndo: boolean;
  canRedo: boolean;
  onUndo: () => void;
  onRedo: () => void;
  onInsertRowAbove: () => void;
  onInsertRowBelow: () => void;
  onDeleteRow: () => void;
  onInsertColumnLeft: () => void;
  onInsertColumnRight: () => void;
  onDeleteColumn: () => void;
  onMerge: () => void;
  onUnmerge: () => void;
  onChangeOutputFormat: (format: OutputFormat) => void;
  onChangeHeaderRows: (n: number) => void;
  onTogglePreview: () => void;
  showPreview: boolean;
  onApply: () => void;
};

export function Toolbar(props: Props) {
  const { model, context, mergeColsCheck } = props;
  const merged = hasMerges(model);
  const multiline = hasLineBreaks(model);
  const mergeColsAvailable = mergeColsCheck?.ok === true;

  // 結合が無ければパイプ表かグリッド表かが自動で決まる。結合があるときだけ選べる。
  const showFormatSelector = merged;

  return (
    <div className="toolbar">
      <div className="toolbar-group">
        <button onClick={props.onUndo} disabled={!props.canUndo} title="元に戻す（Ctrl+Z）">
          元に戻す
        </button>
        <button onClick={props.onRedo} disabled={!props.canRedo} title="やり直す（Ctrl+Y）">
          やり直す
        </button>
      </div>

      <div className="toolbar-group">
        <span className="toolbar-label">行</span>
        <button onClick={props.onInsertRowAbove} title="選択行の上に行を挿入">
          上に追加
        </button>
        <button onClick={props.onInsertRowBelow} title="選択行の下に行を挿入">
          下に追加
        </button>
        <button onClick={props.onDeleteRow} title="選択行を削除">
          削除
        </button>
      </div>

      <div className="toolbar-group">
        <span className="toolbar-label">列</span>
        <button onClick={props.onInsertColumnLeft} title="選択列の左に列を挿入">
          左に追加
        </button>
        <button onClick={props.onInsertColumnRight} title="選択列の右に列を挿入">
          右に追加
        </button>
        <button onClick={props.onDeleteColumn} title="選択列を削除">
          削除
        </button>
      </div>

      <div className="toolbar-group">
        <button onClick={props.onMerge}>結合</button>
        <button onClick={props.onUnmerge}>結合を解除</button>
      </div>

      <div className="toolbar-group">
        <label>
          ヘッダ行数
          <input
            type="number"
            min={0}
            max={model.rows.length}
            value={model.headerRows}
            onChange={e => props.onChangeHeaderRows(Number(e.target.value))}
          />
        </label>
      </div>

      {showFormatSelector && (
        <div className="toolbar-group">
          <label>
            出力形式
            <select
              value={model.outputFormat === 'mergeCols' ? 'mergeCols' : 'gridTable'}
              onChange={e => props.onChangeOutputFormat(e.target.value as OutputFormat)}
            >
              <option value="gridTable">グリッド表</option>
              <option value="mergeCols" disabled={!mergeColsAvailable}>
                merge-cols
              </option>
            </select>
          </label>
          {!mergeColsAvailable && mergeColsCheck && !mergeColsCheck.ok && (
            <span className="hint" title={mergeColsCheck.reason}>
              merge-cols 不可: {mergeColsCheck.reason}
            </span>
          )}
        </div>
      )}

      {!showFormatSelector && (
        <div className="toolbar-group">
          <span className="hint">
            {multiline
              ? 'セル内改行あり → グリッド表として出力します（箇条書きなども書けます）'
              : '結合なし → 通常のパイプ表として出力します'}
          </span>
        </div>
      )}

      <div className="toolbar-group right">
        {context.partCount > 1 && (
          <span className="badge">
            分割表 {context.partIndex + 1}／{context.partCount}
          </span>
        )}
        <button onClick={props.onTogglePreview}>
          {props.showPreview ? 'プレビューを閉じる' : 'プレビュー'}
        </button>
        <button className="primary" onClick={props.onApply}>
          Apply
        </button>
      </div>
    </div>
  );
}
