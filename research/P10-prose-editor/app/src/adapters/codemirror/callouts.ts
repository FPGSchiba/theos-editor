import { syntaxTree } from '@codemirror/language';
import type { EditorState, Line, Range } from '@codemirror/state';
import { Decoration, ViewPlugin, WidgetType, type DecorationSet, type EditorView, type ViewUpdate } from '@codemirror/view';
import { mount, unmount } from 'svelte';
import NoteHead from '../../components/NoteHead.svelte';
import VoiceHead from '../../components/VoiceHead.svelte';
import type { EditorContext } from '../../harness/types.ts';
import { contextOf, setContext } from './context.ts';
import { markDevWords } from './devwords.ts';
import { inCode } from './links.ts';
import { readState, SEPARATOR } from './notestate.ts';
import { touchedLines } from './rawlines.ts';

/**
 * Callouts as runs of lines: CodeMirror decorates lines, so each line of a `> [!voice]`, `> [!design]`
 * or `> [!note]` block carries the block's attributes (the DOM contract allows a run). The title line
 * is drawn by the shared head component (the design callout gets a small label), the `> ` markers are
 * hidden, and every line a selection touches shows its raw Markdown instead: whatever a delete or
 * a selection can take is on screen. `Design note:` lines get the `tu-note` class. The bytes never change.
 */

type Kind = 'voice' | 'design' | 'note';
interface Callout {
  kind: Kind;
  /** The title line without `> [!kind]`. */
  title: string;
  lines: Line[];
}

const TITLE = /^> ?\[!(voice|design|note)\](?: (.*))?$/i;
const MARKER = /^> ?/;
/** The last line of a voice block that starts with an em dash is its caption; `--` is voice text. */
const CAPTION = /^> ?— /;
const DESIGN_NOTE = /^(?:> ?)?Design note:/;
const LINK = /^\[\[([^\]|]+)(?:\|[^\]]*)?\]\]$/;
const TIER = /^T\d+$/;

const hidden = Decoration.replace({});

/** The callouts whose blocks reach into `from`..`to`, found through the Markdown tree (lazy lines belong to the block). */
function calloutsIn(state: EditorState, from: number, to: number): Callout[] {
  const { doc } = state;
  const found: Callout[] = [];
  syntaxTree(state).iterate({
    from,
    to,
    enter(node) {
      if (node.name !== 'Blockquote') return undefined;
      const first = doc.lineAt(node.from);
      const match = node.from === first.from ? TITLE.exec(first.text) : null;
      if (match !== null) {
        // A block that ends at the start of a line ends on the line before it.
        const end = node.to > node.from && doc.lineAt(node.to).from === node.to ? node.to - 1 : node.to;
        const lines: Line[] = [];
        for (let number = first.number; number <= doc.lineAt(end).number; number++) lines.push(doc.line(number));
        found.push({ kind: match[1]!.toLowerCase() as Kind, title: (match[2] ?? '').trim(), lines });
      }
      return false;
    },
  });
  return found;
}

const linkId = (segment: string | undefined): string | undefined => LINK.exec(segment ?? '')?.[1]?.trim();

/** What a head shows, as one string of everything it draws: equal heads are not rebuilt. */
interface Head {
  signature: string;
  make: (host: HTMLElement, ctx: EditorContext) => () => void;
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
    make: (host, context) => {
      const component = mount(VoiceHead, { target: host, props: { ...props, ctx: context } });
      return () => void unmount(component);
    },
  };
}

/** The note id, its state and (when the title names them) the version and the resolving version. */
function noteParts(title: string): { note: string; state: 'open' | 'dropped' | 'resolved'; version?: string; resolvedBy?: string } {
  const segments = title.split(SEPARATOR).map((segment) => segment.trim());
  const last = segments.length > 1 ? segments[segments.length - 1] : undefined;
  const state = readState(last);
  const middle = segments.slice(1, state === undefined ? undefined : -1);
  const version = middle.flatMap((segment) => linkId(segment) ?? [])[0];
  return {
    note: segments[0] ?? '',
    state: state?.state ?? 'open',
    ...(version === undefined ? {} : { version }),
    ...(state?.resolvedBy === undefined || state.resolvedBy === '' ? {} : { resolvedBy: state.resolvedBy }),
  };
}

function noteHead(parts: ReturnType<typeof noteParts>, ctx: EditorContext): Head {
  const seen = [parts.version, parts.resolvedBy].map((id) => (id === undefined ? undefined : ctx.resolve(id)));
  return {
    signature: JSON.stringify(['note', parts, seen.map((info) => [info?.kind, info?.title])]),
    make: (host, context) => {
      const component = mount(NoteHead, { target: host, props: { ...parts, ctx: context } });
      return () => void unmount(component);
    },
  };
}

