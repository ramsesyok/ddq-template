/**
 * Undo / Redo の履歴。
 *
 * モデルはイミュータブルに扱っているので、スナップショットを積むだけで済む
 * （差分を記録する方式より単純で、行列操作・結合・貼り付けもすべて同じ経路に乗る）。
 */

export type History<T> = {
  past: T[];
  present: T;
  future: T[];
};

/** 積みすぎてメモリを食わないための上限。1 表の編集でこれを超えて戻ることはまず無い。 */
export const HISTORY_LIMIT = 100;

export function createHistory<T>(present: T): History<T> {
  return { past: [], present, future: [] };
}

/** 新しい状態を積む。同じ参照なら何もしない（変更の無い操作を履歴に残さない）。 */
export function pushHistory<T>(history: History<T>, next: T): History<T> {
  if (next === history.present) return history;
  const past = [...history.past, history.present];
  if (past.length > HISTORY_LIMIT) past.splice(0, past.length - HISTORY_LIMIT);
  return { past, present: next, future: [] };
}

export function canUndo<T>(history: History<T>): boolean {
  return history.past.length > 0;
}

export function canRedo<T>(history: History<T>): boolean {
  return history.future.length > 0;
}

export function undoHistory<T>(history: History<T>): History<T> {
  if (!canUndo(history)) return history;
  const past = history.past.slice(0, -1);
  const present = history.past[history.past.length - 1];
  return { past, present, future: [history.present, ...history.future] };
}

export function redoHistory<T>(history: History<T>): History<T> {
  if (!canRedo(history)) return history;
  const [present, ...future] = history.future;
  return { past: [...history.past, history.present], present, future };
}
