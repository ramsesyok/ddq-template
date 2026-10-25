import { describe, it, expect } from 'vitest';
import {
  createHistory,
  pushHistory,
  undoHistory,
  redoHistory,
  canUndo,
  canRedo,
  HISTORY_LIMIT
} from './history';

describe('history', () => {
  it('積んで戻してやり直せる', () => {
    let h = createHistory('a');
    expect(canUndo(h)).toBe(false);
    h = pushHistory(h, 'b');
    h = pushHistory(h, 'c');
    expect(h.present).toBe('c');
    h = undoHistory(h);
    expect(h.present).toBe('b');
    expect(canRedo(h)).toBe(true);
    h = undoHistory(h);
    expect(h.present).toBe('a');
    expect(canUndo(h)).toBe(false);
    h = undoHistory(h);
    expect(h.present).toBe('a');
    h = redoHistory(h);
    h = redoHistory(h);
    expect(h.present).toBe('c');
    expect(redoHistory(h).present).toBe('c');
  });

  it('戻した後に新しく積むとやり直しは消える', () => {
    let h = pushHistory(pushHistory(createHistory('a'), 'b'), 'c');
    h = undoHistory(h);
    h = pushHistory(h, 'd');
    expect(canRedo(h)).toBe(false);
    expect(h.present).toBe('d');
    expect(undoHistory(h).present).toBe('b');
  });

  it('同じ参照は積まない', () => {
    const obj = { v: 1 };
    const h = createHistory(obj);
    expect(pushHistory(h, obj)).toBe(h);
  });

  it('上限を超えた古い履歴は捨てる', () => {
    let h = createHistory(0);
    for (let i = 1; i <= HISTORY_LIMIT + 10; i++) h = pushHistory(h, i);
    expect(h.past.length).toBe(HISTORY_LIMIT);
    expect(h.past[0]).toBe(10);
  });
});
