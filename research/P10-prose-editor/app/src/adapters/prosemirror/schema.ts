import { schema as markdownSchema } from 'prosemirror-markdown';
import { Schema } from 'prosemirror-model';
import { calloutCaptionSpec, calloutSpec, calloutTitleSpec } from './callouts.ts';
import { wikilinkSpec } from './links.ts';

/** Write mode: the stock schema of prosemirror-markdown plus the wikilink and the callout nodes. */
export const schema = new Schema({
  nodes: markdownSchema.spec.nodes
    .addToEnd('wikilink', wikilinkSpec)
    .addToEnd('callout', calloutSpec)
    .addToEnd('callout_title', calloutTitleSpec)
    .addToEnd('callout_caption', calloutCaptionSpec),
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
