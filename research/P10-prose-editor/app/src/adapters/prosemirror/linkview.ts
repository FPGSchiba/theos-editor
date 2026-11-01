import { mount, unmount } from 'svelte';
import type { Node } from 'prosemirror-model';
import type { NodeView } from 'prosemirror-view';
import LinkChip from '../../components/LinkChip.svelte';
import type { EditorContext } from '../../harness/types.ts';

/**
 * The node view of a wikilink: a wrapper span that mounts the shared `LinkChip` (mounted in the
 * constructor, unmounted in `destroy`). The chip is an atom, so ProseMirror never looks inside it
 * and every change the component makes to its own DOM is ignored.
 */
export class LinkView implements NodeView {
  readonly dom = document.createElement('span');
  private component: ReturnType<typeof mount> | undefined;
  /** What the index said about the id when the chip was drawn; a change redraws the chip. */
  private seen: string;

  constructor(
    private node: Node,
    private readonly ctx: () => EditorContext,
    private readonly views: Set<LinkView>,
  ) {
    this.dom.className = 'pm-link';
    this.dom.contentEditable = 'false';
    this.dom.spellcheck = false;
    this.seen = this.snapshot();
    this.draw();
    views.add(this);
  }

  /** Same link: keep the DOM. A link with other attributes is a new chip. */
  update(node: Node): boolean {
    if (!node.sameMarkup(this.node)) return false;
    this.node = node;
    return true;
  }

  /** The index or the findings changed: draw the chip again, only when what it shows changed. */
  refresh(): void {
    const next = this.snapshot();
    if (next === this.seen) return;
    this.seen = next;
    this.undraw();
    this.draw();
  }

  ignoreMutation(): boolean {
    return true;
  }

  stopEvent(): boolean {
    return false;
  }

  destroy(): void {
    this.undraw();
    this.views.delete(this);
  }

  private snapshot(): string {
    return JSON.stringify(this.ctx().resolve(this.node.attrs.id as string) ?? null);
  }

  private draw(): void {
    this.component = mount(LinkChip, {
      target: this.dom,
      props: { id: this.node.attrs.id as string, shown: (this.node.attrs.shown as string | null) ?? undefined, ctx: this.ctx() },
    });
  }

  private undraw(): void {
    if (this.component !== undefined) void unmount(this.component);
    this.component = undefined;
    this.dom.textContent = '';
  }
}
