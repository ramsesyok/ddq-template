import { describe, it, expect } from 'vitest';
import { insertLineBreak } from './cellTextEditing';

describe('insertLineBreak', () => {
  it('キャレット位置に改行を入れる', () => {
    expect(insertLineBreak('abcd', 2, 2)).toEqual({ text: 'ab\ncd', caret: 3 });
  });

  it('選択範囲は改行で置き換える', () => {
    expect(insertLineBreak('abcd', 1, 3)).toEqual({ text: 'a\nd', caret: 2 });
  });

  it('選択の向きが逆でも同じ結果', () => {
    expect(insertLineBreak('abcd', 3, 1)).toEqual({ text: 'a\nd', caret: 2 });
  });

  it('末尾でも先頭でも入る', () => {
    expect(insertLineBreak('abc', 3, 3)).toEqual({ text: 'abc\n', caret: 4 });
    expect(insertLineBreak('abc', 0, 0)).toEqual({ text: '\nabc', caret: 1 });
  });

  it('既にある改行を壊さない', () => {
    expect(insertLineBreak('a\nb', 3, 3)).toEqual({ text: 'a\nb\n', caret: 4 });
  });

  it('範囲外の位置は文字列内に丸める', () => {
    expect(insertLineBreak('abc', 99, 99)).toEqual({ text: 'abc\n', caret: 4 });
    expect(insertLineBreak('abc', -5, -5)).toEqual({ text: '\nabc', caret: 1 });
  });
});
