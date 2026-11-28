// The status bar's notice (P11 §5.9): what the editor did on its own and the user should know,
// such as a tab closed because its record was deleted outside the editor.

/** The notice, or `null`. */
export const notice = $state<{ text: string | null }>({ text: null });

/** Shows `text` in the status bar, replacing the notice before it. */
export function say(text: string): void {
  notice.text = text;
}

/** Removes the notice. */
export function hush(): void {
  notice.text = null;
}
