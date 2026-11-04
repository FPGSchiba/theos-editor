// A content key for the chips of one line. A viewport editor recycles its DOM elements and may
// re-measure line heights between two scroll steps, so neither an element nor a pixel position
// says whether two reads saw the same chip. The line's own text does.

/** A line as a list of parts: the text between chips, and the chips as `{ chip: <link id> }`. */
export type LinePart = string | { chip: string };

/** How many characters of the line's text before and after a chip go into its key. */
export const CONTEXT = 20;

/**
 * One key per chip of the line, in order: the link id, up to 20 characters of the line before and
 * after the chip (other chips read as their link id in double brackets), and the occurrence index
 * of that key on the line, so two chips with the same id and the same surroundings stay two.
 * It runs in the page too, so it is self-contained.
 */
export function chipKeys(parts: LinePart[]): string[] {
  const context = 20;
  const flat = parts.map((part) => (typeof part === 'string' ? part : `[[${part.chip}]]`));
  const seen = new Map<string, number>();
  const keys: string[] = [];
  parts.forEach((part, index) => {
    if (typeof part === 'string') return;
    const before = flat.slice(0, index).join('').slice(-context);
    const after = flat.slice(index + 1).join('').slice(0, context);
    const base = JSON.stringify([part.chip, before, after]);
    const occurrence = seen.get(base) ?? 0;
    seen.set(base, occurrence + 1);
    keys.push(`${base}#${occurrence}`);
  });
  return keys;
}

/** The items with the first of each key, in order. */
export function uniqueByKey<T>(items: T[], key: (item: T) => string): T[] {
  const found = new Map<string, T>();
  for (const item of items) if (!found.has(key(item))) found.set(key(item), item);
  return [...found.values()];
}
