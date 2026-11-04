import { mount, unmount } from 'svelte';
import type { Node } from 'prosemirror-model';
import { TextSelection } from 'prosemirror-state';
import type { EditorView, NodeView, ViewMutationRecord } from 'prosemirror-view';
import NoteHead from '../../components/NoteHead.svelte';
import VoiceHead from '../../components/VoiceHead.svelte';
import type { EditorContext } from '../../harness/types.ts';
import { noteParts, SEPARATOR } from './notestate.ts';

/**
 * The node view of a callout: a wrapper (`tu-voice` for a voice block, `hx-callout` for the others)
 * with the data attributes of the DOM contract, a head drawn by the shared `VoiceHead` or
 * `NoteHead` (mounted, redrawn when what it shows changed, unmounted in `destroy`), and the
 * content: the raw title line (hidden until the caret is in it), the text, the caption.
 */

const LINK = /^\[\[([^\]|]+)(?:\|[^\]]*)?\]\]$/;
const TIER = /^T\d+$/;

const linkId = (segment: string | undefined): string | undefined => LINK.exec(segment ?? '')?.[1]?.trim();

type Draw = (host: HTMLElement, ctx: EditorContext) => () => void;
interface Head {
  /** Everything the head shows, as one string: an equal head is not drawn again. */
  signature: string;
  draw: Draw;
}

function voiceHead(title: string, ctx: EditorContext): Head {
  const segments = title.split(SEPARATOR).map((segment) => segment.trim());
  const first = segments[0] ?? '';
  const speaker = linkId(first) ?? first;
  const beliefs = segments.slice(1).flatMap((segment) => linkId(segment) ?? []);
  const tier = segments.slice(1).find((segment) => TIER.test(segment));
  const props = { speaker, beliefs, ...(tier === undefined ? {} : { tier }) };
  return {
    signature: JSON.stringify(['voice', props, ctx.resolve(speaker)?.title]),
    draw: (host, context) => {
      const component = mount(VoiceHead, { target: host, props: { ...props, ctx: context } });
      return () => void unmount(component);
    },
  };
}

function noteHead(title: string, ctx: EditorContext): Head {
  const parts = noteParts(title);
  const props = { note: parts.note, state: parts.state, ...(parts.version === undefined ? {} : { version: linkId(parts.version) ?? parts.version }), ...(parts.resolvedBy === undefined ? {} : { resolvedBy: parts.resolvedBy }) };
  const seen = [props.version, props.resolvedBy].map((id) => (id === undefined ? undefined : ctx.resolve(id)));
  return {
    signature: JSON.stringify(['note', props, seen.map((info) => [info?.kind, info?.title])]),
    draw: (host, context) => {
      const component = mount(NoteHead, { target: host, props: { ...props, ctx: context } });
      return () => void unmount(component);
    },
  };
}

/** A small label for a design callout, which has no head component. */
const designHead: Head = {
  signature: 'design',
  draw: (host) => {
    host.textContent = 'design';
    return () => {
      host.textContent = '';
    };
  },
};

export class CalloutView implements NodeView {
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
    private readonly views: Set<CalloutView>,
  ) {
    const type = node.attrs.type as string;
    this.dom.className = type === 'voice' ? 'tu-voice' : 'hx-callout';
    this.dom.setAttribute('data-callout', type);
    this.host.className = 'pm-callout-head';
    this.host.contentEditable = 'false';
    this.host.spellcheck = false;
    if (type === 'design') this.host.classList.add('pm-callout-label');
    this.contentDOM.className = type === 'voice' ? 'tu-voice__body' : 'pm-callout-body';
    this.dom.append(this.host, this.contentDOM);
    this.host.addEventListener('mousedown', (event) => {
      // A click on the head puts the caret on the title line, which then shows raw.
      event.preventDefault();
      this.toTitle();
    });
    this.attributes();
    this.draw();
    views.add(this);
  }

  /** Same kind of callout: keep the DOM, redraw the head when its title changed. */
  update(node: Node): boolean {
    if (node.type !== this.node.type || node.attrs.type !== this.node.attrs.type) return false;
    this.node = node;
    this.attributes();
    this.draw();
    return true;
  }

  /** The index changed: redraw the head when what it shows changed. */
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

  private title(): string {
    return this.node.firstChild?.textContent ?? '';
  }

  private attributes(): void {
    if (this.node.attrs.type !== 'note') return;
    const { note, state } = noteParts(this.title());
    this.dom.setAttribute('data-note', note);
    this.dom.setAttribute('data-note-state', state);
  }

  private head(): Head {
    switch (this.node.attrs.type) {
      case 'voice':
        return voiceHead(this.title(), this.ctx());
      case 'note':
        return noteHead(this.title(), this.ctx());
      default:
        return designHead;
    }
  }

  private draw(): void {
    const head = this.head();
    if (head.signature === this.seen) return;
    this.seen = head.signature;
    this.release?.();
    this.release = head.draw(this.host, this.ctx());
  }

  private toTitle(): void {
    const pos = this.getPos();
    if (pos === undefined) return;
    const end = pos + 2 + (this.node.firstChild?.content.size ?? 0);
    this.view.dispatch(this.view.state.tr.setSelection(TextSelection.create(this.view.state.doc, end)));
    this.view.focus();
  }
}
