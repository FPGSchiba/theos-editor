import { describe, expect, it } from 'vitest';
import {
  editOffset, expectedAfter, expectedAfterExternalUndo, expectedSites, NOTE_STATE, sites, toLf,
} from './oracle.ts';
import type { ScriptedEdit } from './types.ts';

type WithoutId<T> = T extends unknown ? Omit<T, 'id'> : never;
const edit = (e: WithoutId<ScriptedEdit>): ScriptedEdit => ({ id: 't', ...e }) as ScriptedEdit;

const FM = '---\nid: x\n---\n';

describe('toLf', () => {
  it('turns CRLF and lone CR into LF', () => {
    expect(toLf('a\r\nb\rc\nd')).toBe('a\nb\nc\nd');
  });
});

describe('expectedAfter insert and delete', () => {
  it('insert puts the text at the anchor', () => {
    const e = edit({ kind: 'insert', at: { find: 'two', offset: 3 }, text: 'X' });
    expect(expectedAfter('one two three', e)).toBe('one twoX three');
  });

  it('insert at the end of a file without a final newline adds no newline', () => {
    const e = edit({ kind: 'insert', at: { find: 'end', offset: 3 }, text: '!' });
    expect(expectedAfter('the end', e)).toBe('the end!');
  });

  it('insert keeps U+00A0 and decomposed letters as given', () => {
    const e = edit({ kind: 'insert', at: { find: 'a', offset: 1 }, text: '\u00a0e\u0301' });
    expect(expectedAfter('ab', e)).toBe('a\u00a0e\u0301b');
  });

  it('delete forward removes length characters from the anchor', () => {
    const e = edit({ kind: 'delete', at: { find: 'two ' }, length: 4 });
    expect(expectedAfter('one two three', e)).toBe('one three');
  });

  it('delete forward stops at the end of the text', () => {
    const e = edit({ kind: 'delete', at: { find: 'three' }, length: 99 });
    expect(expectedAfter('one three', e)).toBe('one ');
  });

  it('delete backward removes the characters before the anchor', () => {
    const e = edit({ kind: 'delete', at: { find: 'three' }, length: 4, backward: true });
    expect(expectedAfter('one two three', e)).toBe('one three');
  });

  it('delete backward clips at the body start', () => {
    const text = `${FM}Body`;
    const e = edit({ kind: 'delete', at: { find: 'Body' }, length: 3, backward: true });
    expect(expectedAfter(text, e)).toBe(text);
  });

  it('delete backward from inside the first word removes only body characters', () => {
    const text = `${FM}Body`;
    const e = edit({ kind: 'delete', at: { find: 'Body', offset: 2 }, length: 9, backward: true });
    expect(expectedAfter(text, e)).toBe(`${FM}dy`);
  });

  it('works on the LF form of a CRLF original', () => {
    const e = edit({ kind: 'insert', at: { find: 'b' }, text: 'X' });
    expect(expectedAfter('a\r\nb\r\n', e)).toBe('a\nXb\n');
  });
});

