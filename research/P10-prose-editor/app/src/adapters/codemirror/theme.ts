import { EditorView } from '@codemirror/view';

/** The look of the prose editor, from the design tokens of the harness page. */
export const theme = [
  EditorView.lineWrapping,
  EditorView.theme({
    '&': { color: 'var(--textProse)', background: 'var(--panel)', border: '1px solid var(--border)' },
    '&.cm-focused': { outline: 'none', borderColor: 'var(--borderHi)' },
    '.cm-scroller': { fontFamily: 'var(--font-ui)', fontSize: '15px', lineHeight: '24px' },
    '.cm-content': { padding: '12px 14px', caretColor: 'var(--text)' },
    '.cm-line': { padding: 0 },
    '.ms-title, .ms-h2, .ms-h3': { color: 'var(--text)', paddingTop: '8px' },
    '.tu-em': { fontStyle: 'italic' },
    '.tu-strong': { fontWeight: '700', color: 'var(--text)' },
    '&.cm-mode-source .cm-scroller': { fontFamily: 'var(--font-mono)', fontSize: '13px', lineHeight: '20px' },
    '.cm-frontmatter': { color: 'var(--textMute)' },
  }),
];
