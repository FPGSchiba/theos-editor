import type MarkdownIt from 'markdown-it';
import type { StateCore, Token } from 'markdown-it';
import type { MarkdownSerializerState } from 'prosemirror-markdown';
import type { Node, NodeSpec } from 'prosemirror-model';
import { BEAT_KEY_ORDER, type BeatKey } from '../../harness/types.ts';

/**
 * Beats as document nodes: `## Beat <n> — <name>` and the `key:: value` lines directly under it.
 *
 * - `beat_heading` holds the heading line behind `## ` as raw text (code-like), so the line
 *   is written back exactly; the node view draws `<n> · <name>` instead of it.
 * - `beat_attrs` (attr `beat`) holds the attribute lines as raw text joined by `\n`, one line each,
 *   extra spaces after `::` included; the node view draws the shared `BeatRow`. A beat with no
 *   attribute line has an empty `beat_attrs`, which the serializer does not write.
 *
 * The markdown-it core rule below cuts both out of the token stream; the serializer rules write
 * the lines back with a single newline between heading and lines. That is the feature itself, not
 * source preservation: whatever the stock serializer does around the beat stays as it is.
 */

/** A beat heading as the oracle reads it (a top-level ATX `## Beat <n>` line). */
const HEADING_LINE = /^## Beat (\d+)\b/;
/** The text of a heading node: the number, and the name behind the em dash (absent: no dash). */
const HEADING_TEXT = /^Beat (\d+)\b(?:[ \t]+—[ \t]*(.*))?/;
const ATTRIBUTE = /^([a-z]+)::([ \t]*)(.*)$/;

export interface BeatHeadingInfo {
  beat: number;
  name: string;
  /** Whether the heading has the `— <name>` part; one without it stays raw. */
  dash: boolean;
}

/** What the text of a `beat_heading` says; undefined when it no longer starts with `Beat <n>`. */
export function readHeading(text: string): BeatHeadingInfo | undefined {
  const found = HEADING_TEXT.exec(text);
  if (found === null) return undefined;
  return { beat: Number(found[1]), name: (found[2] ?? '').trim(), dash: found[2] !== undefined };
}

export interface AttributeLine {
  key: string;
  /** The blanks after `::`. */
  gap: string;
  /** The value behind the blanks. */
  rest: string;
  /** Offsets of the line in the text of the node. */
  from: number;
  to: number;
}

/** The attribute lines at the start of the node text: consecutive `key:: value` lines (the oracle's rule). */
export function attributeLines(text: string): AttributeLine[] {
  const found: AttributeLine[] = [];
  let from = 0;
  for (const line of text.split('\n')) {
    const match = ATTRIBUTE.exec(line);
    if (match === null) break;
    found.push({ key: match[1] ?? '', gap: match[2] ?? '', rest: match[3] ?? '', from, to: from + line.length });
    from += line.length + 1;
  }
  return found;
}

/**
 * The change a `beat-param` edit makes to the text of a `beat_attrs` node (the rule of the oracle):
 * the value of an existing line keeps `key::` and its blanks; a new line goes after the last present
 * key that precedes it in at, with, says, reveals, when, else before the first line.
 */
export function beatParamChange(text: string, key: BeatKey, value: string): { from: number; to: number; insert: string } {
  const lines = attributeLines(text);
  const existing = lines.find((line) => line.key === key);
  if (existing !== undefined) {
    const insert = existing.gap === '' && existing.rest === '' ? ` ${value}` : value;
    return { from: existing.from + existing.key.length + 2 + existing.gap.length, to: existing.to, insert };
  }
  const order: readonly string[] = BEAT_KEY_ORDER;
  const rank = order.indexOf(key);
  let after: AttributeLine | undefined;
  let afterRank = -1;
  for (const line of lines) {
    const lineRank = order.indexOf(line.key);
    if (lineRank > afterRank && lineRank < rank) {
      afterRank = lineRank;
      after = line;
    }
  }
  const added = `${key}:: ${value}`;
  if (after !== undefined) return { from: after.to, to: after.to, insert: `\n${added}` };
  return { from: 0, to: 0, insert: text === '' ? added : `${added}\n` };
}

export const beatHeadingSpec: NodeSpec = {
  group: 'block',
  content: 'text*',
  marks: '',
  code: true,
  defining: true,
  // The node view draws the heading; this is only what a DOM serializer would write.
  toDOM: () => ['h2', { class: 'pm-beat-heading' }, 0],
};

export const beatAttrsSpec: NodeSpec = {
  group: 'block',
  content: 'text*',
  marks: '',
  code: true,
  defining: true,
  // `tight`: the block that follows starts on the very next line (no blank line between).
  attrs: { beat: { default: 0 }, tight: { default: false } },
  toDOM: () => ['div', { class: 'pm-beat-attrs' }, 0],
};

