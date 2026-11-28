// The toolbar's find field, which the command "Find record" focuses (P11 §5.11). The toolbar
// registers its input here; Task 13's find component keeps doing so.

let field: HTMLInputElement | null = null;

/** Registers the find field's input, or `null` when it goes away. */
export function registerFindField(input: HTMLInputElement | null): void {
  field = input;
}

/** Focuses the find field and selects its text; nothing when no field is registered. */
export function focusFindField(): void {
  field?.focus();
  field?.select();
}