describe('expectedAfter beat-param', () => {
  it('changes the value and keeps the spaces after ::', () => {
    const e = edit({ kind: 'beat-param', beat: 1, key: 'at', value: '[[y]]' });
    expect(expectedAfter('## Beat 1 — A\nat::  [[x]]\n', e)).toBe('## Beat 1 — A\nat::  [[y]]\n');
  });

  it('changes only the asked beat, and beat 1 is not beat 10', () => {
    const text = '## Beat 10 — X\nat:: [[a]]\n## Beat 1 — Y\nat:: [[b]]\n';
    const e = edit({ kind: 'beat-param', beat: 1, key: 'at', value: '[[c]]' });
    expect(expectedAfter(text, e)).toBe('## Beat 10 — X\nat:: [[a]]\n## Beat 1 — Y\nat:: [[c]]\n');
  });

  it('gives an empty value one space after ::', () => {
    const e = edit({ kind: 'beat-param', beat: 1, key: 'at', value: '[[y]]' });
    expect(expectedAfter('## Beat 1 — A\nat::\nProse\n', e)).toBe('## Beat 1 — A\nat:: [[y]]\nProse\n');
  });

  it('adds a key after the last preceding key in BEAT_KEY_ORDER', () => {
    const text = '## Beat 1 — A\nat:: [[x]]\nwhen:: T1\nProse\n';
    const e = edit({ kind: 'beat-param', beat: 1, key: 'with', value: '[[n]]' });
    expect(expectedAfter(text, e)).toBe('## Beat 1 — A\nat:: [[x]]\nwith:: [[n]]\nwhen:: T1\nProse\n');
  });

  it('adds after the highest preceding key when several precede', () => {
    const text = '## Beat 1 — A\nat:: [[x]]\nwith:: [[n]]\nwhen:: T1\n';
    const e = edit({ kind: 'beat-param', beat: 1, key: 'says', value: '[[b]]' });
    expect(expectedAfter(text, e)).toBe(
      '## Beat 1 — A\nat:: [[x]]\nwith:: [[n]]\nsays:: [[b]]\nwhen:: T1\n',
    );
  });

  it('goes by BEAT_KEY_ORDER, not by the order of the lines, when the keys are out of order', () => {
    const text = '## Beat 1 — A\nwith:: [[n]]\nat:: [[x]]\n';
    const e = edit({ kind: 'beat-param', beat: 1, key: 'says', value: '[[b]]' });
    expect(expectedAfter(text, e)).toBe('## Beat 1 — A\nwith:: [[n]]\nsays:: [[b]]\nat:: [[x]]\n');
  });

  it('adds directly after the heading when no existing key precedes', () => {
    const text = '## Beat 1 — A\nwhen:: T1\nProse\n';
    const e = edit({ kind: 'beat-param', beat: 1, key: 'at', value: '[[x]]' });
    expect(expectedAfter(text, e)).toBe('## Beat 1 — A\nat:: [[x]]\nwhen:: T1\nProse\n');
  });

  it('adds after the heading when the beat has no attributes', () => {
    const text = '## Beat 1 — A\nProse\n\n## Beat 2 — B\nat:: [[z]]\n';
    const e = edit({ kind: 'beat-param', beat: 1, key: 'when', value: 'T2' });
    expect(expectedAfter(text, e)).toBe('## Beat 1 — A\nwhen:: T2\nProse\n\n## Beat 2 — B\nat:: [[z]]\n');
  });

  it('adds a line after a last line that has no newline', () => {
    const e = edit({ kind: 'beat-param', beat: 1, key: 'at', value: '[[x]]' });
    expect(expectedAfter('## Beat 1 — A', e)).toBe('## Beat 1 — A\nat:: [[x]]');
  });

  it('does not take prose after the attribute lines for attributes', () => {
    const text = '## Beat 1 — A\nat:: [[x]]\n\nwhen:: not an attribute\n';
    const e = edit({ kind: 'beat-param', beat: 1, key: 'when', value: 'T3' });
    expect(expectedAfter(text, e)).toBe('## Beat 1 — A\nat:: [[x]]\nwhen:: T3\n\nwhen:: not an attribute\n');
  });

  it('throws when the beat does not exist', () => {
    const e = edit({ kind: 'beat-param', beat: 7, key: 'at', value: '[[x]]' });
    expect(() => expectedAfter('## Beat 1 — A\n', e)).toThrow(/Beat 7/);
  });
});

describe('expectedAfter note-state', () => {
  it('replaces the last segment', () => {
    const e = edit({ kind: 'note-state', note: 'n3', state: 'dropped' });
    expect(expectedAfter('> [!note] n3 · [[f04-terrain-v2]] · open\n', e)).toBe(
      '> [!note] n3 · [[f04-terrain-v2]] · dropped\n',
    );
  });

  it('replaces a resolved-by segment', () => {
    const e = edit({ kind: 'note-state', note: 'n4', state: 'open' });
    expect(expectedAfter('> [!note] n4 · resolved by [[fates-shrine-v5]]\n', e)).toBe(
      '> [!note] n4 · open\n',
    );
  });

  it('writes a resolved-by state', () => {
    const e = edit({ kind: 'note-state', note: 'n3', state: 'resolved by [[f04-terrain-v3]]' });
    expect(expectedAfter('> [!note] n3 · [[f04-terrain-v2]] · open\n> Body\n', e)).toBe(
      '> [!note] n3 · [[f04-terrain-v2]] · resolved by [[f04-terrain-v3]]\n> Body\n',
    );
  });

  it('picks the note by its whole id', () => {
    const text = '> [!note] n30 · open\n\n> [!note] n3 · open\n';
    const e = edit({ kind: 'note-state', note: 'n3', state: 'dropped' });
    expect(expectedAfter(text, e)).toBe('> [!note] n30 · open\n\n> [!note] n3 · dropped\n');
  });

  it('appends the state when the title has none', () => {
    const e = edit({ kind: 'note-state', note: 'n5', state: 'open' });
    expect(expectedAfter('> [!note] n5\n', e)).toBe('> [!note] n5 · open\n');
    expect(expectedAfter('> [!note] n5 · [[f04-terrain-v2]]\n', e)).toBe(
      '> [!note] n5 · [[f04-terrain-v2]] · open\n',
    );
  });

  it('throws when the note does not exist', () => {
    const e = edit({ kind: 'note-state', note: 'n9', state: 'open' });
    expect(() => expectedAfter('> [!note] n3 · open\n', e)).toThrow(/n9/);
  });
});

