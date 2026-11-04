import { syntaxTree } from '@codemirror/language';
import { StateField, type EditorState, type Range } from '@codemirror/state';
import { Decoration, EditorView, WidgetType, type DecorationSet } from '@codemirror/view';
import { mount, unmount } from 'svelte';
import BeatRow from '../../components/BeatRow.svelte';
import type { BeatKey, EditorContext } from '../../harness/types.ts';
import { beatParamChange, findBeats } from './beats.ts';
import { contextOf, setContext } from './context.ts';
import { inCode } from './links.ts';
import { touchedLines } from './rawlines.ts';

/**
 * Beats: the heading `## Beat <n> — <name>` is drawn as `<n> · <name>` and the attribute lines
 * `key:: value` below it as the shared `BeatRow`, one block widget over those lines (block
 * decorations come from a state field). A line the caret or a selection touches shows raw, so no
 * delete or selection removes a line the author did not see. The row's edits are transactions
 * that change one line or insert one.
 */

const mounted = new WeakMap<HTMLElement, () => void>();

/** Make a `beat-param` change in the document: the same rule as the oracle. */
export function setBeatParam(view: EditorView, beat: number, key: BeatKey, value: string): void {
  const { from, to, insert } = beatParamChange(view.state.doc, beat, key, value);
  view.dispatch({ changes: { from, to, insert }, userEvent: 'input.beat-param' });
}

class HeadingWidget extends WidgetType {
  constructor(readonly beat: number, readonly name: string) {
    super();
  }

  eq(other: HeadingWidget): boolean {
    return other.beat === this.beat && other.name === this.name;
  }

  toDOM(): HTMLElement {
    const el = document.createElement('span');
    el.className = 'hx-beat-heading cm-beat-heading';
    el.dataset.beat = String(this.beat);
    el.textContent = this.name === '' ? String(this.beat) : `${this.beat} · ${this.name}`;
    return el;
  }
}

class RowWidget extends WidgetType {
  constructor(
    readonly beat: number,
    readonly params: Array<{ key: string; value: string }>,
    readonly ctx: EditorContext,
  ) {
    super();
  }

  /** A new context (index, findings) redraws the row; so does any change of its lines. */
  eq(other: RowWidget): boolean {
    return other.beat === this.beat && other.ctx === this.ctx && JSON.stringify(other.params) === JSON.stringify(this.params);
  }

  toDOM(view: EditorView): HTMLElement {
    const host = document.createElement('div');
    host.className = 'cm-beat-row';
    const component = mount(BeatRow, {
      target: host,
      props: {
        beat: this.beat,
        params: this.params,
        ctx: this.ctx,
        oncommit: (key: BeatKey, value: string) => setBeatParam(view, this.beat, key, value),
      },
    });
    mounted.set(host, () => void unmount(component));
    return host;
  }

  destroy(dom: HTMLElement): void {
    mounted.get(dom)?.();
    mounted.delete(dom);
  }

  /** The row handles its own clicks and keys; the editor sees none of them. */
  ignoreEvent(): boolean {
    return true;
  }
}

function build(state: EditorState): DecorationSet {
  const ctx = contextOf(state);
  const tree = syntaxTree(state);
  const isRaw = touchedLines(state);
  const found: Array<Range<Decoration>> = [];
  for (const block of findBeats(state, (pos) => inCode(tree, pos))) {
    if (block.dash && !isRaw(block.heading.from, block.heading.to)) {
      found.push(Decoration.replace({ widget: new HeadingWidget(block.beat, block.name) }).range(block.heading.from, block.heading.to));
    }
    if (block.attrs === undefined) {
      found.push(Decoration.widget({ widget: new RowWidget(block.beat, block.params, ctx), block: true, side: 1 }).range(block.heading.to));
    } else if (!isRaw(block.attrs.from, block.attrs.to)) {
      found.push(
        Decoration.replace({ widget: new RowWidget(block.beat, block.params, ctx), block: true }).range(block.attrs.from, block.attrs.to),
      );
    }
  }
  return Decoration.set(found, true);
}

const beatField = StateField.define<DecorationSet>({
  create: build,
  update(value, tr) {
    if (tr.docChanged || tr.selection !== undefined || tr.effects.some((e) => e.is(setContext))
      || syntaxTree(tr.startState) !== syntaxTree(tr.state)) {
      return build(tr.state);
    }
    return value;
  },
  provide: (field) => [
    EditorView.decorations.from(field),
    EditorView.atomicRanges.of((view) => view.state.field(field)),
  ],
});

export const beats = [beatField];
