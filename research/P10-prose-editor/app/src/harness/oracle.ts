import { resolveAnchor } from './anchor.ts';
import { splitFrontMatter } from './frontmatter.ts';
import { BEAT_KEY_ORDER } from './types.ts';
import type { ScriptedEdit, TextChange } from './types.ts';

/**
 * The oracle: the expected file is the original with the same text change applied to the
 * source by the test, never by the editor (spec §4.6). Every function works on the LF form of
 * the original, because offsets are UTF-16 offsets into the LF form (the CRLF corpus file is
 * expected to equal its LF form). Anchors resolve against the original, not step by step.
 */

/** The LF form of a text: CRLF and a lone CR become LF. */
export function toLf(text: string): string {
  return text.replace(/\r\n?/g, '\n');
}

/** The text change an edit stands for, and where that change starts in the original. */
interface Plan {
  changes: TextChange[];
  offset: number | undefined;
}

interface Line {
  start: number;
  /** Offset of the line's newline, or the text length for the last line. */
  end: number;
  text: string;
}

function splitLines(text: string): Line[] {
  const lines: Line[] = [];
  let start = 0;
  for (;;) {
    const newline = text.indexOf('\n', start);
    if (newline === -1) {
      lines.push({ start, end: text.length, text: text.slice(start) });
      return lines;
    }
    lines.push({ start, end: newline, text: text.slice(start, newline) });
    start = newline + 1;
  }
}

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

const single = (change: TextChange, offset: number = change.from): Plan => ({ changes: [change], offset });
const insertAt = (at: number, insert: string): TextChange => ({ from: at, to: at, insert });

function applyChanges(text: string, changes: TextChange[]): string {
  // Last change first, so the offsets of the earlier ones stay valid.
  return [...changes]
    .sort((a, b) => b.from - a.from)
    .reduce((acc, change) => acc.slice(0, change.from) + change.insert + acc.slice(change.to), text);
}

/** `beat-param`: change the value of an attribute line, or add the line in BEAT_KEY_ORDER. */
function planBeatParam(text: string, edit: ScriptedEdit & { kind: 'beat-param' }): Plan {
  const lines = splitLines(text);
  const heading = new RegExp(`^## Beat ${edit.beat}\\b`);
  const headingIndex = lines.findIndex((line) => heading.test(line.text));
  const headingLine = lines[headingIndex];
  if (headingLine === undefined) throw new Error(`beat-param: there is no "## Beat ${edit.beat}" heading`);

  // The attribute lines are the lines directly after the heading that look like `key:: value`.
  const attributes: Array<{ key: string; gap: string; rest: string; line: Line }> = [];
  for (let i = headingIndex + 1; i < lines.length; i++) {
    const line = lines[i];
    const match = line === undefined ? null : /^([a-z]+)::([ \t]*)([\s\S]*)$/.exec(line.text);
    if (line === undefined || match === null) break;
    attributes.push({ key: match[1] ?? '', gap: match[2] ?? '', rest: match[3] ?? '', line });
  }

  const existing = attributes.find((attribute) => attribute.key === edit.key);
  if (existing !== undefined) {
    // Keep `key::` and the spaces after it; an empty value gets one space.
    const valueStart = existing.line.start + existing.key.length + 2 + existing.gap.length;
    const bare = existing.gap === '' && existing.rest === '';
    const insert = bare ? ` ${edit.value}` : edit.value;
    return single({ from: valueStart, to: existing.line.end, insert }, existing.line.start);
  }

  // A new line goes after the last existing key that precedes it in BEAT_KEY_ORDER, or directly
  // after the heading when none does.
  const order: readonly string[] = BEAT_KEY_ORDER;
  const rank = order.indexOf(edit.key);
  let after = headingLine;
  let afterRank = -1;
  for (const attribute of attributes) {
    const attributeRank = order.indexOf(attribute.key);
    if (attributeRank > afterRank && attributeRank < rank) {
      afterRank = attributeRank;
      after = attribute.line;
    }
  }
  const newLine = `${edit.key}:: ${edit.value}`;
  return single(
    after.end < text.length
      ? insertAt(after.end + 1, `${newLine}\n`)
      : insertAt(text.length, `\n${newLine}`),
  );
}

const NOTE_STATE = /^(?:open|dropped|resolved by \[\[[^\]]*\]\])$/;

