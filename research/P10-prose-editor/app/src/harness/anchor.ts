import type { Anchor } from './types.ts';

/**
 * Resolve an anchor to an offset into `text` (UTF-16 code units): the start of the nth
 * non-overlapping occurrence of `find` (1-based, default 1) plus `offset` (default 0). Throws
 * when `find` is empty, the occurrence does not exist, or the result lies outside the text.
 */
export function resolveAnchor(text: string, anchor: Anchor): number {
  const { find, occurrence = 1, offset = 0 } = anchor;
  if (find === '') throw new Error('anchor: find must not be empty');
  if (!Number.isInteger(occurrence) || occurrence < 1) {
    throw new Error(`anchor: occurrence must be an integer of at least 1, got ${occurrence}`);
  }
  let start = -1;
  for (let seen = 0; seen < occurrence; seen++) {
    start = text.indexOf(find, start === -1 ? 0 : start + find.length);
    if (start === -1) {
      throw new Error(
        `anchor: ${JSON.stringify(find)} has no occurrence ${occurrence} (found ${seen} in ${text.length} characters)`,
      );
    }
  }
  const resolved = start + offset;
  if (resolved < 0 || resolved > text.length) {
    throw new Error(
      `anchor: ${JSON.stringify(find)} occurrence ${occurrence} plus offset ${offset} is outside the text`,
    );
  }
  return resolved;
}