const unmounts = new WeakMap<HTMLElement, () => void>();

/** A replacement for a title line: the shared head component, or a small label for a design callout. */
class HeadWidget extends WidgetType {
  constructor(readonly head: Head | undefined, readonly ctx: EditorContext) {
    super();
  }

  eq(other: HeadWidget): boolean {
    return other.head?.signature === this.head?.signature;
  }

  toDOM(): HTMLElement {
    const host = document.createElement('div');
    host.className = 'cm-callout-head';
    if (this.head === undefined) {
      host.classList.add('cm-callout-label');
      host.spellcheck = false;
      host.textContent = 'design';
    } else {
      unmounts.set(host, this.head.make(host, this.ctx));
    }
    return host;
  }

  /** CodeMirror hands the element of an equal widget on to a newer one, so the component is found by element. */
  destroy(dom: HTMLElement): void {
    unmounts.get(dom)?.();
    unmounts.delete(dom);
  }

  /** A click on the head puts the caret on the title line, which then shows raw. */
  ignoreEvent(): boolean {
    return false;
  }
}

const lineAttributes = (callout: Callout, index: number, raw: boolean, caption: boolean): Record<string, string> => {
  const classes = ['cm-callout', `cm-callout-${callout.kind}`];
  if (callout.kind === 'voice') classes.push('tu-voice');
  if (caption) classes.push('tu-voice__by');
  if (index === 0) classes.push('cm-callout-first');
  if (index === callout.lines.length - 1) classes.push('cm-callout-last');
  if (raw) classes.push('cm-callout-raw');
  const attributes: Record<string, string> = { class: classes.join(' '), 'data-callout': callout.kind };
  if (callout.kind === 'note') {
    const { note, state } = noteParts(callout.title);
    attributes['data-note'] = note;
    attributes['data-note-state'] = state;
  }
  return attributes;
};

function decorate(found: Array<Range<Decoration>>, callout: Callout, ctx: EditorContext, isRaw: (from: number, to: number) => boolean): void {
  const last = callout.lines.length - 1;
  callout.lines.forEach((line, index) => {
    const raw = isRaw(line.from, line.to);
    const caption = callout.kind === 'voice' && index === last && index > 0 && CAPTION.test(line.text);
    found.push(Decoration.line({ attributes: lineAttributes(callout, index, raw, caption) }).range(line.from));
    if (index === 0) {
      if (!raw) {
        const head = callout.kind === 'voice' ? voiceHead(callout.title, ctx)
          : callout.kind === 'note' ? noteHead(noteParts(callout.title), ctx) : undefined;
        found.push(Decoration.replace({ widget: new HeadWidget(head, ctx) }).range(line.from, line.to));
      }
      return;
    }
    const marker = (caption ? CAPTION : MARKER).exec(line.text)?.[0].length ?? 0;
    if (marker > 0 && !raw) found.push(hidden.range(line.from, line.from + marker));
    if (callout.kind === 'voice' && !DESIGN_NOTE.test(line.text)) {
      markDevWords(found, line.text.slice(marker), line.from + marker, ctx.developerWords);
    }
  });
}

const note = Decoration.line({ class: 'tu-note' });

/** The lines of `from`..`to` that start with `Design note:` and are not code. */
function designNotes(found: Array<Range<Decoration>>, state: EditorState, from: number, to: number): void {
  const tree = syntaxTree(state);
  for (let at = from; at <= to;) {
    const line = state.doc.lineAt(at);
    if (line.text.startsWith('Design note:') && !inCode(tree, line.from)) found.push(note.range(line.from));
    at = line.to + 1;
  }
}

function build(view: EditorView): DecorationSet {
  const { state } = view;
  const ctx = contextOf(state);
  const isRaw = touchedLines(state);
  const found: Array<Range<Decoration>> = [];
  const done = new Set<number>();
  for (const { from, to } of view.visibleRanges) {
    for (const callout of calloutsIn(state, from, to)) {
      const start = callout.lines[0]!.from;
      if (done.has(start)) continue;
      done.add(start);
      decorate(found, callout, ctx, isRaw);
    }
    designNotes(found, state, from, to);
  }
  return Decoration.set(found, true);
}

export const callouts = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = build(view);
    }

    update(update: ViewUpdate): void {
      if (
        update.docChanged || update.selectionSet || update.viewportChanged
        || syntaxTree(update.startState) !== syntaxTree(update.state)
        || update.transactions.some((tr) => tr.effects.some((effect) => effect.is(setContext)))
      ) {
        this.decorations = build(update.view);
      }
    }
  },
  { decorations: (plugin) => plugin.decorations },
);
