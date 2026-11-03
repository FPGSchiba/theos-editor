import type { Fragment, Node, ResolvedPos } from 'prosemirror-model';
import { linkSource } from '../prosemirror/links.ts';
import { serializeBody } from '../prosemirror/markdown.ts';
import { schema } from '../prosemirror/schema.ts';
import { docOf } from './edit.ts';

/**
 * The source text of the content a change inserts, in the forms to try (raw first, then the stock
 * serializer's), and the blank lines around inserted whole blocks. `spanedit.ts` tries them at the
 * candidate offsets; `verify.ts` decides which one stands for the document.
 */

/** Blanks between blocks; not U+00A0, which is text. */
const BLANK = /[ \t\n]/;

/** The inline content as typed: text raw, a link as its syntax, a hard break as a backslash break. */
function inlineRaw(content: Fragment): string | undefined {
  let text = '';
  let known = true;
  content.forEach((node) => {
    if (node.isText) text += node.text ?? '';
    else if (node.type.name === 'wikilink') text += linkSource(node.attrs.id as string, node.attrs.shown as string | null);
    else if (node.type.name === 'hard_break') text += '\\\n';
    else known = false;
  });
  return known ? text : undefined;
}

/** The inline content as the stock serializer writes it in a paragraph: marks and escapes included. */
function inlineSerialized(content: Fragment): string | undefined {
  try {
    return serializeBody(docOf([schema.node('paragraph', null, content)]));
  } catch {
    return undefined;
  }
}

/** Both forms of a piece of inline content of `block` (raw first); code-like blocks only raw. */
function inlineForms(block: Node, content: Fragment): string[] | undefined {
  const raw = inlineRaw(content);
  const forms = raw === undefined ? [] : [raw];
  if (block.type.spec.code !== true) {
    const written = inlineSerialized(content);
    if (written !== undefined && written !== raw) forms.push(written);
  }
  return forms.length === 0 ? undefined : forms;
}

/** What stands in front of the text of a top-level textblock on its first line, for a block a change starts. */
function linePrefix(block: Node): string | undefined {
  switch (block.type.name) {
    case 'paragraph':
      return '';
    case 'heading':
      return `${'#'.repeat(block.attrs.level as number)} `;
    case 'beat_heading':
      return '## ';
    default:
      return undefined;
  }
}

/** Whole top-level blocks: paragraphs and headings raw, separated by a blank line; otherwise only the serializer's form. */
function blockForms(nodes: readonly Node[]): string[] {
  const written = serializeBody(docOf(nodes));
  const raw: string[] = [];
  for (const node of nodes) {
    const prefix = node.type.name === 'paragraph' || node.type.name === 'heading' ? linePrefix(node) : undefined;
    const inline = prefix === undefined ? undefined : inlineRaw(node.content);
    if (prefix === undefined || inline === undefined) return [written];
    raw.push(prefix + inline);
  }
  const joined = raw.join('\n\n');
  return joined === written ? [joined] : [joined, written];
}

/** A candidate source text for inserted content. */
export interface Insert {
  text: string;
  /** The text starts with a whole block, so it needs a blank line after what stands before it. */
  opensBlock: boolean;
  /** The text ends with a whole block, so what follows needs a blank line after it. */
  closesBlock: boolean;
}

/** What stands in front of a line of a container (quote marks; list markers as blanks): the start of a new line in it. */
const CONTAINER = /^(?:[ \t]*>[ \t]?|[ \t]*(?:[-*+]|\d{1,9}[.)])[ \t]+)*[ \t]*/;

/** The prefix a new line takes at offset `at` of `file`: the container prefix of the line `at` is on. */
export function continuation(file: string, at: number): string {
  const line = file.slice(file.lastIndexOf('\n', at - 1) + 1, at);
  return (CONTAINER.exec(line)?.[0] ?? '').replace(/[-*+]|\d{1,9}[.)]/g, (marker) => ' '.repeat(marker.length));
}

/**
 * A change across sibling textblocks of one container (a paragraph split by Enter or by a pasted
 * paragraph, at the top level or inside a callout or quote): the rest of the first textblock, the
 * paragraphs between, and the start of the last, separated by a blank line of the container
 * (`prefix` is the prefix of a new line there, `> ` in a callout).
 */
