import type MarkdownIt from 'markdown-it';
import type { StateCore, Token } from 'markdown-it';
import type { MarkdownSerializerState } from 'prosemirror-markdown';
import type { Node, NodeSpec } from 'prosemirror-model';

/**
 * Callouts as document nodes: `> [!voice] ...`, `> [!design]` and `> [!note] ...` blocks.
 *
 * - `callout` (attr `type`) holds a `callout_title`, any blocks, and for a voice block an optional
 *   `callout_caption`.
 * - `callout_title` is the raw title line behind `[!type]`, a code-like textblock: it is drawn as
 *   a head (see `calloutview.ts`) and shows raw while the caret is in it.
 * - `callout_caption` is the `— ` line that ends a voice or note block (the dash is drawn by the CSS).
 *
 * The markdown-it core rule below turns the blockquote tokens of such a block into these nodes;
 * the serializer rules write `> [!type] title` and the `> `-prefixed content back. That is the
 * feature itself, not source preservation: the soft line breaks, the escapes and the final
 * newline are still the stock serializer's.
 */

export type CalloutType = 'voice' | 'design' | 'note';

/** The first line of a callout: `[!type]`, then a space and the title. */
const HEAD = /^\[!(voice|design|note)\](?: (.*))?$/i;
/** The caption is the last line of a voice or note block, starting with an em dash and a space (`--` is voice text). */
const CAPTION = '— ';

export const calloutSpec: NodeSpec = {
  group: 'block',
  content: 'callout_title block* callout_caption?',
  defining: true,
  attrs: { type: { default: 'design' } },
  // The node view draws the block; this is only what a DOM serializer would write.
  toDOM: (node) => ['div', { 'data-callout': node.attrs.type as string }, 0],
};

export const calloutTitleSpec: NodeSpec = {
  content: 'text*',
  marks: '',
  code: true,
  defining: true,
  toDOM: () => ['p', { class: 'pm-callout-title' }, 0],
};

export const calloutCaptionSpec: NodeSpec = {
  content: 'inline*',
  // `tight`: the caption was the last line of a paragraph, with no empty `>` line before it.
  attrs: { tight: { default: false } },
  toDOM: () => ['p', { class: 'tu-voice__by' }, 0],
};

type Lines = [number, number];

const isBreak = (token: Token): boolean => token.type === 'softbreak' || token.type === 'hardbreak';

function block(state: StateCore, type: string, nesting: 1 | 0 | -1, level: number, map: Lines | null, meta?: unknown): Token {
  const token = new state.Token(type, '', nesting);
  token.level = level;
  token.map = map;
  token.block = true;
  if (meta !== undefined) token.meta = meta;
  return token;
}

function inlineOf(state: StateCore, level: number, content: string, children: Token[]): Token {
  const token = new state.Token('inline', '', 0);
  token.level = level;
  token.content = content;
  token.children = children;
  return token;
}

function textOf(state: StateCore, content: string): Token {
  const token = new state.Token('text', '', 0);
  token.content = content;
  return token;
}

/** A paragraph of `children` on the source lines `map`. */
function paragraph(state: StateCore, level: number, map: Lines, children: Token[], content: string): Token[] {
  return [block(state, 'paragraph_open', 1, level, map), inlineOf(state, level + 1, content, children), block(state, 'paragraph_close', -1, level, null)];
}

/**
 * The last line of a voice or note block is its caption when it starts with `— `. A paragraph that is
 * only the caption becomes a caption; a paragraph that ends with it is split in two, the lines
 * before it and the caption, which is written back directly under them.
 */
function splitCaption(state: StateCore, body: Token[], level: number): Token[] {
  const inline = body[body.length - 2];
  const open = body[body.length - 3];
  if (body[body.length - 1]?.type !== 'paragraph_close' || inline?.type !== 'inline' || open?.type !== 'paragraph_open') return body;
  const children = inline.children ?? [];
  const lastBreak = children.map(isBreak).lastIndexOf(true);
  const first = children[lastBreak + 1];
  if (first?.type !== 'text' || !first.content.startsWith(CAPTION)) return body;
  const [from, to] = open.map ?? [0, 0];
  const before = body.slice(0, -3);
  const cut = inline.content.lastIndexOf('\n') + 1;
  const caption = [textOf(state, first.content.slice(CAPTION.length)), ...children.slice(lastBreak + 2)];
  const captionTokens = (map: Lines, tight: boolean): Token[] => [
    block(state, 'callout_caption_open', 1, level, map, { tight }),
    inlineOf(state, level + 1, inline.content.slice(cut + CAPTION.length), caption),
    block(state, 'callout_caption_close', -1, level, null),
  ];
  if (lastBreak === -1) return [...before, ...captionTokens([from, to], false)];
  const lines = children.slice(0, lastBreak + 1).filter(isBreak).length;
  return [
    ...before,
    ...paragraph(state, level, [from, from + lines], children.slice(0, lastBreak), inline.content.slice(0, cut - 1)),
    ...captionTokens([from + lines, to], true),
  ];
}

