import { defaultMarkdownParser, defaultMarkdownSerializer, MarkdownParser, MarkdownSerializer } from 'prosemirror-markdown';
import type { Node } from 'prosemirror-model';
import { linkSource, wikilinkToken, wikilinkTokenizer } from './links.ts';
import { schema } from './schema.ts';

/** The Markdown tokens of a body, with the `map` line ranges markdown-it gives block tokens. */
export type MarkdownToken = ReturnType<typeof defaultMarkdownParser.tokenizer.parse>[number];

/** The stock parser setup (markdown-it "commonmark", no HTML, the library's token handlers) on its own tokenizer, plus the wikilink token. */
const parser = new MarkdownParser(schema, wikilinkTokenizer(), {
  ...defaultMarkdownParser.tokens,
  wikilink: wikilinkToken,
});

/** The stock serializer plus the wikilink rule: `[[id]]` or `[[id|shown]]`, written raw (never escaped). */
const serializer = new MarkdownSerializer(
  {
    ...defaultMarkdownSerializer.nodes,
    wikilink(state, node) {
      state.write(linkSource(node.attrs.id as string, node.attrs.shown as string | null));
    },
  },
  defaultMarkdownSerializer.marks,
);

export function parseBody(body: string): Node {
  return parser.parse(body) as Node;
}

/** Write the tree out again; nothing of the source is kept. */
export function serializeBody(doc: Node): string {
  return serializer.serialize(doc);
}

/** The block tokens of `body` (what `parseBody` is built from), for the line ranges of the blocks. */
export function tokenize(body: string): MarkdownToken[] {
  return parser.tokenizer.parse(body, {});
}
