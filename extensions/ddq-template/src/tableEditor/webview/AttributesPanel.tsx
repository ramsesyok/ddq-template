import type { TableModel } from '../model/TableModel';
import { isValidLabel } from '../formats/tbl/parseTblAttributes';

type Props = {
  model: TableModel;
  /** 分割表（M>1）では div 属性が全パートに波及するため編集できない。 */
  editable: boolean;
  enclosingClasses: string[];
  onChange: (model: TableModel) => void;
};

export function AttributesPanel({ model, editable, enclosingClasses, onChange }: Props) {
  const attrs = model.attributes;
  const labelInvalid = !!attrs.label && !isValidLabel(attrs.label);

  const setAttrs = (patch: Partial<typeof attrs>) =>
    onChange({ ...model, attributes: { ...attrs, ...patch } });

  return (
    <div className="attributes">
      <label>
        caption
        <input
          type="text"
          value={attrs.caption ?? ''}
          disabled={!editable}
          placeholder="付けると採番されます"
          onChange={e => setAttrs({ caption: e.target.value || undefined })}
        />
      </label>

      <label>
        label
        <input
          type="text"
          value={attrs.label ?? ''}
          disabled={!editable || attrs.unnumbered === true}
          placeholder="tbl-xxx"
          className={labelInvalid ? 'invalid' : undefined}
          onChange={e =>
            setAttrs({ label: e.target.value.replace(/^#/, '') || undefined })
          }
        />
      </label>

      <label className="checkbox">
        <input
          type="checkbox"
          checked={attrs.unnumbered === true}
          disabled={!editable}
          onChange={e => setAttrs({ unnumbered: e.target.checked || undefined })}
        />
        .unnumbered（番号を出さずキャプションだけ）
      </label>

      {labelInvalid && (
        <span className="hint error">label は tbl- で始まる必要があります。</span>
      )}

      {!editable && (
        <span className="hint">
          分割表のため div 属性は編集できません（変更すると全パートに影響するため）。
        </span>
      )}

      {enclosingClasses.length > 0 && (
        <span className="hint">
          外側の div: {enclosingClasses.map(c => `.${c}`).join(' ')}（そのまま保持します）
        </span>
      )}
    </div>
  );
}
