import { EditorState } from 'prosemirror-state';
import { describe, expect, it } from 'vitest';
import { parseBody } from './markdown.ts';
import { PositionMap, Tracker } from './positions.ts';

const body = '## Title\n\nA *bold* line\nover two lines.\n\n> quoted text\n\n- item one\n- item two\n';

describe('PositionMap', () => {
  const doc = parseBody(body);
  const map = new PositionMap(body, 0, doc);

  it('pairs every textblock with its source lines', () => {
    expect(map.exact).toBe(true);
  });

  it('puts a position at the source character it shows', () => {
    // The heading text starts behind "## ".
    expect(map.toOffset(1)).toBe(3);
    expect(body.slice(map.toOffset(1), map.toOffset(1) + 5)).toBe('Title');
  });

  it('round-trips offsets inside prose, over markers and soft line breaks', () => {
    for (const find of ['Title', 'bold', 'over two', 'quoted', 'item two']) {
      const at = body.indexOf(find) + 2;
      expect(map.toOffset(map.toPos(at))).toBe(at);
    }
  });
});

describe('Tracker', () => {
  it('follows an insertion and a deletion in the position source', () => {
    const doc = parseBody(body);
    const tracker = new Tracker(body, 0, doc);
    const at = tracker.toPos(body.indexOf('quoted'));
    const state = EditorState.create({ doc });
    const typed = state.apply(state.tr.insertText('XY', at)).doc;
    tracker.sync(typed);
    expect(tracker.text()).toBe(body.replace('quoted', 'XYquoted'));
    const after = EditorState.create({ doc: typed });
    const deleted = after.apply(after.tr.delete(at, at + 2)).doc;
    tracker.sync(deleted);
    expect(tracker.text()).toBe(body);
  });
});
