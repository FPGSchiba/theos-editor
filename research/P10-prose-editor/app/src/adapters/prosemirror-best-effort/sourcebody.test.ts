import { Fragment, Slice } from 'prosemirror-model';
import { AllSelection, EditorState, TextSelection, type Transaction } from 'prosemirror-state';
import { joinBackward, splitBlock } from 'prosemirror-commands';
import { describe, expect, it } from 'vitest';
import { canUndo, history, undo } from '../prosemirror/history.ts';
import { parseBody } from '../prosemirror/markdown.ts';
import { schema } from '../prosemirror/schema.ts';
import { SourceBody } from './sourcebody.ts';

// The best-effort body on an editor state without a view (synthetic text, not the corpus): every
// change is a transaction, as the view would dispatch it, and `save` is what `text()` returns.

const FRONT = '---\n# synthetic fixture (P10), not canon\nid: x\n---\n';

function editor(body: string) {
  const file = FRONT + body;
  const model = new SourceBody();
  let state = EditorState.create({ doc: parseBody(body), plugins: [history()] });
  model.reset(file, FRONT.length, state.doc);
  const dispatch = (tr: Transaction): void => {
    const applied = state.applyTransaction(tr);
    for (const one of applied.transactions) if (one.docChanged) model.observe(one);
    state = applied.state;
  };
  const pos = (find: string, offset = 0): number => model.toPos(state.doc, file.indexOf(find) + offset);
  return {
    get state() {
      return state;
    },
    dispatch,
    pos,
    /** The position of `find` in the saved text as it is now. */
    posNow: (find: string, offset = 0): number => model.toPos(state.doc, model.source(state.doc).indexOf(find) + offset),
    body: (): string => model.save(state.doc, FRONT).slice(FRONT.length),
    level: (): string => model.level(),
    undo: (): void => {
      while (canUndo(state)) undo(state, dispatch);
    },
    undoOnce: (): void => {
      undo(state, dispatch);
    },
  };
}

describe('SourceBody: untouched text', () => {
  it('saves the loaded bytes, soft breaks, underscores, brackets and the final newline included', () => {
    const body = 'A line\nwrapped _here_ with [[x]] and [!y].\n\n\nTwo blank lines above.\n';
    const ed = editor(body);
    expect(ed.body()).toBe(body);
    expect(ed.level()).toBe('span');
  });
});

