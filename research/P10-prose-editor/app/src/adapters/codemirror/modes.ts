import { Compartment, EditorSelection, type Extension } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import type { Mode } from '../../harness/types.ts';
import { beats } from './beatrow.ts';
import { calloutLook } from './calloutlook.ts';
import { callouts } from './callouts.ts';
import { bodyStart, hideFrontMatter, showFrontMatter } from './frontmatter.ts';
import { linkChips } from './links.ts';
import { livePreview } from './livepreview.ts';
import { linkPopover } from './popover.ts';
import { typingField } from './typing.ts';

/** Write and Source mode are two sets of extensions in one compartment of the same view. */
const slot = new Compartment();

const write: Extension = [
  hideFrontMatter, livePreview, callouts, calloutLook, beats, typingField, linkChips, linkPopover,
  EditorView.editorAttributes.of({ class: 'cm-mode-write' }),
];
const source: Extension = [showFrontMatter, EditorView.editorAttributes.of({ class: 'cm-mode-source' })];
const forMode = (mode: Mode): Extension => (mode === 'write' ? write : source);

export const modeExtension = (mode: Mode): Extension => slot.of(forMode(mode));

/** Switch the mode; a caret left in the front matter moves to the body, which Write mode hides it from. */
export function switchMode(view: EditorView, mode: Mode): void {
  const end = view.state.field(bodyStart);
  const selection = view.state.selection;
  const inside = mode === 'write' && selection.ranges.some((range) => range.from < end);
  view.dispatch({
    effects: slot.reconfigure(forMode(mode)),
    selection: inside
      ? EditorSelection.create(
        selection.ranges.map((range) => EditorSelection.range(Math.max(range.anchor, end), Math.max(range.head, end))),
        selection.mainIndex,
      )
      : undefined,
  });
}
