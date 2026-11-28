// A layout preset is data (P11 §5.6): a tree of rows and columns of windows with flex sizes.
// `Dock` draws any such tree. Splits are not resizable in P11.

import type { Component } from 'svelte';

/** A window's active tab when state outside the window holds it (the Codex tree tab). */
export interface TabSelection {
  /** The index of the active tab. */
  readonly index: number;
  /** Makes the tab at `index` active. */
  select(index: number): void;
}

/** A count beside a window's title (`tu-badge`), in the colour of its signal. */
export interface TabBadge {
  count: number;
  signal: 'reject' | 'warn';
}

/**
 * A tab title with parts: a record tab has a `kind` (its glyph before the id) and is a tab even
 * when it is the only one; a title may carry a badge.
 */
export interface RecordTabTitle {
  /** The title, a record's id for a record tab. */
  title: string;
  /** The record's kind, for its glyph; present on record tabs only. */
  kind?: string | null;
  /** A count beside the title. */
  badge?: TabBadge;
}

/** A tab's title: plain text, or a title with parts. */
export type TabTitle = string | RecordTabTitle;

/** The text of the tab title `tab`. */
export function tabText(tab: TabTitle): string {
  return typeof tab === 'string' ? tab : tab.title;
}

/**
 * A window: its id, its tab titles and the component that draws its body. Without `selected`,
 * the window keeps its active tab itself. A window whose tabs change (the record window) gives
 * `tabs` as a getter over its state.
 */
export interface WindowSpec<P extends object = object> {
  id: string;
  readonly tabs: readonly TabTitle[];
  component: Component<P>;
  props?: P;
  selected?: TabSelection;
}

/** A row or a column of windows and splits, each with its CSS `flex`. */
export interface Split {
  dir: 'row' | 'column';
  items: Array<{ flex: string; child: Split | WindowSpec }>;
}

/** The dock tree of one rail entry. */
export interface Preset {
  id: string;
  windows: Split;
}

/** Whether `child` is a split rather than a window. */
export function isSplit(child: Split | WindowSpec): child is Split {
  return 'dir' in child;
}

/**
 * A window whose component takes the props `P`. The one place a window's own props type is
 * widened, so a split can hold windows of any component.
 */
export function windowOf<P extends object>(spec: WindowSpec<P>): WindowSpec {
  return spec as unknown as WindowSpec;
}

/** A row of `items`, each a `[flex, child]` pair. */
export function row(...items: Array<[string, Split | WindowSpec]>): Split {
  return { dir: 'row', items: items.map(([flex, child]) => ({ flex, child })) };
}

/** A column of `items`, each a `[flex, child]` pair. */
export function column(...items: Array<[string, Split | WindowSpec]>): Split {
  return { dir: 'column', items: items.map(([flex, child]) => ({ flex, child })) };
}
