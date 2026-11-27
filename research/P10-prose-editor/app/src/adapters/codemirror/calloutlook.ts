import { EditorView } from '@codemirror/view';

/**
 * The look of the callout lines. Each line of a block carries the block's classes, so the box is drawn
 * per line: the sides on every line, the top on the first and the bottom on the last. The values are
 * the design system's (`.tu-voice` of the bundle) and the harness callout look, from the same tokens.
 */

const voice = '.cm-line.cm-callout-voice';
const dashed = '1px dashed var(--border)';

/** The box of a design or a note callout, which share one look and differ in the colour of the left edge. */
const aside = (kind: 'design' | 'note', colour: string): Record<string, Record<string, string | number>> => {
  const line = `.cm-line.cm-callout-${kind}`;
  return {
    [line]: {
      boxSizing: 'border-box',
      maxWidth: 'var(--measure-prose)',
      padding: '0 14px',
      background: 'var(--panel)',
      borderLeft: `2px solid ${colour}`,
      borderRight: dashed,
      font: '400 13px/20px var(--font-ui)',
      color: 'var(--textDim)',
    },
    [`${line}.cm-callout-first`]: { paddingTop: '8px', borderTop: dashed },
    [`${line}.cm-callout-last`]: { paddingBottom: '8px', borderBottom: dashed },
  };
};

export const calloutLook = EditorView.theme({
  '.cm-callout-head': { display: 'inline-block', width: '100%', verticalAlign: 'top', whiteSpace: 'normal' },
  '.cm-callout-label': { font: '700 10.5px/16px var(--font-mono)', letterSpacing: '0.4px', textTransform: 'uppercase', color: 'var(--textDim)' },

  [voice]: {
    boxSizing: 'border-box',
    maxWidth: 'calc(var(--measure-inworld) + 40px)',
    padding: '0 20px',
    background: 'var(--panel-inworld)',
    border: '0 solid var(--line-inworld)',
    borderWidth: '0 1px',
    borderRadius: 0,
    font: '400 17px/28px var(--font-serif)',
    color: 'var(--ink-inworld)',
  },
  [`${voice}.cm-callout-first`]: {
    paddingTop: '14px',
    borderTopWidth: '1px',
    borderTopLeftRadius: 'var(--radius-manuscript)',
    borderTopRightRadius: 'var(--radius-manuscript)',
  },
  [`${voice}.cm-callout-last`]: {
    paddingBottom: '18px',
    borderBottomWidth: '1px',
    borderBottomLeftRadius: 'var(--radius-manuscript)',
    borderBottomRightRadius: 'var(--radius-manuscript)',
  },
  // The caption is smaller, and `ch` follows the font, so the box keeps the width of the 17px body.
  [`${voice}.tu-voice__by`]: {
    margin: 0,
    paddingTop: '12px',
    maxWidth: 'calc(var(--measure-inworld) * 17 / 14 + 40px)',
    font: 'italic 400 14px/20px var(--font-serif)',
    color: 'var(--ink-inworld-dim)',
  },
  // The caption's dash is drawn by the design system; raw, the dash is text.
  [`${voice}.tu-voice__by.cm-callout-raw::before`]: { content: 'none' },

  ...aside('design', 'var(--sig-info)'),
  ...aside('note', 'var(--sig-attention)'),
  '.cm-line.cm-callout-note[data-note-state="resolved"]': { borderLeftColor: 'var(--sig-approve)' },
  // A dropped note recedes by its edge and a dimmed id; its text keeps `textDim`, never an opacity.
  '.cm-line.cm-callout-note[data-note-state="dropped"]': { borderLeftColor: 'var(--textMute)' },
  '.cm-line.cm-callout-note[data-note-state="dropped"] .hx-note__id': { color: 'var(--textDim)' },
});
