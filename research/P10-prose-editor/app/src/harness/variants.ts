import type { VariantInfo } from './types.ts';

/**
 * The metadata of every registered variant. It lives apart from `registry.ts` because the
 * Playwright process (Node) reads it at collection time (`variantsUnderTest`), and the registry
 * imports the adapters, which need a browser. A prototype adds its entry here and its factory
 * to `registry.ts`.
 */
export const variants: VariantInfo[] = [
  {
    name: 'baseline',
    component: 'HTML textarea',
    version: 'built into the browser',
    wysiwyg: false,
    baseline: true,
    packages: [],
  },
  {
    name: 'codemirror',
    component: 'CodeMirror 6',
    version: '6.43.15',
    wysiwyg: false,
    packages: ['@codemirror/commands', '@codemirror/language', '@codemirror/state', '@codemirror/view', '@lezer/markdown'],
  },
  {
    name: 'prosemirror:stock',
    component: 'ProseMirror',
    version: '1.42.9',
    wysiwyg: true,
    column: 'stock',
    packages: [
      'prosemirror-commands', 'prosemirror-history', 'prosemirror-keymap', 'prosemirror-markdown', 'prosemirror-model',
      'prosemirror-state', 'prosemirror-view',
    ],
  },
];
