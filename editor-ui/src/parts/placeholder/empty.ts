// The windows of the layouts whose phase is not built yet (P11 §5.9): each one empty, saying what
// will appear there and which phase brings it.

import { windowOf, type WindowSpec } from '../layout';
import EmptyWindow from './EmptyWindow.svelte';

/** What an empty window shows. */
export interface Empty {
  /** The glyph in the dashed frame. */
  mark: string;
  /** What will appear here, and what to do. */
  text: string;
  /** The phase that brings it; none for a window this build fills itself. */
  phase?: string;
}

/** An empty window with the id `id` and the tab titles `tabs`. */
export function empty(id: string, tabs: string[], body: Empty): WindowSpec {
  return windowOf({ id, tabs, component: EmptyWindow, props: body });
}