type Lines = [number, number];

function block(state: StateCore, type: string, nesting: 1 | 0 | -1, map: Lines | null, meta?: unknown): Token {
  const token = new state.Token(type, '', nesting);
  token.map = map;
  token.block = true;
  if (meta !== undefined) token.meta = meta;
  return token;
}

function rawText(state: StateCore, text: string): Token {
  const inline = new state.Token('inline', '', 0);
  inline.level = 1;
  inline.content = text;
  inline.children = [];
  if (text !== '') {
    const child = new state.Token('text', '', 0);
    child.content = text;
    inline.children.push(child);
  }
  return inline;
}

const isBreak = (token: Token): boolean => token.type === 'softbreak' || token.type === 'hardbreak';

/** Rewrites every top-level beat heading, and the attribute lines directly under it, of the token stream. */
function rewrite(state: StateCore): void {
  const tokens = state.tokens;
  const lines = state.src.split('\n');
  for (let at = 0; at < tokens.length; at++) {
    const open = tokens[at]!;
    if (open.type !== 'heading_open' || open.level !== 0 || open.markup !== '##' || open.map === null) continue;
    const line = lines[open.map[0]] ?? '';
    const found = HEADING_LINE.exec(line);
    if (found === null) continue;
    const beat = Number(found[1]);
    const end = open.map[1];
    const para = tokens[at + 3];
    const inline = tokens[at + 4];
    // The attribute lines are the lines that follow the heading directly, at the top of its paragraph.
    let count = 0;
    let total = 0;
    if (para?.type === 'paragraph_open' && para.level === 0 && para.map?.[0] === end && inline?.type === 'inline') {
      total = para.map[1] - para.map[0];
      while (count < total && ATTRIBUTE.test(lines[end + count] ?? '')) count++;
    }
    const next = tokens[at + 6];
    const tight = count > 0 && para !== undefined && (count < total || (next?.map != null && next.map[0] === para.map![1]));
    const replacement: Token[] = [
      block(state, 'beat_heading_open', 1, open.map),
      rawText(state, line.slice(3)),
      block(state, 'beat_heading_close', -1, null),
      block(state, 'beat_attrs_open', 1, [end, end + count], { beat, tight }),
      rawText(state, lines.slice(end, end + count).join('\n')),
      block(state, 'beat_attrs_close', -1, null),
    ];
    if (count === 0 || para === undefined || inline === undefined) {
      tokens.splice(at, 3, ...replacement);
    } else if (count === total) {
      tokens.splice(at, 6, ...replacement);
    } else {
      // The rest of the paragraph stays a paragraph: the children behind the last attribute line.
      const children = inline.children ?? [];
      let seen = 0;
      const cut = children.findIndex((child) => isBreak(child) && ++seen === count);
      const rest = new state.Token('inline', '', 0);
      rest.level = 1;
      rest.content = lines.slice(end + count, para.map![1]).join('\n');
      rest.children = children.slice(cut + 1);
      const restOpen = block(state, 'paragraph_open', 1, [end + count, para.map![1]]);
      const restClose = block(state, 'paragraph_close', -1, null);
      tokens.splice(at, 6, ...replacement, restOpen, rest, restClose);
    }
    at += replacement.length - 1;
  }
}

/** Add the beat rule to a markdown-it instance. */
export function withBeats(tokenizer: MarkdownIt): MarkdownIt {
  tokenizer.core.ruler.push('beat', rewrite);
  return tokenizer;
}

/** The token-to-node mapping for the parser. */
export const beatTokens = {
  beat_heading: { block: 'beat_heading' },
  beat_attrs: { block: 'beat_attrs', getAttrs: (token: Token) => ({ beat: (token.meta as { beat: number }).beat, tight: (token.meta as { tight: boolean }).tight }) },
};

type State = MarkdownSerializerState & { flushClose(size?: number): void };

/** The serializer rules: the heading line raw, the attribute lines raw directly under it. */
export const beatRules = {
  beat_heading(state: MarkdownSerializerState, node: Node): void {
    state.write(`## ${node.textContent}`);
    state.closeBlock(node);
  },
  beat_attrs(plain: MarkdownSerializerState, node: Node): void {
    // A beat with no attribute line writes nothing; the next block then follows the stock separation.
    if (node.content.size === 0) return;
    const state = plain as State;
    state.flushClose(1);
    state.write(node.textContent);
    // A block that started on the next line stays there; otherwise the stock separation follows.
    if (node.attrs.tight === true) state.ensureNewLine();
    else state.closeBlock(node);
  },
};