describe('SourceBody: span edits', () => {
  it('types next to emphasis on the side the marks say', () => {
    const body = 'Emphasis with *stars* and _underscores_.\n';
    const ed = editor(body);
    // At the end of "stars" the typed text carries the emphasis (inside the markers).
    ed.dispatch(ed.state.tr.insertText('X', ed.pos('stars', 5)));
    expect(ed.body()).toBe('Emphasis with *starsX* and _underscores_.\n');
    // In front of "underscores" it does not (outside the markers).
    const at = ed.posNow('underscores');
    ed.dispatch(ed.state.tr.setSelection(TextSelection.create(ed.state.doc, at)).insertText('Y '));
    expect(ed.body()).toBe('Emphasis with *starsX* and Y _underscores_.\n');
    expect(ed.level()).toBe('span');
  });

  it('writes the block again when no span edit stands for the change (an underscore run cannot open inside a word)', () => {
    const ed = editor('Words and _underscores_.\n\nNext _block_.\n');
    const at = ed.pos('underscores');
    ed.dispatch(ed.state.tr.setSelection(TextSelection.create(ed.state.doc, at)).insertText('Y'));
    expect(ed.body()).toBe('Words and Y*underscores*.\n\nNext _block_.\n');
    expect(ed.level()).toBe('block');
  });

  it('removes the markers with the whole emphasised run, and undo brings the bytes back', () => {
    const body = 'Keep _this_ word.\n';
    const ed = editor(body);
    const from = ed.pos('this');
    ed.dispatch(ed.state.tr.delete(from, from + 4));
    expect(ed.body()).toBe('Keep  word.\n');
    ed.undo();
    expect(ed.body()).toBe(body);
  });

  it('keeps a soft line break between two edits of one paragraph', () => {
    const body = 'First line of a paragraph\nsecond line of it.\n';
    const ed = editor(body);
    ed.dispatch(ed.state.tr.insertText('A ', ed.pos('First')));
    ed.dispatch(ed.state.tr.insertText(' B', ed.posNow('it.', 3)));
    expect(ed.body()).toBe('A First line of a paragraph\nsecond line of it. B\n');
  });

  it('escapes typed text only when it would parse as markup', () => {
    const ed = editor('Plain text.\n');
    ed.dispatch(ed.state.tr.insertText(' [[open and **bold**', ed.pos('.')));
    expect(ed.body()).toBe('Plain text \\[\\[open and \\*\\*bold\\*\\*.\n');
    const raw = editor('Plain text.\n');
    raw.dispatch(raw.state.tr.insertText(' [[open', raw.pos('.')));
    expect(raw.body()).toBe('Plain text [[open.\n');
  });

  it('types behind the prefix of a later line of a callout', () => {
    const body = '> [!voice] [[x]]\n> First line.\n> Second line.\n';
    const ed = editor(body);
    ed.dispatch(ed.state.tr.insertText('Very ', ed.pos('Second')));
    expect(ed.body()).toBe('> [!voice] [[x]]\n> First line.\n> Very Second line.\n');
  });

  it('writes an inserted link chip as its syntax and removes a deleted one whole', () => {
    const body = 'See [[a|the a]] and more.\n';
    const ed = editor(body);
    const chip = ed.pos('[[a|');
    ed.dispatch(ed.state.tr.delete(chip, chip + 1));
    expect(ed.body()).toBe('See  and more.\n');
    ed.dispatch(ed.state.tr.insert(ed.posNow('more'), schema.nodes.wikilink!.create({ id: 'b' })));
    expect(ed.body()).toBe('See  and [[b]]more.\n');
  });

  it('gives the first attribute line of a bare beat its own line', () => {
    const body = '## Beat 3 — A beat\n\nText after it.\n';
    const ed = editor(body);
    // The empty attribute block sits right behind the heading.
    let attrs = -1;
    ed.state.doc.forEach((node, offset) => {
      if (node.type.name === 'beat_attrs') attrs = offset + 1;
    });
    ed.dispatch(ed.state.tr.insertText('at:: [[place]]', attrs));
    expect(ed.body()).toBe('## Beat 3 — A beat\nat:: [[place]]\n\nText after it.\n');
    ed.undo();
    expect(ed.body()).toBe(body);
  });

  it('replaces the whole body on select-all and typing, and undo restores it', () => {
    const body = 'One.\n\n## Two\n\nThree.\n';
    const ed = editor(body);
    ed.dispatch(ed.state.tr.setSelection(new AllSelection(ed.state.doc)).insertText('New body.'));
    expect(ed.body()).toBe('New body.');
    ed.undo();
    expect(ed.body()).toBe(body);
  });

  it('splits a paragraph with a blank line and joins it back', () => {
    const body = 'Before split here after.\n\nNext.\n';
    const ed = editor(body);
    ed.dispatch(ed.state.tr.setSelection(TextSelection.create(ed.state.doc, ed.pos(' here'))));
    splitBlock(ed.state, ed.dispatch);
    expect(ed.body()).toBe('Before split\n\n here after.\n\nNext.\n');
    joinBackward(ed.state, ed.dispatch);
    expect(ed.body()).toBe(body);
    expect(ed.level()).toBe('span');
  });

  it('pastes paragraphs into a paragraph as a paragraph break (the stock schema has no soft break)', () => {
    const ed = editor('Start.\n');
    const slice = new Slice(Fragment.from([
      schema.node('paragraph', null, [schema.text(' one')]),
      schema.node('paragraph', null, [schema.text('two')]),
    ]), 1, 1);
    ed.dispatch(ed.state.tr.setSelection(TextSelection.create(ed.state.doc, ed.pos('.', 1))).replaceSelection(slice));
    expect(ed.body()).toBe('Start. one\n\ntwo\n');
    expect(ed.level()).toBe('span');
  });

  it('keeps a typed trailing space, which Markdown cannot hold, as typed', () => {
    const ed = editor('Ends here.\n\nNext.\n');
    ed.dispatch(ed.state.tr.insertText(' ', ed.pos('here.', 5)));
    expect(ed.body()).toBe('Ends here. \n\nNext.\n');
    expect(ed.level()).toBe('span');
  });
});

