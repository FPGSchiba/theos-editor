import type { Node } from 'prosemirror-model';
import { parseBody } from '../prosemirror/markdown.ts';

/**
 * Whether a candidate source text stands for the document: the text alone is parsed with the
 * adapter's own parser and compared with the blocks it has to give. `strict` is `Node.eq`. The
 * loose comparison also accepts differences that are only whitespace Markdown cannot hold (spaces
 * at the edges of a textblock, several spaces in a row): the typed bytes are kept as typed.
 */
export type Strictness = 'strict' | 'loose';

/** A text without line breaks for the inline content of a textblock: marks and leaves spelled out. */
function inlineKey(node: Node, loose: boolean): string {
  const parts: string[] = [];
  node.forEach((child) => {
    if (child.isText) parts.push(`${child.marks.map((mark) => `${mark.type.name}${JSON.stringify(mark.attrs)}`).join('+')}\u0001${child.text ?? ''}`);
    else parts.push(`\u0002${child.type.name}${JSON.stringify(child.attrs)}\u0002`);
  });
  const key = parts.join('\u0000');
  if (!loose) return key;
  const collapsed = node.type.spec.code === true ? key : key.replace(/[ \t]+/g, ' ');
  return collapsed.replace(/\u0001[ \t]+/, '\u0001').replace(/[ \t]+$/, '');
}

function sameNode(a: Node, b: Node, loose: boolean): boolean {
  if (!loose) return a.eq(b);
  if (a.type !== b.type || JSON.stringify(a.attrs) !== JSON.stringify(b.attrs)) return false;
  if (a.isTextblock) return inlineKey(a, true) === inlineKey(b, true);
  if (a.childCount !== b.childCount) return false;
  for (let index = 0; index < a.childCount; index++) if (!sameNode(a.child(index), b.child(index), true)) return false;
  return true;
}

/** Parses of candidate texts, so the strict and the loose pass parse each text once. */
export class Verifier {
  private readonly parsed = new Map<string, Node>();

  /** Does `text`, parsed alone, give exactly the blocks `nodes`? */
  check(text: string, nodes: readonly Node[], strictness: Strictness): boolean {
    let doc = this.parsed.get(text);
    if (doc === undefined) {
      doc = parseBody(text);
      this.parsed.set(text, doc);
    }
    if (doc.childCount !== nodes.length) return false;
    const loose = strictness === 'loose';
    for (let index = 0; index < nodes.length; index++) if (!sameNode(doc.child(index), nodes[index]!, loose)) return false;
    return true;
  }
}
