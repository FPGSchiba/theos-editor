import { schema as markdownSchema } from 'prosemirror-markdown';
import { Schema } from 'prosemirror-model';

/** Write mode: the stock schema of prosemirror-markdown, unchanged (this is the stock column). */
export const schema = markdownSchema;

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