describe('expectedAfter links', () => {
  it('link-insert inserts [[pick]] at the anchor', () => {
    const e = edit({ kind: 'link-insert', at: { find: 'see ', offset: 4 }, query: 'te', pick: 'f04-terrain-v2' });
    expect(expectedAfter('see  now', e)).toBe('see [[f04-terrain-v2]] now');
  });

  it('link-delete removes through the closing brackets', () => {
    const e = edit({ kind: 'link-delete', link: { find: '[[' } });
    expect(expectedAfter('see [[a|b]] now', e)).toBe('see  now');
  });

  it('link-delete removes the asked link only', () => {
    const e = edit({ kind: 'link-delete', link: { find: '[[', occurrence: 2 } });
    expect(expectedAfter('[[a]] and [[b]] end', e)).toBe('[[a]] and  end');
  });

  it('link-delete throws when the anchor is not at a link', () => {
    const e = edit({ kind: 'link-delete', link: { find: 'and' } });
    expect(() => expectedAfter('[[a]] and [[b]]', e)).toThrow();
  });
});

describe('expectedAfter select-all-type, paste, external, undo-all', () => {
  it('select-all-type keeps the front matter and replaces the body', () => {
    const e = edit({ kind: 'select-all-type', text: 'New body' });
    expect(expectedAfter(`${FM}Old body\n\nMore\n`, e)).toBe(`${FM}New body`);
  });

  it('select-all-type without front matter replaces everything', () => {
    const e = edit({ kind: 'select-all-type', text: 'New' });
    expect(expectedAfter('Old\n', e)).toBe('New');
  });

  it('paste inserts the plain text with LF line ends', () => {
    const e = edit({ kind: 'paste', at: { find: 'two' }, plain: 'a\r\nb' });
    expect(expectedAfter('one two', e)).toBe('one a\nbtwo');
  });

  it('paste ignores the html', () => {
    const e = edit({ kind: 'paste', at: { find: 'two' }, plain: 'plain', html: '<p><b>rich</b></p>' });
    expect(expectedAfter('one two', e)).toBe('one plaintwo');
  });

  it('external applies both inserts', () => {
    const e = edit({
      kind: 'external',
      local: { at: { find: 'AAA', offset: 3 }, text: 'L' },
      external: { at: { find: 'CCC', offset: 3 }, text: 'E' },
    });
    expect(expectedAfter('AAA\nBBB\nCCC\n', e)).toBe('AAAL\nBBB\nCCCE\n');
  });

  it('external applies both inserts when the external one comes first in the text', () => {
    const e = edit({
      kind: 'external',
      local: { at: { find: 'CCC', offset: 3 }, text: 'L' },
      external: { at: { find: 'AAA', offset: 3 }, text: 'E' },
    });
    expect(expectedAfter('AAA\nBBB\nCCC\n', e)).toBe('AAAE\nBBB\nCCCL\n');
  });

  it('external throws when both anchors resolve to the same offset', () => {
    const e = edit({
      kind: 'external',
      local: { at: { find: 'AAA' }, text: 'L' },
      external: { at: { find: 'AAA' }, text: 'E' },
    });
    expect(() => expectedAfter('AAA\n', e)).toThrow(/same offset/);
  });

  it('expectedAfterExternalUndo keeps only the external insert', () => {
    const e: ScriptedEdit & { kind: 'external' } = {
      id: 't',
      kind: 'external',
      local: { at: { find: 'AAA', offset: 3 }, text: 'L' },
      external: { at: { find: 'CCC', offset: 3 }, text: 'E' },
    };
    expect(expectedAfterExternalUndo('AAA\nBBB\nCCC\n', e)).toBe('AAA\nBBB\nCCCE\n');
  });

  it('undo-all expects the original', () => {
    const e = edit({
      kind: 'undo-all',
      edits: [
        { id: 'a', kind: 'insert', at: { find: 'one' }, text: 'X' },
        { id: 'b', kind: 'delete', at: { find: 'two' }, length: 1 },
      ],
    });
    expect(expectedAfter('one two\n', e)).toBe('one two\n');
  });
});