/** Rewrites every callout blockquote of the token stream (it runs after the inline rules). */
function rewrite(state: StateCore): void {
  const tokens = state.tokens;
  for (let at = 0; at < tokens.length; at++) {
    const open = tokens[at]!;
    if (open.type !== 'blockquote_open') continue;
    const para = tokens[at + 1];
    const inline = tokens[at + 2];
    if (para?.type !== 'paragraph_open' || inline?.type !== 'inline') continue;
    const found = HEAD.exec(inline.content.split('\n', 1)[0]!);
    if (found === null) continue;
    const type = found[1]!.toLowerCase() as CalloutType;
    const title = found[2] ?? '';
    // The first paragraph is `at + 1 .. at + 3`; the block ends at the closing token of its level.
    let close = at + 4;
    while (!(tokens[close]?.type === 'blockquote_close' && tokens[close]!.level === open.level)) close++;

    const level = open.level + 1;
    const [line, end] = para.map ?? open.map ?? [0, 0];
    const children = inline.children ?? [];
    const firstBreak = children.findIndex(isBreak);
    const rest = firstBreak === -1 ? [] : children.slice(firstBreak + 1);
    let body: Token[] = [
      ...(rest.length === 0 ? [] : paragraph(state, level, [line + 1, end], rest, inline.content.slice(inline.content.indexOf('\n') + 1))),
      ...tokens.slice(at + 4, close),
    ];
    // Data model 9.1 and 9.3 show a caption line on voice blocks and on notes; 9.2 has none for design.
    if (type !== 'design') body = splitCaption(state, body, level);
    tokens.splice(
      at,
      close - at + 1,
      block(state, 'callout_open', 1, open.level, open.map, { type }),
      block(state, 'callout_title_open', 1, level, [line, line + 1]),
      inlineOf(state, level + 1, title, title === '' ? [] : [textOf(state, title)]),
      block(state, 'callout_title_close', -1, level, null),
      ...body,
      block(state, 'callout_close', -1, open.level, null),
    );
  }
}

/** Add the callout rule to a markdown-it instance. */
export function withCallouts(tokenizer: MarkdownIt): MarkdownIt {
  tokenizer.core.ruler.push('callout', rewrite);
  return tokenizer;
}

/** The token-to-node mapping for the parser. */
export const calloutTokens = {
  callout: { block: 'callout', getAttrs: (token: Token) => ({ type: (token.meta as { type: CalloutType }).type }) },
  callout_title: { block: 'callout_title' },
  callout_caption: { block: 'callout_caption', getAttrs: (token: Token) => ({ tight: (token.meta as { tight: boolean }).tight }) },
};

/** The serializer rules: the title line raw, then the content behind `> `. */
/** The serializer state with the two members the library implements but its typings leave out. */
type State = MarkdownSerializerState & { delim: string; flushClose(size?: number): void };

export const calloutRules = {
  callout(plain: MarkdownSerializerState, node: Node): void {
    const state = plain as State;
    const title = node.firstChild?.textContent ?? '';
    state.write(`> [!${node.attrs.type as string}]${title === '' ? '' : ` ${title}`}`);
    state.ensureNewLine();
    const outer = state.delim;
    state.delim += '> ';
    node.forEach((child, _offset, index) => {
      if (index > 0) state.render(child, node, index);
    });
    state.delim = outer;
    state.closeBlock(node);
  },
  // Written by the rule of the callout, which owns the line.
  callout_title(): void {},
  callout_caption(plain: MarkdownSerializerState, node: Node): void {
    const state = plain as State;
    // A caption that ended a paragraph follows its lines directly; one after an empty `>` line keeps it.
    if (node.attrs.tight === true) state.flushClose(1);
    state.write(CAPTION);
    state.renderInline(node);
    state.closeBlock(node);
  },
};
