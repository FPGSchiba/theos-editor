import { defaultMarkdownParser } from 'prosemirror-markdown';
import type { Node, NodeSpec } from 'prosemirror-model';

/**
 * The wikilink as a document node: `[[id]]` and `[[id|shown]]` are one inline atom. Everything the
 * stock `prosemirror-markdown` column needs for it lives here: the node spec, the markdown-it
 * inline rule that finds the syntax, the token mapping, and the serializer rule that writes it
 * back. (The serializer rule is part of the feature: without it the syntax would not survive.)
 */

type Tokenizer = typeof defaultMarkdownParser.tokenizer;
type InlineRule = Parameters<Tokenizer['inline']['ruler']['before']>[2];

/** The link as it is written in Markdown. */
export function linkSource(id: string, shown: string | null | undefined): string {
  return shown === null || shown === undefined ? `[[${id}]]` : `[[${id}|${shown}]]`;
}

export const wikilinkSpec: NodeSpec = {
  group: 'inline',
  inline: true,
  atom: true,
  selectable: true,
  draggable: false,
  attrs: { id: {}, shown: { default: null } },
  leafText: (node) => linkSource(node.attrs.id as string, node.attrs.shown as string | null),
  // The node view draws the chip; this is only what a DOM serializer would write.
  toDOM: (node) => ['span', { class: 'tu-link', 'data-link-id': node.attrs.id as string }, (node.attrs.shown ?? node.attrs.id) as string],
  parseDOM: [{ tag: 'span.tu-link[data-link-id]', getAttrs: (dom) => ({ id: (dom as HTMLElement).getAttribute('data-link-id') }) }],
};

/** The text a leaf node stands for in the source: the link syntax, `\n` for a hard break or an image. */
export function sourceLeaf(node: Node): string {
  return node.type.name === 'wikilink' ? linkSource(node.attrs.id as string, node.attrs.shown as string | null) : '\n';
}

/** An id has no brackets, bars or line breaks; the shown text no brackets and no line break. */
const WIKILINK = /^\[\[([^[\]|\n]+)(?:\|([^[\]\n]+))?\]\]/;

const rule: InlineRule = (state, silent) => {
  if (state.src.charCodeAt(state.pos) !== 0x5b || state.src.charCodeAt(state.pos + 1) !== 0x5b) return false;
  const found = WIKILINK.exec(state.src.slice(state.pos));
  if (found === null) return false;
  if (!silent) {
    const token = state.push('wikilink', '', 0);
    token.meta = { id: found[1], shown: found[2] ?? null };
    token.content = found[0];
  }
  state.pos += found[0].length;
  return true;
};

let installed = false;

/** Teach the stock tokenizer the wikilink; it runs before the link rule, after code spans have been read. */
export function installWikilinkRule(tokenizer: Tokenizer): void {
  if (installed) return;
  installed = true;
  tokenizer.inline.ruler.before('link', 'wikilink', rule);
}

/** The token-to-node mapping for the parser. */
export const wikilinkToken = {
  node: 'wikilink',
  getAttrs: (token: { meta: { id: string; shown: string | null } }) => ({ id: token.meta.id, shown: token.meta.shown }),
};
