import type { Node } from 'prosemirror-model';
import { Plugin } from 'prosemirror-state';
import { Decoration, DecorationSet } from 'prosemirror-view';

/**
 * Developer words in voice text (the in-world vocabulary check): the words of the host's list are
 * marked, dotted underline in the harness CSS, and nothing else is touched. Only the text blocks
 * and the caption of voice callouts are searched; prose, design notes, titles and notes are not voice text.
 */

const escapeRegExp = (text: string): string => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

let cached: { key: string; pattern: RegExp | undefined } | undefined;

/** One pattern for the whole list: longest words first, and a match is a whole word (not `tier` in `frontier`). */
function patternFor(words: readonly string[]): RegExp | undefined {
  const key = words.join('\n');
  if (cached?.key !== key) {
    const listed = words.filter((word) => word !== '').sort((a, b) => b.length - a.length).map(escapeRegExp);
    cached = {
      key,
      pattern: listed.length === 0 ? undefined : new RegExp(`(?<![\\p{L}\\p{N}_])(?:${listed.join('|')})(?![\\p{L}\\p{N}_])`, 'giu'),
    };
  }
  return cached.pattern;
}

function mark(found: Decoration[], block: Node, start: number, pattern: RegExp): void {
  block.forEach((child, offset) => {
    if (!child.isText) return;
    for (const match of (child.text ?? '').matchAll(pattern)) {
      const from = start + offset + match.index;
      found.push(Decoration.inline(from, from + match[0].length, { 'data-dev-word': '', title: 'developer word' }));
    }
  });
}

function build(doc: Node, words: readonly string[]): DecorationSet {
  const pattern = patternFor(words);
  if (pattern === undefined) return DecorationSet.empty;
  const found: Decoration[] = [];
  doc.descendants((node, pos) => {
    if (node.type.name !== 'callout' || node.attrs.type !== 'voice') return node.type.name !== 'callout_title' && !node.isTextblock;
    node.forEach((child, offset) => {
      if (child.type.name === 'callout_title' || child.textContent.startsWith('Design note:')) return;
      if (child.isTextblock) mark(found, child, pos + 1 + offset + 1, pattern);
      else child.descendants((inner, innerPos) => {
        if (inner.isTextblock) mark(found, inner, pos + 1 + offset + 1 + innerPos + 1, pattern);
        return !inner.isTextblock;
      });
    });
    return false;
  });
  return DecorationSet.create(doc, found);
}

export const devWords = (words: () => readonly string[]): Plugin =>
  new Plugin({ props: { decorations: (state) => build(state.doc, words()) } });
