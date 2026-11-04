import { EditorView, ViewPlugin, type ViewUpdate } from '@codemirror/view';
import { mount, unmount } from 'svelte';
import LinkPopover from '../../components/LinkPopover.svelte';
import { contextOf, setContext } from './context.ts';
import { completeLink, dismissTyping, placeOf, typingField } from './typing.ts';

/**
 * The `[[` popover: the shared `LinkPopover`, mounted while a link is being typed. It sits in its
 * own element on the page body, below the caret (above it when there is no room), so it never
 * touches CodeMirror's DOM. The component owns the arrow keys, Enter and Escape while it is
 * mounted; the editor keeps the focus. It is unmounted when typing ends, on Escape, or on blur.
 */

type Popover = ReturnType<typeof LinkPopover>;

class PopoverPlugin {
  private host: HTMLElement | undefined;
  private popover: Popover | undefined;

  constructor(private readonly view: EditorView) {
    // The popover is fixed to the page, so it follows the caret when anything scrolls.
    window.addEventListener('scroll', this.follow, true);
  }

  update(update: ViewUpdate): void {
    const typing = update.state.field(typingField);
    if (typing === undefined || typing.dismissed) {
      this.close();
      return;
    }
    // An update that leaves the typed text and the index alone keeps the popover as it is (its active row too).
    const before = update.startState.field(typingField, false);
    const same = before?.from === typing.from && before.to === typing.to && before.query === typing.query && !before.dismissed;
    if (same && this.popover !== undefined && !update.transactions.some((tr) => tr.effects.some((effect) => effect.is(setContext)))) {
      this.place(typing.to);
      return;
    }
    const place = placeOf(update.state, typing.from);
    const options = contextOf(update.state).search(typing.query, place);
    if (this.popover === undefined || this.host === undefined) {
      this.host = document.createElement('div');
      this.host.className = 'cm-link-popover';
      Object.assign(this.host.style, { position: 'fixed', zIndex: '1000', visibility: 'hidden' });
      document.body.append(this.host);
      this.popover = mount(LinkPopover, {
        target: this.host,
        props: {
          options,
          query: typing.query,
          place,
          onpick: (id: string) => this.pick(id),
          onclose: () => this.view.dispatch({ effects: dismissTyping.of(null) }),
        },
      });
    } else {
      this.popover.update({ options, query: typing.query });
    }
    this.place(typing.to);
  }

  destroy(): void {
    window.removeEventListener('scroll', this.follow, true);
    this.close();
  }

  private readonly follow = (): void => {
    const typing = this.view.state.field(typingField);
    if (typing !== undefined) this.place(typing.to);
  };

  /** Replace the typed `[[query` with the picked record's link. */
  private pick(id: string): void {
    const typing = this.view.state.field(typingField);
    if (typing !== undefined) completeLink(this.view, typing.from, typing.to, id);
  }

  /** Put the popover below the caret, or above it when the page ends too soon; layout is read in the measure phase. */
  private place(at: number): void {
    const host = this.host;
    if (host === undefined) return;
    this.view.requestMeasure({
      key: this,
      read: (view) => ({ caret: view.coordsAtPos(at), height: host.offsetHeight, width: host.offsetWidth }),
      write: ({ caret, height, width }) => {
        if (caret === null) return;
        const below = caret.bottom + 4;
        const fits = below + height <= window.innerHeight || caret.top - 4 - height < 0;
        host.style.top = `${fits ? below : caret.top - 4 - height}px`;
        host.style.left = `${Math.max(0, Math.min(caret.left, window.innerWidth - width - 8))}px`;
        host.style.visibility = 'visible';
      },
    });
  }

  private close(): void {
    if (this.popover !== undefined) void unmount(this.popover);
    this.host?.remove();
    this.popover = undefined;
    this.host = undefined;
  }
}

export const linkPopover = [
  ViewPlugin.fromClass(PopoverPlugin),
  // Without the focus the popover would keep Enter and the arrows for itself.
  EditorView.domEventHandlers({
    blur(_event, view) {
      if (view.state.field(typingField)?.dismissed === false) view.dispatch({ effects: dismissTyping.of(null) });
      return false;
    },
  }),
];