describe('editOffset', () => {
  it('gives the anchor offset for insert', () => {
    const e = edit({ kind: 'insert', at: { find: 'two', offset: 1 }, text: 'X' });
    expect(editOffset('one two', e)).toBe(5);
  });

  it('gives the anchor offset for paste, link-insert and link-delete', () => {
    expect(editOffset('one two', edit({ kind: 'paste', at: { find: 'two' }, plain: 'p' }))).toBe(4);
    expect(
      editOffset('one two', edit({ kind: 'link-insert', at: { find: 'two' }, query: 'q', pick: 'p' })),
    ).toBe(4);
    expect(editOffset('one [[a]]', edit({ kind: 'link-delete', link: { find: '[[' } }))).toBe(4);
  });

  it('gives the anchor for a forward delete and anchor minus length for a backward one', () => {
    const text = `${FM}one two three`;
    const at = text.indexOf('three');
    expect(editOffset(text, edit({ kind: 'delete', at: { find: 'three' }, length: 2 }))).toBe(at);
    expect(editOffset(text, edit({ kind: 'delete', at: { find: 'three' }, length: 4, backward: true }))).toBe(
      at - 4,
    );
  });

  it('clips a backward delete at the body start', () => {
    const e = edit({ kind: 'delete', at: { find: 'Body' }, length: 3, backward: true });
    expect(editOffset(`${FM}Body`, e)).toBe(FM.length);
  });

  it('gives the start of the at:: line for a beat-param change', () => {
    const text = 'Intro\n\n## Beat 1 — A\nwhen:: T1\nat:: [[x]]\n';
    const e = edit({ kind: 'beat-param', beat: 1, key: 'at', value: '[[y]]' });
    expect(editOffset(text, e)).toBe(text.indexOf('at::'));
  });

  it('gives the start of the new line for a beat-param add', () => {
    const text = '## Beat 1 — A\nat:: [[x]]\nwhen:: T1\n';
    const e = edit({ kind: 'beat-param', beat: 1, key: 'with', value: '[[n]]' });
    expect(editOffset(text, e)).toBe(text.indexOf('when::'));
  });

  it('gives the start of the title line for a note-state change', () => {
    const text = 'Intro\n\n> [!note] n3 · open\n';
    const e = edit({ kind: 'note-state', note: 'n3', state: 'dropped' });
    expect(editOffset(text, e)).toBe(text.indexOf('> [!note]'));
  });

  it('gives the body start for select-all-type', () => {
    expect(editOffset(`${FM}Body`, edit({ kind: 'select-all-type', text: 'x' }))).toBe(FM.length);
    expect(editOffset('Body', edit({ kind: 'select-all-type', text: 'x' }))).toBe(0);
  });

  it('gives the smaller anchor offset for external', () => {
    const e = edit({
      kind: 'external',
      local: { at: { find: 'CCC' }, text: 'L' },
      external: { at: { find: 'AAA' }, text: 'E' },
    });
    expect(editOffset('AAA\nBBB\nCCC\n', e)).toBe(0);
    const swapped = edit({
      kind: 'external',
      local: { at: { find: 'AAA', offset: 1 }, text: 'L' },
      external: { at: { find: 'CCC' }, text: 'E' },
    });
    expect(editOffset('AAA\nBBB\nCCC\n', swapped)).toBe(1);
  });

  it('gives undefined for undo-all', () => {
    const e = edit({ kind: 'undo-all', edits: [{ id: 'a', kind: 'insert', at: { find: 'one' }, text: 'X' }] });
    expect(editOffset('one', e)).toBeUndefined();
  });

  it('works on the LF form of a CRLF original', () => {
    const e = edit({ kind: 'insert', at: { find: 'b' }, text: 'X' });
    expect(editOffset('a\r\nb', e)).toBe(2);
  });
});

