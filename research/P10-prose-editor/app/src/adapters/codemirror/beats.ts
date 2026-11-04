import type { EditorState, Text } from '@codemirror/state';
import { BEAT_KEY_ORDER, type BeatKey } from '../../harness/types.ts';
import { bodyStart } from './frontmatter.ts';

/** A beat as the document has it: `## Beat <n> — <name>` and the attribute lines directly after it. */
export interface BeatBlock {
  beat: number;
  name: string;
  /** Whether the heading has the `— <name>` part; a heading without it stays raw. */
  dash: boolean;
  /** The heading line. */
  heading: { from: number; to: number };
  /** The attribute lines in order; `value` is what follows `key::` and its blanks. */
  params: Array<{ key: string; value: string }>;
  /** From the start of the first attribute line to the end of the last; undefined with no attribute line. */
  attrs: { from: number; to: number } | undefined;
}

const HEADING = /^## Beat (\d+)\b(?:[ \t]+—[ \t]*(.*))?/;
const ATTRIBUTE = /^([a-z]+)::([ \t]*)([\s\S]*)$/;

/** The attribute lines that follow the heading line (same rule as the oracle). */
function attributesAfter(doc: Text, headingLine: number) {
  const found: Array<{ key: string; gap: string; rest: string; from: number; to: number }> = [];
  for (let n = headingLine + 1; n <= doc.lines; n++) {
    const line = doc.line(n);
    const match = ATTRIBUTE.exec(line.text);
    if (match === null) break;
    found.push({ key: match[1] ?? '', gap: match[2] ?? '', rest: match[3] ?? '', from: line.from, to: line.to });
  }
  return found;
}

/** Every beat of the body, in document order. `isCode` tells whether a position is in code. */
export function findBeats(state: EditorState, isCode: (pos: number) => boolean): BeatBlock[] {
  const { doc } = state;
  const found: BeatBlock[] = [];
  for (let n = doc.lineAt(state.field(bodyStart)).number; n <= doc.lines; n++) {
    const line = doc.line(n);
    if (!line.text.startsWith('## Beat ')) continue;
    const match = HEADING.exec(line.text);
    if (match === null || isCode(line.from)) continue;
    const attributes = attributesAfter(doc, n);
    const first = attributes[0];
    const last = attributes[attributes.length - 1];
    found.push({
      beat: Number(match[1]),
      name: (match[2] ?? '').trim(),
      dash: match[2] !== undefined,
      heading: { from: line.from, to: line.to },
      params: attributes.map((a) => ({ key: a.key, value: a.rest })),
      attrs: first === undefined || last === undefined ? undefined : { from: first.from, to: last.to },
    });
    n += attributes.length;
  }
  return found;
}

/**
 * The change a `beat-param` edit makes: the value of an existing line, or a new line after the last
 * present key that precedes it in the order at, with, says, reveals, when (else after the heading).
 * Only that line changes.
 */
export function beatParamChange(doc: Text, beat: number, key: BeatKey, value: string): { from: number; to: number; insert: string } {
  const heading = new RegExp(`^## Beat ${beat}\\b`);
  let headingLine = 0;
  for (let n = 1; n <= doc.lines; n++) {
    if (heading.test(doc.line(n).text)) {
      headingLine = n;
      break;
    }
  }
  if (headingLine === 0) throw new Error(`beat-param: there is no "## Beat ${beat}" heading`);
  const attributes = attributesAfter(doc, headingLine);
  const existing = attributes.find((a) => a.key === key);
  if (existing !== undefined) {
    const from = existing.from + existing.key.length + 2 + existing.gap.length;
    const insert = existing.gap === '' && existing.rest === '' ? ` ${value}` : value;
    return { from, to: existing.to, insert };
  }
  const order: readonly string[] = BEAT_KEY_ORDER;
  const rank = order.indexOf(key);
  let after = doc.line(headingLine).to;
  let afterRank = -1;
  for (const a of attributes) {
    const r = order.indexOf(a.key);
    if (r > afterRank && r < rank) {
      afterRank = r;
      after = a.to;
    }
  }
  const line = `${key}:: ${value}`;
  if (after < doc.length) return { from: after + 1, to: after + 1, insert: `${line}\n` };
  return { from: doc.length, to: doc.length, insert: `\n${line}` };
}
