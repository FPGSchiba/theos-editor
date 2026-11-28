// Whether the command palette is open (P11 §5.11). The command "Command palette" toggles it. The
// element that had the focus before it opened gets the focus back when it closes.

import { tick } from 'svelte';

/** The palette's state. */
export const palette = $state<{ open: boolean }>({ open: false });

/** The element that had the focus when the palette opened. */
let returnTo: HTMLElement | null = null;

/** Opens the palette, remembering where the focus was. */
export function openPalette(): void {
  if (palette.open) {
    return;
  }
  returnTo = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  palette.open = true;
}

/** Closes the palette and gives the focus back to where it was. */
export async function closePalette(): Promise<void> {
  if (!palette.open) {
    return;
  }
  palette.open = false;
  await tick();
  returnTo?.focus();
  returnTo = null;
}

/** Opens the palette, or closes it when it is open. */
export function togglePalette(): void {
  if (palette.open) {
    void closePalette();
  } else {
    openPalette();
  }
}
