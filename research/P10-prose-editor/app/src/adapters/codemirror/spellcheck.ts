import { EditorView } from '@codemirror/view';

/** CodeMirror turns spell check off by default; prose wants the browser's own. */
export const spellcheck = EditorView.contentAttributes.of({ spellcheck: 'true' });