/** `note-state`: replace the last segment of the note's title line with the new state. */
function planNoteState(text: string, edit: ScriptedEdit & { kind: 'note-state' }): Plan {
  const title = new RegExp(`^> \\[!note\\] ${escapeRegExp(edit.note)}(?: · .*)?$`, 's');
  const line = splitLines(text).find((candidate) => title.test(candidate.text));
  if (line === undefined) throw new Error(`note-state: there is no note ${JSON.stringify(edit.note)}`);

  // Segment 0 is `> [!note] <id>`. The last segment is replaced when it is a state; a title
  // without a state (id only, or id and version) gets the state appended.
  const segments = line.text.split(' · ');
  const last = segments.length > 1 ? segments[segments.length - 1] : undefined;
  if (last !== undefined && NOTE_STATE.test(last)) segments[segments.length - 1] = edit.state;
  else segments.push(edit.state);
  return single({ from: line.start, to: line.end, insert: segments.join(' · ') });
}

/** `link-delete`: remove the link from its `[[` through the closing `]]`. */
function planLinkDelete(text: string, edit: ScriptedEdit & { kind: 'link-delete' }): Plan {
  const from = resolveAnchor(text, edit.link);
  if (!text.startsWith('[[', from)) throw new Error('link-delete: the anchor does not point at a "[["');
  const close = text.indexOf(']]', from + 2);
  if (close === -1) throw new Error('link-delete: the link has no closing "]]"');
  return single({ from, to: close + 2, insert: '' });
}

/** The two inserts of an `external` edit, resolved against the original. */
function externalChanges(
  text: string,
  edit: ScriptedEdit & { kind: 'external' },
): { local: TextChange; external: TextChange } {
  const local = insertAt(resolveAnchor(text, edit.local.at), edit.local.text);
  const external = insertAt(resolveAnchor(text, edit.external.at), edit.external.text);
  if (local.from === external.from) {
    throw new Error('external: the local and the external anchor resolve to the same offset');
  }
  return { local, external };
}

function plan(text: string, edit: ScriptedEdit): Plan {
  switch (edit.kind) {
    case 'insert':
      return single(insertAt(resolveAnchor(text, edit.at), edit.text));
    case 'delete': {
      const at = resolveAnchor(text, edit.at);
      if (edit.backward === true) {
        // Backspace never reaches into the front matter.
        const from = Math.min(at, Math.max(at - edit.length, splitFrontMatter(text).frontMatter.length));
        return single({ from, to: at, insert: '' });
      }
      return single({ from: at, to: Math.min(at + edit.length, text.length), insert: '' });
    }
    case 'beat-param':
      return planBeatParam(text, edit);
    case 'note-state':
      return planNoteState(text, edit);
    case 'link-insert':
      return single(insertAt(resolveAnchor(text, edit.at), `[[${edit.pick}]]`));
    case 'link-delete':
      return planLinkDelete(text, edit);
    case 'select-all-type':
      return single({ from: splitFrontMatter(text).frontMatter.length, to: text.length, insert: edit.text });
    case 'paste':
      return single(insertAt(resolveAnchor(text, edit.at), toLf(edit.plain)));
    case 'external': {
      const { local, external } = externalChanges(text, edit);
      return { changes: [local, external], offset: Math.min(local.from, external.from) };
    }
    case 'undo-all':
      return { changes: [], offset: undefined };
  }
}

/** The file the edit should leave behind, as the LF form of `original` with the change applied. */
export function expectedAfter(original: string, edit: ScriptedEdit): string {
  const text = toLf(original);
  return applyChanges(text, plan(text, edit).changes);
}

/**
 * After an `external` edit and an undo of the local edit: the original with only the external
 * insert applied.
 */
export function expectedAfterExternalUndo(
  original: string,
  edit: ScriptedEdit & { kind: 'external' },
): string {
  const text = toLf(original);
  return applyChanges(text, [externalChanges(text, edit).external]);
}

/**
 * Where the edit's change starts in the LF form of `original`: the anchor for anchored kinds
 * (for a backward delete the anchor minus the length, clipped at the body start), the start of
 * the changed line for `beat-param` and `note-state`, the body start for `select-all-type`, the
 * smaller anchor for `external`, and `undefined` for `undo-all`.
 */
export function editOffset(original: string, edit: ScriptedEdit): number | undefined {
  return plan(toLf(original), edit).offset;
}
