import { describe, expect, it } from 'vitest';
import { diffStep, History, mapPosition } from './history.ts';

/** Type `insert` at `from` in `text` as a local edit, the way the adapter records it. */
function type(history: History, text: string, from: number, insert: string, to = from): string {
  const next = text.slice(0, from) + insert + text.slice(to);
  history.record({ from, removed: text.slice(from, to), inserted: insert });
  return next;
}

describe('diffStep', () => {
  it('is undefined for equal texts', () => {
    expect(diffStep('abc', 'abc')).toBeUndefined();
  });

  it('finds an insert, a delete and a replacement', () => {
    expect(diffStep('ac', 'abc')).toEqual({ from: 1, removed: '', inserted: 'b' });
    expect(diffStep('abc', 'ac')).toEqual({ from: 1, removed: 'b', inserted: '' });
    expect(diffStep('abc', 'aXc')).toEqual({ from: 1, removed: 'b', inserted: 'X' });
  });

  it('does not let the suffix run into the prefix', () => {
    expect(diffStep('aa', 'aaa')).toEqual({ from: 2, removed: '', inserted: 'a' });
    expect(diffStep('aaa', 'aa')).toEqual({ from: 2, removed: 'a', inserted: '' });
  });
});

describe('mapPosition', () => {
  it('keeps a position at or before the change and shifts one behind it', () => {
    const insert = { from: 5, to: 5, insert: 'xyz' };
    expect(mapPosition(4, insert)).toBe(4);
    expect(mapPosition(5, insert)).toBe(5);
    expect(mapPosition(6, insert)).toBe(9);
    expect(mapPosition(5, { from: 2, to: 4, insert: '' })).toBe(3);
    expect(mapPosition(3, { from: 2, to: 4, insert: 'ab' })).toBe(4);
  });
});

describe('History', () => {
  it('undoes and redoes local steps one by one', () => {
    const history = new History();
    let text = type(history, 'one two', 3, ' and');
    text = type(history, text, 0, 'Say: ');
    expect(text).toBe('Say: one and two');

    const first = history.undo(text);
    expect(first).toEqual({ text: 'one and two', caret: 0 });
    const second = history.undo(first!.text);
    expect(second).toEqual({ text: 'one two', caret: 3 });
    expect(history.canUndo()).toBe(false);
    expect(history.undo(second!.text)).toBeUndefined();

    expect(history.redo(second!.text)).toEqual({ text: 'one and two', caret: 7 });
    expect(history.redo('one and two')).toEqual({ text: 'Say: one and two', caret: 5 });
  });

  it('drops the redo stack at a new edit', () => {
    const history = new History();
    const text = type(history, 'ab', 1, 'X');
    history.undo(text);
    expect(history.canRedo()).toBe(true);
    type(history, 'ab', 0, 'Y');
    expect(history.canRedo()).toBe(false);
  });

  it('forgets everything on clear', () => {
    const history = new History();
    type(history, 'ab', 1, 'X');
    history.clear();
    expect(history.canUndo()).toBe(false);
  });

  it('keeps undo local when an external change lands before the local edit', () => {
    const history = new History();
    let text = type(history, 'abc def', 4, 'local ');
    const external = { from: 0, to: 0, insert: 'EXT ' };
    text = external.insert + text;
    history.rebase(external);
    expect(history.undo(text)?.text).toBe('EXT abc def');
  });

  it('keeps undo local when an external change lands after the local edit', () => {
    const history = new History();
    let text = type(history, 'abc def', 0, 'local ');
    const external = { from: text.length, to: text.length, insert: ' EXT' };
    text += external.insert;
    history.rebase(external);
    expect(history.undo(text)?.text).toBe('abc def EXT');
  });

  it('rebases through several steps', () => {
    const history = new History();
    let text = type(history, 'abcdef', 3, '-1-');
    text = type(history, text, 0, '-2-');
    // An external insert behind both steps, in the coordinates of the current text.
    const external = { from: text.length, to: text.length, insert: '!' };
    text += external.insert;
    history.rebase(external);
    const once = history.undo(text)!;
    expect(once.text).toBe('abc-1-def!');
    expect(history.undo(once.text)?.text).toBe('abcdef!');
  });

  it('drops a step that the external change overlaps, and the steps below it', () => {
    const history = new History();
    let text = type(history, 'abcdef', 1, 'XY');
    text = type(history, text, 6, 'Z');
    expect(text).toBe('aXYbcdZef');
    // An external replacement of "XY" overlaps the older step only.
    const external = { from: 1, to: 3, insert: '__' };
    text = text.slice(0, 1) + '__' + text.slice(3);
    history.rebase(external);
    expect(history.undo(text)?.text).toBe('a__bcdef');
    expect(history.canUndo()).toBe(false);
  });
});
