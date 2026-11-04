import type { Node } from 'prosemirror-model';
import type { EditorView } from 'prosemirror-view';
import type { BeatKey } from '../../harness/types.ts';
import { beatParamChange } from './beats.ts';

/**
 * `beat-param` as a transaction on the `beat_attrs` node at `pos`: only the value of one line
 * changes, or one line is added, with the rule of the oracle.
 */
export function setBeatParamAt(view: EditorView, pos: number, node: Node, key: BeatKey, value: string): void {
  const { from, to, insert } = beatParamChange(node.textContent, key, value);
  view.dispatch(view.state.tr.insertText(insert, pos + 1 + from, pos + 1 + to));
}

/** `beat-param` for the first beat with this number (the first `## Beat <n>` heading, as the oracle). */
export function setBeatParam(view: EditorView, beat: number, key: BeatKey, value: string): void {
  let found: { pos: number; node: Node } | undefined;
  view.state.doc.forEach((node, offset) => {
    if (found === undefined && node.type.name === 'beat_attrs' && node.attrs.beat === beat) found = { pos: offset, node };
  });
  if (found === undefined) throw new Error(`beat-param: there is no "## Beat ${beat}" heading`);
  setBeatParamAt(view, found.pos, found.node, key, value);
}
