import { useEffect, useLayoutEffect, useRef, useState } from 'react';

export type MenuItem =
  | { kind: 'item'; label: string; shortcut?: string; disabled?: boolean; onSelect: () => void }
  | { kind: 'separator' };

type Props = {
  x: number;
  y: number;
  items: MenuItem[];
  onClose: () => void;
};

/**
 * セルの右クリックメニュー。
 *
 * Webview には OS のメニューが無いので自前で描く。項目のクリックは mousedown で
 * フォーカスを奪わないようにしてある（表のキー操作を続けられるように）。
 */
export function ContextMenu({ x, y, items, onClose }: Props) {
  const ref = useRef<HTMLDivElement | null>(null);
  const [pos, setPos] = useState({ x, y });

  // 画面からはみ出す分を内側へ寄せる
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    setPos({
      x: Math.max(0, Math.min(x, window.innerWidth - rect.width - 4)),
      y: Math.max(0, Math.min(y, window.innerHeight - rect.height - 4))
    });
  }, [x, y]);

  useEffect(() => {
    const onMouseDown = (event: MouseEvent) => {
      if (ref.current?.contains(event.target as Node)) return;
      onClose();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        onClose();
      }
    };
    window.addEventListener('mousedown', onMouseDown, true);
    window.addEventListener('keydown', onKeyDown, true);
    window.addEventListener('blur', onClose);
    window.addEventListener('resize', onClose);
    window.addEventListener('scroll', onClose, true);
    return () => {
      window.removeEventListener('mousedown', onMouseDown, true);
      window.removeEventListener('keydown', onKeyDown, true);
      window.removeEventListener('blur', onClose);
      window.removeEventListener('resize', onClose);
      window.removeEventListener('scroll', onClose, true);
    };
  }, [onClose]);

  return (
    <div
      ref={ref}
      className="context-menu"
      role="menu"
      style={{ left: pos.x, top: pos.y }}
      onContextMenu={e => e.preventDefault()}
    >
      {items.map((item, i) =>
        item.kind === 'separator' ? (
          <div key={i} className="context-menu-separator" />
        ) : (
          <button
            key={i}
            type="button"
            role="menuitem"
            className="context-menu-item"
            disabled={item.disabled}
            onMouseDown={e => e.preventDefault()}
            onClick={() => {
              onClose();
              item.onSelect();
            }}
          >
            <span>{item.label}</span>
            {item.shortcut && <span className="context-menu-shortcut">{item.shortcut}</span>}
          </button>
        )
      )}
    </div>
  );
}
