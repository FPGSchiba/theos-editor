import { schema as markdownSchema } from 'prosemirror-markdown';
import { Schema } from 'prosemirror-model';
import { wikilinkSpec } from './links.ts';

/** Write mode: the stock schema of prosemirror-markdown plus one node, the wikilink. */
export const schema = new Schema({
  nodes: markdownSchema.spec.nodes.addToEnd('wikilink', wikilinkSpec),
  marks: markdownSchema.spec.marks,
});

/**
 * Source mode: the whole file as the text of one code-like block, so the offsets of the file
 * are the positions of the document minus one.
 */
export const sourceSchema = new Schema({
  nodes: {
    doc: { content: 'source' },
    source: {
      content: 'text*',
      marks: '',
      code: true,
      defining: true,
      parseDOM: [{ tag: 'pre', preserveWhitespace: 'full' }],
      toDOM: () => ['pre', { class: 'pm-source' }, ['code', 0]],
    },
    text: {},
  },
});
