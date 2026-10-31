import { syntaxTree } from '@codemirror/language';
import type { Range } from '@codemirror/state';
import { Decoration, EditorView, ViewPlugin, WidgetType, type DecorationSet, type ViewUpdate } from '@codemirror/view';
import { mount, unmount } from 'svelte';
import LinkChip from '../../components/LinkChip.svelte';
import type { EditorContext, LinkInfo } from '../../harness/types.ts';
import { contextOf, setContext } from './context.ts';
import { bodyStart } from './frontmatter.ts';
import { typingField } from './typing.ts';

/**
 * Wikilinks as chips: `[[id]]` and `[[id|shown text]]` are replaced by the shared `LinkChip`,
 * an atomic unit for the caret and for Backspace and Delete. The bytes are never changed, only
 * what is drawn. A link that is being typed (see `typing.ts`) and a link in code stay raw.
 */

/** A link on one line: the id, then optionally `|` and the text to show. */
const LINK = /\[\[([^[\]|\n]+)(?:\|([^\]\n]+))?\]\]/g;
const CODE = /^(?:InlineCode|FencedCode|CodeBlock)$/;

/** Svelte components mounted into the chips on screen, by the element they sit in. */
const mounted = new WeakMap<HTMLElement, () => void>();

class ChipWidget extends WidgetType {
  constructor(
    readonly id: string,
    readonly shown: string | undefined,
    readonly info: LinkInfo | undefined,
    readonly ctx: EditorContext,
  ) {
    super();
  }

  /** What the chip draws is its id, its alias and the record's kind and title: equal draws are not rebuilt. */
  eq(other: ChipWidget): boolean {
    return other.id === this.id && other.shown === this.shown
      && other.info?.kind === this.info?.kind && other.info?.title === this.info?.title;
  }

  toDOM(): HTMLElement {
    const host = document.createElement('span');
    host.className = 'cm-link';
    const chip = mount(LinkChip, { target: host, props: { id: this.id, shown: this.shown, ctx: this.ctx } });
    mounted.set(host, () => void unmount(chip));
    return host;
  }

  /** CodeMirror may hand the element of an equal widget on to a newer one, so the component is found by element. */
  destroy(dom: HTMLElement): void {
    mounted.get(dom)?.();
    mounted.delete(dom);
  }

  /** The editor handles clicks on a chip, so the caret lands before or behind it. */
  ignoreEvent(): boolean {
    return false;
  }
}

type Tree = ReturnType<typeof syntaxTree>;

/** Whether a position is in code, where `[[` is not a link. */
export function inCode(tree: Tree, pos: number): boolean {
  for (let node: ReturnType<Tree['resolveInner']> | null = tree.resolveInner(pos, 1); node !== null; node = node.parent) {
    if (CODE.test(node.name)) return true;
  }
  return false;
}

function build(view: EditorView): DecorationSet {
  const { state } = view;
  const ctx = contextOf(state);
  const typing = state.field(typingField);
  const body = state.field(bodyStart);
  const tree = syntaxTree(state);
  const found: Array<Range<Decoration>> = [];
  for (const range of view.visibleRanges) {
    if (range.to < body) continue;
    const start = state.doc.lineAt(Math.max(range.from, body)).from;
    const text = state.doc.sliceString(start, state.doc.lineAt(range.to).to);
    for (const match of text.matchAll(LINK)) {
      const from = start + match.index;
      const to = from + match[0].length;
      if (typing !== undefined && from < typing.to && to > typing.from) continue;
      if (inCode(tree, from)) continue;
      const id = match[1]!.trim();
      found.push(Decoration.replace({ widget: new ChipWidget(id, match[2], ctx.resolve(id), ctx) }).range(from, to));
    }
  }
  return Decoration.set(found, true);
}

export const linkChips = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
      this.decorations = build(view);
    }

    update(update: ViewUpdate): void {
      if (
        update.docChanged || update.viewportChanged
        || syntaxTree(update.startState) !== syntaxTree(update.state)
        || update.startState.field(typingField, false) !== update.state.field(typingField)
        || update.transactions.some((tr) => tr.effects.some((effect) => effect.is(setContext)))
      ) {
        this.decorations = build(update.view);
      }
    }
  },
  {
    decorations: (plugin) => plugin.decorations,
    provide: (plugin) => EditorView.atomicRanges.of((view) => view.plugin(plugin)?.decorations ?? Decoration.none),
  },
);