describe('sites', () => {
  const text = 'AAA\nBBB\nCCC\n';

  it('is the edit offset for a single-site edit', () => {
    expect(sites(text, edit({ kind: 'insert', at: { find: 'BBB', offset: 1 }, text: 'X' }))).toEqual([5]);
    expect(sites(`${FM}Body`, edit({ kind: 'select-all-type', text: 'x' }))).toEqual([FM.length]);
    expect(sites(text, edit({ kind: 'delete', at: { find: 'CCC' }, length: 2 }))).toEqual([8]);
  });

  it('is the local and then the external anchor for external, in that order', () => {
    const e = edit({
      kind: 'external',
      local: { at: { find: 'CCC' }, text: 'L' },
      external: { at: { find: 'AAA', offset: 3 }, text: 'E' },
    });
    expect(sites(text, e)).toEqual([8, 3]);
  });

  it('is the site of every inner step, resolved against the original, for undo-all', () => {
    const e = edit({
      kind: 'undo-all',
      edits: [
        { id: 'a', kind: 'insert', at: { find: 'CCC' }, text: 'XXXX' },
        { id: 'b', kind: 'delete', at: { find: 'AAA' }, length: 2 },
        { id: 'c', kind: 'insert', at: { find: 'BBB', offset: 3 }, text: 'Y' },
      ],
    });
    expect(sites(text, e)).toEqual([8, 0, 7]);
  });

  it('clips the site of a backward inner delete at the body start', () => {
    const e = edit({
      kind: 'undo-all',
      edits: [{ id: 'a', kind: 'delete', at: { find: 'AAA', offset: 1 }, length: 5, backward: true }],
    });
    expect(sites(`${FM}AAA\n`, e)).toEqual([FM.length]);
  });

  it('is empty for an undo-all without steps', () => {
    expect(sites(text, edit({ kind: 'undo-all', edits: [] }))).toEqual([]);
  });

  it('works on the LF form of a CRLF original', () => {
    expect(sites('a\r\nb\r\nc', edit({ kind: 'insert', at: { find: 'c' }, text: 'X' }))).toEqual([4]);
  });
});

describe('expectedSites', () => {
  const text = 'AAA\nBBB\nCCC\n';
  /** The local insert comes later in the text than the external one. */
  const laterLocal = edit({
    kind: 'external',
    local: { at: { find: 'CCC', offset: 3 }, text: 'L' },
    external: { at: { find: 'AAA', offset: 3 }, text: 'EXT' },
  });
  /** The local insert comes earlier in the text than the external one. */
  const earlierLocal = edit({
    kind: 'external',
    local: { at: { find: 'AAA', offset: 1 }, text: 'LL' },
    external: { at: { find: 'CCC', offset: 3 }, text: 'E' },
  });

  it('equals sites for every kind but external', () => {
    const e = edit({ kind: 'insert', at: { find: 'BBB' }, text: 'X' });
    expect(expectedSites(text, e)).toEqual(sites(text, e));
    const undo = edit({ kind: 'undo-all', edits: [{ id: 'a', kind: 'insert', at: { find: 'CCC' }, text: 'X' }] });
    expect(expectedSites(text, undo)).toEqual(sites(text, undo));
  });

  it('moves a site behind an insert that comes before it, after an external edit', () => {
    // 'AAAEXT\nBBB\nCCCL\n': the local L sits at 14, the external EXT at 3.
    expect(expectedAfter(text, laterLocal)).toBe('AAAEXT\nBBB\nCCCL\n');
    expect(expectedSites(text, laterLocal)).toEqual([14, 3]);
    // 'ALLAA\nBBB\nCCCE\n': LL at 1, E at 11 + 2.
    expect(expectedAfter(text, earlierLocal)).toBe('ALLAA\nBBB\nCCCE\n');
    expect(expectedSites(text, earlierLocal)).toEqual([1, 13]);
  });

  it('puts the local site where the leftover local text would be after one undo', () => {
    // Only EXT is in: 'AAAEXT\nBBB\nCCC\n'; the local text would be at the end of CCC (14).
    expect(expectedSites(text, laterLocal, 'undo')).toEqual([14, 3]);
    // Only E is in, behind the local site: 'AAA\nBBB\nCCCE\n'.
    expect(expectedSites(text, earlierLocal, 'undo')).toEqual([1, 11]);
  });
});

describe('NOTE_STATE', () => {
  it('matches the three note states and nothing else', () => {
    for (const state of ['open', 'dropped', 'resolved by [[f04-terrain-v1]]']) expect(NOTE_STATE.test(state)).toBe(true);
    for (const other of ['n1', '[[f04-terrain-v1]]', 'resolved', 'open now']) expect(NOTE_STATE.test(other)).toBe(false);
  });
});
