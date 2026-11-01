import { defaultMarkdownParser, defaultMarkdownSerializer } from 'prosemirror-markdown';
import type { Node } from 'prosemirror-model';

/** The Markdown tokens of a body, with the `map` line ranges markdown-it gives block tokens. */
export type MarkdownToken = ReturnType<typeof defaultMarkdownParser.tokenizer.parse>[number];

/** The stock parser of prosemirror-markdown: markdown-it "commonmark", no HTML, the library's token handlers. */
export function parseBody(body: string): Node {
  return defaultMarkdownParser.parse(body) as Node;
}

/** The stock serializer; it writes the tree out again and keeps nothing of the source. */
export function serializeBody(doc: Node): string {
  return defaultMarkdownSerializer.serialize(doc);
}

/** The block tokens of `body` (what `parseBody` is built from), for the line ranges of the blocks. */
export function tokenize(body: string): MarkdownToken[] {
  return defaultMarkdownParser.tokenizer.parse(body, {});
}