describe('SourceBody: separators and the final newline at block boundaries', () => {
  for (const save of [false, true]) {
    it(`gives a paragraph made by Enter at the end of a paragraph its lines (${save ? 'with' : 'without'} a save before typing)`, () => {
      for (const body of ['Words here.\n\nNext.\n', 'A paragraph.\n\n## Heading two\n', 'Words here.\n', '## Heading\n']) {
        const ed = editor(body);
        const end = body.indexOf('\n');
        ed.dispatch(ed.state.tr.setSelection(TextSelection.create(ed.state.doc, ed.pos(body.slice(0, end), end))));
        splitBlock(ed.state, ed.dispatch);
        // The empty paragraph owns a blank line between two blank lines.
        if (save) expect(ed.body(), body).toBe(`${body.slice(0, end)}\n\n${body.slice(end)}`);
        ed.dispatch(ed.state.tr.insertText('New one.'));
        expect(ed.body(), body).toBe(`${body.slice(0, end)}\n\nNew one.${body.slice(end)}`);
        expect(ed.level()).toBe('span');
        expect(parseBody(ed.body()).eq(ed.state.doc)).toBe(true);
      }
    });
  }

  it('keeps the final newline when a block is added at the end, whatever step adds it', () => {
    for (const body of ['Words here.\n', '## Heading\n']) {
      const node = editor(body);
      node.dispatch(node.state.tr.insert(node.state.doc.content.size, schema.node('paragraph', null, [schema.text('New one.')])));
      expect(node.body()).toBe(`${body}\nNew one.\n`);
      const typed = editor(body);
      typed.dispatch(typed.state.tr.insertText('New one.', typed.state.doc.content.size));
      expect(typed.body()).toBe(`${body}\nNew one.\n`);
      expect(typed.level()).toBe('span');
    }
  });

  it('keeps a file without a final newline without one', () => {
    const ed = editor('Words here.');
    ed.dispatch(ed.state.tr.insert(ed.state.doc.content.size, schema.node('paragraph', null, [schema.text('New one.')])));
    expect(ed.body()).toBe('Words here.\n\nNew one.');
  });
});

describe('SourceBody: paragraph breaks inside a callout', () => {
  it('writes Enter inside a callout paragraph as a blank quote line, and Backspace joins it back', () => {
    const body = 'Intro.\n\n> [!design]\n> A remark. Over two\n> lines, _kept_.\n\nOutro.\n';
    const ed = editor(body);
    ed.dispatch(ed.state.tr.setSelection(TextSelection.create(ed.state.doc, ed.pos('Over'))));
    splitBlock(ed.state, ed.dispatch);
    expect(ed.body()).toBe('Intro.\n\n> [!design]\n> A remark. \n>\n> Over two\n> lines, _kept_.\n\nOutro.\n');
    expect(ed.level()).toBe('span');
    joinBackward(ed.state, ed.dispatch);
    expect(ed.body()).toBe(body);
  });
});