function acrossSiblings($start: ResolvedPos, $end: ResolvedPos, prefix: string): string[] | undefined {
  const heads = inlineForms($start.parent, $start.parent.content.cut($start.parentOffset));
  const tailPrefix = linePrefix($end.parent);
  const tails = inlineForms($end.parent, $end.parent.content.cut(0, $end.parentOffset));
  if (heads === undefined || tails === undefined || tailPrefix === undefined) return undefined;
  const container = $start.node(-1);
  const between: Node[] = [];
  for (let index = $start.index(-1) + 1; index < $end.index(-1); index++) between.push(container.child(index));
  const top = $start.depth === 1;
  let middles = [''];
  if (between.length > 0) {
    if (top) middles = blockForms(between);
    else {
      const raw = between.map((node) => (node.type.name === 'paragraph' ? inlineRaw(node.content) : undefined));
      if (raw.some((text) => text === undefined)) return undefined;
      middles = [raw.join(`\n${prefix.trimEnd()}\n${prefix}`)];
    }
  }
  const separator = `\n${prefix.trimEnd()}\n${prefix}`;
  const texts = new Set<string>();
  for (let form = 0; form < Math.max(heads.length, middles.length, tails.length); form++) {
    const parts = [heads[Math.min(form, heads.length - 1)]!, ...(between.length > 0 ? [middles[Math.min(form, middles.length - 1)]!] : []), tailPrefix + tails[Math.min(form, tails.length - 1)]!];
    // Inside a container every line needs its prefix: content with line breaks of its own is not written here.
    if (!top && parts.some((part) => part.includes('\n'))) return undefined;
    texts.add(parts.join(separator));
  }
  return [...texts];
}

/**
 * The source of the content between `start` and `end` of `doc` (the document after a change),
 * in the forms to try, or undefined when it cannot be written as a span: content inside one
 * textblock (at any depth), content across sibling textblocks of one container, or content
 * across top-level blocks (the parts of a top-level textblock at either end, and whole top-level
 * blocks between). `prefix` is the prefix a new line takes where the change starts.
 */
export function insertsFor(doc: Node, start: number, end: number, prefix: string): Insert[] | undefined {
  if (start === end) return [{ text: '', opensBlock: false, closesBlock: false }];
  const $start = doc.resolve(start);
  const $end = doc.resolve(end);
  if ($start.sameParent($end) && $start.parent.isTextblock) {
    const forms = inlineForms($start.parent, $start.parent.content.cut($start.parentOffset, $end.parentOffset));
    return forms?.map((text) => ({ text, opensBlock: false, closesBlock: false }));
  }
  if ($start.depth > 0 && $start.depth === $end.depth && $start.parent.isTextblock && $end.parent.isTextblock && $start.node(-1) === $end.node(-1)) {
    return acrossSiblings($start, $end, prefix)?.map((text) => ({ text, opensBlock: false, closesBlock: false }));
  }
  if ($start.depth > 1 || $end.depth > 1) return undefined;
  if (($start.depth === 1 && !$start.parent.isTextblock) || ($end.depth === 1 && !$end.parent.isTextblock)) return undefined;

  // The head: the rest of the textblock the change starts in.
  let heads = [''];
  if ($start.depth === 1) {
    const forms = inlineForms($start.parent, $start.parent.content.cut($start.parentOffset));
    if (forms === undefined) return undefined;
    heads = forms;
  }
  // The whole blocks between.
  const first = $start.depth === 1 ? $start.index(0) + 1 : $start.index(0);
  const last = $end.index(0);
  const whole: Node[] = [];
  for (let index = first; index < last; index++) whole.push(doc.child(index));
  const middles = whole.length === 0 ? [''] : blockForms(whole);
  // The tail: the start of the textblock the change ends in, with the prefix of its first line.
  let tails = [''];
  if ($end.depth === 1) {
    const prefix = linePrefix($end.parent);
    const forms = inlineForms($end.parent, $end.parent.content.cut(0, $end.parentOffset));
    if (prefix === undefined || forms === undefined) return undefined;
    tails = forms.map((form) => prefix + form);
  }
  const parts = (head: string, middle: string, tail: string): string =>
    [...($start.depth === 1 ? [head] : []), ...(whole.length > 0 ? [middle] : []), ...($end.depth === 1 ? [tail] : [])].join('\n\n');
  const texts = new Set<string>();
  // Raw forms first, the serializer's after.
  for (let form = 0; form < Math.max(heads.length, middles.length, tails.length); form++) {
    texts.add(parts(heads[Math.min(form, heads.length - 1)]!, middles[Math.min(form, middles.length - 1)]!, tails[Math.min(form, tails.length - 1)]!));
  }
  return [...texts].map((text) => ({ text, opensBlock: $start.depth === 0, closesBlock: $end.depth === 0 }));
}

/** The separator in front of inserted whole blocks at `at`: whatever makes a blank line after the text before. */
export function leadSeparator(source: string, bodyStart: number, at: number): string {
  let index = at;
  while (index > bodyStart && BLANK.test(source[index - 1]!)) index--;
  if (index === bodyStart) return '';
  const blanks = source.slice(index, at);
  const newlines = blanks.split('\n').length - 1;
  return newlines >= 2 ? '' : newlines === 1 ? '\n' : '\n\n';
}

/** The separator behind inserted whole blocks at `at`: whatever makes a blank line before the text after. */
export function trailSeparator(source: string, at: number): string {
  let index = at;
  while (index < source.length && BLANK.test(source[index]!)) index++;
  if (index === source.length) return '';
  const newlines = source.slice(at, index).split('\n').length - 1;
  return newlines >= 1 ? '\n' : '\n\n';
}
