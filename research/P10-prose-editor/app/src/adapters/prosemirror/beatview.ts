import { mount, unmount } from 'svelte';
import type { Node } from 'prosemirror-model';
import { TextSelection } from 'prosemirror-state';
import type { EditorView, NodeView, ViewMutationRecord } from 'prosemirror-view';
import BeatRow from '../../components/BeatRow.svelte';
import type { BeatKey, EditorContext } from '../../harness/types.ts';
import { attributeLines, readHeading } from './beats.ts';
import { setBeatParamAt } from './beatparam.ts';

/**
 * The node views of a beat. The heading draws `<n> · <name>` (`[data-beat]`) and hides its raw
 * text until the selection touches it; the attribute lines draw the shared `BeatRow` (mounted,
 * redrawn when what it shows changed, unmounted in `destroy`) and hide their raw text likewise.
 * The row's own events (clicks, the value field) never reach the editor (`stopEvent`).
 */

/** Put the caret at the end of the node view's text; the node then shows raw. */
function caretIn(view: EditorView, pos: number | undefined, node: Node): void {
  if (pos === undefined) return;
  view.dispatch(view.state.tr.setSelection(TextSelection.create(view.state.doc, pos + 1 + node.content.size)));
  view.focus();
}

export class BeatHeadingView implements NodeView {
  readonly dom = document.createElement('h2');
  readonly contentDOM = document.createElement('span');
  private readonly shown = document.createElement('span');

  constructor(
    private node: Node,
    private readonly view: EditorView,
    private readonly getPos: () => number | undefined,
  ) {
    this.dom.className = 'pm-beat-heading';
    this.shown.className = 'pm-beat-shown';
    this.shown.contentEditable = 'false';
    this.shown.addEventListener('mousedown', (event) => {
      event.preventDefault();
      caretIn(this.view, this.getPos(), this.node);
    });
    this.contentDOM.className = 'pm-beat-text';
    this.dom.append(this.shown, this.contentDOM);
    this.draw();
  }

  update(node: Node): boolean {
    if (node.type !== this.node.type) return false;
    this.node = node;
    this.draw();
    return true;
  }

  ignoreMutation(mutation: ViewMutationRecord): boolean {
    return !this.contentDOM.contains(mutation.target);
  }

  stopEvent(event: Event): boolean {
    return event.target instanceof globalThis.Node && this.shown.contains(event.target);
  }

  private draw(): void {
    const info = readHeading(this.node.textContent);
    // A heading without the dash stays raw.
    this.dom.classList.toggle('is-bare', info === undefined || !info.dash);
    if (info === undefined || !info.dash) {
      this.shown.textContent = '';
      this.shown.removeAttribute('data-beat');
      return;
    }
    this.shown.setAttribute('data-beat', String(info.beat));
    this.shown.textContent = info.name === '' ? String(info.beat) : `${info.beat} · ${info.name}`;
  }
}

export class BeatAttrsView implements NodeView {
  readonly dom = document.createElement('div');
  readonly contentDOM = document.createElement('div');
  private readonly host = document.createElement('div');
  private release: (() => void) | undefined;
  private seen = '';

  constructor(
    private node: Node,
    private readonly view: EditorView,
    private readonly getPos: () => number | undefined,
    private readonly ctx: () => EditorContext,
    private readonly views: Set<BeatAttrsView>,
  ) {
    this.dom.className = 'pm-beat-attrs';
    this.host.className = 'pm-beat-row';
    this.host.contentEditable = 'false';
    this.host.spellcheck = false;
    this.contentDOM.className = 'pm-beat-lines';
    this.dom.append(this.host, this.contentDOM);
    this.draw();
    views.add(this);
  }

  update(node: Node): boolean {
    if (node.type !== this.node.type) return false;
    this.node = node;
    this.draw();
    return true;
  }

  /** The index or the findings changed: draw the row again when what it shows changed. */
  refresh(): void {
    this.draw();
  }

  ignoreMutation(mutation: ViewMutationRecord): boolean {
    return !this.contentDOM.contains(mutation.target);
  }

  stopEvent(event: Event): boolean {
    return event.target instanceof globalThis.Node && this.host.contains(event.target);
  }

  destroy(): void {
    this.release?.();
    this.release = undefined;
    this.views.delete(this);
  }

  private draw(): void {
    const beat = this.node.attrs.beat as number;
    const params = attributeLines(this.node.textContent).map((line) => ({ key: line.key, value: line.rest }));
    const ctx = this.ctx();
    const ids = params.flatMap((param) => [...param.value.matchAll(/\[\[([^\]|]+)/g)].map((match) => match[1]!));
    const signature = JSON.stringify([
      beat,
      params,
      ctx.findings.filter((finding) => 'beat' in finding.at && finding.at.beat === beat),
      ids.map((id) => ctx.resolve(id) ?? null),
    ]);
    if (signature === this.seen) return;
    this.seen = signature;
    this.release?.();
    const component = mount(BeatRow, {
      target: this.host,
      props: { beat, params, ctx, oncommit: (key: BeatKey, value: string) => this.commit(key, value) },
    });
    this.release = () => void unmount(component);
  }

  private commit(key: BeatKey, value: string): void {
    const pos = this.getPos();
    if (pos !== undefined) setBeatParamAt(this.view, pos, this.node, key, value);
  }
}