describe('SourceBody: fallback per block', () => {
  it('writes a block it cannot map again, leaves the others, and undo restores its bytes', () => {
    const body = 'Intro _kept_.\n\n- one\n- two items\n\nOutro _kept_.\n';
    const ed = editor(body);
    // Enter inside an item of a tight list: no Markdown holds a tight list with a two-paragraph item.
    ed.dispatch(ed.state.tr.setSelection(TextSelection.create(ed.state.doc, ed.pos('items'))));
    splitBlock(ed.state, ed.dispatch);
    const saved = ed.body();
    expect(ed.level()).toBe('block');
    expect(saved.startsWith('Intro _kept_.\n\n')).toBe(true);
    expect(saved.endsWith('\n\nOutro _kept_.\n')).toBe(true);
    ed.undo();
    expect(ed.body()).toBe(body);
    expect(ed.level()).toBe('span');
  });
});

describe('SourceBody: the level of joined and split rewritten blocks', () => {
  it('reports block while a joined or split block holds serializer bytes', () => {
    const joined = editor('Prev line.\n\nWords _under_ tail.\n');
    joined.dispatch(joined.state.tr.insertText('Y', joined.pos('under')));
    expect(joined.body()).toBe('Prev line.\n\nWords Y*under* tail.\n');
    joined.dispatch(joined.state.tr.setSelection(TextSelection.create(joined.state.doc, joined.posNow('Words'))));
    joinBackward(joined.state, joined.dispatch);
    expect(joined.body()).toBe('Prev line.Words Y*under* tail.\n');
    expect(joined.level()).toBe('block');

    const split = editor('Lead _a_ first. Then _more_ end.\n');
    split.dispatch(split.state.tr.insertText('Y', split.pos('a_')));
    split.dispatch(split.state.tr.setSelection(TextSelection.create(split.state.doc, split.posNow('Then'))));
    splitBlock(split.state, split.dispatch);
    split.dispatch(split.state.tr.delete(1, split.posNow('Then')));
    expect(split.body()).toBe('Then *more* end.\n');
    expect(split.level()).toBe('block');
  });
});

describe('SourceBody: a document no Markdown stands for', () => {
  it('keeps the other blocks when an attribute line stops being one, and undo brings the bytes back', () => {
    const body = 'Intro.\n\n## Beat 1 — A\nat:: [[place]]\nsays:: [[belief]]\nwhen:: T1\n\nOutro _kept_.\n';
    const ed = editor(body);
    // An upper-case letter in the key: the parser would no longer read the line as an attribute line.
    ed.dispatch(ed.state.tr.insertText('Q', ed.pos('says', 2)));
    expect(ed.body()).toBe(body.replace('says', 'saQys'));
    expect(ed.level()).toBe('block');
    // A later edit elsewhere in the same block is written with it.
    ed.dispatch(ed.state.tr.insertText('2', ed.posNow('T1', 2)));
    expect(ed.body()).toBe(body.replace('says', 'saQys').replace('T1', 'T12'));
    ed.dispatch(ed.state.tr.insertText('X', ed.posNow('Outro', 0)));
    expect(ed.body()).toBe(body.replace('says', 'saQys').replace('T1', 'T12').replace('Outro', 'XOutro'));
    ed.undo();
    expect(ed.body()).toBe(body);
    expect(ed.level()).toBe('span');
  });
});

describe('SourceBody: offsets', () => {
  it('maps offsets on the saved text, also after the first line of a bare beat', () => {
    const body = '## Beat 1 — A\n\nProse under it.\n';
    const ed = editor(body);
    let attrs = -1;
    ed.state.doc.forEach((node, offset) => {
      if (node.type.name === 'beat_attrs') attrs = offset + 1;
    });
    ed.dispatch(ed.state.tr.insertText('when:: T1', attrs));
    const saved = ed.body();
    const at = FRONT.length + saved.indexOf('under');
    const pos = ed.posNow('under');
    ed.dispatch(ed.state.tr.insertText('Q', pos));
    expect(ed.body().slice(at - FRONT.length, at - FRONT.length + 6)).toBe('Qunder');
  });
});
