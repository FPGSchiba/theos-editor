import { Plugin, TextSelection } from 'prosemirror-state';
import type { EditorView } from 'prosemirror-view';
import { mount, unmount } from 'svelte';
import LinkPopover from '../../components/LinkPopover.svelte';
import type { EditorContext, LinkInfo, LinkPlace } from '../../harness/types.ts';
import { dismissTyping, placeOf, typingKey, type Typing } from './typing.ts';

/**
 * The `[[` popover: the shared `LinkPopover`, mounted while a link is being typed, in its own
 * element on the page body (below the caret, above it when there is no room), so it never
 * touches ProseMirror's DOM. The component owns the arrow keys, Enter and Escape while it is
 * mounted; the editor keeps the focus. It is unmounted when typing ends, on Escape (`onclose`),
 * on blur, and with the editor.
 */

type Popover = ReturnType<typeof LinkPopover>;

class PopoverView {
  private host: HTMLElement | undefined;
  private popover: Popover | undefined;
  /** The context the options were searched with: another one means the index changed. */
  private searched: EditorContext | undefined;
  private last: Typing | undefined;

  constructor(private readonly view: EditorView, private readonly ctx: () => EditorContext) {
    // The popover is fixed to the page, so it follows the caret when anything scrolls.
    window.addEventListener('scroll', this.follow, true);
    this.update();
  }

  update(): void {
    const typing = typingKey.getState(this.view.state);
    if (typing === undefined || typing.dismissed) {
      this.close();
      return;
    }
    const ctx = this.ctx();
    const before = this.last;
    const same = before?.from === typing.from && before.to === typing.to && before.query === typing.query;
    if (same && this.popover !== undefined && this.searched === ctx) {
      this.place(typing.to);
      return;
    }
    const place = placeOf(this.view.state, typing.from);
    const options = ctx.search(typing.query, place);
    this.searched = ctx;
    this.last = typing;
    if (this.popover === undefined || this.host === undefined) this.open(options, typing.query, place);
    else this.popover.update({ options, query: typing.query });
    this.place(typing.to);
  }

  destroy(): void {
    window.removeEventListener('scroll', this.follow, true);
    this.close();
  }

  private open(options: LinkInfo[], query: string, place: LinkPlace): void {
    this.host = document.createElement('div');
    this.host.className = 'pm-link-popover';
    Object.assign(this.host.style, { position: 'fixed', zIndex: '1000', visibility: 'hidden' });
    document.body.append(this.host);
    this.popover = mount(LinkPopover, {
      target: this.host,
      props: {
        options,
        query,
        place,
        onpick: (id: string) => this.pick(id),
        onclose: () => this.view.dispatch(dismissTyping(this.view.state)),
      },
    });
  }

  private readonly follow = (): void => {
    const typing = typingKey.getState(this.view.state);
    if (typing !== undefined) this.place(typing.to);
  };

  /** Replace the typed `[[query` with the picked record's link node; the caret goes behind it. */
  private pick(id: string): void {
    const { state } = this.view;
    const typing = typingKey.getState(state);
    if (typing === undefined) return;
    const tr = state.tr.replaceWith(typing.from, typing.to, state.schema.nodes.wikilink!.create({ id }));
    this.view.dispatch(tr.setSelection(TextSelection.create(tr.doc, typing.from + 1)).scrollIntoView());
  }

  private place(at: number): void {
    const host = this.host;
    if (host === undefined) return;
    const caret = this.view.coordsAtPos(at);
    const height = host.offsetHeight;
    const width = host.offsetWidth;
    const below = caret.bottom + 4;
    const fits = below + height <= window.innerHeight || caret.top - 4 - height < 0;
    host.style.top = `${fits ? below : caret.top - 4 - height}px`;
    host.style.left = `${Math.max(0, Math.min(caret.left, window.innerWidth - width - 8))}px`;
    host.style.visibility = 'visible';
  }

  private close(): void {
    if (this.popover !== undefined) void unmount(this.popover);
    this.host?.remove();
    this.popover = undefined;
    this.host = undefined;
    this.searched = undefined;
    this.last = undefined;
  }
}

export const popoverPlugin = (ctx: () => EditorContext): Plugin =>
  new Plugin({
    view: (view) => new PopoverView(view, ctx),
    props: {
      handleDOMEvents: {
        // Without the focus the popover would keep Enter and the arrows for itself.
        blur(view) {
          if (typingKey.getState(view.state)?.dismissed === false) view.dispatch(dismissTyping(view.state));
          return false;
        },
      },
    },
  });
